//! Workflow execution engine with checkpoint-based resume support.

use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::checkpoint::{Checkpoint, CheckpointConfig, Checkpointer};
use crate::workflow::{
    ApprovalRecord, Workflow, WorkflowContext, WorkflowError, WorkflowResult, WorkflowStatus,
};

/// Serializable snapshot of a running workflow for checkpointing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WorkflowCheckpoint {
    workflow_name: String,
    context: WorkflowContext,
    status: WorkflowStatus,
}

/// Runs workflows with checkpoint-based pause/resume.
pub struct WorkflowRunner {
    checkpointer: Arc<dyn Checkpointer>,
}

impl WorkflowRunner {
    pub fn new(checkpointer: Arc<dyn Checkpointer>) -> Self {
        Self { checkpointer }
    }

    /// Start a new workflow execution.
    pub async fn start(
        &self,
        workflow: &Workflow,
        input: Value,
    ) -> Result<WorkflowExecution, WorkflowError> {
        let resume_token = Uuid::new_v4().to_string();

        let ctx = WorkflowContext {
            state: input,
            resume_token: resume_token.clone(),
            step_index: 0,
            approvals: Vec::new(),
        };

        self.run_from(workflow, ctx).await
    }

    /// Resume a paused workflow (e.g. after approval).
    pub async fn resume(
        &self,
        workflow: &Workflow,
        resume_token: &str,
        approval: Option<Value>,
    ) -> Result<WorkflowExecution, WorkflowError> {
        // Load checkpoint
        let config = CheckpointConfig {
            thread_id: format!("workflow:{}", resume_token),
            checkpoint_id: None,
        };
        let cp = self
            .checkpointer
            .get(&config)
            .await
            .map_err(|e| WorkflowError::Other(e.to_string()))?
            .ok_or(WorkflowError::InvalidResumeToken)?;

        let snapshot: WorkflowCheckpoint = serde_json::from_value(cp.state)
            .map_err(|e| WorkflowError::Other(format!("corrupt checkpoint: {}", e)))?;

        let mut ctx = snapshot.context;

        // Record the approval
        if let WorkflowStatus::WaitingApproval { ref step, .. } = snapshot.status {
            ctx.approvals.push(ApprovalRecord {
                step_name: step.clone(),
                approved: true,
                data: approval,
                timestamp: chrono_now(),
            });
            // Advance past the approval step
            ctx.step_index += 1;
        }

        self.run_from(workflow, ctx).await
    }

    /// Query the status of a workflow by its resume token.
    pub async fn status(&self, resume_token: &str) -> Result<WorkflowStatus, WorkflowError> {
        let config = CheckpointConfig {
            thread_id: format!("workflow:{}", resume_token),
            checkpoint_id: None,
        };
        let cp = self
            .checkpointer
            .get(&config)
            .await
            .map_err(|e| WorkflowError::Other(e.to_string()))?
            .ok_or(WorkflowError::InvalidResumeToken)?;

        let snapshot: WorkflowCheckpoint = serde_json::from_value(cp.state)
            .map_err(|e| WorkflowError::Other(format!("corrupt checkpoint: {}", e)))?;

        Ok(snapshot.status)
    }

    /// Internal: run workflow steps starting from current context.
    async fn run_from(
        &self,
        workflow: &Workflow,
        mut ctx: WorkflowContext,
    ) -> Result<WorkflowExecution, WorkflowError> {
        while ctx.step_index < workflow.steps.len() {
            let step = &workflow.steps[ctx.step_index];

            // Save running status
            self.save_checkpoint(
                &workflow.name,
                &ctx,
                WorkflowStatus::Running {
                    step: step.name.clone(),
                },
            )
            .await?;

            // Check if step requires approval and we haven't gotten it yet
            if step.requires_approval && !ctx.approvals.iter().any(|a| a.step_name == step.name) {
                let prompt = format!("Step '{}' requires approval to proceed.", step.name);
                let status = WorkflowStatus::WaitingApproval {
                    step: step.name.clone(),
                    prompt: prompt.clone(),
                    resume_token: ctx.resume_token.clone(),
                };
                self.save_checkpoint(&workflow.name, &ctx, status.clone())
                    .await?;
                return Ok(WorkflowExecution {
                    resume_token: ctx.resume_token,
                    status,
                    output: None,
                });
            }

            // Execute the step handler
            let result = step.handler.execute(&mut ctx).await?;

            match result {
                WorkflowResult::Continue(new_state) => {
                    ctx.state = new_state;
                    ctx.step_index += 1;
                }
                WorkflowResult::NeedApproval(prompt) => {
                    let status = WorkflowStatus::WaitingApproval {
                        step: step.name.clone(),
                        prompt,
                        resume_token: ctx.resume_token.clone(),
                    };
                    self.save_checkpoint(&workflow.name, &ctx, status.clone())
                        .await?;
                    return Ok(WorkflowExecution {
                        resume_token: ctx.resume_token,
                        status,
                        output: None,
                    });
                }
                WorkflowResult::Branch(target) => {
                    let idx = workflow
                        .steps
                        .iter()
                        .position(|s| s.name == target)
                        .ok_or(WorkflowError::StepNotFound(target))?;
                    ctx.step_index = idx;
                }
                WorkflowResult::Done(output) => {
                    let status = WorkflowStatus::Completed {
                        output: output.clone(),
                    };
                    self.save_checkpoint(&workflow.name, &ctx, status.clone())
                        .await?;
                    return Ok(WorkflowExecution {
                        resume_token: ctx.resume_token,
                        status,
                        output: Some(output),
                    });
                }
            }
        }

        // All steps completed
        let status = WorkflowStatus::Completed {
            output: ctx.state.clone(),
        };
        self.save_checkpoint(&workflow.name, &ctx, status.clone())
            .await?;
        Ok(WorkflowExecution {
            resume_token: ctx.resume_token,
            status,
            output: Some(ctx.state),
        })
    }

    async fn save_checkpoint(
        &self,
        workflow_name: &str,
        ctx: &WorkflowContext,
        status: WorkflowStatus,
    ) -> Result<(), WorkflowError> {
        let snapshot = WorkflowCheckpoint {
            workflow_name: workflow_name.to_string(),
            context: ctx.clone(),
            status,
        };

        let config = CheckpointConfig {
            thread_id: format!("workflow:{}", ctx.resume_token),
            checkpoint_id: None,
        };

        let cp = Checkpoint {
            id: format!("wf-{}", Uuid::new_v4()),
            state: serde_json::to_value(&snapshot)
                .map_err(|e| WorkflowError::Other(e.to_string()))?,
            next_node: None,
            parent_id: None,
            metadata: std::collections::HashMap::new(),
        };

        self.checkpointer
            .put(&config, &cp)
            .await
            .map_err(|e| WorkflowError::Other(e.to_string()))?;

        Ok(())
    }
}

/// Result of starting or resuming a workflow.
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub resume_token: String,
    pub status: WorkflowStatus,
    pub output: Option<Value>,
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}
