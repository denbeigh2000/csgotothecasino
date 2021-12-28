use std::convert::Infallible;
use std::time::Duration;

use chrono::{DateTime, Utc};
use hyper::header::AUTHORIZATION;
use reqwest::{Client, Url};
use tokio::time::interval;

use crate::steam::{SteamClient, SteamCredentials, UnhydratedUnlock};

use self::config::Config;

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
        steam_username: String,
        pre_shared_key: String,
        creds: SteamCredentials,
        poll_interval: Duration,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Self, Infallible> {
        let http_client = Client::new();
        let steam_client = SteamClient::new(steam_username, creds).await.unwrap();
        Ok(Self {
            http_client,
            steam_client,
            pre_shared_key,

            poll_interval,
            last_unboxing: start_time,
            last_parsed_history_id: None,
        })
    }

    pub async fn from_config(
        cfg: Config,
        creds: SteamCredentials,
        poll_interval: Duration,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Self, Infallible> {
        Self::new(
            cfg.steam_username,
            cfg.pre_shared_key,
            creds,
            poll_interval,
            start_time,
        )
        .await
    }

    pub async fn run(&mut self) -> Result<(), Infallible> {
        let mut tick = interval(self.poll_interval);

        loop {
            tokio::select! {
                _ = tick.tick() => self.poll().await?,
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn poll(&mut self) -> Result<(), Infallible> {
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

    async fn send_results(&self, items: &[UnhydratedUnlock]) -> Result<(), Infallible> {
        let data = serde_json::to_vec(items).unwrap();
        let url = COLLECTION_URL.clone();
        self.http_client
            .post(url)
            .body(data)
            .header(AUTHORIZATION, &self.pre_shared_key)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        Ok(())
    }
}
