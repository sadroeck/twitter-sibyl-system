use crate::config::ScraperConfig;
use crate::scraper::metrics::{Sample, TimeSeries};
use crate::tweet::Tweet;
use chrono::NaiveDateTime;
use futures::stream::Stream;
use log::{error, info, warn};
use std::sync::Arc;
use twitter_stream::Token;

pub mod metrics;
mod rate_controlled_stream;
mod sentiment;

const TWITTER_DATE_FORMAT: &'static str = "%a %b %d %H:%M:%S %z %Y";

pub struct Scraper {
    api_token: Token<String, String>,
    runtime: tokio::runtime::Runtime,
    metrics: Vec<Arc<TimeSeries>>,
}

impl Scraper {
    pub fn new(config: ScraperConfig) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .core_threads(config.topics.len())
            .build()
            .expect("Could not initialize scraper runtime");

        let api_token = Token::new(
            config.consumer_key,
            config.consumer_secret,
            config.access_key,
            config.access_secret,
        );
        let mut scraper = Self {
            runtime,
            api_token,
            metrics: Vec::new(),
        };
        config
            .topics
            .into_iter()
            .for_each(|topic| scraper.subscribe_to(topic));
        scraper
    }

    /// Subscribe to a stream of tweets containing the specified topic
    pub fn subscribe_to(&mut self, topic: String) {
        info!("Subscribing to topic {}", &topic);

        // Add a time series reference
        let time_series = Arc::new(TimeSeries::new(topic.as_str()));
        self.metrics.push(time_series.clone());

        let tweet_analyzer = rate_controlled_stream::RateLimitedStream::from_topic(
            self.api_token.clone(),
            topic.clone(),
        )
        .map_err(|err| error!("Error while processing twitter stream: {}", err))
        .and_then(|item| {
            serde_json::from_str::<Tweet>(&item).map_err(|err| {
                error!(
                    "Error while parsing tweet as JSON: {}\nTweet: {}",
                    err, item
                )
            })
        })
        //        .inspect(|tweet| info!("Tweet: {tweet:?}", tweet = tweet))
        .filter_map(|tweet| match tweet {
            Tweet::ApiLimit(limit) => {
                warn!("Limit warning: {}", limit.track);
                None
            }
            Tweet::Content(content) => parse_time(content.created_at.as_str())
                .map(|time| Sample {
                    time: time.timestamp(),
                    value: sentiment::message_value(content.text.as_str()),
                })
                .ok(),
            Tweet::Disconnect(disconnect) => {
                warn!(
                    "[{topic}] Stream disconnected: {reason}",
                    topic = disconnect.stream_name,
                    reason = disconnect.reason
                );
                None
            }
            Tweet::StallWarning(warning) => {
                warn!(
                    "[{topic}] Stream stalling: {perc}",
                    topic = warning.stream_name,
                    perc = warning.percent_full
                );
                None
            }
        })
        .for_each(move |sample| {
            time_series
                .data
                .write()
                .map(|mut store| {
                    store.push(sample);
                })
                .map_err(|err| error!("Error storing sample: {}", err))
        });

        self.runtime.spawn(tweet_analyzer);
    }

    pub fn metrics(&self) -> Vec<Arc<TimeSeries>> {
        self.metrics.clone()
    }

    //    pub fn run(&mut self) -> Result<(), ()> {
    //        self.runtime.block_on(futures::future::empty())
    //    }
}

fn parse_time(time_str: &str) -> Result<NaiveDateTime, ()> {
    NaiveDateTime::parse_from_str(time_str, TWITTER_DATE_FORMAT)
        .map_err(|err| error!("Error parsing date: {}", err))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_twitter_date_string() {
        let sample_string = "Wed Oct 16 20:18:02 +0000 2019";
        let parsed = parse_time(sample_string);
        assert!(
            parsed.is_ok(),
            "Could not parse Twitter date string: {}",
            sample_string
        );
    }
}
