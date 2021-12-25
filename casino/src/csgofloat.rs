use core::fmt;
use std::{collections::HashMap, convert::Infallible, fmt::Display, sync::Arc};

use bb8_redis::{bb8::Pool, RedisConnectionManager, redis::{IntoConnectionInfo, RedisError}};
use hyper::Body;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use crate::cache::Cache;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sticker {
    #[serde(rename(deserialize = "stickerId"))]
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
    #[serde(rename(deserialize = "paintseed"))]
    paint_seed: u32,
    #[serde(rename(deserialize = "defindex"))]
    def_index: u32,
    stickers: Vec<Sticker>,
    #[serde(rename(deserialize = "floatid"))]
    float_id: String,
    #[serde(rename(deserialize = "floatvalue"))]
    float_value: f32,
    s: String,
    m: String,
    #[serde(rename(deserialize = "imageurl"))]
    image_url: String,
    min: f32,
    max: f32,
    weapon_type: String,
    item_name: String,
    rarity_name: String,
    quality_name: String,
    origin_name: String,
    wear_name: String,
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
    Deserializing,
}

impl From<reqwest::Error> for CsgoFloatFetchError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            eprintln!("Error deserializing JSON response: {}", e);
            Self::Deserializing
        } else {
            Self::Transport(e)
        }
    }
}

pub async fn get_by_market_url(
    client: &Client,
    market_url: &str,
) -> Result<ItemDescription, CsgoFloatFetchError> {
    let url = format!("https://api.csgofloat.com?url={}", market_url);
    let resp = client.get(url).send().await?;

    match resp.status() {
        StatusCode::OK => {
            let data: FloatItemResponse = resp.json().await?;
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
    urls: &Vec<&str>,
) -> Result<HashMap<String, ItemDescription>, CsgoFloatFetchError> {
    let url_map: HashMap<String, String> = urls.iter().fold(HashMap::new(), |mut acc, url| {
        let key = url
            .split('A')
            .nth(1)
            .unwrap()
            .split('D')
            .next()
            .unwrap()
            .to_string();
        acc.insert(url.to_string(), key);

        acc
    });

    let links = urls
        .iter()
        .map(|l| BulkRequestItem {
            link: l.to_string(),
        })
        .collect();
    let bulk_req = BulkRequest { links };
    let req_data = serde_json::to_vec(&bulk_req).unwrap();

    let resp: HashMap<String, ItemDescription> = client
        .post("https://api.csgofloat.com/bulk")
        .body(Body::from(req_data))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

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
    cache: Cache<ItemDescription>,
    client: Client,
}

impl CsgoFloatClient {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self, RedisError> {

        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(bb8_redis::bb8::Pool::builder().build(mgr).await?);

        let cache = Cache::new(pool, "floatcache".to_string());
        let client = Client::new();

        Ok(Self { cache, client })
    }

    pub async fn get(&self, url: &str) -> Result<ItemDescription, Infallible> {
        if let Some(entry) = self.cache.get(url).await.unwrap() {
            return Ok(entry);
        }

        let res = get_by_market_url(&self.client, url).await.unwrap();
        self.cache.set(url, &res).await?;

        Ok(res)
    }

    pub async fn get_bulk(
        &self,
        urls: &[&str],
    ) -> Result<HashMap<String, ItemDescription>, Infallible> {
        let res = self.cache.get_bulk(urls).await?;
        let missing: Vec<&str> = urls
            .iter()
            .filter(|u| res.contains_key(**u))
            .map(|u| *u)
            .collect();
        if missing.is_empty() {
            return Ok(res);
        }

        let fresh = get_bulk_by_market_url(&self.client, &missing)
            .await
            .unwrap();

        self.cache.set_bulk(&fresh).await.unwrap();
        let res = fresh.into_iter().fold(res, |mut acc, (k, v)| {
            acc.insert(k, v);
            acc
        });

        Ok(res)
    }
}
