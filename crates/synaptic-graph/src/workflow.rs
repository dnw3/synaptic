//! Deterministic workflow engine (Lobster-compatible).
//!
//! Unlike the general-purpose `StateGraph`, workflows are linear/branching pipelines
//! where each step can pause for human approval and resume via a token.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A deterministic workflow composed of named steps.
#[derive(Clone)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
}

/// A single step in a workflow pipeline.
pub struct WorkflowStep {
    pub name: String,
    pub handler: Box<dyn WorkflowHandler>,
    pub requires_approval: bool,
    pub timeout: Option<Duration>,
}

impl Clone for WorkflowStep {
    fn clone(&self) -> Self {
        // WorkflowStep is not truly cloneable due to handler; used only for metadata.
        panic!("WorkflowStep cannot be cloned at runtime — clone the Workflow metadata instead")
    }
}

/// Handler trait for workflow steps.
#[async_trait]
pub trait WorkflowHandler: Send + Sync {
    /// Execute this workflow step.
    async fn execute(&self, ctx: &mut WorkflowContext) -> Result<WorkflowResult, WorkflowError>;
}

/// Result of a workflow step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowResult {
    /// Continue to the next step with updated state.
    Continue(Value),
    /// Pause and wait for human approval with a prompt message.
    NeedApproval(String),
    /// Branch to a named step.
    Branch(String),
    /// Workflow is done with final output.
    Done(Value),
}

/// Mutable context passed to each workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Current state (accumulated from previous steps).
    pub state: Value,
    /// Unique resume token for this execution.
    pub resume_token: String,
    /// Index of the current step.
    pub step_index: usize,
    /// Record of approvals received so far.
    pub approvals: Vec<ApprovalRecord>,
}

/// A recorded approval decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub step_name: String,
    pub approved: bool,
    pub data: Option<Value>,
    pub timestamp: String,
}

/// Current status of a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running {
        step: String,
    },
    WaitingApproval {
        step: String,
        prompt: String,
        resume_token: String,
    },
    Completed {
        output: Value,
    },
    Failed {
        error: String,
    },
}

/// Workflow execution error.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("step '{0}' not found")]
    StepNotFound(String),
    #[error("invalid resume token")]
    InvalidResumeToken,
    #[error("workflow timed out at step '{0}'")]
    Timeout(String),
    #[error("approval rejected at step '{0}'")]
    Rejected(String),
    #[error("{0}")]
    Other(String),
}
