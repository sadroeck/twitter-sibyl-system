use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Limit {
    pub track: u64,
    pub timestamp_ms: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content {
    pub created_at: String,
    //    pub id_str: String,
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
    ApiLimit(ApiLimit),
    Content(Content),
    Disconnect(Disconnect),
    StallWarning(StallWarning),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_limit_msg() {
        let msg = "{\"limit\":{\"track\":1678,\"timestamp_ms\":\"1571317682725\"}}";
        serde_json::from_str::<Tweet>(&msg).expect("Could not decode limit msg");
    }
}
