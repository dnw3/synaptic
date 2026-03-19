//! OS, environment variable, and binary existence filters for skill eligibility.

use super::skill_def::SkillDef;

/// Expand `~` at the start of a path to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

/// Check if a skill's prerequisites are met.
///
/// Returns `true` if all `required_env` vars are set, all `required_bins`
/// are found in `PATH`, OS filter matches, etc.
pub fn is_eligible(skill: &SkillDef) -> bool {
    // Always gate: skip all eligibility checks
    if skill.always {
        return true;
    }

    // G-NEW-2: OS filter
    if !skill.os.is_empty() && !skill.os.iter().any(|o| o == std::env::consts::OS) {
        return false;
    }

    // Required env vars (check override_env first)
    for env_var in &skill.required_env {
        if skill.override_env.contains_key(env_var) {
            continue; // satisfied by override
        }
        if std::env::var(env_var).is_err() {
            return false;
        }
    }

    // Required bins (all must be in PATH)
    if !skill.required_bins.is_empty() {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_var.split(':').collect();
        for bin in &skill.required_bins {
            let found = paths
                .iter()
                .any(|p| std::path::Path::new(p).join(bin).exists());
            if !found {
                return false;
            }
        }
    }

    // G-NEW-3: Any bins (at least one must be in PATH)
    if !skill.required_any_bins.is_empty() {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let paths: Vec<&str> = path_var.split(':').collect();
        let any_found = skill.required_any_bins.iter().any(|bin| {
            paths
                .iter()
                .any(|p| std::path::Path::new(p).join(bin).exists())
        });
        if !any_found {
            return false;
        }
    }

    // G-NEW-1: Required config files
    for config_path in &skill.required_config {
        let expanded = expand_tilde(config_path);
        if !std::path::Path::new(&expanded).exists() {
            return false;
        }
    }

    true
}
