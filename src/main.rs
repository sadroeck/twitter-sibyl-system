use crate::tweet::Tweet;
use clap::{App, Arg};
use futures::future::Future;
use futures::stream::Stream;
use log::{error, info, Level};
use twitter_stream::{Token, TwitterStreamBuilder};

mod config;
mod tweet;

fn cmd_line_config() -> String {
    let matches = App::new("twitter-sibyl-system")
        .version("0.1")
        .about("Real-time sentiment analysis for twitter topic streams")
        .author("Sam De Roeck")
        .arg(
            Arg::with_name("config")
                .short("c")
                .value_name("config-file")
                .takes_value(true)
                .help("Configuration file"),
        )
        .get_matches();

    String::from(matches.value_of("config").unwrap_or("config.toml").trim())
}

fn main() {
    // Initialize logging
    simple_logger::init_with_level(Level::Info)
        .expect("Could not initialize the logging framework");

    // Fetch configuration
    let config_uri = cmd_line_config();
    let config = config::load_config(&config_uri).expect("Invalid configuration");
    let twitter_api_token = Token::new(
        config.scraper.consumer_key,
        config.scraper.consumer_secret,
        config.scraper.access_key,
        config.scraper.access_secret,
    );
    let single_stream_printer = TwitterStreamBuilder::filter(twitter_api_token)
        .track(config.scraper.topics.join(",").as_str())
        .listen()
        .unwrap()
        .flatten_stream()
        .map_err(|err| error!("Error while processing twitter stream: {}", err))
        .and_then(|x| {
            serde_json::from_str::<Tweet>(&x)
                .map_err(|err| error!("Error while parsing tweet as JSON: {}", err))
        })
        .for_each(|json| Ok(info!("[{topic}] {data:?}", topic = "?", data = json)));

    tokio::runtime::run(single_stream_printer);
}
