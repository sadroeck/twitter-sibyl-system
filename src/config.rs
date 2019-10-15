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

pub fn default_workers() -> usize {
    20
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    // Host address to listen on
    pub host: String,
    // Port to listen on
    pub port: u16,
    // Number of threadpool workers
    #[serde(default = "default_workers")]
    pub workers: usize,
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
    pub workers: Option<usize>,
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_key: String,
    pub access_secret: String,
    pub topics: Vec<String>,
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
