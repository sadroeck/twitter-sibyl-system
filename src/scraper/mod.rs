use crate::config::ScraperConfig;
use crate::scraper::metrics::{Sample, TimeSeries};
use crate::tweet::Tweet;
use chrono::NaiveDateTime;
use futures::stream::Stream;
use log::{error, info, warn};
use metrics_runtime::{Controller, Receiver};
use std::sync::Arc;
use std::time::Instant;
use twitter_stream::Token;

pub mod metrics;
mod rate_controlled_stream;
mod sentiment;

const TWITTER_DATE_FORMAT: &'static str = "%a %b %d %H:%M:%S %z %Y";

pub struct Scraper {
    api_token: Token<String, String>,
    runtime: tokio::runtime::Runtime,
    time_series: Vec<Arc<TimeSeries>>,
    metrics: Receiver,
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
        let receiver = Receiver::builder()
            .build()
            .expect("failed to create metrics receiver");
        let mut scraper = Self {
            runtime,
            api_token,
            time_series: Vec::new(),
            metrics: receiver,
        };
        config
            .topics
            .into_iter()
            .for_each(|topic| scraper.subscribe_to(topic));
        scraper
    }

    pub fn metrics(&self) -> Controller {
        self.metrics.get_controller()
    }

    /// Subscribe to a stream of tweets containing the specified topic
    pub fn subscribe_to(&mut self, topic: String) {
        info!("Subscribing to topic {}", &topic);
        let mut sink = self.metrics.get_sink();
        let tweets_queued = sink.gauge_with_labels("tweets_queued", &[("topic", topic.clone())]);
        let stall_level = sink.gauge_with_labels("stall_level", &[("topic", topic.clone())]);
        let processed_tweets =
            sink.counter_with_labels("tweets_processed", &[("topic", topic.clone())]);
        let processing_time =
            sink.histogram_with_labels("processing_time", &[("topic", topic.clone())]);
        let storage_time = sink.histogram_with_labels("storage_time", &[("topic", topic.clone())]);

        // Add a time series reference
        let time_series = Arc::new(TimeSeries::new(topic.as_str()));
        self.time_series.push(time_series.clone());

        let tweet_analyzer = rate_controlled_stream::RateLimitedStream::from_topic(
            self.api_token.clone(),
            topic.clone(),
        )
        .map_err(|err| error!("Error while processing twitter stream: {}", err))
        //        .inspect(|tweet| info!("Tweet: {tweet:?}", tweet = tweet))
        .and_then(|item| {
            let now = Instant::now();
            serde_json::from_str::<Tweet>(&item)
                .map_err(|err| {
                    error!(
                        "Error while parsing tweet as JSON: {}\nTweet: {}",
                        err, item
                    )
                })
                .map(|x| (now, x))
        })
        //            .inspect(|tweet| info!("Tweet: {tweet:?}", tweet = tweet))
        .filter_map(move |(start, tweet)| match tweet {
            Tweet::ApiLimit(limit) => {
                tweets_queued.record(limit.limit.track as i64);
                None
            }
            Tweet::Content(content) => parse_time(content.created_at.as_str())
                .map(|time| Sample {
                    time: time.timestamp(),
                    value: sentiment::message_value(content.text.as_str()),
                })
                .ok()
                .map(|sample| (start, sample)),
            Tweet::Disconnect(disconnect) => {
                warn!(
                    "[{topic}] Stream disconnected: {reason}",
                    topic = disconnect.stream_name,
                    reason = disconnect.reason
                );
                None
            }
            Tweet::StallWarning(warning) => {
                stall_level.record(warning.percent_full as i64);
                None
            }
        })
        .for_each(move |(start, sample)| {
            let storage_start = Instant::now();
            processed_tweets.increment();
            processing_time.record_timing(start, storage_start);
            let result = time_series
                .data
                .write()
                .map(|mut store| {
                    store.push(sample);
                })
                .map_err(|err| error!("Error storing sample: {}", err));
            storage_time.record_timing(storage_start, Instant::now());
            result
        });

        self.runtime.spawn(tweet_analyzer);
    }

    pub fn time_series(&self) -> Vec<Arc<TimeSeries>> {
        self.time_series.clone()
    }
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
