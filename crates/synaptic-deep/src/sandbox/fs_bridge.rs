use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use synaptic_core::SynapticError;

use crate::backend::{Backend, DirEntry, ExecResult, GrepOutputMode};

/// A mount mapping between host and container paths.
#[derive(Debug, Clone)]
pub struct MountMapping {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
}

/// Backend decorator that translates paths between host and container,
/// enforces allowed roots, and respects read-only mounts.
///
/// Unlike `PathGuard`, this performs simple `starts_with` validation
/// without `canonicalize()`, since container-side paths don't exist
/// on the host filesystem.
pub struct FsBridge {
    inner: Arc<dyn Backend>,
    mounts: Vec<MountMapping>,
    allowed_roots: Vec<PathBuf>,
}

impl FsBridge {
    /// Create a new FsBridge wrapping the given backend.
    /// `allowed_roots` are container-side paths (e.g. /workspace, /agent).
    pub fn new(
        inner: Arc<dyn Backend>,
        mounts: Vec<MountMapping>,
        allowed_roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            inner,
            mounts,
            allowed_roots,
        }
    }

    /// Resolve a user-provided path to a container-side path,
    /// validating it against allowed roots.
    fn resolve_path(&self, path: &str) -> Result<String, SynapticError> {
        let p = Path::new(path);

        // Reject path traversal
        if p.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(SynapticError::Tool(
                "sandbox: path traversal rejected".into(),
            ));
        }

        let resolved = if p.is_absolute() {
            p.to_path_buf()
        } else {
            // Relative path — resolve against the first mount's container path
            let mount = self
                .mounts
                .first()
                .ok_or_else(|| SynapticError::Tool("sandbox: no mounts configured".into()))?;
            mount.container_path.join(p)
        };

        // Validate against allowed roots (simple starts_with, no canonicalize)
        let inside = self
            .allowed_roots
            .iter()
            .any(|root| resolved.starts_with(root));

        if !inside {
            return Err(SynapticError::Tool(format!(
                "sandbox: path outside allowed roots: {}",
                resolved.display()
            )));
        }

        Ok(resolved.to_string_lossy().to_string())
    }

    /// Check if a path is within a read-only mount.
    fn is_read_only(&self, path: &str) -> bool {
        let p = Path::new(path);
        self.mounts
            .iter()
            .any(|m| m.read_only && p.starts_with(&m.container_path))
    }

    /// Validate that a write operation is allowed on this path.
    fn check_writable(&self, path: &str) -> Result<(), SynapticError> {
        if self.is_read_only(path) {
            return Err(SynapticError::Tool(format!(
                "sandbox: write denied on read-only mount: {}",
                path
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl Backend for FsBridge {
    async fn ls(&self, path: &str) -> Result<Vec<DirEntry>, SynapticError> {
        let resolved = self.resolve_path(path)?;
        self.inner.ls(&resolved).await
    }

    async fn read_file(
        &self,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<String, SynapticError> {
        let resolved = self.resolve_path(path)?;
        self.inner.read_file(&resolved, offset, limit).await
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), SynapticError> {
        let resolved = self.resolve_path(path)?;
        self.check_writable(&resolved)?;
        self.inner.write_file(&resolved, content).await
    }

    async fn edit_file(
        &self,
        path: &str,
        old_text: &str,
        new_text: &str,
        replace_all: bool,
    ) -> Result<(), SynapticError> {
        let resolved = self.resolve_path(path)?;
        self.check_writable(&resolved)?;
        self.inner
            .edit_file(&resolved, old_text, new_text, replace_all)
            .await
    }

    async fn glob(&self, pattern: &str, base: &str) -> Result<Vec<String>, SynapticError> {
        let resolved_base = self.resolve_path(base)?;
        self.inner.glob(pattern, &resolved_base).await
    }

    async fn grep(
        &self,
        pattern: &str,
        path: Option<&str>,
        file_glob: Option<&str>,
        output_mode: GrepOutputMode,
    ) -> Result<String, SynapticError> {
        let resolved_path = match path {
            Some(p) => Some(self.resolve_path(p)?),
            None => None,
        };
        self.inner
            .grep(pattern, resolved_path.as_deref(), file_glob, output_mode)
            .await
    }

    async fn execute(
        &self,
        command: &str,
        timeout: Option<Duration>,
    ) -> Result<ExecResult, SynapticError> {
        // Execute passes through — command runs inside the sandbox container
        self.inner.execute(command, timeout).await
    }

    fn supports_execution(&self) -> bool {
        self.inner.supports_execution()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge_with_mounts(mounts: Vec<MountMapping>, roots: Vec<PathBuf>) -> FsBridge {
        // Use a dummy backend that always errors — we only test path resolution
        struct DummyBackend;

        #[async_trait]
        impl Backend for DummyBackend {
            async fn ls(&self, _: &str) -> Result<Vec<DirEntry>, SynapticError> {
                Ok(vec![])
            }
            async fn read_file(
                &self,
                _: &str,
                _: usize,
                _: usize,
            ) -> Result<String, SynapticError> {
                Ok(String::new())
            }
            async fn write_file(&self, _: &str, _: &str) -> Result<(), SynapticError> {
                Ok(())
            }
            async fn edit_file(
                &self,
                _: &str,
                _: &str,
                _: &str,
                _: bool,
            ) -> Result<(), SynapticError> {
                Ok(())
            }
            async fn glob(&self, _: &str, _: &str) -> Result<Vec<String>, SynapticError> {
                Ok(vec![])
            }
            async fn grep(
                &self,
                _: &str,
                _: Option<&str>,
                _: Option<&str>,
                _: GrepOutputMode,
            ) -> Result<String, SynapticError> {
                Ok(String::new())
            }
        }

        FsBridge::new(Arc::new(DummyBackend), mounts, roots)
    }

    #[test]
    fn allows_absolute_path_inside_root() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/project".into(),
                container_path: "/workspace".into(),
                read_only: false,
            }],
            vec![PathBuf::from("/workspace")],
        );
        assert!(bridge.resolve_path("/workspace/src/main.rs").is_ok());
    }

    #[test]
    fn rejects_absolute_path_outside_root() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/project".into(),
                container_path: "/workspace".into(),
                read_only: false,
            }],
            vec![PathBuf::from("/workspace")],
        );
        assert!(bridge.resolve_path("/etc/passwd").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/project".into(),
                container_path: "/workspace".into(),
                read_only: false,
            }],
            vec![PathBuf::from("/workspace")],
        );
        assert!(bridge.resolve_path("/workspace/../etc/passwd").is_err());
    }

    #[test]
    fn resolves_relative_path() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/project".into(),
                container_path: "/workspace".into(),
                read_only: false,
            }],
            vec![PathBuf::from("/workspace")],
        );
        let resolved = bridge.resolve_path("src/main.rs").unwrap();
        assert_eq!(resolved, "/workspace/src/main.rs");
    }

    #[test]
    fn read_only_mount_blocks_write() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/data".into(),
                container_path: "/data".into(),
                read_only: true,
            }],
            vec![PathBuf::from("/data")],
        );
        assert!(bridge.check_writable("/data/file.txt").is_err());
    }

    #[test]
    fn rw_mount_allows_write() {
        let bridge = bridge_with_mounts(
            vec![MountMapping {
                host_path: "/home/user/project".into(),
                container_path: "/workspace".into(),
                read_only: false,
            }],
            vec![PathBuf::from("/workspace")],
        );
        assert!(bridge.check_writable("/workspace/file.txt").is_ok());
    }

    #[test]
    fn no_mounts_rejects_relative() {
        let bridge = bridge_with_mounts(vec![], vec![PathBuf::from("/workspace")]);
        assert!(bridge.resolve_path("file.txt").is_err());
    }
}
