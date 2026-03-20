//! Synaptic Core — foundational types, traits, and errors for the Synaptic framework.

#[cfg(feature = "schemars")]
pub use schemars;

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

pub mod bot;
pub mod channel;
pub mod channel_status;
pub mod chat_model;
pub mod checkpoint;
pub mod context_budget;
pub mod delivery;
pub mod dm_policy;
pub mod embeddings;
pub mod error;
pub mod message;
pub mod message_ir;
pub mod provenance;
pub mod runnable;
pub mod store;
pub mod token_counter;
pub mod tool;
pub mod types;
pub mod vectorstore;

// ---------------------------------------------------------------------------
// Re-exports — keep the flat public API intact
// ---------------------------------------------------------------------------

pub use channel::*;
pub use channel_status::*;
pub use checkpoint::{Checkpoint, CheckpointConfig, Checkpointer};
pub use context_budget::{ContextBudget, ContextSlot, Priority, SlotTrimStrategy};
pub use delivery::DeliveryContext;
pub use dm_policy::{DmAccessDenied, DmPolicy, DmPolicyEnforcer, PairingChallenge, PairingError};
pub use error::SynapticError;
pub use provenance::{InputProvenance, ProvenanceKind};
pub use token_counter::{HeuristicTokenCounter, TokenCounter};

// Message types
pub use message::{
    filter_messages, get_buffer_string, merge_message_runs, trim_messages, AIMessageChunk,
    ContentBlock, Message, TrimStrategy,
};

// Chat model types
pub use chat_model::{
    ChatModel, ChatRequest, ChatResponse, ChatStream, InputTokenDetails, ModelProfile,
    OutputTokenDetails, ThinkingConfig, TokenUsage,
};

// Tool types
pub use tool::{
    InvalidToolCall, RuntimeAwareTool, RuntimeAwareToolAdapter, Tool, ToolCall, ToolCallChunk,
    ToolChoice, ToolDefinition, ToolRuntime,
};

// Store types
pub use store::{
    encode_namespace, now_iso, parse_age_secs, validate_table_name, Embeddings, Item, MemoryStore,
    SearchOptions, Store,
};

// Miscellaneous types
pub use types::{
    CallbackHandler, Document, Entrypoint, EntrypointConfig, EntrypointFn, LlmCache, Loader,
    Retriever, RunEvent, RunnableConfig, Runtime, StreamWriter, VectorStore,
};
