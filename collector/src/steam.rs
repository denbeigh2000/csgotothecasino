use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use crate::parsing::InventoryId;
use crate::parsing::ParseResult;
use crate::parsing::UnhydratedUnlock;
use crate::parsing::{parse_unhydrated_unlock, TRADE_SELECTOR};

use chrono::{DateTime, Utc};
use reqwest::cookie::Jar;
use reqwest::{Client, StatusCode, Url};
use scraper::Html;
use serde::Deserialize;
use serde::Serialize;

pub struct Unlock {
    pub key: Option<Item>,
    pub case: Item,
    pub item: Item,
}

pub struct Item {
    pub name: String,
    pub id: InventoryId,
    pub variant: String,
    pub icon_url: String,
}

pub struct SteamCredentials {
    session_id: String,
    login_token: String,
}

impl SteamCredentials {
    pub fn into_jar(self) -> Jar {
        let jar = Jar::default();
        let url = "https://steamcommunity.com".parse().unwrap();
        let cookie_str = format!(
            "sessionid={}; steamLoginSecure={}",
            self.session_id, self.login_token
        );
        jar.add_cookie_str(&cookie_str, &url);

        jar
    }
}

pub enum FetchNewItemsError {
    TransportError(reqwest::Error),
    AuthenticationFailure,
    UnhandledStatusCode(StatusCode),
    NoHistoryFound,
}

impl From<reqwest::Error> for FetchNewItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::TransportError(e)
    }
}

#[derive(Deserialize)]
pub struct Inventory {
    descriptions: Vec<InventoryDescription>,
}

#[derive(Serialize, Deserialize)]
pub struct InventoryDescription {
    #[serde(rename(deserialize = "classid"))]
    class_id: String,
    #[serde(rename(deserialize = "instanceid"))]
    instance_id: String,
    #[serde(rename(deserialize = "icon_url_large"))]
    icon_url: String,
    name: String,
    #[serde(rename = "type")]
    variant: String,
}

impl Into<Item> for &InventoryDescription {
    fn into(self) -> Item {
        Item {
            name: self.name.clone(),
            id: InventoryId {
                class_id: self.class_id.parse().unwrap(),
                instance_id: self.instance_id.parse().unwrap(),
            },
            variant: self.variant.clone(),
            icon_url: self.icon_url.clone(),
        }
    }
}

pub struct SteamClient {
    username: String,
    user_id: u64,
    http_client: Client,

    inventory_url: Url,
    inventory_history_url: Url,
}

impl SteamClient {
    pub fn new(
        username: String,
        user_id: u64,
        creds: SteamCredentials,
    ) -> Result<Self, Infallible> {
        let http_client = Client::builder()
            .cookie_provider(Arc::new(creds.into_jar()))
            .build()
            .unwrap();

        let inventory_url = format!(
            "https://steamcommunity.com/inventory/{}/730/2?l=english&count=25",
            user_id
        )
        .parse()
        .unwrap();
        let inventory_history_url = format!(
            "https://steamcommunity.com/id/{}/inventoryhistory/?app[]=730",
            username
        )
        .parse()
        .unwrap();

        Ok(Self {
            username,
            user_id,
            http_client,

            inventory_url,
            inventory_history_url,
        })
    }

    pub async fn fetch_new_items(
        self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchNewItemsError> {
        let resp = self
            .http_client
            .get(self.inventory_history_url.clone())
            .send()
            .await?;
        let status = resp.status();

        match status {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(FetchNewItemsError::AuthenticationFailure)
            }
            _ => return Err(FetchNewItemsError::UnhandledStatusCode(status)),
        }

        let data = resp.text().await?;
        let parsed_data = Html::parse_document(&data);

        let trades = parsed_data.select(&TRADE_SELECTOR);
        let mut seen_any = false;

        let mut unlocks: Vec<UnhydratedUnlock> = Vec::new();

        for trade in trades {
            match parse_unhydrated_unlock(trade, since) {
                ParseResult::Success(v) => unlocks.push(v),
                ParseResult::TooOld => return Ok(unlocks),
                ParseResult::Unparseable => panic!("failed to parse html??"),
                ParseResult::WrongTransactionType => {
                    seen_any = true;
                    continue;
                }
            }
        }

        if !seen_any {
            return Err(FetchNewItemsError::NoHistoryFound);
        }

        Ok(unlocks)
    }

    pub async fn hydrate_unlocks(
        &self,
        items: Vec<UnhydratedUnlock>,
    ) -> Result<Vec<Unlock>, Infallible> {
        let resp = self
            .http_client
            .get(self.inventory_url.clone())
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let inv: Inventory = resp.json().await.unwrap();
        let data_map: HashMap<InventoryId, InventoryDescription> = inv
            .descriptions
            .into_iter()
            .fold(HashMap::new(), |mut acc, item| {
                let id = InventoryId {
                    class_id: item.class_id.parse().unwrap(),
                    instance_id: item.instance_id.parse().unwrap(),
                };

                acc.insert(id, item);
                acc
            });

        let results = items.into_iter().map(|i| {
            let case = data_map.get(&i.case).unwrap().into();
            let key = i.key.map(|k| data_map.get(&k).unwrap().into());
            let item = data_map.get(&i.item).unwrap().into();

            Unlock { key, case, item }
        }).collect();

        Ok(results)
    }
}
