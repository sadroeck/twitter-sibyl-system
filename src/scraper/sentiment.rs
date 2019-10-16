/// Express sentiment as a value between `-INF` & `+INF`.
/// Negative values indicate a strong
pub type Value = i64;

pub fn message_value(content: &str) -> Value {
    // TODO: Replace with basic sentiment analysis
    content.len() as Value
}
