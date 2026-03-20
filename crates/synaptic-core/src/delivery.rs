use serde::{Deserialize, Serialize};

/// Unified routing metadata for cross-channel message delivery.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DeliveryContext {
    /// Channel identifier: "webchat", "slack", "telegram", "lark", "discord", etc.
    pub channel: String,
    /// Target identifier: "user:123", "channel:abc", "chat:456"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Multi-account disambiguator: workspace ID, guild ID, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Thread/topic identifier: Slack thread_ts, Telegram topic_id, Discord thread_id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Platform-specific extension fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_delivery_context() {
        let ctx = DeliveryContext::default();
        assert_eq!(ctx.channel, "");
        assert!(ctx.to.is_none());
        assert!(ctx.account_id.is_none());
        assert!(ctx.thread_id.is_none());
        assert!(ctx.meta.is_none());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let ctx = DeliveryContext {
            channel: "slack".into(),
            to: Some("channel:general".into()),
            account_id: Some("W123".into()),
            thread_id: Some("1234567890.123456".into()),
            meta: Some(serde_json::json!({"unfurl_links": false})),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: DeliveryContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, deserialized);
    }

    #[test]
    fn test_omits_none_fields() {
        let ctx = DeliveryContext {
            channel: "telegram".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("channel"));
        assert!(!json.contains("account_id"));
        assert!(!json.contains("thread_id"));
        assert!(!json.contains("meta"));
    }

    #[test]
    fn test_partial_fields() {
        let ctx = DeliveryContext {
            channel: "lark".into(),
            to: Some("chat:oc_abc123".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: DeliveryContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.channel, "lark");
        assert_eq!(deserialized.to.as_deref(), Some("chat:oc_abc123"));
        assert!(deserialized.account_id.is_none());
    }
}
