use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("deserialisation error: {0}")]
    Serde(#[from] serde_yaml::Error),
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub steam_profile_url: String,
    pub pre_shared_key: String,
}

impl Config {
    pub async fn try_from_path<P: AsRef<Path>>(p: P) -> Result<Self, ConfigLoadError> {
        let mut buf: Vec<u8> = Vec::new();
        File::open(p).await?.read_to_end(&mut buf).await?;
        let parsed = serde_yaml::from_slice(&buf)?;

        Ok(parsed)
    }
}
