use std::fmt::{self, Display};
use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection, RunError};
pub use bb8_redis::redis::aio::Connection;
pub use bb8_redis::redis::{self, IntoConnectionInfo, RedisError, RedisResult};
use bb8_redis::redis::{from_redis_value, AsyncCommands, Client, FromRedisValue, ToRedisArgs};
use bb8_redis::RedisConnectionManager;
use futures_util::{Stream, StreamExt};

use crate::steam::{UnhydratedUnlock, Unlock};

type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug)]
pub enum Error {
    ConnectionTimeout,
    Redis(RedisError),
    Serde(serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionTimeout => write!(f, "connection timeout"),
            Self::Redis(e) => write!(f, "error interacting with redis: {}", e),
            Self::Serde(e) => write!(f, "error serialising/deserialising: {}", e),
        }
    }
}

impl From<RunError<RedisError>> for Error {
    fn from(e: RunError<RedisError>) -> Self {
        match e {
            RunError::User(e) => Self::Redis(e),
            RunError::TimedOut => Self::ConnectionTimeout,
        }
    }
}

impl From<RedisError> for Error {
    fn from(e: RedisError) -> Self {
        Self::Redis(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
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
        Ok(match redis_keys.len() {
            0 => vec![],
            1 => conn.get(&redis_keys.get(0).unwrap()).await?,
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

    pub async fn publish(&self, entry: &Unlock) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let _res: () = conn.publish(&EVENT_KEY, entry).await?;

        Ok(())
    }

    pub async fn get_event_stream(&self) -> Result<impl Stream<Item = Unlock>> {
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
