//! Channel status tracking types.

use std::time::SystemTime;

use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Connected,
    Connecting,
    Disconnected,
}

impl std::fmt::Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connected => write!(f, "connected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Disconnected => write!(f, "disconnected"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisconnectInfo {
    pub at: SystemTime,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelAccountSnapshot {
    pub channel: String,
    pub account_id: String,
    pub state: ChannelState,
    pub running: bool,
    pub busy: bool,
    pub active_runs: u32,
    pub connected_at: Option<SystemTime>,
    pub last_event_at: Option<SystemTime>,
    pub last_inbound_at: Option<SystemTime>,
    pub last_outbound_at: Option<SystemTime>,
    pub last_error: Option<String>,
    pub last_disconnect: Option<DisconnectInfo>,
    pub reconnect_count: u32,
    pub mode: Option<String>,
}

impl ChannelAccountSnapshot {
    pub fn new(channel: impl Into<String>, account_id: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            account_id: account_id.into(),
            state: ChannelState::Disconnected,
            running: false,
            busy: false,
            active_runs: 0,
            connected_at: None,
            last_event_at: None,
            last_inbound_at: None,
            last_outbound_at: None,
            last_error: None,
            last_disconnect: None,
            reconnect_count: 0,
            mode: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChannelStatusPatch {
    pub state: Option<ChannelState>,
    pub running: Option<bool>,
    pub busy: Option<bool>,
    pub active_runs: Option<u32>,
    pub last_event_at: Option<SystemTime>,
    pub last_error: Option<Option<String>>,
    pub last_disconnect: Option<Option<DisconnectInfo>>,
    pub mode: Option<String>,
}

pub trait ChannelStatusHandle: Send + Sync {
    fn get(&self) -> ChannelAccountSnapshot;
    fn set(&self, patch: ChannelStatusPatch);
}

#[async_trait]
pub trait ChannelProbe: Send + Sync {
    async fn probe(&self) -> Result<(), String>;
}
