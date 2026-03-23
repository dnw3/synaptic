use std::path::Path;

use synaptic_core::SynapticError;

use super::types::*;

const RESERVED_CONTAINER_TARGETS: &[&str] = &["/workspace", "/agent"];

/// Validate sandbox security configuration before creating a container.
///
/// Checks:
/// 1. Reject NetworkMode::Host
/// 2. Validate bind mount host paths against blocked_host_paths (path component matching)
/// 3. Reject bind mounts targeting reserved container paths
/// 4. Reject seccomp/apparmor "unconfined"
/// 5. Validate all host paths are absolute
pub fn validate_sandbox_security(
    config: &SandboxSecurityConfig,
    mounts: &[BindMount],
) -> Result<(), SynapticError> {
    // 1. Network mode
    if config.network_mode == NetworkMode::Host {
        return Err(SynapticError::Security(
            "sandbox: network mode 'host' is not allowed — it bypasses network isolation"
                .to_string(),
        ));
    }

    // 2, 3, 5. Bind mount validation
    for mount in mounts {
        // 5. Host paths must be absolute
        if !mount.host_path.is_absolute() {
            return Err(SynapticError::Security(format!(
                "sandbox: bind mount host path must be absolute: {}",
                mount.host_path.display()
            )));
        }

        // 2. Check against blocked host paths (use Path::starts_with for component matching)
        let host_path = &mount.host_path;
        for blocked in &config.blocked_host_paths {
            let blocked_path = Path::new(blocked);
            if host_path.starts_with(blocked_path) {
                return Err(SynapticError::Security(format!(
                    "sandbox: bind mount source '{}' overlaps blocked host path '{}'",
                    mount.host_path.display(),
                    blocked
                )));
            }
        }

        // 3. Check reserved container targets
        let container_str = mount.container_path.to_string_lossy();
        for reserved in RESERVED_CONTAINER_TARGETS {
            if container_str.as_ref() == *reserved {
                return Err(SynapticError::Security(format!(
                    "sandbox: bind mount cannot target reserved container path '{}'",
                    reserved
                )));
            }
        }
    }

    // 4. Security profiles
    if let Some(ref profile) = config.seccomp_profile {
        if profile == "unconfined" {
            return Err(SynapticError::Security(
                "sandbox: seccomp profile 'unconfined' is not allowed".to_string(),
            ));
        }
    }
    if let Some(ref profile) = config.apparmor_profile {
        if profile == "unconfined" {
            return Err(SynapticError::Security(
                "sandbox: apparmor profile 'unconfined' is not allowed".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn default_config() -> SandboxSecurityConfig {
        SandboxSecurityConfig::default()
    }

    #[test]
    fn test_valid_config_no_mounts() {
        assert!(validate_sandbox_security(&default_config(), &[]).is_ok());
    }

    #[test]
    fn test_reject_host_network() {
        let mut config = default_config();
        config.network_mode = NetworkMode::Host;
        let result = validate_sandbox_security(&config, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn test_reject_blocked_host_path() {
        let mounts = vec![BindMount {
            host_path: PathBuf::from("/etc/passwd"),
            container_path: PathBuf::from("/data/passwd"),
            read_only: true,
        }];
        let result = validate_sandbox_security(&default_config(), &mounts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("/etc"));
    }

    #[test]
    fn test_accept_similar_but_not_blocked_path() {
        // /etc-custom should NOT match /etc (component-level matching)
        let mounts = vec![BindMount {
            host_path: PathBuf::from("/etc-custom/data"),
            container_path: PathBuf::from("/data"),
            read_only: true,
        }];
        let result = validate_sandbox_security(&default_config(), &mounts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_reserved_container_target() {
        let mounts = vec![BindMount {
            host_path: PathBuf::from("/home/user/data"),
            container_path: PathBuf::from("/workspace"),
            read_only: false,
        }];
        let result = validate_sandbox_security(&default_config(), &mounts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_reject_relative_host_path() {
        let mounts = vec![BindMount {
            host_path: PathBuf::from("relative/path"),
            container_path: PathBuf::from("/data"),
            read_only: true,
        }];
        let result = validate_sandbox_security(&default_config(), &mounts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute"));
    }

    #[test]
    fn test_reject_unconfined_seccomp() {
        let mut config = default_config();
        config.seccomp_profile = Some("unconfined".to_string());
        assert!(validate_sandbox_security(&config, &[]).is_err());
    }

    #[test]
    fn test_reject_unconfined_apparmor() {
        let mut config = default_config();
        config.apparmor_profile = Some("unconfined".to_string());
        assert!(validate_sandbox_security(&config, &[]).is_err());
    }

    #[test]
    fn test_valid_mount() {
        let mounts = vec![BindMount {
            host_path: PathBuf::from("/home/user/project"),
            container_path: PathBuf::from("/project"),
            read_only: true,
        }];
        assert!(validate_sandbox_security(&default_config(), &mounts).is_ok());
    }
}
