use bb8_redis::redis::{from_redis_value, self, RedisResult, ToRedisArgs, FromRedisValue};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
pub use csgofloat::ItemDescription;

use crate::parsing::TrivialItem;
use crate::{UnhydratedUnlock, MarketPrices};

impl FromRedisValue for UnhydratedUnlock {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        let data: Vec<u8> = from_redis_value(v)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }
}

impl ToRedisArgs for UnhydratedUnlock {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let data = serde_json::to_vec(self).unwrap();
        out.write_arg(&data)
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Unlock {
    pub key: Option<TrivialItem>,
    pub case: TrivialItem,
    pub case_value: MarketPrices,
    pub item: ItemDescription,
    pub item_value: MarketPrices,

    pub at: DateTime<Utc>,
    pub name: String,
}

impl FromRedisValue for Unlock {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        let data: Vec<u8> = from_redis_value(v)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }
}

impl ToRedisArgs for Unlock {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let data = serde_json::to_vec(self).unwrap();
        out.write_arg(&data)
    }
}
