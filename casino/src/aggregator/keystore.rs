use std::collections::HashMap;
use std::fmt::{self, Display};
use std::path::Path;

use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

#[derive(Deserialize)]
pub struct KeyStore {
    keys: HashMap<String, String>,
}

impl KeyStore {
    pub async fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Self, KeyStoreLoadSaveError> {
        let mut data: Vec<u8> = Vec::new();
        File::open(p).await?.read_to_end(&mut data).await?;
        let parsed = serde_yaml::from_slice(&data)?;

        Ok(parsed)
    }

    pub fn new(keys: HashMap<String, String>) -> Self {
        Self { keys }
    }

    pub fn get_user(&self, given_key: &str) -> Option<String> {
        self.keys.get(given_key).map(|v| v.to_string())
    }
}

#[derive(Debug)]
pub enum KeyStoreLoadSaveError {
    IO(io::Error),
    Serde(serde_yaml::Error),
}

impl From<io::Error> for KeyStoreLoadSaveError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<serde_yaml::Error> for KeyStoreLoadSaveError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Serde(e)
    }
}

impl Display for KeyStoreLoadSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(e) => write!(f, "io error: {}", e),
            Self::Serde(e) => write!(f, "ser/deserialisation error: {}", e),
        }
    }
}
