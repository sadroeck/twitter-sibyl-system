use crate::scraper::Scraper;
use clap::{App, Arg};

use log::Level;

mod config;
mod scraper;
mod server;
mod tweet;

fn cmd_line_config() -> Option<String> {
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

    matches.value_of("config").map(|x| x.trim().to_owned())
}

fn main() -> std::io::Result<()> {
    // Initialize logging
    simple_logger::init_with_level(Level::Info)
        .expect("Could not initialize the logging framework");

    // Fetch configuration
    let config_uri = cmd_line_config();
    let config = config_uri
        .and_then(|cfg_uri| config::load_config(&cfg_uri).ok())
        .or_else(|| config::from_env().ok())
        .expect("Could not assemble a valid configuration");

    // Initialize Scraper
    let scraper = Scraper::new(config.scraper);

    // Initialize actix runtime
    let actor_system = actix_rt::System::new("webservice");

    // Initialize server
    server::run(config.server, scraper.time_series(), scraper.metrics())
        .expect("Could not start server");

    // Block until actor system has stopped
    actor_system.run()
}
