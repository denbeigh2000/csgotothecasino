use std::collections::HashMap;

use bb8_redis::redis::{self, from_redis_value, FromRedisValue, RedisResult, ToRedisArgs};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CountdownRequest {
    pub delays: HashMap<String, u32>,
}

impl FromRedisValue for CountdownRequest {
    fn from_redis_value(v: &redis::Value) -> RedisResult<Self> {
        let data: Vec<u8> = from_redis_value(v)?;
        Ok(serde_json::from_slice(&data).unwrap())
    }
}

impl ToRedisArgs for CountdownRequest {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let data = serde_json::to_vec(self).unwrap();
        out.write_arg(&data)
    }
}
