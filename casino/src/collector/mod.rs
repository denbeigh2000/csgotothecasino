use std::fmt;
use std::fmt::Display;
use std::time::Duration;

use chrono::{DateTime, Utc};
use hyper::header::AUTHORIZATION;
use reqwest::{Client, Url};
use tokio::time::interval;

use crate::steam::errors::FetchItemsError;
use crate::steam::{SteamClient, UnhydratedUnlock};

lazy_static::lazy_static! {
    static ref COLLECTION_URL: Url = "https://casino.denb.ee/api/upload".parse().unwrap();
}

pub mod config;

pub struct Collector {
    http_client: Client,
    steam_client: SteamClient,
    pre_shared_key: String,

    poll_interval: Duration,
    last_unboxing: Option<DateTime<Utc>>,
    last_parsed_history_id: Option<String>,
}

impl Collector {
    pub async fn new(
        steam_client: SteamClient,
        pre_shared_key: String,
        poll_interval: Duration,
        start_time: Option<DateTime<Utc>>,
    ) -> Self {
        let http_client = Client::new();
        Self {
            http_client,
            steam_client,
            pre_shared_key,

            poll_interval,
            last_unboxing: start_time,
            last_parsed_history_id: None,
        }
    }

    pub async fn run(&mut self) -> Result<(), CollectorError> {
        let mut tick = interval(self.poll_interval);

        loop {
            tokio::select! {
                _ = tick.tick() => self.poll().await?,
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn poll(&mut self) -> Result<(), CollectorError> {
        let since = self.last_unboxing.as_ref();
        let last_id = self.last_parsed_history_id.as_deref();
        let mut new_items = self
            .steam_client
            .fetch_new_items(since, last_id)
            .await
            .unwrap();

        if new_items.is_empty() {
            return Ok(());
        }

        self.send_results(&new_items).await?;
        let last = new_items.remove(0);
        self.last_unboxing = Some(last.at);
        self.last_parsed_history_id = Some(last.history_id);

        Ok(())
    }

    async fn send_results(&self, items: &[UnhydratedUnlock]) -> Result<(), ResultsSendError> {
        let data = serde_json::to_vec(items)?;
        let url = COLLECTION_URL.clone();
        self.http_client
            .post(url)
            .body(data)
            .header(AUTHORIZATION, &self.pre_shared_key)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum CollectorError {
    FetchingItems(FetchItemsError),
    SendingResults(ResultsSendError),
}

impl From<FetchItemsError> for CollectorError {
    fn from(e: FetchItemsError) -> Self {
        Self::FetchingItems(e)
    }
}

impl From<ResultsSendError> for CollectorError {
    fn from(e: ResultsSendError) -> Self {
        Self::SendingResults(e)
    }
}

impl Display for CollectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchingItems(e) => write!(f, "error fetching items: {}", e),
            Self::SendingResults(e) => write!(f, "error sending results: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum ResultsSendError {
    Serde(serde_json::Error),
    Transport(reqwest::Error),
}

impl From<serde_json::Error> for ResultsSendError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl From<reqwest::Error> for ResultsSendError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

impl Display for ResultsSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultsSendError::Serde(e) => write!(f, "error serialising outbound items: {}", e),
            ResultsSendError::Transport(e) => write!(f, "http error: {}", e),
        }
    }
}
