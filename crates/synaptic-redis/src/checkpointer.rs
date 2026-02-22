use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use synaptic_core::SynapticError;
use synaptic_graph::checkpoint::{Checkpoint, CheckpointConfig, Checkpointer};

/// Configuration for the Redis-backed graph checkpointer.
#[derive(Debug, Clone)]
pub struct RedisCheckpointerConfig {
    /// Redis URL (e.g. `redis://127.0.0.1/`).
    pub url: String,
    /// Optional TTL in seconds for checkpoint keys. `None` means no expiry.
    pub ttl: Option<u64>,
    /// Key prefix. Defaults to `"synaptic"`.
    pub prefix: String,
}

impl RedisCheckpointerConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ttl: None,
            prefix: "synaptic".to_string(),
        }
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
}

/// Redis-backed graph checkpointer.
///
/// Stores checkpoints as JSON values in Redis using the key scheme:
/// - Checkpoint data: `{prefix}:checkpoint:{thread_id}:{checkpoint_id}`
/// - Thread index (ordered list of checkpoint IDs): `{prefix}:idx:{thread_id}`
pub struct RedisCheckpointer {
    manager: ConnectionManager,
    config: RedisCheckpointerConfig,
}

impl RedisCheckpointer {
    /// Create a new checkpointer from a [`RedisCheckpointerConfig`].
    pub async fn new(config: RedisCheckpointerConfig) -> Result<Self, SynapticError> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| SynapticError::Store(format!("Redis connect: {e}")))?;
        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| SynapticError::Store(format!("Redis connection manager: {e}")))?;
        Ok(Self { manager, config })
    }

    /// Create a new checkpointer from a Redis URL with default config.
    pub async fn from_url(url: impl Into<String>) -> Result<Self, SynapticError> {
        Self::new(RedisCheckpointerConfig::new(url)).await
    }

    fn checkpoint_key(&self, thread_id: &str, checkpoint_id: &str) -> String {
        format!(
            "{}:checkpoint:{}:{}",
            self.config.prefix, thread_id, checkpoint_id
        )
    }

    fn index_key(&self, thread_id: &str) -> String {
        format!("{}:idx:{}", self.config.prefix, thread_id)
    }
}

#[async_trait]
impl Checkpointer for RedisCheckpointer {
    async fn put(
        &self,
        config: &CheckpointConfig,
        checkpoint: &Checkpoint,
    ) -> Result<(), SynapticError> {
        let mut conn = self.manager.clone();
        let data = serde_json::to_string(checkpoint)
            .map_err(|e| SynapticError::Store(format!("Serialize checkpoint: {e}")))?;

        let ck = self.checkpoint_key(&config.thread_id, &checkpoint.id);
        let idx = self.index_key(&config.thread_id);

        if let Some(ttl) = self.config.ttl {
            let _: () = conn
                .set_ex(&ck, &data, ttl)
                .await
                .map_err(|e| SynapticError::Store(format!("Redis SET EX: {e}")))?;
        } else {
            let _: () = conn
                .set(&ck, &data)
                .await
                .map_err(|e| SynapticError::Store(format!("Redis SET: {e}")))?;
        }

        // Append checkpoint ID to the ordered index for this thread
        let _: () = conn
            .rpush(&idx, &checkpoint.id)
            .await
            .map_err(|e| SynapticError::Store(format!("Redis RPUSH: {e}")))?;

        if let Some(ttl) = self.config.ttl {
            let _: () = conn
                .expire(&idx, ttl as i64)
                .await
                .map_err(|e| SynapticError::Store(format!("Redis EXPIRE idx: {e}")))?;
        }

        Ok(())
    }

    async fn get(&self, config: &CheckpointConfig) -> Result<Option<Checkpoint>, SynapticError> {
        let mut conn = self.manager.clone();
        let idx = self.index_key(&config.thread_id);

        let checkpoint_id: Option<String> = if let Some(ref id) = config.checkpoint_id {
            Some(id.clone())
        } else {
            // Get the latest (last) checkpoint ID from the index
            conn.lrange::<_, Vec<String>>(&idx, -1, -1)
                .await
                .map_err(|e| SynapticError::Store(format!("Redis LRANGE: {e}")))?
                .into_iter()
                .next()
        };

        let id = match checkpoint_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let ck = self.checkpoint_key(&config.thread_id, &id);
        let data: Option<String> = conn
            .get(&ck)
            .await
            .map_err(|e| SynapticError::Store(format!("Redis GET: {e}")))?;

        match data {
            None => Ok(None),
            Some(json) => {
                let cp: Checkpoint = serde_json::from_str(&json)
                    .map_err(|e| SynapticError::Store(format!("Deserialize checkpoint: {e}")))?;
                Ok(Some(cp))
            }
        }
    }

    async fn list(&self, config: &CheckpointConfig) -> Result<Vec<Checkpoint>, SynapticError> {
        let mut conn = self.manager.clone();
        let idx = self.index_key(&config.thread_id);

        let ids: Vec<String> = conn
            .lrange(&idx, 0, -1)
            .await
            .map_err(|e| SynapticError::Store(format!("Redis LRANGE: {e}")))?;

        let mut checkpoints = Vec::with_capacity(ids.len());
        for id in ids {
            let ck = self.checkpoint_key(&config.thread_id, &id);
            let data: Option<String> = conn
                .get(&ck)
                .await
                .map_err(|e| SynapticError::Store(format!("Redis GET: {e}")))?;
            if let Some(json) = data {
                let cp: Checkpoint = serde_json::from_str(&json)
                    .map_err(|e| SynapticError::Store(format!("Deserialize checkpoint: {e}")))?;
                checkpoints.push(cp);
            }
        }

        Ok(checkpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let cfg = RedisCheckpointerConfig::new("redis://127.0.0.1/");
        assert_eq!(cfg.prefix, "synaptic");
        assert!(cfg.ttl.is_none());
    }

    #[test]
    fn config_builder() {
        let cfg = RedisCheckpointerConfig::new("redis://localhost/")
            .with_ttl(3600)
            .with_prefix("myapp");
        assert_eq!(cfg.ttl, Some(3600));
        assert_eq!(cfg.prefix, "myapp");
    }
}
