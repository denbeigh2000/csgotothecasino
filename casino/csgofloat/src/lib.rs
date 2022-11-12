use std::collections::HashMap;
use std::sync::Arc;

use bb8_redis::bb8::Pool;
use bb8_redis::redis::{IntoConnectionInfo, RedisError};
use bb8_redis::RedisConnectionManager;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Body, Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cache::Cache;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sticker {
    #[serde(alias = "stickerId")]
    sticker_id: u32,
    slot: u8,
    codename: String,
    material: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct FloatItemResponse {
    pub iteminfo: ItemDescription,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDescription {
    origin: u32,
    quality: u32,
    rarity: u32,
    a: String,
    d: String,
    #[serde(alias = "paintseed")]
    paint_seed: u32,
    #[serde(alias = "defindex")]
    def_index: u32,
    stickers: Vec<Sticker>,
    #[serde(alias = "floatvalue")]
    float_value: f32,
    s: String,
    m: String,
    #[serde(alias = "imageurl")]
    image_url: Option<String>,
    min: f32,
    max: f32,
    weapon_type: String,
    item_name: String,
    rarity_name: String,
    quality_name: String,
    origin_name: String,
    wear_name: Option<String>,
    full_item_name: String,
}

#[derive(Debug, Error)]
#[repr(u8)]
pub enum CsgoFloatError {
    #[error("Improper parameter structure")]
    ImproperParameterStructure = 1,
    #[error("Invalid Inspect Link Structure")]
    InvalidInspectLinkStructure = 2,
    #[error("You have too many pending requests open at once")]
    TooManyPendingRequests = 3,
    #[error("Valve's servers didn't reply in time")]
    ValveServerTimeout = 4,
    #[error("Valve's servers appear to be offline, please try again later")]
    ValveOffline = 5,
    #[error("Something went wrong on our end, please try again")]
    CsgoFloatInternalError = 6,
    #[error("Something went wrong on our end, please try again")]
    ImproperBodyFormat = 7,
    #[error("Bad Secret")]
    BadSecret = 8,
}

#[derive(Debug, Error)]
pub enum CsgoFloatFetchError {
    #[error("error from api: {0}")]
    CsgoFloat(#[from] CsgoFloatError),
    #[error("http error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("deserialisation error: {0}")]
    Deserializing(#[from] serde_json::Error),
    #[error("error parsing steam url: {0}")]
    SteamURLFormat(#[from] SteamURLParseError),
}

#[derive(Debug, Error)]
pub enum SteamURLParseError {
    #[error("url missing \"A\" marker")]
    MissingAssetMarker,
    #[error("url missing \"D\" marker")]
    MissingDMarker,
}

pub async fn get_by_market_url(
    client: &Client,
    key: &str,
    market_url: &str,
) -> Result<ItemDescription, CsgoFloatFetchError> {
    let url = format!("https://api.csgofloat.com?url={}", market_url);
    let resp = client.get(&url).header(AUTHORIZATION, key).send().await?;

    match resp.status() {
        StatusCode::OK => {
            let data = resp.bytes().await?;
            let data: FloatItemResponse = serde_json::from_slice(&data)?;
            Ok(data.iteminfo)
        }
        status => {
            log::error!("CSGOFloat responded with error status {}", status);
            resp.json().await.map_err(|e| e.into())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BulkRequestItem {
    pub link: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BulkRequest {
    pub links: Vec<BulkRequestItem>,
}

pub async fn get_bulk_by_market_url(
    client: &Client,
    key: &str,
    urls: &[&str],
) -> Result<HashMap<String, ItemDescription>, CsgoFloatFetchError> {
    let url_map: HashMap<String, String> =
        urls.iter().try_fold(HashMap::new(), |mut acc, url| {
            let key = url
                .split('A')
                .nth(1)
                .ok_or(SteamURLParseError::MissingAssetMarker)?
                .split('D')
                .next()
                .ok_or(SteamURLParseError::MissingDMarker)?
                .to_string();
            acc.insert(url.to_string(), key);

            Ok::<_, SteamURLParseError>(acc)
        })?;

    let links = urls
        .iter()
        .map(|l| BulkRequestItem {
            link: l.to_string(),
        })
        .collect();
    let bulk_req = BulkRequest { links };
    let req_data = serde_json::to_vec(&bulk_req)?;

    let req = client
        .post("https://api.csgofloat.com/bulk")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, key)
        .body(Body::from(req_data));

    let body = req.send().await?.text().await?;
    let resp: HashMap<String, ItemDescription> = serde_json::from_str(&body)?;

    let items_by_url = url_map
        .into_iter()
        .fold(HashMap::new(), |mut acc, (url, asset_id)| {
            let item = resp.get(&asset_id).unwrap();
            acc.insert(url, item.clone());

            acc
        });

    Ok(items_by_url)
}

#[derive(Debug, Error)]
#[error("error creating csgofloat client cache: {0}")]
pub struct CsgoFloatClientCreateError(#[from] RedisError);

pub struct CsgoFloatClient {
    key: String,
    cache: Cache<ItemDescription>,
    client: Client,
}

impl CsgoFloatClient {
    pub async fn new<S: Into<String>, T: IntoConnectionInfo>(
        key: S,
        i: T,
    ) -> Result<Self, CsgoFloatClientCreateError> {
        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(Pool::builder().build(mgr).await?);

        let cache = Cache::new(pool, "floatcache".to_string());
        let client = Client::new();

        let key = key.into();

        Ok(Self { key, cache, client })
    }

    pub async fn get(&self, url: &str) -> Result<ItemDescription, CsgoFloatFetchError> {
        match self.cache.get(url).await {
            Ok(Some(entry)) => return Ok(entry),
            Ok(None) => (),
            Err(e) => log::warn!("error fetching from cache: {}", e),
        };

        let res = get_by_market_url(&self.client, &self.key, url).await?;

        if let Err(e) = self.cache.set(url, &res).await {
            log::warn!("failed to set cache entry: {}", e);
        }

        Ok(res)
    }

    pub async fn get_bulk(
        &self,
        urls: &[&str],
    ) -> Result<HashMap<String, ItemDescription>, CsgoFloatFetchError> {
        let res = self.cache.get_bulk(urls).await.unwrap_or_else(|e| {
            log::warn!("failed to get items from cache: {}", e);
            HashMap::with_capacity(0)
        });
        let missing: Vec<&str> = urls
            .iter()
            .filter(|&u| !res.contains_key(*u))
            .copied()
            .collect();

        if missing.is_empty() {
            return Ok(res);
        }

        let mut fresh = HashMap::with_capacity(missing.len());
        for item in missing {
            let desc = get_by_market_url(&self.client, &self.key, item).await?;
            fresh.insert(item.to_string(), desc);
        }

        if let Err(e) = self.cache.set_bulk(&fresh).await {
            log::warn!("failed to set items in cache: {}", e);
        }

        let res = fresh.into_iter().fold(res, |mut acc, (k, v)| {
            acc.insert(k, v);
            acc
        });

        Ok(res)
    }
}
