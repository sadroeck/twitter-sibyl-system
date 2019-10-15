use crate::config::ScraperConfig;
use crate::tweet::Tweet;
use futures::future::Future;
use futures::stream::Stream;
use log::{error, info};
use twitter_stream::{Token, TwitterStreamBuilder};

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

    pub fn subscribe_to(&mut self, topic: String) {
        info!("Subscribing to topic {}", &topic);
        let single_stream_printer = TwitterStreamBuilder::filter(self.api_token.clone())
            .stall_warnings(true)
            .track(topic.as_str())
            .listen()
            .unwrap()
            .flatten_stream()
            .map_err(|err| error!("Error while processing twitter stream: {}", err))
            .and_then(|x| {
                serde_json::from_str::<Tweet>(&x)
                    .map_err(|err| error!("Error while parsing tweet as JSON: {}", err))
            })
            .for_each(move |json| Ok(info!("[{topic}] {data:?}", topic = &topic, data = json)));
        self.runtime.spawn(single_stream_printer);
    }

    pub fn run(&mut self) -> Result<(), ()> {
        self.runtime.block_on(futures::future::empty())
    }
}
