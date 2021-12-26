use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection, RunError};
use bb8_redis::redis::{AsyncCommands, RedisError};
use bb8_redis::RedisConnectionManager;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Cache<T: DeserializeOwned> {
    pool: Arc<Pool<RedisConnectionManager>>,
    key: String,
    _data: PhantomData<T>,
}

#[derive(Debug)]
pub enum CacheError {
    RedisError(RedisError),
    SerdeError(serde_json::Error),
    ConnectionTimeout,
}

impl From<RedisError> for CacheError {
    fn from(e: RedisError) -> Self {
        Self::RedisError(e)
    }
}

impl From<RunError<RedisError>> for CacheError {
    fn from(e: RunError<RedisError>) -> Self {
        match e {
            RunError::User(e) => Self::RedisError(e),
            RunError::TimedOut => Self::ConnectionTimeout,
        }
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl<T: DeserializeOwned + Serialize> Cache<T> {
    pub fn new(pool: Arc<Pool<RedisConnectionManager>>, key: String) -> Self {
        let _data = PhantomData;
        Self { pool, key, _data }
    }

    async fn get_conn<'a, 'b>(
        &'a self,
    ) -> Result<PooledConnection<'b, RedisConnectionManager>, CacheError>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await?)
    }

    fn format_key(&self, given: &str) -> String {
        format!("{}_{}", self.key, given)
    }

    pub async fn get(&self, key: &str) -> Result<Option<T>, CacheError> {
        let redis_key = self.format_key(key);
        let mut conn = self.get_conn().await?;

        let res_raw: Option<Vec<u8>> = conn.get(&redis_key).await?;
        let decoded = res_raw.map(|r| serde_json::from_slice(&r)).transpose()?;

        Ok(decoded)
    }

    pub async fn get_bulk(&self, keys: &[&str]) -> Result<HashMap<String, T>, CacheError> {
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
        let raw_results: Vec<Option<String>> = conn.get(redis_keys).await?;

        let results = raw_results.into_iter().zip(keys.iter()).try_fold(
            HashMap::new(),
            |mut acc, (raw, key)| {
                if let Some(r) = raw {
                    let parsed: T = serde_json::from_str(&r)?;
                    acc.insert(key.to_string(), parsed);
                }

                Ok::<_, serde_json::Error>(acc)
            },
        )?;

        Ok(results)
    }

    pub async fn set(&self, key: &str, data: &T) -> Result<(), CacheError> {
        let redis_key = self.format_key(key);
        let serialised = serde_json::to_vec(data)?;

        let mut conn = self.get_conn().await?;

        let _: () = conn.set(redis_key, serialised).await?;

        Ok(())
    }

    pub async fn set_bulk(&self, entries: &HashMap<String, T>) -> Result<(), CacheError> {
        let serialised: Vec<(String, Vec<u8>)> = entries
            .iter()
            .map(|(k, v)| {
                let key = self.format_key(k);
                let data = serde_json::to_vec(v)?;

                Ok((key, data))
            })
            .collect::<Result<_, serde_json::Error>>()?;

        let mut conn = self.get_conn().await?;
        let _: () = conn.set_multiple(&serialised).await?;

        Ok(())
    }
}
