#![allow(clippy::let_unit_value)]

use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection, RunError};
pub use bb8_redis::redis::aio::Connection;
pub use bb8_redis::redis::{self, IntoConnectionInfo, RedisError, RedisResult};
use bb8_redis::redis::{AsyncCommands, Client};
use bb8_redis::RedisConnectionManager;
use futures_util::{Stream, StreamExt};
use thiserror::Error;

use countdown::CountdownRequest;
use steam::{UnhydratedUnlock, Unlock};

type Result<T> = std::result::Result<T, StoreError>;

const UNLOCK_EVENT_KEY: &str = "new_unlock_events";
const SYNC_EVENT_KEY: &str = "new_sync_events";

/// Persists information about our application state.
pub struct Store {
    client: Client,
    pool: Arc<Pool<RedisConnectionManager>>,
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            pool: Arc::clone(&self.pool),
        }
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("connection timeout")]
    ConnectionTimeout,
    #[error("error interacting with redis: {0}")]
    Redis(#[from] RedisError),
    #[error("error serialising/deserialising: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<RunError<RedisError>> for StoreError {
    fn from(e: RunError<RedisError>) -> Self {
        match e {
            RunError::User(e) => Self::Redis(e),
            RunError::TimedOut => Self::ConnectionTimeout,
        }
    }
}

impl Store {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self> {
        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(bb8_redis::bb8::Pool::builder().build(mgr).await?);
        let client = Client::open(conn_info)?;

        Ok(Self { client, pool })
    }

    async fn make_conn(&self) -> RedisResult<Connection> {
        self.client.get_async_connection().await
    }

    async fn get_conn<'a, 'b>(&'a self) -> Result<PooledConnection<'b, RedisConnectionManager>>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await?)
    }

    pub async fn get_entries(&self) -> Result<Vec<UnhydratedUnlock>> {
        let mut conn = self.get_conn().await?;
        let keys: Vec<String> = match conn.zrevrange("entries", 0, -1).await? {
            Some(keys) => keys,
            None => return Ok(Vec::new()),
        };
        let redis_keys: Vec<String> = keys.iter().map(|k| format!("unlock_{}", k)).collect();
        Ok(match &redis_keys[..] {
            [] => vec![],
            [only] => conn.get(only).await?,
            _ => conn.get(redis_keys).await?,
        })
    }

    pub async fn append_entry(&self, entry: &UnhydratedUnlock) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let ts = entry.at.timestamp_millis();
        let id = &entry.history_id;
        let data_key = format!("unlock_{}", id);
        let data = serde_json::to_vec(&entry)?;
        let _res: () = redis::pipe()
            .cmd("ZADD")
            .arg("entries")
            .arg(ts)
            .arg(id)
            .cmd("SET")
            .arg(data_key)
            .arg(data)
            .query_async(&mut *conn)
            .await?;

        Ok(())
    }

    pub async fn publish_unlock(&self, entry: &Unlock) -> Result<()> {
        self.publish(UNLOCK_EVENT_KEY, entry).await
    }

    pub async fn start_countdown(&self, entry: &CountdownRequest) -> Result<()> {
        self.publish(SYNC_EVENT_KEY, entry).await
    }

    async fn publish<T: redis::ToRedisArgs + std::marker::Sync>(
        &self,
        topic: &str,
        entry: &T,
    ) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let _res: () = conn.publish(topic, entry).await?;

        Ok(())
    }

    pub async fn get_unlock_stream(&self) -> Result<impl Stream<Item = Unlock>> {
        self.get_redis_stream(UNLOCK_EVENT_KEY).await
    }

    pub async fn get_sync_stream(&self) -> Result<impl Stream<Item = CountdownRequest>> {
        self.get_redis_stream(SYNC_EVENT_KEY).await
    }

    async fn get_redis_stream<T: serde::de::DeserializeOwned>(
        &self,
        topic: &str,
    ) -> Result<impl Stream<Item = T>> {
        let mut conn = self.make_conn().await?.into_pubsub();
        conn.subscribe(topic).await?;

        let stream = conn.into_on_message().filter_map(|msg| async move {
            let raw_data: String = match msg.get_payload() {
                Ok(d) => d,
                Err(e) => {
                    log::error!("failed to decode raw data: {}", e);
                    return None;
                }
            };

            match serde_json::from_str(&raw_data) {
                Ok(u) => Some(u),
                Err(e) => {
                    log::error!("failed to unmarshal response to json: {}", e);
                    log::error!("raw data was {}", &raw_data);
                    None
                }
            }
        });

        Ok(stream)
    }
}
