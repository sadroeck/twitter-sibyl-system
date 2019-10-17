use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub scraper: ScraperConfig,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        toml::ser::to_string_pretty(self)
            .map_err(|_| fmt::Error)
            .and_then(|value| write!(f, "{}", value))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    // Host address to listen on
    pub host: String,
    // Port to listen on
    pub port: u16,
}

impl fmt::Display for ScraperConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        toml::ser::to_string_pretty(self)
            .map_err(|_| fmt::Error)
            .and_then(|value| write!(f, "{}", value))
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ScraperConfig {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_key: String,
    pub access_secret: String,
    pub topics: Vec<String>,
    pub batch_size: Option<usize>,
}

pub fn load_config(config_path: &str) -> Result<Config, String> {
    let mut file_str = String::new();
    let file_path = Path::new(config_path);
    let mut open_file = File::open(file_path).or_else(|e| {
        Err(format!(
            "Could not open file: {file}. Reason: {error}",
            file = config_path,
            error = e
        ))
    })?;
    open_file.read_to_string(&mut file_str).or_else(|e| {
        Err(format!(
            "Could not read the config file: {file}. Reason: {error}",
            file = config_path,
            error = e
        ))
    })?;
    toml::from_str(&file_str).or_else(|e| {
        Err(format!(
            "Unable to load config: {file}. Reason: {error}",
            file = config_path,
            error = e
        ))
    })
}

fn number_from_env<T: std::str::FromStr>(label: &str) -> Result<T, String> {
    std::env::var(label)
        .map_err(|_| format!("No {} in environment variables", label))
        .and_then(|x| str::parse::<T>(&x).map_err(|_| format!("Could not parse {} as u16", label)))
}

pub fn from_env() -> Result<Config, String> {
    let server_config = number_from_env::<u16>("PORT").map(|port| {
        let host = std::env::var("HOST")
            .map_err(|_| format!("No HOST in environment variables"))
            .unwrap_or("0.0.0.0".to_owned());

        ServerConfig { host, port }
    });

    let scraper_config = std::env::var("CONSUMER_KEY")
        .map_err(|_| format!("No CONSUMER_KEY in environment variables"))
        .and_then(|consumer_key| {
            std::env::var("CONSUMER_SECRET")
                .map_err(|_| format!("No CONSUMER_SECRET in environment variables"))
                .map(|consumer_secret| (consumer_key, consumer_secret))
        })
        .and_then(|(consumer_key, consumer_secret)| {
            std::env::var("ACCESS_KEY")
                .map_err(|_| format!("No ACCESS_KEY in environment variables"))
                .map(|access_key| (consumer_key, consumer_secret, access_key))
        })
        .and_then(|(consumer_key, consumer_secret, access_key)| {
            std::env::var("ACCESS_SECRET")
                .map_err(|_| format!("No ACCESS_SECRET in environment variables"))
                .map(|access_secret| (consumer_key, consumer_secret, access_key, access_secret))
        })
        .and_then(
            |(consumer_key, consumer_secret, access_key, access_secret)| {
                std::env::var("TOPICS")
                    .map_err(|_| format!("No TOPICS in environment variables"))
                    .map(|topics| topics.split(",").map(|x| x.to_owned()).collect())
                    .map(|topics| ScraperConfig {
                        consumer_key,
                        consumer_secret,
                        access_key,
                        access_secret,
                        topics,
                        batch_size: number_from_env("BATCH_SIZE").ok(),
                    })
            },
        );

    server_config.and_then(|server_config| {
        scraper_config.map(|scraper_config| Config {
            scraper: scraper_config,
            server: server_config,
        })
    })
}
