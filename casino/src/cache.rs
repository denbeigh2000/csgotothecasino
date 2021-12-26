use std::collections::HashMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection};
use bb8_redis::redis::AsyncCommands;
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Cache<T: DeserializeOwned> {
    pool: Arc<Pool<RedisConnectionManager>>,
    key: String,
    _data: PhantomData<T>,
}

impl<T: DeserializeOwned + Serialize> Cache<T> {
    pub fn new(pool: Arc<Pool<RedisConnectionManager>>, key: String) -> Self {
        let _data = PhantomData;
        Self { pool, key, _data }
    }

    async fn get_conn<'a, 'b>(
        &'a self,
    ) -> Result<PooledConnection<'b, RedisConnectionManager>, Infallible>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await.unwrap())
    }

    fn format_key(&self, given: &str) -> String {
        format!("{}_{}", self.key, given)
    }

    pub async fn get(&self, key: &str) -> Result<Option<T>, Infallible> {
        let redis_key = self.format_key(key);
        let mut conn = self.get_conn().await?;

        let res_raw: Option<Vec<u8>> = conn.get(&redis_key).await.unwrap();
        let res_raw = match res_raw {
            Some(r) => r,
            None => return Ok(None),
        };

        Ok(Some(serde_json::from_slice(&res_raw).unwrap()))
    }

    pub async fn get_bulk(&self, keys: &[&str]) -> Result<HashMap<String, T>, Infallible> {
        // NOTE: We defer to the singular variety here if we have a single item
        // to retreieve, because redis-rs' internal implementation can't
        // distinguish between a single item and a single-len vec, meaning it
        // issues a GET instead of an MGET, and returns a non-vec response.
        if keys.is_empty() {
            return Ok(HashMap::new());
        } else if keys.len() == 1 {
            let i = keys.get(0).unwrap();
            return match self.get(i).await? {
                None => Ok(HashMap::new()),
                Some(r) => {
                    let mut m = HashMap::with_capacity(1);
                    m.insert(i.to_string(), r);
                    Ok(m)
                }
            };
        }

        let mut conn = self.get_conn().await?;
        let redis_keys: Vec<String> = keys.iter().map(|k| self.format_key(k)).collect();
        let raw_results: Vec<Option<String>> = conn.get(redis_keys).await.unwrap();

        let results =
            raw_results
                .into_iter()
                .zip(keys.iter())
                .fold(HashMap::new(), |mut acc, (raw, key)| {
                    if let Some(r) = raw {
                        let parsed: T = serde_json::from_str(&r).unwrap();
                        acc.insert(key.to_string(), parsed);
                    }

                    acc
                });

        Ok(results)
    }

    pub async fn set(&self, key: &str, data: &T) -> Result<(), Infallible> {
        let redis_key = self.format_key(key);
        let serialised = serde_json::to_vec(data).unwrap();

        let mut conn = self.get_conn().await?;

        let _: () = conn.set(redis_key, serialised).await.unwrap();

        Ok(())
    }

    pub async fn set_bulk(&self, entries: &HashMap<String, T>) -> Result<(), Infallible> {
        let serialised: Vec<(String, Vec<u8>)> = entries
            .iter()
            .map(|(k, v)| {
                let key = self.format_key(k);
                let data = serde_json::to_vec(v).unwrap();

                (key, data)
            })
            .collect();

        let mut conn = self.get_conn().await.unwrap();
        let _: () = conn.set_multiple(&serialised).await.unwrap();

        Ok(())
    }
}
