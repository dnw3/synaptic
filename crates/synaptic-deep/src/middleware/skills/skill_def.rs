//! SkillDef struct, frontmatter parsing, and loading from path.

use std::collections::HashMap;

use serde_json::Value;

use super::eligibility::{expand_tilde, is_eligible};

// ---------------------------------------------------------------------------
// SkillDef — full OpenClaw / Agent Skills Standard
// ---------------------------------------------------------------------------

/// A discovered skill with metadata parsed from YAML frontmatter.
///
/// Compatible with the [Agent Skills Standard](https://agentskills.io/) and
/// the Claude Code / OpenClaw SKILL.md format.
#[derive(Debug, Clone, Default)]
pub struct SkillDef {
    // --- Agent Skills Standard (required) ---
    pub name: String,
    pub description: String,
    /// Full path to the SKILL.md file (relative to backend root).
    pub path: String,
    /// Directory containing the SKILL.md file.
    pub base_dir: String,
    /// Markdown body after the frontmatter.
    pub body: String,

    // --- Agent Skills Standard (optional) ---
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: Option<Value>,

    // --- Claude Code / OpenClaw extensions ---
    /// If true, the model cannot auto-invoke this skill (default: false).
    pub disable_model_invocation: bool,
    /// If true, users can invoke via `/name` (default: true).
    pub user_invocable: bool,
    /// Argument hint shown to users, e.g. "[environment]".
    pub argument_hint: Option<String>,
    /// Tool whitelist for this skill.
    pub allowed_tools: Option<Vec<String>>,
    /// Model override for execution.
    pub model: Option<String>,
    /// Execution context: "fork" spawns an isolated sub-agent.
    pub context: Option<String>,
    /// Agent type when context="fork".
    pub agent: Option<String>,
    /// Lifecycle hooks.
    pub hooks: Option<Value>,
    /// Required environment variables for eligibility.
    pub required_env: Vec<String>,
    /// Required binaries that must be in PATH for eligibility.
    pub required_bins: Vec<String>,

    // --- G-NEW-2: OS filter ---
    /// Operating system filter. Values: "darwin", "linux", "windows".
    /// Empty = all OSes allowed.
    pub os: Vec<String>,

    // --- G-NEW-1: Required config files ---
    /// Config file paths that must exist for eligibility (supports ~ expansion).
    pub required_config: Vec<String>,

    // --- G-NEW-3: Any bins (at least one must be in PATH) ---
    /// At least one of these binaries must be in PATH (vs required_bins which needs all).
    pub required_any_bins: Vec<String>,

    // --- G-NEW-5: Command dispatch ---
    /// If "tool", bypass model and dispatch directly to a tool.
    pub command_dispatch: Option<String>,
    /// Target tool name for command_dispatch = "tool".
    pub command_tool: Option<String>,
    /// Argument mode for command dispatch: "passthrough" or "json".
    pub command_arg_mode: Option<String>,

    // --- G-NEW-6: Per-skill config override env ---
    /// Environment variable overrides injected by SkillOverride.
    pub override_env: HashMap<String, String>,

    // --- Homepage ---
    /// URL for the skill's homepage / documentation.
    pub homepage: Option<String>,

    // --- Always gate ---
    /// If true, this skill is always injected (bypasses eligibility checks).
    pub always: bool,

    // --- Install specs (from metadata.openclaw.install) ---
    pub install: Vec<InstallSpec>,

    // --- Additional metadata fields ---
    pub emoji: Option<String>,
    pub skill_key: Option<String>,
    pub primary_env: Option<String>,
    pub version: Option<String>,
}

/// A dependency installation specification from SKILL.md metadata.
/// Compatible with OpenClaw's install spec format.
#[derive(Debug, Clone, Default)]
pub struct InstallSpec {
    /// Install method: "brew", "node", "go", "uv", "download"
    pub kind: String,
    /// Package/formula/module identifier
    pub package: String,
    /// Expected binaries after installation
    pub bins: Vec<String>,
    /// OS filter for this install spec
    pub os: Vec<String>,
    /// Human-readable label
    pub label: Option<String>,
    /// Unique install spec ID
    pub id: Option<String>,
    /// For brew: use cask instead of formula
    pub cask: bool,
    /// For download: target directory
    pub target_dir: Option<String>,
    /// For download: strip N path components from archive
    pub strip_components: Option<u32>,
}

/// Comprehensive skill status report for UI dashboards and CLI diagnostics.
#[derive(Debug, Clone)]
pub struct SkillStatusReport {
    pub name: String,
    pub description: String,
    pub source: String,
    pub path: String,
    pub eligible: bool,
    pub enabled: bool,
    pub always: bool,
    pub user_invocable: bool,
    pub emoji: Option<String>,
    pub homepage: Option<String>,
    pub skill_key: Option<String>,
    /// Missing environment variables
    pub missing_env: Vec<String>,
    /// Missing binaries (from required_bins)
    pub missing_bins: Vec<String>,
    /// Missing any-bins (none of the alternatives found)
    pub missing_any_bins: Vec<String>,
    /// Missing config files
    pub missing_config: Vec<String>,
    /// OS mismatch (skill requires different OS)
    pub os_mismatch: bool,
    /// Available installation specs
    pub install_specs: Vec<InstallSpec>,
}

impl SkillDef {
    /// Check if this skill's prerequisites are met.
    ///
    /// Returns `true` if all `required_env` vars are set and all
    /// `required_bins` are found in `PATH`.
    pub fn is_eligible(&self) -> bool {
        is_eligible(self)
    }

    /// Produce a detailed status report showing what's missing and why.
    pub fn diagnose(&self, source: &str, enabled: bool) -> SkillStatusReport {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_var.split(':').collect();

        let missing_env: Vec<String> = self
            .required_env
            .iter()
            .filter(|e| !self.override_env.contains_key(*e) && std::env::var(e).is_err())
            .cloned()
            .collect();

        let missing_bins: Vec<String> = self
            .required_bins
            .iter()
            .filter(|bin| {
                !paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists())
            })
            .cloned()
            .collect();

        let missing_any_bins: Vec<String> = if !self.required_any_bins.is_empty() {
            let any_found = self.required_any_bins.iter().any(|bin| {
                paths
                    .iter()
                    .any(|p| std::path::Path::new(p).join(bin).exists())
            });
            if any_found {
                Vec::new()
            } else {
                self.required_any_bins.clone()
            }
        } else {
            Vec::new()
        };

        let missing_config: Vec<String> = self
            .required_config
            .iter()
            .filter(|c| !std::path::Path::new(&expand_tilde(c)).exists())
            .cloned()
            .collect();

        let os_mismatch = !self.os.is_empty() && !self.os.iter().any(|o| o == std::env::consts::OS);

        SkillStatusReport {
            name: self.name.clone(),
            description: self.description.clone(),
            source: source.to_string(),
            path: self.path.clone(),
            eligible: self.is_eligible(),
            enabled,
            always: self.always,
            user_invocable: self.user_invocable,
            emoji: self.emoji.clone(),
            homepage: self.homepage.clone(),
            skill_key: self.skill_key.clone(),
            missing_env,
            missing_bins,
            missing_any_bins,
            missing_config,
            os_mismatch,
            install_specs: self.install.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// YAML frontmatter parser
// ---------------------------------------------------------------------------

/// Parse install specs from metadata.openclaw.install array.
fn parse_install_specs(metadata: Option<&Value>) -> Vec<InstallSpec> {
    let oc = metadata.and_then(|m| {
        m.get("openclaw")
            .or_else(|| m.get("clawdbot"))
            .or_else(|| m.get("clawdis"))
    });
    let install_arr = match oc.and_then(|o| o.get("install")).and_then(|i| i.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    install_arr
        .iter()
        .filter_map(|spec| {
            let obj = spec.as_object()?;
            let kind = obj.get("kind").and_then(|v| v.as_str())?.to_string();

            // Package field varies by kind
            let package = match kind.as_str() {
                "brew" => obj.get("formula").and_then(|v| v.as_str()),
                "node" => obj.get("package").and_then(|v| v.as_str()),
                "go" => obj.get("module").and_then(|v| v.as_str()),
                "uv" => obj.get("package").and_then(|v| v.as_str()),
                "download" => obj.get("url").and_then(|v| v.as_str()),
                _ => None,
            }?
            .to_string();

            let bins = obj
                .get("bins")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let os = obj
                .get("os")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(InstallSpec {
                kind,
                package,
                bins,
                os,
                label: obj.get("label").and_then(|v| v.as_str()).map(String::from),
                id: obj.get("id").and_then(|v| v.as_str()).map(String::from),
                cask: obj.get("cask").and_then(|v| v.as_bool()).unwrap_or(false),
                target_dir: obj
                    .get("targetDir")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                strip_components: obj
                    .get("stripComponents")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
            })
        })
        .collect()
}

/// Extract metadata from the openclaw/clawdbot/clawdis block.
fn get_openclaw_metadata(metadata: Option<&Value>) -> Option<&Value> {
    metadata.and_then(|m| {
        m.get("openclaw")
            .or_else(|| m.get("clawdbot"))
            .or_else(|| m.get("clawdis"))
    })
}

/// Parse YAML frontmatter between `---` markers and the markdown body.
pub fn parse_skill_frontmatter(content: &str, path: &str) -> Option<SkillDef> {
    let content = content.trim_start_matches('\u{feff}'); // BOM
    let mut lines = content.lines();

    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut fm_lines = Vec::new();
    let mut body = String::new();
    let mut in_body = false;

    for line in lines {
        if !in_body {
            if line.trim() == "---" {
                in_body = true;
                continue;
            }
            fm_lines.push(line);
        } else {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }

    let yaml_str = fm_lines.join("\n");
    let yaml: serde_json::Value = serde_yml::from_str(&yaml_str).ok()?;
    let map = yaml.as_object()?;

    let name = get_str(map, "name")?;

    // Compute base_dir from path
    let base_dir = path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();

    Some(SkillDef {
        name,
        description: get_str(map, "description").unwrap_or_default(),
        path: path.to_string(),
        base_dir,
        body,

        license: get_str(map, "license"),
        compatibility: get_str(map, "compatibility"),
        metadata: map.get("metadata").cloned(),

        disable_model_invocation: get_bool(map, "disable-model-invocation")
            .or_else(|| get_bool(map, "disable_model_invocation"))
            .unwrap_or(false),
        user_invocable: get_bool(map, "user-invocable")
            .or_else(|| get_bool(map, "user_invocable"))
            .unwrap_or(true),
        argument_hint: get_str(map, "argument-hint").or_else(|| get_str(map, "argument_hint")),
        allowed_tools: parse_allowed_tools(map),
        model: get_str(map, "model"),
        context: get_str(map, "context"),
        agent: get_str(map, "agent"),
        hooks: map.get("hooks").cloned(),
        required_env: get_str_vec(map, "required-env")
            .or_else(|| get_str_vec(map, "required_env"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("env"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        required_bins: get_str_vec(map, "required-bins")
            .or_else(|| get_str_vec(map, "required_bins"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("bins"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        os: get_str_vec(map, "os").unwrap_or_default(),
        required_config: get_str_vec(map, "required-config")
            .or_else(|| get_str_vec(map, "required_config"))
            .unwrap_or_default(),
        required_any_bins: get_str_vec(map, "required-any-bins")
            .or_else(|| get_str_vec(map, "required_any_bins"))
            .or_else(|| {
                get_openclaw_metadata(map.get("metadata"))
                    .and_then(|oc| oc.get("requires"))
                    .and_then(|r| r.get("anyBins"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
            })
            .unwrap_or_default(),
        command_dispatch: get_str(map, "command-dispatch")
            .or_else(|| get_str(map, "command_dispatch")),
        command_tool: get_str(map, "command-tool").or_else(|| get_str(map, "command_tool")),
        command_arg_mode: get_str(map, "command-arg-mode")
            .or_else(|| get_str(map, "command_arg_mode")),
        override_env: HashMap::new(),
        homepage: get_str(map, "homepage"),
        always: get_bool(map, "always").unwrap_or(false),
        install: parse_install_specs(map.get("metadata")),
        emoji: get_openclaw_metadata(map.get("metadata"))
            .and_then(|oc| oc.get("emoji"))
            .and_then(|v| v.as_str())
            .map(String::from),
        skill_key: get_str(map, "skillKey").or_else(|| {
            get_openclaw_metadata(map.get("metadata"))
                .and_then(|oc| oc.get("skillKey"))
                .and_then(|v| v.as_str())
                .map(String::from)
        }),
        primary_env: get_openclaw_metadata(map.get("metadata"))
            .and_then(|oc| oc.get("primaryEnv"))
            .and_then(|v| v.as_str())
            .map(String::from),
        version: get_str(map, "version"),
    })
}

fn get_str(map: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_bool(map: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool())
}

fn get_str_vec(map: &serde_json::Map<String, Value>, key: &str) -> Option<Vec<String>> {
    map.get(key).and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
    })
}

/// Parse `allowed-tools` which can be a space-separated string or array.
fn parse_allowed_tools(map: &serde_json::Map<String, Value>) -> Option<Vec<String>> {
    let val = map
        .get("allowed-tools")
        .or_else(|| map.get("allowed_tools"))?;

    match val {
        Value::String(s) => Some(s.split_whitespace().map(|t| t.to_string()).collect()),
        Value::Array(arr) => Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        ),
        _ => None,
    }
}
