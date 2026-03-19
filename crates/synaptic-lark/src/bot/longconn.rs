use async_trait::async_trait;
use lru::LruCache;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use synaptic_core::SynapticError;

use crate::LarkConfig;

use synaptic_core::ChannelStatusHandle;

use super::client::LarkBotClient;
use super::events::{parse_event, LarkEventHandler};
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
    /// Full event handler (receives all event types including card actions).
    event_handler: Option<Arc<dyn LarkEventHandler>>,
    /// Optional status handle for reporting connection health.
    status_handle: Option<Arc<dyn ChannelStatusHandle>>,
}

impl LarkLongConnListener {
    pub fn new(config: LarkConfig) -> Self {
        let cap = 512;
        Self {
            config,
            dedup_capacity: cap,
            dedup: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(cap).unwrap()))),
            message_handler: None,
            event_handler: None,
            status_handle: None,
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

    /// Set a full-event handler (receives all event types).
    /// Use this instead of [`with_message_handler`] for rich event handling
    /// including card actions, bot lifecycle events, etc.
    pub fn with_event_handler<H: LarkEventHandler + 'static>(mut self, h: H) -> Self {
        self.event_handler = Some(Arc::new(h));
        self
    }

    /// Set an optional channel status handle for reporting connection health.
    pub fn with_status_handle(mut self, handle: Arc<dyn ChannelStatusHandle>) -> Self {
        self.status_handle = Some(handle);
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

        // Route to event_handler if set (handles all event types)
        if let Some(event_handler) = &self.event_handler {
            let config_clone = self.config.clone();
            let handler = event_handler.clone();
            let p = payload.clone();
            tokio::spawn(async move {
                let event = match parse_event(&p) {
                    Ok(e) => e,
                    Err(err) => {
                        tracing::warn!("LarkLongConnListener: failed to parse event: {err}");
                        return;
                    }
                };
                let client = LarkBotClient::new(config_clone);
                if let Err(e) = handler.handle(event, &client).await {
                    tracing::error!("LarkLongConnListener: event_handler error: {e}");
                }
            });
            return Ok(());
        }

        // Fallback to legacy message_handler
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
        // WS endpoint lives at /callback/ws/endpoint on the domain root,
        // NOT under /open-apis.
        let url = format!("{}/callback/ws/endpoint", self.config.base_url);
        let raw = reqwest::Client::new()
            .post(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SynapticError::Tool(format!("ws endpoint: {e}")))?;
        let status = raw.status();
        let body = raw
            .text()
            .await
            .map_err(|e| SynapticError::Tool(format!("ws endpoint read: {e}")))?;
        tracing::debug!(status = %status, body_len = body.len(), "lark ws endpoint raw response: {body}");
        let resp: Value = serde_json::from_str(&body).map_err(|e| {
            SynapticError::Tool(format!(
                "ws endpoint parse: {e} — raw({status}): {}",
                &body[..body.len().min(500)]
            ))
        })?;
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
