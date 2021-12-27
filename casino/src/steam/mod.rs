use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use crate::cache::Cache;
pub use crate::csgofloat::ItemDescription;
use crate::steam::errors::{FetchItemsError, FetchNewUnpreparedItemsError, PrepareItemsError};
pub use crate::steam::parsing::TrivialItem;
use crate::steam::parsing::{
    is_authenticated, parse_raw_unlock, InventoryId, ParseSuccess, RawUnlock, TRADE_SELECTOR,
};

use bb8_redis::bb8::Pool;
use bb8_redis::redis::{IntoConnectionInfo, RedisError};
use bb8_redis::RedisConnectionManager;
use chrono::{DateTime, Utc};
use reqwest::{Client, Request, StatusCode, Url};
use scraper::Html;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;

use self::errors::MarketPriceFetchError;

pub mod errors;
mod parsing;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnhydratedUnlock {
    pub history_id: String,

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
}

#[derive(Debug, Deserialize)]
pub struct Inventory {
    assets: Vec<Asset>,
    descriptions: Vec<InventoryDescription>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryDescription {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "classid"))]
    pub class_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "instanceid"))]
    pub instance_id: u64,
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
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "appid"))]
    app_id: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "assetid"))]
    asset_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "classid"))]
    class_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "instanceid"))]
    instance_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Action {
    link: String,
    name: String,
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

    fn inv_history_req(&self) -> Request {
        self.http_client
            .get(self.inventory_history_url.clone())
            .header("Cookie", &self.cookie_str)
            .build()
            .unwrap()
    }

    pub async fn fetch_new_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_id: Option<&str>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchItemsError> {
        let unhydrated = self.fetch_new_unprepared_items(since, last_id).await?;
        let prepared = self
            .prepare_unlocks(unhydrated, self.username.clone())
            .await?;

        Ok(prepared)
    }

    async fn fetch_new_unprepared_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_id: Option<&str>,
    ) -> Result<Vec<RawUnlock>, FetchNewUnpreparedItemsError> {
        let resp = self.http_client.execute(self.inv_history_req()).await?;

        match resp.status() {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(FetchNewUnpreparedItemsError::AuthenticationFailure)
            }
            status => return Err(FetchNewUnpreparedItemsError::UnhandledStatusCode(status)),
        }

        let data = resp.text().await?;
        let parsed_data = Html::parse_document(&data);

        if !is_authenticated(&parsed_data)? {
            return Err(FetchNewUnpreparedItemsError::NotAuthenticated);
        }

        let trades = parsed_data.select(&TRADE_SELECTOR);
        let mut seen_any = false;

        let mut unlocks: Vec<RawUnlock> = Vec::new();

        for trade in trades {
            match parse_raw_unlock(trade, since, last_id)? {
                ParseSuccess::ValidItem(v) => unlocks.push(v),
                ParseSuccess::TooOld => return Ok(unlocks),
                ParseSuccess::WrongTransactionType => {
                    seen_any = true;
                    continue;
                }
            }
        }

        if !seen_any {
            return Err(FetchNewUnpreparedItemsError::NoHistoryFound);
        }

        Ok(unlocks)
    }

    async fn prepare_unlocks(
        &self,
        items: Vec<RawUnlock>,
        name: String,
    ) -> Result<Vec<UnhydratedUnlock>, PrepareItemsError> {
        let resp = self
            .http_client
            .get(self.inventory_url.clone())
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let inv: Inventory = serde_json::from_slice(&resp)?;
        let data_map: HashMap<InventoryId, InventoryDescription> = inv
            .descriptions
            .into_iter()
            .fold(HashMap::new(), |mut acc, item| {
                let id = InventoryId::new(item.class_id, item.instance_id);
                acc.insert(id, item);
                acc
            });

        let asset_map: HashMap<InventoryId, Asset> =
            inv.assets
                .into_iter()
                .fold(HashMap::new(), |mut acc, item| {
                    let id = InventoryId::new(item.class_id, item.instance_id);
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

                let history_id = i.history_id;
                let at = i.at;
                let name = name.clone();

                UnhydratedUnlock {
                    history_id,

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawMarketPrices {
    lowest_price: Option<String>,
    median_price: Option<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    volume: i32,
}

impl From<RawMarketPrices> for MarketPrices {
    fn from(raw: RawMarketPrices) -> Self {
        Self {
            lowest_price: raw.lowest_price.as_deref().map(parse_currency).flatten(),
            median_price: raw.median_price.as_deref().map(parse_currency).flatten(),
            volume: raw.volume,
        }
    }
}

fn parse_currency(amt: &str) -> Option<f32> {
    let mut chars = amt.chars();
    chars.next();
    chars.as_str().parse::<f32>().ok()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketPrices {
    lowest_price: Option<f32>,
    median_price: Option<f32>,
    volume: i32,
}

pub async fn get_market_price(
    client: &Client,
    market_name: &str,
) -> Result<MarketPrices, MarketPriceFetchError> {
    let url = format!(
        "https://steamcommunity.com/market/priceoverview/?appid=730&currency=1&market_hash_name={}",
        market_name
    );
    let resp = client.get(url).send().await?.bytes().await?;
    let parsed: RawMarketPrices = serde_json::from_slice(&resp)?;

    Ok(parsed.into())
}

pub struct MarketPriceClient {
    client: Client,
    cache: Cache<MarketPrices>,
}

impl MarketPriceClient {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self, RedisError> {
        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(Pool::builder().build(mgr).await?);
        let client = Client::new();

        let cache = Cache::new(pool, "market".to_string());

        Ok(Self { client, cache })
    }

    pub async fn get(&self, market_name: &str) -> Result<MarketPrices, MarketPriceFetchError> {
        match self.cache.get(market_name).await {
            Ok(Some(price)) => return Ok(price),
            Ok(None) => (),
            Err(e) => eprintln!("failed to read entry from cache: {:?}", e),
        };

        let price = get_market_price(&self.client, market_name).await?;

        if let Err(e) = self.cache.set(market_name, &price).await {
            eprintln!("error updating market cache: {:?}", e);
        }

        Ok(price)
    }
}
