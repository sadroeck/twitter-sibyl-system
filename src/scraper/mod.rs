use crate::config::ScraperConfig;
use crate::tweet::Tweet;
use futures::stream::Stream;
use log::{error, info};
use twitter_stream::Token;

mod rate_controlled_stream;

pub struct Scraper {
    api_token: Token<String, String>,
    runtime: tokio::runtime::Runtime,
}

impl Scraper {
    pub fn new(config: ScraperConfig) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .core_threads(config.workers.unwrap_or_else(|| num_cpus::get()))
            .build()
            .expect("Could not initialize scraper runtime");

        let api_token = Token::new(
            config.consumer_key,
            config.consumer_secret,
            config.access_key,
            config.access_secret,
        );

        let mut scraper = Self { runtime, api_token };
        config
            .topics
            .into_iter()
            .for_each(|topic| scraper.subscribe_to(topic));
        scraper
    }

    /// Subscribe to a stream of tweets containing the specified topic
    pub fn subscribe_to(&mut self, topic: String) {
        info!("Subscribing to topic {}", &topic);
        let tweet_logger = rate_controlled_stream::RateLimitedStream::from_topic(
            self.api_token.clone(),
            topic.clone(),
        )
        .map_err(|err| error!("Error while processing twitter stream: {}", err))
        .and_then(|item| {
            serde_json::from_str::<Tweet>(&item)
                .map_err(|err| error!("Error while parsing tweet as JSON: {}", err))
        })
        .for_each(move |tweet| Ok(info!("[{topic}] {tweet:?}", topic = &topic, tweet = tweet)));

        self.runtime.spawn(tweet_logger);
    }

    pub fn run(&mut self) -> Result<(), ()> {
        self.runtime.block_on(futures::future::empty())
    }
}
