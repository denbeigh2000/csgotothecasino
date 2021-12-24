use std::convert::Infallible;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url};
use reqwest::cookie::Jar;
use scraper::Html;

static COLLECTION_URL: &str = "https://";

pub struct Collector {
    user_friendly_name: String,
    auth_cookie: String,

    client: SteamClient,

    last_unboxing: Option<DateTime<Utc>>,
    last_unboxed: Option<InventoryId>,
}

impl Collector {}

pub struct InventoryId {
    pub class_id: u64,
    pub instance_id: u64,
}

pub struct Unlock {
    pub required_key: bool,
    pub case_name: String,
    pub item: Item,
}

pub struct UnhydratedItem {
    pub id: InventoryId,
}

pub struct Item {
    pub name: String,
    pub float: f64,
    pub id: InventoryId,
    pub variant: String,
}

struct SteamCredentials {
    session_id: String,
    login_token: String,
}

impl SteamCredentials {
    pub fn into_jar(self) -> Jar {
        let mut jar = Jar::default();
        let url = "https://steamcommunity.com".parse().unwrap();
        let cookie_str = format!("sessionid={}; steamLoginSecure={}", self.session_id, self.login_token);
        jar.add_cookie_str(&cookie_str, &url);

        jar
    }
}

struct SteamClient {
    username: String,
    user_id: u64,
    http_client: Client,

    inventory_url: Url,
    inventory_history_url: Url,
}

pub enum FetchNewItemsError {
    TransportError(reqwest::Error),
    AuthenticationFailure,
    UnhandledStatusCode(StatusCode),
}

impl From<reqwest::Error> for FetchNewItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::TransportError(e)
    }
}

impl SteamClient {
    pub fn new(username: String, user_id: u64, creds: SteamCredentials) -> Result<Self, Infallible> {
        let http_client = Client::builder()
            .cookie_provider(Arc::new(creds.into_jar()))
            .build()
            .unwrap();

        let inventory_url = format!("https://steamcommunity.com/inventory/{}/730/2?l=english&count=25", user_id).parse().unwrap();
        let inventory_history_url = format!("https://steamcommunity.com/id/{}/inventoryhistory/?app[]=730", username).parse().unwrap();

        Ok(Self {
            username,
            user_id,
            http_client,

            inventory_url,
            inventory_history_url,
        })
    }

    pub async fn fetch_new_items(&mut self, since: &DateTime<Utc>) -> Result<Vec<Unlock>, FetchNewItemsError> {
        let resp = self.http_client.get(self.inventory_history_url.clone()).send().await?;
        let status = resp.status();

        match status {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(FetchNewItemsError::AuthenticationFailure),
            _ => return Err(FetchNewItemsError::UnhandledStatusCode(status)),
        }

        let data = resp.text().await?;
        let parsed_data = Html::parse_document(&data);

        unimplemented!()
    }
}
