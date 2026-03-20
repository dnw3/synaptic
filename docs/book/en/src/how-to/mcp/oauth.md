# MCP OAuth 2.1

Authenticate MCP HTTP and SSE connections with OAuth 2.1, including PKCE support. The `synaptic-mcp` crate provides `McpOAuthConfig` and `OAuthTokenManager` for automatic token management.

## Setup

```toml
[dependencies]
synaptic = { version = "0.4", features = ["mcp"] }
```

## Configuration

```rust,ignore
use synaptic::mcp::{McpOAuthConfig, OAuthTokenManager};

let config = McpOAuthConfig {
    client_id: "my-client-id".to_string(),
    client_secret: Some("my-secret".to_string()),
    token_url: "https://auth.example.com/token".to_string(),
    authorize_url: None,
    scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
    pkce: false,
};
```

### McpOAuthConfig Fields

| Field | Type | Description |
|-------|------|-------------|
| `client_id` | `String` | OAuth client identifier |
| `client_secret` | `Option<String>` | Client secret (for confidential clients) |
| `token_url` | `String` | Token endpoint URL |
| `authorize_url` | `Option<String>` | Authorization endpoint (for authorization code flow) |
| `scopes` | `Vec<String>` | Requested scopes |
| `pkce` | `bool` | Enable PKCE (S256 code challenge) |

## Token Manager

`OAuthTokenManager` handles the client_credentials flow with automatic token caching and refresh:

```rust,ignore
use std::sync::Arc;
use synaptic::mcp::OAuthTokenManager;

let manager = OAuthTokenManager::new(config);
let token = manager.get_token().await?;
// Token is cached and automatically refreshed when expired
```

## PKCE Support

Enable PKCE for public clients (no client_secret):

```rust,ignore
let config = McpOAuthConfig {
    client_id: "my-public-client".to_string(),
    client_secret: None,
    token_url: "https://auth.example.com/token".to_string(),
    authorize_url: Some("https://auth.example.com/authorize".to_string()),
    scopes: vec![],
    pkce: true,
};

// Generate PKCE code verifier and challenge
use synaptic::mcp::oauth::{generate_code_verifier, generate_code_challenge};
let verifier = generate_code_verifier();
let challenge = generate_code_challenge(&verifier);
// challenge is SHA-256 + base64url encoded
```

## With MCP Connections

OAuth is automatically injected into HTTP and SSE connections when configured on the `McpTool`:

```rust,ignore
use synaptic::mcp::{MultiServerMcpClient, McpOAuthConfig};

// OAuth config is applied when creating HTTP/SSE connections
// The token manager automatically adds Authorization: Bearer <token> headers
```
