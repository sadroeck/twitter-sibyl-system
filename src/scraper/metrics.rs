use crate::scraper::sentiment;
use serde_derive::Serialize;
use std::sync::RwLock;

const DEFAULT_TIMESERIES_SIZE: usize = 10000;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize)]
/// Time series sample for sentiment tracking
pub struct Sample {
    /// Epoch time
    pub time: i64,
    /// Sentiment score
    pub value: sentiment::Value,
}

pub struct TimeSeries {
    pub topic: String,
    pub data: RwLock<Vec<Sample>>,
}

impl TimeSeries {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_owned(),
            data: RwLock::new(Vec::with_capacity(DEFAULT_TIMESERIES_SIZE)),
        }
    }
}
