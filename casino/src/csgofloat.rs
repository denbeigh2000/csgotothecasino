use core::fmt;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use bb8_redis::bb8::Pool;
use bb8_redis::redis::{IntoConnectionInfo, RedisError};
use bb8_redis::RedisConnectionManager;
use hyper_tungstenite::hyper::Body;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use crate::cache::Cache;

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

#[derive(Debug)]
pub struct CsgoFloatError {
    code: CsgoFloatErrorCode,
}

impl Display for CsgoFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CSGOFloat request failed: {}", self.code)
    }
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum CsgoFloatErrorCode {
    ImproperParameterStructure = 1,
    InvalidInspectLinkStructure = 2,
    TooManyPendingRequests = 3,
    ValveServerTimeout = 4,
    ValveOffline = 5,
    CsgoFloatInternalError = 6,
    ImproperBodyFormat = 7,
    BadSecret = 8,
}

impl Display for CsgoFloatErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImproperParameterStructure => write!(f, "Improper parameter structure"),
            Self::InvalidInspectLinkStructure => write!(f, "Invalid Inspect Link Structure"),
            Self::TooManyPendingRequests => {
                write!(f, "You have too many pending requests open at once")
            }
            Self::ValveServerTimeout => write!(f, "Valve's servers didn't reply in time"),
            Self::ValveOffline => write!(
                f,
                "Valve's servers appear to be offline, please try again later"
            ),
            Self::CsgoFloatInternalError => {
                write!(f, "Something went wrong on our end, please try again")
            }
            Self::ImproperBodyFormat => write!(f, "Improper body format"),
            Self::BadSecret => write!(f, "Bad Secret"),
        }
    }
}

#[derive(Debug)]
pub enum CsgoFloatFetchError {
    CsgoFloat(CsgoFloatError),
    Transport(reqwest::Error),
    Deserializing(serde_json::Error),
    SteamURLFormat(SteamURLParseError),
}

impl Display for CsgoFloatFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CsgoFloat(e) => write!(f, "error from api: {}", e),
            Self::Transport(e) => write!(f, "http error: {}", e),
            Self::Deserializing(e) => write!(f, "deserialisation error: {}", e),
            Self::SteamURLFormat(e) => write!(f, "error parsing steam url: {}", e),
        }
    }
}

impl From<SteamURLParseError> for CsgoFloatFetchError {
    fn from(e: SteamURLParseError) -> Self {
        Self::SteamURLFormat(e)
    }
}

#[derive(Debug)]
pub enum SteamURLParseError {
    MissingAssetMarker,
    MissingDMarker,
}

impl Display for SteamURLParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssetMarker => write!(f, "url missing \"A\" marker"),
            Self::MissingDMarker => write!(f, "url missing \"D\" marker"),
        }
    }
}

impl From<reqwest::Error> for CsgoFloatFetchError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

impl From<serde_json::Error> for CsgoFloatFetchError {
    fn from(e: serde_json::Error) -> Self {
        Self::Deserializing(e)
    }
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
            eprintln!("CSGOFloat responded with error status {}", status);
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
    eprintln!("received from csgofloat: {}", body);
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

pub struct CsgoFloatClient {
    key: String,
    cache: Cache<ItemDescription>,
    client: Client,
}

impl CsgoFloatClient {
    pub async fn new<S: Into<String>, T: IntoConnectionInfo>(
        key: S,
        i: T,
    ) -> Result<Self, RedisError> {
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
            Err(e) => eprintln!("error fetching from cache: {}", e),
        };

        let res = get_by_market_url(&self.client, &self.key, url).await?;

        if let Err(e) = self.cache.set(url, &res).await {
            eprintln!("failed to set cache entry: {}", e);
        }

        Ok(res)
    }

    pub async fn get_bulk(
        &self,
        urls: &[&str],
    ) -> Result<HashMap<String, ItemDescription>, CsgoFloatFetchError> {
        let res = self.cache.get_bulk(urls).await.unwrap_or_else(|e| {
            eprintln!("failed to get items from cache: {}", e);
            HashMap::with_capacity(0)
        });
        let missing: Vec<&str> = urls
            .iter()
            .filter(|u| !res.contains_key(**u))
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
            eprintln!("failed to set items in cache: {}", e);
        }

        let res = fresh.into_iter().fold(res, |mut acc, (k, v)| {
            acc.insert(k, v);
            acc
        });

        Ok(res)
    }
}
