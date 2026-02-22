use async_trait::async_trait;
use lru::LruCache;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use synaptic_core::SynapticError;

use crate::LarkConfig;

use super::client::LarkBotClient;
use super::session::LarkMessageEvent;

/// Handler trait for incoming bot messages.
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(
        &self,
        event: LarkMessageEvent,
        client: &LarkBotClient,
    ) -> Result<(), SynapticError>;
}

/// Long-connection (WebSocket) event listener for Feishu bots.
///
/// No public IP needed — opens an outbound WebSocket to Lark's endpoint.
pub struct LarkLongConnListener {
    config: LarkConfig,
    dedup_capacity: usize,
    dedup: Arc<Mutex<LruCache<String, ()>>>,
    message_handler: Option<Arc<dyn MessageHandler>>,
}

impl LarkLongConnListener {
    pub fn new(config: LarkConfig) -> Self {
        let cap = 512;
        Self {
            config,
            dedup_capacity: cap,
            dedup: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap()))),
            message_handler: None,
        }
    }

    pub fn with_dedup_capacity(mut self, cap: usize) -> Self {
        let cap = cap.max(1);
        self.dedup_capacity = cap;
        self.dedup = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap())));
        self
    }

    pub fn dedup_capacity(&self) -> usize {
        self.dedup_capacity
    }

    pub fn with_message_handler<H: MessageHandler + 'static>(mut self, h: H) -> Self {
        self.message_handler = Some(Arc::new(h));
        self
    }

    /// Dispatch a pre-parsed event payload.
    pub async fn dispatch_payload(&self, payload: &Value) -> Result<(), SynapticError> {
        let event_id = payload["header"]["event_id"].as_str().unwrap_or("");
        let event_type = payload["header"]["event_type"].as_str().unwrap_or("");

        // Dedup check
        if !event_id.is_empty() {
            let mut cache = self.dedup.lock().unwrap();
            if cache.contains(event_id) {
                tracing::debug!("LarkLongConnListener: dedup skip event_id={event_id}");
                return Ok(());
            }
            cache.put(event_id.to_string(), ());
        }

        match event_type {
            "im.message.receive_v1" => {
                if let Some(handler) = &self.message_handler {
                    let msg_event = LarkMessageEvent::from_payload(payload)?;
                    let client = LarkBotClient::new(self.config.clone());
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler.handle(msg_event, &client).await {
                            tracing::error!("LarkLongConnListener: handler error: {e}");
                        }
                    });
                }
            }
            other => {
                tracing::debug!("LarkLongConnListener: unhandled event_type='{other}'");
            }
        }
        Ok(())
    }

    async fn get_ws_endpoint(&self, token: &str) -> Result<String, SynapticError> {
        let url = format!("{}/callback/v1/ws/endpoint", self.config.base_url);
        let resp: Value = reqwest::Client::new()
            .post(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("ws endpoint: {e}")))?
            .json()
            .await
            .map_err(|e| SynapticError::Tool(format!("ws endpoint parse: {e}")))?;
        if resp["code"].as_i64().unwrap_or(-1) != 0 {
            return Err(SynapticError::Tool(format!(
                "ws endpoint error: {}",
                resp["msg"].as_str().unwrap_or("unknown")
            )));
        }
        resp["data"]["url"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| SynapticError::Tool("ws endpoint: missing url".to_string()))
    }

    /// Start the long-connection event loop. Blocks until an unrecoverable error.
    pub async fn run(self) -> Result<(), SynapticError> {
        use futures::{SinkExt, StreamExt};
        use tokio::time::{sleep, Duration};
        use tokio_tungstenite::connect_async;

        let listener = Arc::new(self);
        let mut backoff_secs = 1u64;

        loop {
            let token = listener.config.clone().token_cache().get_token().await?;
            let ws_url = match listener.get_ws_endpoint(&token).await {
                Ok(url) => url,
                Err(e) => {
                    tracing::warn!("LarkLongConnListener: failed to get ws endpoint: {e}");
                    sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(60);
                    continue;
                }
            };

            tracing::info!("LarkLongConnListener: connecting to {ws_url}");
            let (mut ws_stream, _) = match connect_async(&ws_url).await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::warn!("LarkLongConnListener: connect failed: {e}");
                    sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(60);
                    continue;
                }
            };
            backoff_secs = 1;
            tracing::info!("LarkLongConnListener: connected");

            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                        let payload: Value = match serde_json::from_str(text.as_str()) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!("LarkLongConnListener: invalid JSON: {e}");
                                continue;
                            }
                        };
                        let ack = serde_json::json!({ "code": 0 });
                        let _ = ws_stream
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                ack.to_string(),
                            ))
                            .await;
                        let l = listener.clone();
                        let p = payload.clone();
                        tokio::spawn(async move {
                            if let Err(e) = l.dispatch_payload(&p).await {
                                tracing::error!("dispatch error: {e}");
                            }
                        });
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                        tracing::info!(
                            "LarkLongConnListener: server closed connection, reconnecting"
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("LarkLongConnListener: ws error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            sleep(Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(60);
        }
    }
}
