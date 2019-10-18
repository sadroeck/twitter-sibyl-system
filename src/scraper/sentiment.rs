/// Express sentiment as a value between `-INF` & `+INF`.
/// Negative values indicate a strong
pub type Value = i64;

pub fn message_value(content: &str) -> Value {
    // Note: This is very primitive & inefficient, but as a toy version, just taking it as-is
    sentiment::analyze(content.to_owned()).score as i64
}
