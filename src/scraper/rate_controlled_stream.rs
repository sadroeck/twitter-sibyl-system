use futures::future::Future;
use futures::{Async, FlattenStream, Stream};
use log::{error, info};
use std::time::{Duration, Instant};
use tokio_timer::Delay;
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
            state: StreamAction::Continue,
        }
    }
}

#[derive(Debug)]
enum StreamAction {
    Continue,
    RestartAfter(Delay),
    Exit,
}

/// Derive an action based on twitter error codes
/// c.f.r. https://developer.twitter.com/en/docs/basics/response-codes
fn process_twitter_error(status_code: StatusCode) -> StreamAction {
    match status_code.as_u16() {
        410 => StreamAction::RestartAfter(Delay::new(Instant::now())),
        420 | 429 => StreamAction::RestartAfter(Delay::new(
            Instant::now().checked_add(Duration::from_secs(60)).unwrap(),
        )),
        304 => StreamAction::RestartAfter(Delay::new(
            Instant::now()
                .checked_add(Duration::from_secs(300))
                .unwrap(),
        )),
        400 | 401 | 403 | 404 | 406 | 422 => StreamAction::Exit,
        _ => StreamAction::Continue,
    }
}

impl Stream for RateLimitedStream {
    type Item = <TwitterStream as Stream>::Item;
    type Error = <TwitterStream as Stream>::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        match &mut self.state {
            StreamAction::Continue => match self.inner.poll() {
                Err(TwitterError::Http(status_code)) => {
                    self.state = process_twitter_error(status_code);
                    match &self.state {
                        StreamAction::RestartAfter(delay) => error!(
                            "[{topic}] Reached twitter API limit, restarting stream after delay: {delay:?}",
                            topic = &self.topic,
                            delay = delay.deadline()
                        ),
                        StreamAction::Continue => (),
                        StreamAction::Exit => (),
                    }
                    futures::task::current().notify();
                    Ok(Async::NotReady)
                }
                Err(other_err) => {
                    error!("Received API error {}", other_err);
                    Err(other_err)
                }
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Ok(Async::Ready(Some(content))) => Ok(Async::Ready(Some(content))),
                Ok(Async::Ready(None)) => {
                    error!("[{topic}] Stream has stoppped", topic = &self.topic);
                    Ok(Async::Ready(None))
                }
            },
            StreamAction::RestartAfter(ref mut delay) => match delay.poll() {
                Ok(Async::Ready(())) => {
                    futures::task::current().notify();
                    info!("[{topic}] Restart stream", topic = &self.topic);
                    self.state = StreamAction::Continue;
                    self.inner = create_stream(self.api_token.clone(), self.topic.as_str());
                    Ok(Async::NotReady)
                }
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Err(err) => {
                    error!(
                        "[{topic}] Could not wait for stream restart: {err}",
                        topic = &self.topic,
                        err = err
                    );
                    Err(TwitterError::TimedOut)
                }
            },
            StreamAction::Exit => {
                panic!("[{topic}] Cancel twitter topic stream", topic = &self.topic);
            }
        }
    }
}
