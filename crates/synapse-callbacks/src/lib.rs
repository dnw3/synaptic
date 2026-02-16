use std::sync::Arc;

use async_trait::async_trait;
use synapse_core::{CallbackHandler, RunEvent, SynapseError};
use tokio::sync::RwLock;

#[derive(Default, Clone)]
pub struct RecordingCallback {
    events: Arc<RwLock<Vec<RunEvent>>>,
}

impl RecordingCallback {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn events(&self) -> Vec<RunEvent> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl CallbackHandler for RecordingCallback {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapseError> {
        self.events.write().await.push(event);
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct LoggingCallback;

#[async_trait]
impl CallbackHandler for LoggingCallback {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapseError> {
        match event {
            RunEvent::RunStarted { run_id, session_id } => {
                tracing::info!("run started: run_id={run_id}, session_id={session_id}");
            }
            RunEvent::RunStep { run_id, step } => {
                tracing::info!("run step: run_id={run_id}, step={step}");
            }
            RunEvent::LlmCalled {
                run_id,
                message_count,
            } => {
                tracing::info!("llm called: run_id={run_id}, messages={message_count}");
            }
            RunEvent::ToolCalled { run_id, tool_name } => {
                tracing::info!("tool called: run_id={run_id}, tool={tool_name}");
            }
            RunEvent::RunFinished { run_id, output } => {
                tracing::info!("run finished: run_id={run_id}, output={output}");
            }
            RunEvent::RunFailed { run_id, error } => {
                tracing::error!("run failed: run_id={run_id}, error={error}");
            }
        }
        Ok(())
    }
}
