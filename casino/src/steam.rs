use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use crate::cache::Cache;
pub use crate::csgofloat::ItemDescription;
pub use crate::parsing::TrivialItem;
use crate::parsing::{parse_raw_unlock, InventoryId, Item, ParseResult, RawUnlock, TRADE_SELECTOR};

use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;
use bb8_redis::redis::{IntoConnectionInfo, RedisError};
use chrono::{DateTime, Utc};
use reqwest::cookie::Jar;
use reqwest::{Client, StatusCode, Url};
use scraper::Html;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnhydratedUnlock {
    pub key: Option<TrivialItem>,
    pub case: TrivialItem,
    pub item_market_link: String,
    pub item_market_name: String,

    pub at: DateTime<Utc>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Unlock {
    pub key: Option<TrivialItem>,
    pub case: TrivialItem,
    pub item: ItemDescription,
    pub item_value: MarketPrices,

    pub at: DateTime<Utc>,
    pub name: String,
}

pub struct SteamCredentials {
    session_id: String,
    login_token: String,
}

impl SteamCredentials {
    pub fn new(session_id: String, login_token: String) -> Self {
        Self {
            session_id,
            login_token,
        }
    }

    pub fn into_string(self) -> String {
        format!(
            "sessionid={}; steamLoginSecure={}",
            self.session_id, self.login_token
        )
    }

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

impl std::fmt::Debug for FetchNewItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransportError(e) => write!(f, "HTTP error: {}", e),
            Self::AuthenticationFailure => write!(f, "Authentication failure"),
            Self::UnhandledStatusCode(code) => write!(f, "unhandled status code: {}", code),
            Self::NoHistoryFound => write!(f, "failed to parse any history from steam site"),
        }
    }
}

impl From<reqwest::Error> for FetchNewItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::TransportError(e)
    }
}

#[derive(Debug, Deserialize)]
pub struct Inventory {
    assets: Vec<Asset>,
    descriptions: Vec<InventoryDescription>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryDescription {
    #[serde(rename(deserialize = "classid"))]
    pub class_id: String,
    #[serde(rename(deserialize = "instanceid"))]
    pub instance_id: String,
    #[serde(rename(deserialize = "icon_url_large"))]
    pub icon_url: String,
    #[serde(rename(deserialize = "market_hash_name"))]
    pub name: String,
    #[serde(rename = "type")]
    pub variant: String,

    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    #[serde(rename(deserialize = "appid"))]
    app_id: u32,
    #[serde(rename(deserialize = "assetid"))]
    asset_id: u64,
    #[serde(rename(deserialize = "classid"))]
    class_id: u64,
    #[serde(rename(deserialize = "instanceid"))]
    instance_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Action {
    link: String,
    name: String,
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
    cookie_str: String,
}

impl SteamClient {
    pub fn new(
        username: String,
        user_id: u64,
        creds: SteamCredentials,
    ) -> Result<Self, Infallible> {
        let http_client = Client::builder().build().unwrap();

        let cookie_str = creds.into_string();

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
            cookie_str,
        })
    }

    pub async fn fetch_new_items(
        &self,
        since: Option<&DateTime<Utc>>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchNewItemsError> {
        let unhydrated = self.fetch_new_unprepared_items(since).await?;
        Ok(self
            .prepare_unlocks(unhydrated, self.username.clone())
            .await
            .unwrap())
    }

    async fn fetch_new_unprepared_items(
        &self,
        since: Option<&DateTime<Utc>>,
    ) -> Result<Vec<RawUnlock>, FetchNewItemsError> {
        let resp = self
            .http_client
            .get(self.inventory_history_url.clone())
            .header("Cookie", &self.cookie_str)
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

        let mut unlocks: Vec<RawUnlock> = Vec::new();

        for trade in trades {
            match parse_raw_unlock(trade, since) {
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

    async fn prepare_unlocks(
        &self,
        items: Vec<RawUnlock>,
        name: String,
    ) -> Result<Vec<UnhydratedUnlock>, Infallible> {
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

        let asset_map: HashMap<InventoryId, Asset> =
            inv.assets
                .into_iter()
                .fold(HashMap::new(), |mut acc, item| {
                    let id = InventoryId {
                        class_id: item.class_id,
                        instance_id: item.instance_id,
                    };

                    acc.insert(id, item);
                    acc
                });

        let results = items
            .into_iter()
            .map(|i| {
                let case = i.case;
                let key = i.key;
                let item_data = data_map.get(&i.item).unwrap();
                let item_asset = asset_map.get(&i.item).unwrap();

                let item_market_name = item_data.name.clone();
                let link_tpl = item_data
                    .actions
                    .iter()
                    .find(|a| {
                        a.name.starts_with("Inspect") && a.link.starts_with("steam://rungame/730/")
                    })
                    .unwrap();

                let item_market_link = link_tpl
                    .link
                    .replacen("%assetid%", &item_asset.asset_id.to_string(), 1)
                    .replacen("%owner_steamid%", &self.user_id.to_string(), 1);

                let at = i.at;
                let name = name.clone();

                UnhydratedUnlock {
                    key,
                    case,
                    item_market_link,
                    item_market_name,
                    at,
                    name,
                }
            })
            .collect();

        Ok(results)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawMarketPrices {
    lowest_price: String,
    volume: String,
}

impl TryInto<MarketPrices> for RawMarketPrices {
    type Error = Infallible;

    fn try_into(self) -> Result<MarketPrices, Self::Error> {
        let (_, lowest_price_str) = self.lowest_price.split_at(1);
        let lowest_price: f32 = lowest_price_str.parse().unwrap();
        let volume: i32 = self.volume.parse().unwrap();

        Ok(MarketPrices {
            lowest_price,
            volume,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPrices {
    lowest_price: f32,
    volume: i32,
}

pub async fn get_market_price(
    client: &Client,
    market_name: &str,
) -> Result<MarketPrices, Infallible> {
    let url = format!(
        "https://steamcommunity.com/market/priceoverview/?appid=730&currency=1&market_hash_name={}",
        market_name
    );
    let resp: RawMarketPrices = client.get(url).send().await.unwrap().json().await.unwrap();

    resp.try_into()
}

pub struct MarketPriceClient {
    client: Client,
    cache: Cache<MarketPrices>,
}

impl MarketPriceClient {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self, RedisError> {
        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(bb8_redis::bb8::Pool::builder().build(mgr).await?);
        let client = Client::new();

        let cache = Cache::new(pool, "market".to_string());

        Ok(Self { client, cache })
    }

    pub async fn get(&self, market_name: &str) -> Result<MarketPrices, Infallible> {
        if let Some(price) = self.cache.get(market_name).await? {
            return Ok(price);
        }

        let price = get_market_price(&self.client, market_name).await?;

        if let Err(e) = self.cache.set(market_name, &price).await {
            eprintln!("error updating market cache: {}", e);
        }

        Ok(price)
    }
}
