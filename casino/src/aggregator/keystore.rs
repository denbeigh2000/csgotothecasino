use std::{collections::HashMap, path::Path};

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

    pub fn verify(&self, name: &str, given_key: &str) -> Option<bool> {
        self.keys.get(name).map(|v| v == given_key)
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
