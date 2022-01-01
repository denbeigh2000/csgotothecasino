use std::fmt;
use std::fmt::Display;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, IntoUrl, Url};
use reqwest::header::AUTHORIZATION;
use tokio::time::interval;

use steam::errors::FetchItemsError;
use steam::{SteamClient, UnhydratedUnlock};

pub mod config;

#[derive(Debug)]
pub struct UrlParseError(reqwest::Error);

impl Display for UrlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "given url was not valid: {}", self.0)
    }
}

pub struct Collector {
    collection_url: Url,
    http_client: Client,
    steam_client: SteamClient,
    pre_shared_key: String,

    poll_interval: Duration,
    last_unboxing: Option<DateTime<Utc>>,
    last_parsed_history_id: Option<String>,
}

impl Collector {
    pub async fn new<U>(
        collection_url: U,
        steam_client: SteamClient,
        pre_shared_key: String,
        poll_interval: Duration,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Self, UrlParseError>
    where
        U: IntoUrl,
    {
        let http_client = Client::new();
        let collection_url: Url = collection_url.into_url().map_err(UrlParseError)?;

        Ok(Self {
            collection_url,
            http_client,
            steam_client,
            pre_shared_key,

            poll_interval,
            last_unboxing: start_time,
            last_parsed_history_id: None,
        })
    }

    pub async fn run(&mut self) -> Result<(), CollectorError> {
        let mut tick = interval(self.poll_interval);
        log::info!(
            "checking for new items every {} seconds",
            self.poll_interval.as_secs()
        );

        loop {
            tokio::select! {
                _ = tick.tick() => self.poll().await?,
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn poll(&mut self) -> Result<(), CollectorError> {
        log::info!("checking for new items");
        let since = self.last_unboxing.as_ref();
        let last_id = self.last_parsed_history_id.as_deref();
        let mut new_items = self
            .steam_client
            .fetch_new_items(since, last_id)
            .await?;

        if new_items.is_empty() {
            log::info!("no new items");
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
        log::info!(
            "sending {} new items to {}",
            items.len(),
            self.collection_url
        );
        self.http_client
            .post(self.collection_url.as_ref())
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
