# MCP OAuth 2.1

为 MCP HTTP 和 SSE 连接提供 OAuth 2.1 认证，支持 PKCE。`synaptic-mcp` crate 提供 `McpOAuthConfig` 和 `OAuthTokenManager`，实现自动令牌管理。

## 安装

```toml
[dependencies]
synaptic = { version = "0.4", features = ["mcp"] }
```

## 配置

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

### McpOAuthConfig 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `client_id` | `String` | OAuth 客户端标识符 |
| `client_secret` | `Option<String>` | 客户端密钥（机密客户端使用） |
| `token_url` | `String` | 令牌端点 URL |
| `authorize_url` | `Option<String>` | 授权端点（授权码流程使用） |
| `scopes` | `Vec<String>` | 请求的权限范围 |
| `pkce` | `bool` | 是否启用 PKCE（S256 代码挑战） |

## 令牌管理器

`OAuthTokenManager` 处理 client_credentials 流程，支持自动令牌缓存和刷新：

```rust,ignore
use std::sync::Arc;
use synaptic::mcp::OAuthTokenManager;

let manager = OAuthTokenManager::new(config);
let token = manager.get_token().await?;
// 令牌会被缓存，过期后自动刷新
```

## PKCE 支持

为公开客户端启用 PKCE（无 client_secret）：

```rust,ignore
let config = McpOAuthConfig {
    client_id: "my-public-client".to_string(),
    client_secret: None,
    token_url: "https://auth.example.com/token".to_string(),
    authorize_url: Some("https://auth.example.com/authorize".to_string()),
    scopes: vec![],
    pkce: true,
};

// 生成 PKCE code verifier 和 challenge
use synaptic::mcp::oauth::{generate_code_verifier, generate_code_challenge};
let verifier = generate_code_verifier();
let challenge = generate_code_challenge(&verifier);
// challenge 为 SHA-256 + base64url 编码
```

## 与 MCP 连接集成

OAuth 在配置到 `McpTool` 后会自动注入到 HTTP 和 SSE 连接中：

```rust,ignore
use synaptic::mcp::{MultiServerMcpClient, McpOAuthConfig};

// OAuth 配置在创建 HTTP/SSE 连接时生效
// 令牌管理器自动添加 Authorization: Bearer <token> 请求头
```
