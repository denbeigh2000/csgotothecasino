use std::convert::Infallible;
use std::sync::Arc;

use bb8_redis::bb8::{Pool, PooledConnection};
use bb8_redis::redis::{from_redis_value, AsyncCommands, FromRedisValue, ToRedisArgs};
use bb8_redis::RedisConnectionManager;
use tokio::sync::watch::Sender;

use crate::steam::Unlock;

pub struct Store {
    pool: Arc<Pool<RedisConnectionManager>>,
    writer: Sender<Unlock>,
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

impl Store {
    async fn get_conn<'a, 'b>(
        &'a self,
    ) -> Result<PooledConnection<'b, RedisConnectionManager>, Infallible>
    where
        'a: 'b,
    {
        Ok(self.pool.get().await.unwrap())
    }

    pub async fn get_entries(&self) -> Result<Vec<Unlock>, Infallible> {
        let mut conn = self.get_conn().await?;
        let res: Option<Vec<Unlock>> = conn.lrange("unlocks", 0, -1).await.unwrap();

        Ok(res.unwrap_or_else(Vec::new))
    }

    pub async fn append_entry(&self, entry: Unlock) -> Result<(), Infallible> {
        let mut conn = self.get_conn().await?;
        let _res: () = conn.lpush("unlocks", &entry).await.unwrap();

        self.writer.send(entry).unwrap();
        Ok(())
    }
}
