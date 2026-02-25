//! OAuth 2.1 + PKCE support for MCP server connections.
//!
//! Provides [`McpOAuthConfig`] for configuring OAuth client credentials flow
//! and [`OAuthTokenManager`] for automatic token acquisition, caching, and
//! refresh with PKCE (S256) support.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use synaptic_core::SynapticError;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// OAuth 2.1 configuration for an MCP server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthConfig {
    /// OAuth client identifier.
    pub client_id: String,
    /// OAuth client secret (omit for public clients).
    #[serde(default)]
    pub client_secret: Option<String>,
    /// Token endpoint URL.
    pub token_url: String,
    /// Authorization endpoint URL (for authorization code flows).
    #[serde(default)]
    pub authorize_url: Option<String>,
    /// Requested scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Whether to use PKCE (S256). Defaults to `true`.
    #[serde(default = "default_pkce")]
    pub pkce: bool,
}

fn default_pkce() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Token response (from the OAuth server)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    refresh_token: Option<String>,
}

// ---------------------------------------------------------------------------
// Cached token
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
    refresh_token: Option<String>,
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

/// Generate a code verifier using a deterministic hash-based approach.
///
/// Uses the current timestamp and token_url as entropy source, hashed through
/// SHA-256, then URL-safe base64 encoded (no padding). The result is always
/// 43 characters, satisfying the PKCE spec (43..128).
pub fn generate_code_verifier(seed: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let input = format!("{}{}{}", seed, now.as_nanos(), std::process::id());
    let hash = Sha256::digest(input.as_bytes());
    base64_url_encode(&hash)
}

/// Compute the S256 code challenge from a code verifier.
///
/// `code_challenge = BASE64URL(SHA256(code_verifier))`
pub fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64_url_encode(&hash)
}

/// URL-safe base64 encoding without padding.
fn base64_url_encode(data: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(data)
}

/// Minimal percent-encoding for application/x-www-form-urlencoded values.
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// OAuthTokenManager
// ---------------------------------------------------------------------------

/// Manages OAuth 2.1 token lifecycle: acquisition, caching, and refresh.
///
/// Thread-safe — the internal state is behind a `Mutex` so multiple concurrent
/// tool calls share and reuse the same cached token.
pub struct OAuthTokenManager {
    config: McpOAuthConfig,
    client: reqwest::Client,
    cached: Arc<Mutex<Option<CachedToken>>>,
}

impl OAuthTokenManager {
    /// Create a new token manager for the given OAuth configuration.
    pub fn new(config: McpOAuthConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            cached: Arc::new(Mutex::new(None)),
        }
    }

    /// Return a valid access token, refreshing or re-acquiring as needed.
    pub async fn get_token(&self) -> Result<String, SynapticError> {
        let mut guard = self.cached.lock().await;

        // Return cached token if still valid.
        if let Some(ref cached) = *guard {
            if Instant::now() < cached.expires_at {
                return Ok(cached.access_token.clone());
            }

            // Try refreshing if we have a refresh token.
            if let Some(ref rt) = cached.refresh_token {
                match self.refresh(rt).await {
                    Ok(new_token) => {
                        *guard = Some(new_token.clone());
                        return Ok(new_token.access_token);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "OAuth refresh failed, falling back to client_credentials: {}",
                            e
                        );
                    }
                }
            }
        }

        // Fresh client_credentials grant.
        let token = self.client_credentials().await?;
        let access_token = token.access_token.clone();
        *guard = Some(token);
        Ok(access_token)
    }

    /// Perform a `client_credentials` grant.
    async fn client_credentials(&self) -> Result<CachedToken, SynapticError> {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("grant_type".to_string(), "client_credentials".to_string());
        params.insert("client_id".to_string(), self.config.client_id.clone());

        if let Some(ref secret) = self.config.client_secret {
            params.insert("client_secret".to_string(), secret.clone());
        }

        if !self.config.scopes.is_empty() {
            params.insert("scope".to_string(), self.config.scopes.join(" "));
        }

        // PKCE: include code_verifier and code_challenge for client_credentials.
        if self.config.pkce {
            let verifier = generate_code_verifier(&self.config.token_url);
            let challenge = generate_code_challenge(&verifier);
            params.insert("code_verifier".to_string(), verifier);
            params.insert("code_challenge".to_string(), challenge);
            params.insert("code_challenge_method".to_string(), "S256".to_string());
        }

        self.exchange_token(&params).await
    }

    /// Perform a `refresh_token` grant.
    async fn refresh(&self, refresh_token: &str) -> Result<CachedToken, SynapticError> {
        let mut params: HashMap<String, String> = HashMap::new();
        params.insert("grant_type".to_string(), "refresh_token".to_string());
        params.insert("refresh_token".to_string(), refresh_token.to_string());
        params.insert("client_id".to_string(), self.config.client_id.clone());

        if let Some(ref secret) = self.config.client_secret {
            params.insert("client_secret".to_string(), secret.clone());
        }

        self.exchange_token(&params).await
    }

    /// Shared POST logic: sends form-encoded params to `token_url` and parses
    /// the JSON response into a [`CachedToken`].
    async fn exchange_token(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<CachedToken, SynapticError> {
        // Build URL-encoded form body manually to avoid reqwest `form` feature.
        let body = params
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let resp = self
            .client
            .post(&self.config.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| SynapticError::Mcp(format!("OAuth token request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SynapticError::Mcp(format!(
                "OAuth token endpoint returned {}: {}",
                status, body
            )));
        }

        let token_resp: TokenResponse = resp.json().await.map_err(|e| {
            SynapticError::Mcp(format!("Failed to parse OAuth token response: {}", e))
        })?;

        // Default to 1 hour if expires_in is not provided, with 30s safety margin.
        let expires_in_secs = token_resp.expires_in.unwrap_or(3600);
        let safety_margin = 30;
        let effective_ttl = expires_in_secs.saturating_sub(safety_margin);

        Ok(CachedToken {
            access_token: token_resp.access_token,
            expires_at: Instant::now() + Duration::from_secs(effective_ttl),
            refresh_token: token_resp.refresh_token,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_verifier_length() {
        let verifier = generate_code_verifier("test-seed");
        // SHA-256 output is 32 bytes; URL-safe base64 no-pad of 32 bytes = 43 chars.
        assert!(
            verifier.len() >= 43,
            "code verifier must be >= 43 chars, got {}",
            verifier.len()
        );
    }

    #[test]
    fn code_challenge_is_base64url() {
        let verifier = generate_code_verifier("test-seed");
        let challenge = generate_code_challenge(&verifier);

        // Must not contain standard base64 chars that are NOT URL-safe.
        assert!(!challenge.contains('+'), "challenge must not contain '+'");
        assert!(!challenge.contains('/'), "challenge must not contain '/'");
        assert!(!challenge.contains('='), "challenge must not contain '='");

        // Must be non-empty.
        assert!(!challenge.is_empty());
    }

    #[test]
    fn oauth_config_default_pkce() {
        let json = serde_json::json!({
            "client_id": "my-client",
            "token_url": "https://auth.example.com/token"
        });
        let config: McpOAuthConfig = serde_json::from_value(json).unwrap();
        assert!(config.pkce, "pkce should default to true");
        assert!(config.client_secret.is_none());
        assert!(config.authorize_url.is_none());
        assert!(config.scopes.is_empty());
    }

    #[test]
    fn oauth_config_full_roundtrip() {
        let config = McpOAuthConfig {
            client_id: "cid".to_string(),
            client_secret: Some("secret".to_string()),
            token_url: "https://auth.example.com/token".to_string(),
            authorize_url: Some("https://auth.example.com/authorize".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
            pkce: false,
        };
        let json = serde_json::to_value(&config).unwrap();
        let deserialized: McpOAuthConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.client_id, "cid");
        assert_eq!(deserialized.client_secret.as_deref(), Some("secret"));
        assert!(!deserialized.pkce);
        assert_eq!(deserialized.scopes, vec!["read", "write"]);
    }

    #[test]
    fn code_challenge_deterministic_for_same_input() {
        let challenge1 = generate_code_challenge("same-verifier");
        let challenge2 = generate_code_challenge("same-verifier");
        assert_eq!(challenge1, challenge2);
    }

    #[test]
    fn code_challenge_differs_for_different_input() {
        let challenge1 = generate_code_challenge("verifier-a");
        let challenge2 = generate_code_challenge("verifier-b");
        assert_ne!(challenge1, challenge2);
    }
}
