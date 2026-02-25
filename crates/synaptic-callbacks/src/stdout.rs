use async_trait::async_trait;
use synaptic_core::{CallbackHandler, RunEvent, SynapticError};

/// A callback handler that prints events to stdout.
///
/// When `verbose` is true, additional detail is printed for each event.
pub struct StdOutCallbackHandler {
    verbose: bool,
}

impl StdOutCallbackHandler {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn verbose() -> Self {
        Self { verbose: true }
    }
}

impl Default for StdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CallbackHandler for StdOutCallbackHandler {
    async fn on_event(&self, event: RunEvent) -> Result<(), SynapticError> {
        match event {
            RunEvent::RunStarted { run_id, session_id } => {
                if self.verbose {
                    println!("[RunStarted] run_id={run_id} session_id={session_id}");
                } else {
                    println!("[RunStarted] run_id={run_id}");
                }
            }
            RunEvent::RunStep { run_id, step } => {
                if self.verbose {
                    println!("[RunStep] run_id={run_id} step={step}");
                } else {
                    println!("[RunStep] step={step}");
                }
            }
            RunEvent::LlmCalled {
                run_id,
                message_count,
            } => {
                if self.verbose {
                    println!("[LlmCalled] run_id={run_id} message_count={message_count}");
                } else {
                    println!("[LlmCalled] message_count={message_count}");
                }
            }
            RunEvent::ToolCalled { run_id, tool_name } => {
                if self.verbose {
                    println!("[ToolCalled] run_id={run_id} tool_name={tool_name}");
                } else {
                    println!("[ToolCalled] tool_name={tool_name}");
                }
            }
            RunEvent::RunFinished { run_id, output } => {
                if self.verbose {
                    println!("[RunFinished] run_id={run_id} output={output}");
                } else {
                    println!("[RunFinished] run_id={run_id}");
                }
            }
            RunEvent::RunFailed { run_id, error } => {
                println!("[RunFailed] run_id={run_id} error={error}");
            }
            RunEvent::BeforeToolCall {
                run_id,
                tool_name,
                arguments,
            } => {
                if self.verbose {
                    println!("[BeforeToolCall] run_id={run_id} tool={tool_name} args={arguments}");
                }
            }
            RunEvent::AfterToolCall {
                run_id,
                tool_name,
                result,
            } => {
                if self.verbose {
                    let truncated = if result.len() > 100 {
                        format!("{}...", &result[..100])
                    } else {
                        result
                    };
                    println!("[AfterToolCall] run_id={run_id} tool={tool_name} result={truncated}");
                }
            }
            RunEvent::BeforeMessage {
                run_id,
                message_count,
            } => {
                if self.verbose {
                    println!("[BeforeMessage] run_id={run_id} messages={message_count}");
                }
            }
            RunEvent::AfterMessage {
                run_id,
                response_length,
            } => {
                if self.verbose {
                    println!("[AfterMessage] run_id={run_id} response_len={response_length}");
                }
            }
        }
        Ok(())
    }
}
