use std::convert::Infallible;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use tokio::time::interval;

use crate::steam::{SteamClient, SteamCredentials, UnhydratedUnlock};

lazy_static::lazy_static! {
    static ref COLLECTION_URL: Url = "https://127.0.0.1:7000/upload".parse().unwrap();
    static ref POLL_INTERVAL: Duration = Duration::from_secs(60);
}

pub struct Collector {
    user_friendly_name: String,
    steam_client: SteamClient,
    http_client: Client,

    last_unboxing: Option<DateTime<Utc>>,
}

impl Collector {
    pub fn new(
        steam_username: String,
        steam_id: u64,
        creds: SteamCredentials,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Self, Infallible> {
        let http_client = Client::new();
        let steam_client = SteamClient::new(steam_username.clone(), steam_id, creds).unwrap();
        Ok(Self {
            user_friendly_name: steam_username,
            steam_client,
            http_client,

            last_unboxing: start_time,
        })
    }

    pub async fn run(&mut self) -> Result<(), Infallible> {
        let mut tick = interval(*POLL_INTERVAL);

        loop {
            tokio::select! {
                _ = tick.tick() => self.poll().await?,
                _ = tokio::signal::ctrl_c() => return Ok(()),
            }
        }
    }

    async fn poll(&mut self) -> Result<(), Infallible> {
        let since = self.last_unboxing.as_ref();
        let new_items = self.steam_client.fetch_new_items(since).await.unwrap();

        if new_items.is_empty() {
            return Ok(());
        }

        self.send_results(&new_items).await?;
        let last = new_items.get(0).unwrap().at;
        self.last_unboxing = Some(last);

        Ok(())
    }

    async fn send_results(&self, items: &[UnhydratedUnlock]) -> Result<(), Infallible> {
        let data = serde_json::to_vec(items).unwrap();
        let url = COLLECTION_URL.clone();
        self.http_client.post(url).body(data).send().await.unwrap();

        Ok(())
    }
}
