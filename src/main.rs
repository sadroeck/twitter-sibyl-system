use crate::scraper::Scraper;
use clap::{App, Arg};

use log::Level;

mod config;
mod scraper;
mod server;
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

fn main() -> std::io::Result<()> {
    // Initialize logging
    simple_logger::init_with_level(Level::Info)
        .expect("Could not initialize the logging framework");

    // Fetch configuration
    let config_uri = cmd_line_config();
    let config = config::load_config(&config_uri).expect("Invalid configuration");

    // Initialize Scraper
    let _scraper = Scraper::new(config.scraper);

    // Initialize actix runtime
    let actor_system = actix_rt::System::new("webservice");

    // Initialize server
    server::run(config.server).expect("Could not start server");

    // Block until actor system has stopped
    actor_system.run()
}
