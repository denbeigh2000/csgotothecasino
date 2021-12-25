use std::convert::Infallible;
use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection};
use bb8_redis::redis::AsyncCommands;
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Cache {
    pool: Arc<Pool<RedisConnectionManager>>,
    key: String,
}

impl Cache {
    pub fn new(pool: Arc<Pool<RedisConnectionManager>>, key: String) -> Self {
        Self { pool, key }
    }

    async fn get_conn<'a, 'b>(
        &'a self,
    ) -> Result<PooledConnection<'b, RedisConnectionManager>, Infallible>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await.unwrap())
    }

    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Infallible> {
        let redis_key = format!("{}_{}", self.key, key);
        let mut conn = self.get_conn().await?;

        let res_raw: Option<Vec<u8>> = conn.get(&redis_key).await.unwrap();
        let res_raw = match res_raw {
            Some(r) => r,
            None => return Ok(None),
        };

        Ok(Some(serde_json::from_slice(&res_raw).unwrap()))
    }

    async fn set<T: Serialize>(&self, key: &str, data: &T) -> Result<(), Infallible> {
        let redis_key = format!("{}_{}", self.key, key);
        let serialised = serde_json::to_vec(data).unwrap();

        let mut conn = self.get_conn().await?;

        let _: () = conn.set(redis_key, serialised).await.unwrap();

        Ok(())
    }
}
