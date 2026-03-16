use std::sync::Arc;

use synaptic_core::{MemoryStore, Message};
use synaptic_graph::{Checkpoint, CheckpointConfig, Checkpointer};
use synaptic_session::{SessionInfo, SessionManager};
use synaptic_store::InMemoryStore;

fn new_manager() -> SessionManager {
    SessionManager::new(Arc::new(InMemoryStore::new()))
}

#[tokio::test]
async fn create_session() {
    let mgr = new_manager();
    let id = mgr.create_session().await.unwrap();
    assert!(!id.is_empty());

    let info = mgr.get_session(&id).await.unwrap().unwrap();
    assert_eq!(info.session_id, id);
    assert!(info.updated_at > 0);
}

#[tokio::test]
async fn list_sessions() {
    let mgr = new_manager();
    mgr.create_session().await.unwrap();
    mgr.create_session().await.unwrap();

    let sessions = mgr.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn delete_session() {
    let mgr = new_manager();
    let id = mgr.create_session().await.unwrap();

    mgr.delete_session(&id).await.unwrap();

    let info = mgr.get_session(&id).await.unwrap();
    assert!(info.is_none());
}

#[tokio::test]
async fn memory_integration() {
    let mgr = new_manager();
    let id = mgr.create_session().await.unwrap();

    let memory = mgr.memory();
    memory.append(&id, Message::human("Hello")).await.unwrap();
    memory.append(&id, Message::ai("Hi there!")).await.unwrap();

    let messages = memory.load(&id).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages[0].is_human());
    assert_eq!(messages[0].content(), "Hello");
    assert!(messages[1].is_ai());
    assert_eq!(messages[1].content(), "Hi there!");
}

#[tokio::test]
async fn checkpointer_integration() {
    let mgr = new_manager();
    let id = mgr.create_session().await.unwrap();

    let cp = mgr.checkpointer();
    let config = CheckpointConfig::new(&id);

    let checkpoint = Checkpoint::new(serde_json::json!({"value": "test"}), None);
    let checkpoint_id = checkpoint.id.clone();
    cp.put(&config, &checkpoint).await.unwrap();

    let loaded = cp.get(&config).await.unwrap().unwrap();
    assert_eq!(loaded.id, checkpoint_id);
    assert_eq!(loaded.state, serde_json::json!({"value": "test"}));
}

#[tokio::test]
async fn delete_session_cleans_up_all_data() {
    let mgr = new_manager();
    let id = mgr.create_session().await.unwrap();

    // Add messages
    let memory = mgr.memory();
    memory.append(&id, Message::human("Hello")).await.unwrap();

    // Add checkpoint
    let cp = mgr.checkpointer();
    let config = CheckpointConfig::new(&id);
    let checkpoint = Checkpoint::new(serde_json::json!({"v": 1}), None);
    cp.put(&config, &checkpoint).await.unwrap();

    // Delete everything
    mgr.delete_session(&id).await.unwrap();

    // Verify all data is gone
    assert!(mgr.get_session(&id).await.unwrap().is_none());
    assert!(memory.load(&id).await.unwrap().is_empty());
    assert!(cp.get(&config).await.unwrap().is_none());
}

/// Old 4-field JSON with `"id"` and `"token_count"` should deserialize into the new struct.
#[test]
fn backward_compat_old_4_field_json() {
    let old_json = serde_json::json!({
        "id": "abc-123",
        "created_at": "2025-01-01T00:00:00Z",
        "token_count": 42,
        "compaction_count": 3
    });

    let info: SessionInfo = serde_json::from_value(old_json).unwrap();
    assert_eq!(info.session_id, "abc-123");
    assert_eq!(info.created_at, "2025-01-01T00:00:00Z");
    assert_eq!(info.total_tokens, 42);
    assert_eq!(info.compaction_count, 3);

    // All new optional fields should be None / default
    assert!(info.session_key.is_none());
    assert!(info.channel.is_none());
    assert!(info.model.is_none());
    assert!(!info.system_sent);
    assert_eq!(info.spawn_depth, 0);
    assert_eq!(info.updated_at, 0);
}

/// New struct serializes with skip_serializing_if working (None fields omitted).
#[test]
fn serialization_skips_none_fields() {
    let info = SessionInfo {
        session_id: "test-id".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        updated_at: 1000,
        total_tokens: 100,
        input_tokens: 40,
        output_tokens: 60,
        ..Default::default()
    };

    let value = serde_json::to_value(&info).unwrap();
    let obj = value.as_object().unwrap();

    // Required fields present
    assert!(obj.contains_key("session_id"));
    assert!(obj.contains_key("created_at"));
    assert!(obj.contains_key("total_tokens"));
    assert!(obj.contains_key("input_tokens"));
    assert!(obj.contains_key("output_tokens"));

    // Optional None fields should be absent
    assert!(!obj.contains_key("session_key"));
    assert!(!obj.contains_key("channel"));
    assert!(!obj.contains_key("chat_type"));
    assert!(!obj.contains_key("display_name"));
    assert!(!obj.contains_key("label"));
    assert!(!obj.contains_key("model"));
    assert!(!obj.contains_key("model_provider"));
    assert!(!obj.contains_key("thinking_level"));
    assert!(!obj.contains_key("verbose_level"));
    assert!(!obj.contains_key("fast_mode"));
    assert!(!obj.contains_key("reasoning_level"));
    assert!(!obj.contains_key("send_policy"));
    assert!(!obj.contains_key("last_channel"));
    assert!(!obj.contains_key("last_to"));
    assert!(!obj.contains_key("last_account_id"));
    assert!(!obj.contains_key("last_thread_id"));
    assert!(!obj.contains_key("spawned_by"));
    assert!(!obj.contains_key("group_id"));
    assert!(!obj.contains_key("subject"));
}

/// Round-trip: serialize → deserialize preserves all fields.
#[test]
fn round_trip_full_session() {
    let info = SessionInfo {
        session_id: "rt-1".into(),
        session_key: Some("agent:default:main".into()),
        created_at: "2025-06-01T00:00:00Z".into(),
        updated_at: 1719792000000,
        channel: Some("lark".into()),
        chat_type: Some("group".into()),
        display_name: Some("Test Session".into()),
        label: Some("important".into()),
        input_tokens: 100,
        output_tokens: 200,
        total_tokens: 300,
        total_tokens_fresh: true,
        compaction_count: 2,
        model: Some("gpt-4o".into()),
        model_provider: Some("openai".into()),
        thinking_level: Some("high".into()),
        verbose_level: Some("debug".into()),
        fast_mode: Some(true),
        reasoning_level: Some("medium".into()),
        system_sent: true,
        aborted_last_run: true,
        send_policy: Some("always".into()),
        last_channel: Some("slack".into()),
        last_to: Some("C12345".into()),
        last_account_id: Some("bot-1".into()),
        last_thread_id: Some("thread-1".into()),
        spawned_by: Some("parent-session".into()),
        spawn_depth: 2,
        group_id: Some("grp-1".into()),
        subject: Some("Project Discussion".into()),
        forked_from_parent: true,
    };

    let json = serde_json::to_value(&info).unwrap();
    let restored: SessionInfo = serde_json::from_value(json).unwrap();

    assert_eq!(restored.session_id, "rt-1");
    assert_eq!(restored.session_key.as_deref(), Some("agent:default:main"));
    assert_eq!(restored.updated_at, 1719792000000);
    assert_eq!(restored.channel.as_deref(), Some("lark"));
    assert_eq!(restored.total_tokens, 300);
    assert!(restored.total_tokens_fresh);
    assert!(restored.system_sent);
    assert!(restored.aborted_last_run);
    assert_eq!(restored.spawn_depth, 2);
    assert!(restored.forked_from_parent);
    assert_eq!(restored.model.as_deref(), Some("gpt-4o"));
    assert_eq!(restored.fast_mode, Some(true));
}
