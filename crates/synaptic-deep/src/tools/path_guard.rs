use std::path::{Component, Path, PathBuf};
use synaptic_core::SynapticError;

/// Security guard for model-facing filesystem tools.
///
/// Validates that paths resolve to within allowed root directories.
/// Rejects path traversal and symlink escapes.
///
/// Not used by internal subsystems (SkillsMiddleware, PluginManager)
/// which access Backend directly.
pub struct PathGuard {
    allowed_roots: Vec<PathBuf>,
}

impl PathGuard {
    /// Create with cwd as sole allowed root. Canonicalizes the path.
    pub fn new(cwd: PathBuf) -> Self {
        let canonical = cwd.canonicalize().unwrap_or(cwd);
        Self {
            allowed_roots: vec![canonical],
        }
    }

    /// Add extra allowed roots. Each is canonicalized.
    pub fn with_extra_roots(mut self, roots: Vec<PathBuf>) -> Self {
        for root in roots {
            let canonical = root.canonicalize().unwrap_or(root);
            self.allowed_roots.push(canonical);
        }
        self
    }

    /// Validate path for read operations (file must exist for canonicalize).
    pub fn validate_read(&self, path: &str) -> Result<PathBuf, SynapticError> {
        self.validate_inner(path, true)
    }

    /// Validate path for write operations (file may not exist yet).
    pub fn validate_write(&self, path: &str) -> Result<PathBuf, SynapticError> {
        self.validate_inner(path, false)
    }

    fn validate_inner(&self, path: &str, must_exist: bool) -> Result<PathBuf, SynapticError> {
        let p = Path::new(path);
        if p.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(SynapticError::Tool("path traversal rejected".into()));
        }

        let resolved = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.allowed_roots[0].join(p)
        };

        let canonical = if must_exist || resolved.exists() {
            resolved
                .canonicalize()
                .map_err(|e| SynapticError::Tool(format!("cannot resolve path: {}", e)))?
        } else {
            // Write to new file: canonicalize parent, append filename
            let parent = resolved
                .parent()
                .ok_or_else(|| SynapticError::Tool("invalid path".into()))?;
            let parent_canon = parent
                .canonicalize()
                .map_err(|e| SynapticError::Tool(format!("parent directory not found: {}", e)))?;
            parent_canon.join(resolved.file_name().unwrap_or_default())
        };

        if !self
            .allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root))
        {
            return Err(SynapticError::Tool(format!(
                "path outside allowed roots: {}",
                path
            )));
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_parent_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        assert!(guard.validate_read("../etc/passwd").is_err());
        assert!(guard.validate_write("../etc/passwd").is_err());
    }

    #[test]
    fn allows_dotdot_in_filename() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("my..config.toml");
        fs::write(&file, "test").unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        assert!(guard.validate_read("my..config.toml").is_ok());
    }

    #[test]
    fn allows_relative_path_inside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let canonical_root = tmp.path().canonicalize().unwrap();
        let file = canonical_root.join("test.txt");
        fs::write(&file, "hello").unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        let result = guard.validate_read("test.txt").unwrap();
        assert_eq!(result, canonical_root.join("test.txt"));
    }

    #[test]
    fn rejects_absolute_path_outside_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        assert!(guard.validate_read("/etc/passwd").is_err());
    }

    #[test]
    fn allows_absolute_path_inside_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("inside.txt");
        fs::write(&file, "ok").unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        let abs = file.to_str().unwrap();
        assert!(guard.validate_read(abs).is_ok());
    }

    #[test]
    fn write_validates_parent_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(tmp.path().to_path_buf());
        // Parent exists, file doesn't — should pass
        assert!(guard.validate_write("new_file.txt").is_ok());
        // Parent doesn't exist — should fail
        assert!(guard.validate_write("nonexistent_dir/file.txt").is_err());
    }

    #[test]
    fn extra_roots_work() {
        let root1 = tempfile::tempdir().unwrap();
        let root2 = tempfile::tempdir().unwrap();
        let file = root2.path().join("extra.txt");
        fs::write(&file, "extra").unwrap();

        let guard = PathGuard::new(root1.path().to_path_buf())
            .with_extra_roots(vec![root2.path().to_path_buf()]);

        let abs = file.to_str().unwrap();
        assert!(guard.validate_read(abs).is_ok());
    }
}
