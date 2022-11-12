use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

pub struct KeyStore {
    keys: HashMap<String, String>,
}

impl KeyStore {
    pub async fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Self, KeyStoreLoadSaveError> {
        let mut data: Vec<u8> = Vec::new();
        File::open(p).await?.read_to_end(&mut data).await?;
        let data: HashMap<String, String> = serde_yaml::from_slice(&data)?;

        // We want to be able to look up key -> user, but for convenience users
        // should be able to provide user -> key
        let inverted = data.into_iter().map(|(user, key)| (key, user)).collect();

        Ok(Self::new(inverted))
    }

    pub fn new(keys: HashMap<String, String>) -> Self {
        Self { keys }
    }

    pub fn get_user(&self, given_key: &str) -> Option<String> {
        self.keys.get(given_key).map(|v| v.to_string())
    }
}

#[derive(Debug, Error)]
pub enum KeyStoreLoadSaveError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("ser/deserialisation error: {0}")]
    Serde(#[from] serde_yaml::Error),
}
