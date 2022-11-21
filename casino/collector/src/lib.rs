use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, IntoUrl, Url};
use reqwest::header::AUTHORIZATION;
use tokio::time::interval;

use steam::errors::FetchItemsError;
use steam::{InventoryId, SteamClient, UnhydratedUnlock};
use thiserror::Error;

pub mod config;

#[derive(Debug, Error)]
#[error("given url was not valid: {0}")]
pub struct UrlParseError(reqwest::Error);

pub struct Collector {
    collection_url: Url,
    http_client: Client,
    steam_client: SteamClient,
    pre_shared_key: String,

    poll_interval: Duration,
    last_unboxing: Option<DateTime<Utc>>,
    last_known_item: Option<InventoryId>,
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
            last_known_item: None,
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
        let last_item = self.last_known_item.as_deref();
        let mut new_items = self
            .steam_client
            .fetch_history_for_new_items(since, last_item)
            .await?;

        if new_items.is_empty() {
            log::info!("no new items");
            return Ok(());
        }

        self.send_results(&new_items).await?;
        let last = new_items.remove(0);
        self.last_unboxing = Some(last.at);
        self.last_known_item = Some(last.history_id);

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

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("error fetching items: {0}")]
    FetchingItems(#[from] FetchItemsError),
    #[error("error sending results: {0}")]
    SendingResults(#[from] ResultsSendError),
}

#[derive(Debug, Error)]
pub enum ResultsSendError {
    #[error("error serialising outbound items: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Transport(#[from] reqwest::Error),
}
