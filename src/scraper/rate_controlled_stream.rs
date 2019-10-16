use futures::future::Future;
use futures::{try_ready, Async, Stream};
use std::convert::From;
use std::time::Duration;
use twitter_stream::error::Error as TwitterError;
use twitter_stream::{types::StatusCode, Token, TwitterStream, TwitterStreamBuilder};

type ApiToken = Token<String, String>;

pub struct RateLimitedStream {
    inner: Box<
        dyn Stream<
                Item = Input<<TwitterStream as Stream>::Item>,
                Error = <TwitterStream as Stream>::Error,
            > + Send,
    >,
    topic: String,
    api_token: ApiToken,
}

impl RateLimitedStream {
    fn create_stream(
        api_token: ApiToken,
        topic: &str,
    ) -> Box<
        dyn Stream<
                Item = Input<<TwitterStream as Stream>::Item>,
                Error = <TwitterStream as Stream>::Error,
            > + Send,
    > {
        let stream = TwitterStreamBuilder::filter(api_token)
            .stall_warnings(true)
            .track(topic)
            .listen()
            .unwrap()
            .flatten_stream()
            .then(|result| match result {
                Ok(val) => Ok(Input::Content(val)),
                Err(TwitterError::Http(status_code)) => {
                    Ok(Input::Action(process_twitter_error(status_code)))
                }
                Err(other_err) => Err(other_err),
            });
        Box::new(stream)
    }

    pub fn from_topic(api_token: ApiToken, topic: String) -> Self {
        RateLimitedStream {
            inner: RateLimitedStream::create_stream(api_token.clone(), topic.as_str()),
            topic,
            api_token,
        }
    }
}

enum Input<T> {
    Action(StreamAction),
    Content(T),
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
        match try_ready!(self.inner.poll()) {
            Some(Input::Content(content)) => Ok(Async::Ready(Some(content))),
            Some(Input::Action(action)) => match action {
                StreamAction::Exit => panic!("Aborting the stream"),
                StreamAction::Halt(duration) => {
                    std::thread::sleep(duration);
                    Ok(Async::NotReady)
                }
                StreamAction::RestartNow => {
                    self.inner = RateLimitedStream::create_stream(
                        self.api_token.clone(),
                        self.topic.as_str(),
                    );
                    Ok(Async::NotReady)
                }
                StreamAction::RestartAfter(duration) => {
                    std::thread::sleep(duration);
                    Ok(Async::NotReady)
                }
                StreamAction::Continue => Ok(Async::NotReady),
            },
            None => return Ok(None.into()),
        }
    }
}
