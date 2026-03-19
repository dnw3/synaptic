use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use async_trait::async_trait;
use serde_json::Value;
use synaptic_core::SynapticError;

use crate::{Interceptor, ToolCallRequest, ToolCaller};

/// Configuration for SSRF protection.
#[derive(Debug, Clone)]
pub struct SsrfGuardConfig {
    /// Block requests to private/loopback IPs (default: true).
    pub block_private: bool,
    /// Additional hostnames to block.
    pub blocklist: HashSet<String>,
    /// Hostnames that are always allowed (overrides block_private for these).
    pub allowlist: HashSet<String>,
    /// Tool argument keys that contain URLs to inspect.
    pub url_keys: Vec<String>,
}

impl Default for SsrfGuardConfig {
    fn default() -> Self {
        Self {
            block_private: true,
            blocklist: HashSet::new(),
            allowlist: HashSet::new(),
            url_keys: vec![
                "url".to_string(),
                "uri".to_string(),
                "endpoint".to_string(),
                "base_url".to_string(),
                "webhook_url".to_string(),
            ],
        }
    }
}

/// Middleware that prevents SSRF (Server-Side Request Forgery) attacks.
///
/// Inspects tool call arguments for URLs pointing to private/loopback
/// addresses and blocks them. Supports configurable allowlist/blocklist.
pub struct SsrfGuardMiddleware {
    config: SsrfGuardConfig,
}

impl SsrfGuardMiddleware {
    pub fn new(config: SsrfGuardConfig) -> Self {
        Self { config }
    }

    /// Check if a URL is safe to access.
    fn check_url(&self, url: &str) -> Result<(), String> {
        // Parse the URL to extract the host
        let host = extract_host(url).ok_or_else(|| format!("invalid URL: {}", url))?;

        // Check allowlist first
        if self.config.allowlist.contains(&host) {
            return Ok(());
        }

        // Check blocklist
        if self.config.blocklist.contains(&host) {
            return Err(format!("host '{}' is blocklisted", host));
        }

        // Check for private/loopback IPs
        if self.config.block_private {
            if let Ok(ip) = host.parse::<IpAddr>() {
                if is_private_ip(&ip) {
                    return Err(format!(
                        "access to private/loopback address {} is blocked",
                        ip
                    ));
                }
            }

            // Also check common private hostnames
            let lower = host.to_lowercase();
            if lower == "localhost"
                || lower == "0.0.0.0"
                || lower.ends_with(".local")
                || lower.ends_with(".internal")
                || lower == "metadata.google.internal"
                || lower == "169.254.169.254"
            // AWS metadata
            {
                return Err(format!("access to private host '{}' is blocked", host));
            }
        }

        Ok(())
    }

    /// Scan tool arguments for URLs and validate them.
    fn scan_args(&self, args: &Value) -> Result<(), String> {
        match args {
            Value::Object(map) => {
                for (key, value) in map {
                    if self.config.url_keys.iter().any(|k| k == key) {
                        if let Some(url) = value.as_str() {
                            self.check_url(url)?;
                        }
                    }
                    // Recurse into nested objects
                    self.scan_args(value)?;
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.scan_args(item)?;
                }
            }
            Value::String(s) => {
                // Check if string looks like a URL
                if (s.starts_with("http://") || s.starts_with("https://")) && s.len() < 2048 {
                    self.check_url(s)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
impl Interceptor for SsrfGuardMiddleware {
    async fn wrap_tool_call(
        &self,
        request: ToolCallRequest,
        next: &dyn ToolCaller,
    ) -> Result<Value, SynapticError> {
        // Scan arguments for suspicious URLs
        if let Err(reason) = self.scan_args(&request.call.arguments) {
            return Err(SynapticError::Security(format!(
                "SSRF blocked: {} (tool: {})",
                reason, request.call.name
            )));
        }

        next.call(request).await
    }
}

/// Extract host from a URL string.
fn extract_host(url: &str) -> Option<String> {
    // Simple URL parsing without pulling in the `url` crate
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host_port = stripped.split('/').next()?;
    let host = host_port.split(':').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// Check if an IP address is private, loopback, or link-local.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || is_cgnat(v4)
                || v4.is_broadcast()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || is_v6_private(v6),
    }
}

fn is_cgnat(ip: &Ipv4Addr) -> bool {
    // 100.64.0.0/10 (Carrier-grade NAT)
    let octets = ip.octets();
    octets[0] == 100 && (octets[1] & 0xC0) == 64
}

fn is_v6_private(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    // fc00::/7 (unique local)
    (segments[0] & 0xFE00) == 0xFC00
        // fe80::/10 (link-local)
        || (segments[0] & 0xFFC0) == 0xFE80
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_guard() -> SsrfGuardMiddleware {
        SsrfGuardMiddleware::new(SsrfGuardConfig::default())
    }

    #[test]
    fn blocks_localhost() {
        let guard = default_guard();
        assert!(guard.check_url("http://localhost/api").is_err());
        assert!(guard.check_url("http://127.0.0.1/api").is_err());
    }

    #[test]
    fn blocks_private_ips() {
        let guard = default_guard();
        assert!(guard.check_url("http://192.168.1.1/api").is_err());
        assert!(guard.check_url("http://10.0.0.1/api").is_err());
        assert!(guard.check_url("http://172.16.0.1/api").is_err());
    }

    #[test]
    fn blocks_aws_metadata() {
        let guard = default_guard();
        assert!(guard
            .check_url("http://169.254.169.254/latest/meta-data/")
            .is_err());
    }

    #[test]
    fn allows_public_urls() {
        let guard = default_guard();
        assert!(guard.check_url("https://api.openai.com/v1/chat").is_ok());
        assert!(guard.check_url("https://example.com").is_ok());
    }

    #[test]
    fn allowlist_overrides_private() {
        let mut config = SsrfGuardConfig::default();
        config.allowlist.insert("localhost".to_string());
        let guard = SsrfGuardMiddleware::new(config);
        assert!(guard.check_url("http://localhost/api").is_ok());
    }

    #[test]
    fn blocklist_blocks_public() {
        let mut config = SsrfGuardConfig::default();
        config.blocklist.insert("evil.com".to_string());
        let guard = SsrfGuardMiddleware::new(config);
        assert!(guard.check_url("https://evil.com/api").is_err());
    }

    #[test]
    fn scans_nested_args() {
        let guard = default_guard();
        let args = serde_json::json!({
            "config": {
                "url": "http://127.0.0.1/steal"
            }
        });
        assert!(guard.scan_args(&args).is_err());
    }

    #[test]
    fn extract_host_works() {
        assert_eq!(
            extract_host("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_host("http://localhost:8080/api"),
            Some("localhost".to_string())
        );
        assert_eq!(extract_host("not-a-url"), None);
    }
}
