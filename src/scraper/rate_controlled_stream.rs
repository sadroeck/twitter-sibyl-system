use futures::future::Future;
use futures::{Async, FlattenStream, Stream};
use log::info;
use std::time::{Duration, Instant};
use twitter_stream::error::Error as TwitterError;
use twitter_stream::{
    types::StatusCode, FutureTwitterStream, Token, TwitterStream, TwitterStreamBuilder,
};

type ApiToken = Token<String, String>;

pub struct RateLimitedStream {
    inner: FlattenStream<FutureTwitterStream>,
    topic: String,
    api_token: ApiToken,
    halted_since: Option<Instant>,
    state: StreamAction,
}

fn create_stream(api_token: ApiToken, topic: &str) -> FlattenStream<FutureTwitterStream> {
    TwitterStreamBuilder::filter(api_token)
        .stall_warnings(true)
        .track(topic)
        .listen()
        .unwrap()
        .flatten_stream()
}

impl RateLimitedStream {
    pub fn from_topic(api_token: ApiToken, topic: String) -> Self {
        RateLimitedStream {
            inner: create_stream(api_token.clone(), topic.as_str()),
            topic,
            api_token,
            halted_since: None,
            state: StreamAction::Continue,
        }
    }
}

enum StreamAction {
    Continue,
    RestartNow,
    RestartAfter(Duration),
    Halt(Duration),
    Exit,
}

/// Derive an action based on twitter error codes
/// c.f.r. https://developer.twitter.com/en/docs/basics/response-codes
fn process_twitter_error(status_code: StatusCode) -> StreamAction {
    match status_code.as_u16() {
        410 => StreamAction::RestartNow,
        420 | 429 => StreamAction::Halt(Duration::from_secs(20)),
        304 => StreamAction::RestartAfter(Duration::from_secs(300)),
        400 | 401 | 403 | 404 | 406 | 422 => StreamAction::Exit,
        _ => StreamAction::Continue,
    }
}

impl Stream for RateLimitedStream {
    type Item = <TwitterStream as Stream>::Item;
    type Error = <TwitterStream as Stream>::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        match &self.state {
            StreamAction::Continue => match self.inner.poll() {
                Err(TwitterError::Http(status_code)) => {
                    self.state = process_twitter_error(status_code);
                    Ok(Async::NotReady)
                }
                Err(other_err) => Err(other_err),
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Ok(Async::Ready(Some(content))) => Ok(Async::Ready(Some(content))),
                Ok(Async::Ready(None)) => {
                    self.state = StreamAction::Halt(Duration::from_secs(2));
                    Ok(Async::NotReady)
                }
            },
            StreamAction::RestartNow => {
                info!("Restarting topic stream immediately");
                self.inner = create_stream(self.api_token.clone(), self.topic.as_str());
                Ok(Async::NotReady)
            }
            StreamAction::RestartAfter(wait_time) => {
                let now = Instant::now();
                match self.halted_since.map(|instant| now.duration_since(instant)) {
                    None => {
                        info!("Restarting topic stream after {:?}", &wait_time);
                        self.halted_since = Some(now);
                        Ok(Async::NotReady)
                    }
                    Some(elapsed) => {
                        if elapsed > *wait_time {
                            self.inner = create_stream(self.api_token.clone(), self.topic.as_str());
                            self.state = StreamAction::Continue;
                        }
                        Ok(Async::NotReady)
                    }
                }
            }
            StreamAction::Halt(wait_time) => {
                let now = Instant::now();
                match self.halted_since.map(|instant| now.duration_since(instant)) {
                    None => {
                        info!("Halting topic stream for {:?}", &wait_time);
                        self.halted_since = Some(now);
                        Ok(Async::NotReady)
                    }
                    Some(elapsed) => {
                        if elapsed > *wait_time {
                            self.state = StreamAction::Continue;
                        }
                        Ok(Async::NotReady)
                    }
                }
            }
            StreamAction::Exit => {
                panic!("Cancel twitter topic stream");
            }
        }
    }
}
