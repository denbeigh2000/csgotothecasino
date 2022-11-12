use std::sync::Arc;

use bb8_redis::RedisConnectionManager;
use bb8_redis::redis::IntoConnectionInfo;
use bb8_redis::bb8::Pool;
use bb8_redis::redis::RedisError;
use cache::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::errors::MarketPriceFetchError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawMarketPrices {
    lowest_price: Option<String>,
    median_price: Option<String>,
    volume: Option<String>,
}

impl From<RawMarketPrices> for MarketPrices {
    fn from(raw: RawMarketPrices) -> Self {
        let volume = raw.volume.map(|v| v.replace(',', "").parse().unwrap());
        Self {
            lowest_price: raw.lowest_price.as_deref().and_then(parse_currency),
            median_price: raw.median_price.as_deref().and_then(parse_currency),
            volume,
        }
    }
}

fn parse_currency(amt: &str) -> Option<f32> {
    let chars = amt.replace(',', "");
    let mut chars = chars.chars();
    chars.next();
    chars.as_str().parse::<f32>().ok()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketPrices {
    lowest_price: Option<f32>,
    median_price: Option<f32>,
    volume: Option<i32>,
}

pub async fn get_market_price(
    client: &Client,
    market_name: &str,
) -> Result<MarketPrices, MarketPriceFetchError> {
    let url = format!(
        "https://steamcommunity.com/market/priceoverview/?appid=730&currency=1&market_hash_name={}",
        market_name
    );

    let resp = client.get(url).send().await?.text().await?;
    let parsed: RawMarketPrices = serde_json::from_str(&resp)?;

    Ok(parsed.into())
}

#[derive(Debug, Error)]
pub enum MarketPriceClientCreateError {
    #[error("invalid redis url given: {0}")]
    InvalidRedisUrl(RedisError),
    #[error("error communicating with redis: {0}")]
    Redis(#[from] RedisError),
}

pub struct MarketPriceClient {
    client: Client,
    cache: Cache<MarketPrices>,
}

impl MarketPriceClient {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self, MarketPriceClientCreateError> {
        let conn_info = i
            .into_connection_info()
            .map_err(MarketPriceClientCreateError::InvalidRedisUrl)?;
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
            Err(e) => log::warn!("failed to read entry from cache: {}", e),
        };

        let price = get_market_price(&self.client, market_name).await?;

        if let Err(e) = self.cache.set(market_name, &price).await {
            log::warn!("error updating market cache: {}", e);
        }

        Ok(price)
    }
}
