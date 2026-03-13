use serde::{Deserialize, Serialize};

/// Tracks the origin of a message for auditing and routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputProvenance {
    pub kind: ProvenanceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    #[default]
    ExternalUser,
    InterSession,
    InternalSystem,
}

impl Default for InputProvenance {
    fn default() -> Self {
        Self {
            kind: ProvenanceKind::ExternalUser,
            source_channel: None,
            source_session_id: None,
            source_tool: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_provenance() {
        let p = InputProvenance::default();
        assert_eq!(p.kind, ProvenanceKind::ExternalUser);
        assert!(p.source_channel.is_none());
        assert!(p.source_session_id.is_none());
        assert!(p.source_tool.is_none());
    }

    #[test]
    fn test_provenance_kind_default() {
        let kind = ProvenanceKind::default();
        assert_eq!(kind, ProvenanceKind::ExternalUser);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let p = InputProvenance {
            kind: ProvenanceKind::InterSession,
            source_channel: Some("slack".into()),
            source_session_id: Some("sess_123".into()),
            source_tool: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        let deserialized: InputProvenance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, ProvenanceKind::InterSession);
        assert_eq!(deserialized.source_channel.as_deref(), Some("slack"));
        assert_eq!(deserialized.source_session_id.as_deref(), Some("sess_123"));
        assert!(deserialized.source_tool.is_none());
    }

    #[test]
    fn test_provenance_kind_serde() {
        let json = serde_json::to_string(&ProvenanceKind::InternalSystem).unwrap();
        assert_eq!(json, "\"internal_system\"");
        let json = serde_json::to_string(&ProvenanceKind::ExternalUser).unwrap();
        assert_eq!(json, "\"external_user\"");
        let json = serde_json::to_string(&ProvenanceKind::InterSession).unwrap();
        assert_eq!(json, "\"inter_session\"");
    }

    #[test]
    fn test_omits_none_fields() {
        let p = InputProvenance::default();
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("kind"));
        assert!(!json.contains("source_channel"));
        assert!(!json.contains("source_session_id"));
        assert!(!json.contains("source_tool"));
    }

    #[test]
    fn test_internal_system_provenance() {
        let p = InputProvenance {
            kind: ProvenanceKind::InternalSystem,
            source_channel: None,
            source_session_id: None,
            source_tool: Some("memory_recall".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("internal_system"));
        assert!(json.contains("memory_recall"));
    }
}
