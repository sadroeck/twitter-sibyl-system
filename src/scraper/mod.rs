use crate::config::ScraperConfig;
use crate::scraper::metrics::{Sample, TimeSeries};
use crate::tweet::Tweet;
//use chrono::NaiveDateTime;
use chrono::Utc;
use futures::stream::Stream;
use log::{error, info, warn};
use metrics_runtime::{Controller, Receiver};
use std::cmp::max;
use std::sync::Arc;
use std::time::Instant;
use twitter_stream::Token;

pub mod metrics;
mod rate_controlled_stream;
mod sentiment;

//const TWITTER_DATE_FORMAT: &'static str = "%a %b %d %H:%M:%S %z %Y";
const DEFAULT_BATCH_SIZE: usize = 100;

pub struct Scraper {
    batch_size: usize,
    api_token: Token<String, String>,
    runtime: tokio::runtime::Runtime,
    time_series: Vec<Arc<TimeSeries>>,
    metrics: Receiver,
}

impl Scraper {
    pub fn new(config: ScraperConfig) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .core_threads(max(config.topics.len() * 2, num_cpus::get()))
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
            batch_size: config.batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
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
        let processing_time =
            sink.histogram_with_labels("processing_time", &[("topic", topic.clone())]);
        let processed_tweets =
            sink.counter_with_labels("tweets_processed", &[("topic", topic.clone())]);
        let storage_time = sink.histogram_with_labels("storage_time", &[("topic", topic.clone())]);
        let batch_size = self.batch_size;
        let executor = self.runtime.executor();

        // Add a time series reference
        let time_series = Arc::new(TimeSeries::new(topic.as_str()));
        self.time_series.push(time_series.clone());

        let tweet_analyzer = rate_controlled_stream::RateLimitedStream::from_topic(
            self.api_token.clone(),
            topic.clone(),
        )
        .map_err(|err| error!("Error processing tweet batch: {}", err))
        .chunks(2)
        .for_each(move |items| {
            // Clone all shared references
            let processed_tweets = processed_tweets.clone();
            let processing_time = processing_time.clone();
            let stall_level = stall_level.clone();
            let storage_time = storage_time.clone();
            let time_series = time_series.clone();
            let tweets_queued = tweets_queued.clone();

            // Lazily schedule the batch processing onto the threadpool
            let tweet_processing = futures::future::lazy(move || {
                let samples = items.into_iter().filter_map(|item| {
                    let start = Instant::now();
                    serde_json::from_str::<Tweet>(&item)
                        .map_err(|err| {
                            error!(
                                "Error while parsing tweet as JSON: {}\nTweet: {}",
                                err, item
                            )
                        })
                        .ok()
                        .and_then(|tweet| match tweet {
                            Tweet::ApiLimit(limit) => {
                                tweets_queued.record(limit.limit.track as i64);
                                None
                            }
                            Tweet::Content(content) => {
                                Some(sentiment::message_value(content.text.as_str()))
                            }
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
                        .map(|sample| {
                            processing_time.record_timing(start, Instant::now());
                            sample
                        })
                });

                processed_tweets.record(batch_size as u64);
                let storage_start = Instant::now();
                let timestamp = Utc::now().timestamp();
                let result = time_series
                    .data
                    .write()
                    .map(|mut store| {
                        store.extend(samples.map(|value| Sample {
                            time: timestamp,
                            value,
                        }));
                    })
                    .map_err(|err| error!("Error storing sample: {}", err));
                storage_time.record_timing(storage_start, Instant::now());
                result
            });

            // Drive the stream indefinitely on the threadpool
            executor.spawn(tweet_processing);
            Ok(())
        });

        self.runtime.spawn(tweet_analyzer);
    }

    pub fn time_series(&self) -> Vec<Arc<TimeSeries>> {
        self.time_series.clone()
    }
}

//fn parse_time(time_str: &str) -> Result<NaiveDateTime, ()> {
//    NaiveDateTime::parse_from_str(time_str, TWITTER_DATE_FORMAT)
//        .map_err(|err| error!("Error parsing date: {}", err))
//}

//#[cfg(test)]
//mod test {
//    use super::*;
//
//    #[test]
//    fn parse_twitter_date_string() {
//        let sample_string = "Wed Oct 16 20:18:02 +0000 2019";
//        let parsed = parse_time(sample_string);
//        assert!(
//            parsed.is_ok(),
//            "Could not parse Twitter date string: {}",
//            sample_string
//        );
//    }
//}
