//! Skill/Plugin system for Deep agents.
//!
//! A [`Skill`] bundles a set of tools with metadata ([`SkillManifest`]).
//! The [`SkillRegistry`] manages loaded skills, collects their tools,
//! and aggregates system-prompt fragments.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use synaptic_core::{SynapticError, Tool};

/// Metadata about a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique skill identifier (e.g. "web-search", "code-review").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Semantic version.
    pub version: String,
    /// Author or maintainer.
    pub author: Option<String>,
    /// Tags for discovery.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A skill bundles a set of tools with metadata.
#[async_trait]
pub trait Skill: Send + Sync {
    /// Return the skill manifest.
    fn manifest(&self) -> &SkillManifest;

    /// Return the tools provided by this skill.
    fn tools(&self) -> Vec<Arc<dyn Tool>>;

    /// Optional system prompt fragment to inject when this skill is active.
    fn system_prompt_fragment(&self) -> Option<String> {
        None
    }

    /// Initialize the skill (called once when loaded).
    async fn init(&self) -> Result<(), SynapticError> {
        Ok(())
    }
}

/// Registry that manages loaded skills.
pub struct SkillRegistry {
    skills: Vec<Arc<dyn Skill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }

    /// Register a skill.
    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        self.skills.push(skill);
    }

    /// Get all registered skills.
    pub fn skills(&self) -> &[Arc<dyn Skill>] {
        &self.skills
    }

    /// Find a skill by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Skill>> {
        self.skills.iter().find(|s| s.manifest().name == name)
    }

    /// Collect all tools from all registered skills.
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.skills.iter().flat_map(|s| s.tools()).collect()
    }

    /// Collect system prompt fragments from all skills.
    pub fn system_prompt_additions(&self) -> String {
        self.skills
            .iter()
            .filter_map(|s| s.system_prompt_fragment())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Initialize all skills.
    pub async fn init_all(&self) -> Result<(), SynapticError> {
        for skill in &self.skills {
            skill.init().await?;
        }
        Ok(())
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a skill manifest from a TOML file.
pub fn load_manifest(path: &Path) -> Result<SkillManifest, SynapticError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| SynapticError::Config(format!("cannot read manifest: {}", e)))?;
    toml::from_str(&content).map_err(|e| SynapticError::Config(format!("invalid manifest: {}", e)))
}

/// Discover skill manifests in a directory (looks for manifest.toml files).
pub fn discover_skills(dir: &Path) -> Vec<(PathBuf, SkillManifest)> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("manifest.toml");
            if manifest_path.exists() {
                if let Ok(manifest) = load_manifest(&manifest_path) {
                    results.push((entry.path(), manifest));
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSkill {
        manifest: SkillManifest,
    }

    #[async_trait]
    impl Skill for TestSkill {
        fn manifest(&self) -> &SkillManifest {
            &self.manifest
        }
        fn tools(&self) -> Vec<Arc<dyn Tool>> {
            vec![]
        }
        fn system_prompt_fragment(&self) -> Option<String> {
            Some("I am a test skill.".to_string())
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = SkillRegistry::new();
        let skill = Arc::new(TestSkill {
            manifest: SkillManifest {
                name: "test-skill".to_string(),
                description: "A test skill".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                tags: vec!["test".to_string()],
            },
        });
        registry.register(skill);
        assert_eq!(registry.skills().len(), 1);
        assert!(registry.get("test-skill").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_all_tools_empty() {
        let mut registry = SkillRegistry::new();
        let skill = Arc::new(TestSkill {
            manifest: SkillManifest {
                name: "empty".to_string(),
                description: "No tools".to_string(),
                version: "0.1.0".to_string(),
                author: None,
                tags: vec![],
            },
        });
        registry.register(skill);
        assert!(registry.all_tools().is_empty());
    }

    #[test]
    fn test_system_prompt_additions() {
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(TestSkill {
            manifest: SkillManifest {
                name: "s1".to_string(),
                description: "".to_string(),
                version: "0.1.0".to_string(),
                author: None,
                tags: vec![],
            },
        }));
        let additions = registry.system_prompt_additions();
        assert!(additions.contains("test skill"));
    }

    #[test]
    fn test_discover_skills_empty_dir() {
        let dir = std::env::temp_dir().join("synaptic_test_discover_empty");
        let _ = std::fs::create_dir_all(&dir);
        let results = discover_skills(&dir);
        // Just ensure it doesn't panic. Results depend on filesystem.
        let _ = results;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_init_all() {
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(TestSkill {
            manifest: SkillManifest {
                name: "init-test".to_string(),
                description: "".to_string(),
                version: "0.1.0".to_string(),
                author: None,
                tags: vec![],
            },
        }));
        assert!(registry.init_all().await.is_ok());
    }
}
