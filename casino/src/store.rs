use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection, RunError};
pub use bb8_redis::redis::aio::Connection;
use bb8_redis::redis::{from_redis_value, AsyncCommands, Client, FromRedisValue, ToRedisArgs};
pub use bb8_redis::redis::{IntoConnectionInfo, RedisError};
use bb8_redis::RedisConnectionManager;
use futures_util::{Stream, StreamExt};

use crate::steam::{UnhydratedUnlock, Unlock};

const EVENT_KEY: &str = "new_events";

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

impl FromRedisValue for UnhydratedUnlock {
    fn from_redis_value(v: &bb8_redis::redis::Value) -> bb8_redis::redis::RedisResult<Self> {
        let data: Vec<u8> = from_redis_value(v)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }
}

impl ToRedisArgs for UnhydratedUnlock {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + bb8_redis::redis::RedisWrite,
    {
        let data = serde_json::to_vec(self).unwrap();
        out.write_arg(&data)
    }
}

impl FromRedisValue for Unlock {
    fn from_redis_value(v: &bb8_redis::redis::Value) -> bb8_redis::redis::RedisResult<Self> {
        let data: Vec<u8> = from_redis_value(v)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }
}

impl ToRedisArgs for Unlock {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + bb8_redis::redis::RedisWrite,
    {
        let data = serde_json::to_vec(self).unwrap();
        out.write_arg(&data)
    }
}

#[derive(Debug)]
pub enum StoreError {
    ConnectionTimeout,
    RedisError(RedisError),
    SerdeError(serde_json::Error),
}

impl From<RunError<RedisError>> for StoreError {
    fn from(e: RunError<RedisError>) -> Self {
        match e {
            RunError::User(e) => Self::RedisError(e),
            RunError::TimedOut => Self::ConnectionTimeout,
        }
    }
}

impl From<RedisError> for StoreError {
    fn from(e: RedisError) -> Self {
        Self::RedisError(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl Store {
    pub async fn new<T: IntoConnectionInfo>(i: T) -> Result<Self, StoreError> {
        let conn_info = i.into_connection_info()?;
        let mgr = RedisConnectionManager::new(conn_info.clone())?;
        let pool = Arc::new(bb8_redis::bb8::Pool::builder().build(mgr).await?);
        let client = Client::open(conn_info)?;

        Ok(Self { client, pool })
    }

    async fn make_conn(&self) -> Result<Connection, RedisError> {
        self.client.get_async_connection().await
    }

    async fn get_conn<'a, 'b>(
        &'a self,
    ) -> Result<PooledConnection<'b, RedisConnectionManager>, StoreError>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await?)
    }

    pub async fn get_entries(&self) -> Result<Vec<UnhydratedUnlock>, StoreError> {
        let mut conn = self.get_conn().await?;
        let keys: Vec<String> = match conn.zrevrange("entries", 0, -1).await? {
            Some(keys) => keys,
            None => return Ok(Vec::new()),
        };
        let redis_keys: Vec<String> = keys.iter().map(|k| format!("unlock_{}", k)).collect();
        let values: Vec<UnhydratedUnlock> = conn.get(&redis_keys).await?;

        Ok(values)
    }

    pub async fn append_entry(&self, entry: &UnhydratedUnlock) -> Result<(), StoreError> {
        let mut conn = self.get_conn().await?;
        let ts = entry.at.timestamp_millis();
        let id = &entry.history_id;
        let data_key = format!("unlock_{}", id);
        let data = serde_json::to_vec(&entry)?;
        let _res: () = bb8_redis::redis::pipe()
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

    pub async fn publish(&self, entry: &Unlock) -> Result<(), StoreError> {
        let mut conn = self.get_conn().await?;
        let _res: () = conn.publish(&EVENT_KEY, entry).await?;

        Ok(())
    }

    pub async fn get_event_stream(&self) -> Result<impl Stream<Item = Unlock>, StoreError> {
        let mut conn = self.make_conn().await?.into_pubsub();
        conn.subscribe(EVENT_KEY).await?;

        let stream = conn.into_on_message().filter_map(|msg| async move {
            let raw_data: String = match msg.get_payload() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("failed to decode raw data: {}", e);
                    return None;
                }
            };

            match serde_json::from_str(&raw_data) {
                Ok(u) => Some(u),
                Err(e) => {
                    eprintln!("failed to unmarshal response to json: {}", e);
                    eprintln!("raw data was {}", &raw_data);
                    None
                }
            }
        });

        Ok(stream)
    }
}
