use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use std::time::Duration;

use super::{Cache, CacheError};

/// Redis-backed implementation of the `Cache` trait.
///
/// `RedisCache` owns a multiplexed Redis connection which can be cloned
/// for individual operations. This allows multiple async tasks to use
/// the cache concurrently without creating a new Redis connection for
/// every operation.
pub struct RedisCache {
    connection: MultiplexedConnection,
}

impl RedisCache {
    /// Creates a new `RedisCache` and establishes a connection to Redis.
    ///
    /// `url` is expected to be a Redis connection URL, for example:
    /// `redis://localhost:6379`.
    pub async fn new(url: &str) -> Result<Self, CacheError> {
        // Create a Redis client from the supplied connection URL.
        //
        // Creating the client itself does not perform the actual
        // connection; the connection is established below.
        let client = redis::Client::open(url)?;

        // Establish an asynchronous, multiplexed connection to Redis.
        //
        // A multiplexed connection allows multiple commands to be
        // processed concurrently over the same underlying connection.
        let connection = client.get_multiplexed_async_connection().await?;

        Ok(Self { connection })
    }
}

/// Implements the application-level `Cache` abstraction using Redis.
#[async_trait]
impl Cache for RedisCache {
    /// Retrieves a value from Redis by its key.
    ///
    /// Returns `None` when the key does not exist.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        // Clone the multiplexed connection for this operation.
        //
        // MultiplexedConnection is cheap to clone and the clones can
        // safely be used by different async tasks.
        let mut connection = self.connection.clone();

        // Redis stores the value as bytes here.
        //
        // The expected return type is Option<Vec<u8>>:
        //
        //     Some(bytes) -> key exists
        //     None        -> key does not exist
        let value = connection.get(key).await?;

        Ok(value)
    }

    /// Stores a value in Redis with a time-to-live.
    ///
    /// Once `ttl` expires, Redis automatically removes the key.
    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<(), CacheError> {
        // Clone the shared multiplexed connection for this operation.
        let mut connection = self.connection.clone();

        // Convert Rust's Duration into seconds for Redis.
        let ttl_seconds = ttl.as_secs();

        // Store the value and give the key an expiration time.
        //
        // Redis returns a successful response that we don't need,
        // so explicitly tell Rust that we expect `()`.
        //
        // The explicit type is also necessary because newer Rust
        // versions may otherwise fail to infer the Redis response type.
        let _: () = connection.set_ex(key, value, ttl_seconds).await?;

        Ok(())
    }

    /// Deletes a value from Redis by its key.
    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        // Clone the multiplexed connection for this operation.
        let mut connection = self.connection.clone();

        // DEL returns the number of keys that were removed.
        //
        // We don't need that number for our Cache abstraction,
        // so explicitly discard the response as `()`.
        let _: () = connection.del(key).await?;

        Ok(())
    }

    async fn increment_with_expiry(&self, key: &str, ttl: Duration) -> Result<(u64, Duration), CacheError> {
        const SCRIPT: &str = r#"
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then
                redis.call('PEXPIRE', KEYS[1], ARGV[1])
            end
            return {count, redis.call('PTTL', KEYS[1])}
        "#;

        let mut connection = self.connection.clone();
        let ttl_ms = ttl.as_millis().max(1).min(u64::MAX as u128) as u64;
        let (count, remaining_ms): (u64, i64) = redis::Script::new(SCRIPT).key(key).arg(ttl_ms).invoke_async(&mut connection).await?;
        Ok((count, Duration::from_millis(remaining_ms.max(1) as u64)))
    }

    async fn decrement(&self, key: &str) -> Result<(), CacheError> {
        const SCRIPT: &str = r#"
            if redis.call('EXISTS', KEYS[1]) == 1 then
                local count = redis.call('DECR', KEYS[1])
                if count <= 0 then
                    redis.call('DEL', KEYS[1])
                end
            end
            return 1
        "#;

        let mut connection = self.connection.clone();
        let _: i32 = redis::Script::new(SCRIPT).key(key).invoke_async(&mut connection).await?;
        Ok(())
    }
}
