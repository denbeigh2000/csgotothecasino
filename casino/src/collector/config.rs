use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    steam_username: String,
    pre_shared_key: String,
}
