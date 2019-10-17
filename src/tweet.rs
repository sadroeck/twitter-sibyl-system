use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Limit {
    pub track: u64,
    pub timestamp_ms: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content {
    pub created_at: String,
    pub id_str: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiLimit {
    pub limit: Limit,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Disconnect {
    pub code: u16,
    pub stream_name: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StallWarning {
    pub code: u16,
    pub stream_name: String,
    pub message: String,
    pub percent_full: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Tweet {
    ApiLimit(Limit),
    Content(Content),
    Disconnect(Disconnect),
    StallWarning(StallWarning),
}
