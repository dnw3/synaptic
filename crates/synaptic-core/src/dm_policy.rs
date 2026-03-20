//! DM (Direct Message) access control policy.
//!
//! Provides types for controlling who can send direct messages to a bot,
//! including pairing-based access control.

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// DM access control policy for bot channels.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmPolicy {
    /// Unknown senders must complete pairing challenge.
    #[default]
    Pairing,
    /// Anyone can DM the bot.
    Open,
    /// Only pre-approved sender IDs from config.
    Allowlist,
    /// DM disabled entirely.
    Disabled,
}

/// Pairing challenge issued to an unknown sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingChallenge {
    pub code: String,
    pub sender_id: String,
    pub channel: String,
    pub created_at: u64,
    pub ttl_ms: u64,
}

impl PairingChallenge {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            // On clock error, treat as expired (safe-fail)
            .unwrap_or(u64::MAX);
        now > self.created_at.saturating_add(self.ttl_ms)
    }
}

/// Result of a failed DM access check.
#[derive(Debug)]
pub enum DmAccessDenied {
    NeedsPairing(PairingChallenge),
    NotAllowed,
    DmDisabled,
}

/// Error from pairing operations.
#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("pairing code not found")]
    CodeNotFound,
    #[error("pairing code expired")]
    CodeExpired,
    #[error("channel not found")]
    ChannelNotFound,
    #[error("storage error: {0}")]
    StorageError(String),
}

impl fmt::Display for DmAccessDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeedsPairing(c) => write!(f, "DM access denied: needs pairing (code={})", c.code),
            Self::NotAllowed => write!(f, "DM access denied: not allowed"),
            Self::DmDisabled => write!(f, "DM access denied: DMs disabled"),
        }
    }
}

/// Trait for DM access control enforcement.
#[async_trait]
pub trait DmPolicyEnforcer: Send + Sync {
    /// Check if sender is allowed to DM. Returns Ok(()) or Err with challenge/rejection.
    async fn check_access(&self, sender_id: &str, channel: &str) -> Result<(), DmAccessDenied>;
    /// Approve a pairing code. Returns the sender_id that was approved.
    async fn approve_code(&self, channel: &str, code: &str) -> Result<String, PairingError>;
    /// List pending pairing requests for a channel.
    async fn list_pending(&self, channel: &str) -> Vec<PairingChallenge>;
}
