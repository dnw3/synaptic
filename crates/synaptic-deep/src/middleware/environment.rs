use async_trait::async_trait;
use synaptic_core::{RunContext, SynapticError};
use synaptic_middleware::{Interceptor, ModelCaller, ModelRequest, ModelResponse};

/// Channel capability information.
#[derive(Debug, Clone, Default)]
pub struct ChannelInfo {
    /// Channel name: "web", "lark", "slack", "telegram", "discord", "dingtalk", "wechat", "teams", "repl"
    pub name: String,
    /// What the channel supports (e.g. "streaming", "canvas", "file_upload").
    pub capabilities: Vec<String>,
    /// Max message length (0 = unlimited).
    pub message_limit: usize,
}

/// Runtime environment information injected into system prompt.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentInfo {
    pub cwd: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub shell: Option<String>,
    pub timezone: Option<String>,
    pub git_root: Option<String>,
    pub git_branch: Option<String>,
    pub model_id: Option<String>,
    pub channel: Option<ChannelInfo>,
    /// Free-form key-value pairs for product-specific info.
    pub extra: Vec<(String, String)>,
}

impl EnvironmentInfo {
    /// Auto-detect cwd, os, arch, shell, and timezone from the runtime environment.
    pub fn detect() -> Self {
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());

        let os = {
            let base = std::env::consts::OS;
            #[cfg(unix)]
            {
                let version = std::process::Command::new("uname")
                    .arg("-r")
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                        } else {
                            None
                        }
                    });
                match version {
                    Some(v) => Some(format!("{} {}", base, v)),
                    None => Some(base.to_string()),
                }
            }
            #[cfg(not(unix))]
            {
                Some(base.to_string())
            }
        };

        let arch = Some(std::env::consts::ARCH.to_string());
        let shell = std::env::var("SHELL").ok();
        let timezone = std::env::var("TZ").ok();

        Self {
            cwd,
            os,
            arch,
            shell,
            timezone,
            git_root: None,
            git_branch: None,
            model_id: None,
            channel: None,
            extra: Vec::new(),
        }
    }
}

/// Middleware that injects runtime environment and channel information into the
/// agent's system prompt so the agent has self-awareness.
pub struct EnvironmentMiddleware {
    env: EnvironmentInfo,
    /// Optional "self" section with product-specific self-awareness text.
    self_section: Option<String>,
}

impl EnvironmentMiddleware {
    pub fn new(env: EnvironmentInfo) -> Self {
        Self {
            env,
            self_section: None,
        }
    }

    pub fn with_self_section(mut self, section: String) -> Self {
        self.self_section = Some(section);
        self
    }

    fn format_section(&self) -> String {
        let mut lines = Vec::new();
        lines.push("\n# Environment\n".to_string());

        if let Some(ref cwd) = self.env.cwd {
            lines.push(format!("- **cwd**: {}", cwd));
        }

        // Combine os + arch into a single line when both present.
        match (&self.env.os, &self.env.arch) {
            (Some(os), Some(arch)) => lines.push(format!("- **os**: {} ({})", os, arch)),
            (Some(os), None) => lines.push(format!("- **os**: {}", os)),
            _ => {}
        }

        if let Some(ref shell) = self.env.shell {
            lines.push(format!("- **shell**: {}", shell));
        }
        if let Some(ref model_id) = self.env.model_id {
            lines.push(format!("- **model**: {}", model_id));
        }
        if let Some(ref tz) = self.env.timezone {
            lines.push(format!("- **timezone**: {}", tz));
        }
        if let Some(ref git_root) = self.env.git_root {
            lines.push(format!("- **git_root**: {}", git_root));
        }
        if let Some(ref git_branch) = self.env.git_branch {
            lines.push(format!("- **git_branch**: {}", git_branch));
        }

        if let Some(ref ch) = self.env.channel {
            let caps = if ch.capabilities.is_empty() {
                String::new()
            } else {
                ch.capabilities.join(", ")
            };
            let limit = if ch.message_limit == 0 {
                "unlimited".to_string()
            } else {
                ch.message_limit.to_string()
            };
            if caps.is_empty() {
                lines.push(format!(
                    "- **channel**: {} (message_limit: {})",
                    ch.name, limit
                ));
            } else {
                lines.push(format!(
                    "- **channel**: {} ({}; message_limit: {})",
                    ch.name, caps, limit
                ));
            }
        }

        for (key, value) in &self.env.extra {
            lines.push(format!("- **{}**: {}", key, value));
        }

        if let Some(ref self_sec) = self.self_section {
            lines.push(String::new());
            lines.push(self_sec.clone());
        }

        lines.push(String::new());
        lines.join("\n")
    }
}

#[async_trait]
impl Interceptor for EnvironmentMiddleware {
    async fn wrap_model_call(
        &self,
        mut request: ModelRequest,
        ctx: &RunContext,
        next: &dyn ModelCaller,
    ) -> Result<ModelResponse, SynapticError> {
        let section = self.format_section();
        let section_len = section.len();
        if let Some(ref mut prompt) = request.system_prompt {
            prompt.push_str(&section);
        } else {
            request.system_prompt = Some(section);
        }
        tracing::debug!(
            section_len,
            channel = self
                .env
                .channel
                .as_ref()
                .map(|c| c.name.as_str())
                .unwrap_or("unknown"),
            has_self_section = self.self_section.is_some(),
            "EnvironmentMiddleware injected"
        );
        next.call(request, ctx).await
    }
}
