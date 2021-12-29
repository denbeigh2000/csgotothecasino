use std::fmt::{self, Display};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

#[derive(Debug)]
pub enum ConfigLoadError {
    IO(io::Error),
    Serde(serde_yaml::Error),
}

impl From<io::Error> for ConfigLoadError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<serde_yaml::Error> for ConfigLoadError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Serde(e)
    }
}

impl Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigLoadError::IO(e) => write!(f, "io error: {}", e),
            ConfigLoadError::Serde(e) => write!(f, "deserialisation error: {}", e),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub steam_username: String,
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
