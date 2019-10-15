use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Tweet {
    pub created_at: String,
    pub id_str: String,
    pub text: String,
}
