use clap::{App, Arg};
use futures::future::Future;
use futures::stream::Stream;
use log::{error, info, Level};
use twitter_stream::{Token, TwitterStreamBuilder};

mod config;

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
        .track(Some("@Twitter"))
        .listen()
        .unwrap()
        .flatten_stream()
        .for_each(|json| {
            info!("[{topic}] {data}", topic = "?", data = json);
            Ok(())
        })
        .map_err(|e| error!("error: {}", e));

    tokio::runtime::run(single_stream_printer);
}
