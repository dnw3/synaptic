# SSRF Guard

Middleware that prevents Server-Side Request Forgery (SSRF) attacks by inspecting tool call arguments for URLs pointing to private, loopback, or otherwise restricted addresses.

## Why Agents Need SSRF Protection

AI agents that execute tool calls based on LLM output are vulnerable to SSRF. A malicious prompt or injected instruction can trick the model into making requests to internal services -- fetching cloud metadata endpoints (e.g. AWS `169.254.169.254`), scanning private networks, or exfiltrating data through internal APIs. The SSRF guard inspects every tool call before execution, blocking URLs that resolve to dangerous destinations.

## Constructor

```rust,ignore
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};

let mw = SsrfGuardMiddleware::new(SsrfGuardConfig::default());
```

The default configuration blocks all private/loopback addresses and scans the standard URL argument keys.

## SsrfGuardConfig

Customize the guard behavior through `SsrfGuardConfig`:

```rust,ignore
use std::collections::HashSet;
use synaptic::middleware::SsrfGuardConfig;

let config = SsrfGuardConfig {
    block_private: true,
    blocklist: HashSet::from(["evil.com".to_string()]),
    allowlist: HashSet::new(),
    url_keys: vec![
        "url".to_string(),
        "uri".to_string(),
        "endpoint".to_string(),
        "base_url".to_string(),
        "webhook_url".to_string(),
    ],
};
```

**Fields:**

- `block_private` -- When `true` (the default), blocks URLs pointing to private, loopback, and link-local addresses.
- `blocklist` -- Additional hostnames that should always be blocked, even if they resolve to public addresses.
- `allowlist` -- Hostnames that are always allowed, overriding `block_private` for those specific hosts.
- `url_keys` -- Tool argument keys that are inspected for URLs. The middleware also recursively scans nested objects and checks any string value that looks like a URL.

## What Gets Blocked

When `block_private` is enabled, the following are blocked:

- **Loopback:** `localhost`, `127.0.0.1`, `::1`
- **Private IPv4:** `10.x.x.x`, `172.16.x.x` -- `172.31.x.x`, `192.168.x.x`
- **Link-local:** `169.254.x.x`, `fe80::/10`
- **Cloud metadata:** `169.254.169.254` (AWS/GCP metadata endpoint)
- **Private hostnames:** `*.local`, `*.internal`, `metadata.google.internal`
- **CGNAT range:** `100.64.0.0/10`
- **Unspecified:** `0.0.0.0`, `::`
- **Broadcast:** `255.255.255.255`
- **IPv6 unique local:** `fc00::/7`

Public URLs (e.g. `https://api.openai.com`) pass through without interference.

## Usage with `create_agent`

```rust,ignore
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};

let options = AgentOptions {
    middleware: vec![
        Arc::new(SsrfGuardMiddleware::new(SsrfGuardConfig::default())),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

## How It Works

- **Lifecycle hook:** `wrap_tool_call`
- Before each tool call is executed, the middleware scans the tool's JSON arguments.
- It checks keys listed in `url_keys` for URL strings, and also recurses into nested objects and arrays.
- Any standalone string value starting with `http://` or `https://` is also checked.
- If a URL's host is on the blocklist or resolves to a private/restricted address, the middleware returns a `SynapticError::Security` error and the tool call is never executed.
- If the host is on the allowlist, it passes regardless of private IP rules.

## Allowlist Example

Allow a specific internal endpoint while blocking all other private addresses:

```rust,ignore
use std::collections::HashSet;
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};

let mut config = SsrfGuardConfig::default();
config.allowlist.insert("internal-api.company.local".to_string());

let options = AgentOptions {
    middleware: vec![
        Arc::new(SsrfGuardMiddleware::new(config)),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

With this configuration, `http://internal-api.company.local/status` is allowed, but `http://localhost/admin` and `http://192.168.1.1/config` remain blocked.

## Blocklist Example

Block a known-malicious public domain in addition to private addresses:

```rust,ignore
let mut config = SsrfGuardConfig::default();
config.blocklist.insert("evil.com".to_string());
config.blocklist.insert("phishing-site.net".to_string());
```

## Configuration Reference

| Field           | Type              | Default                                               | Description                                             |
|-----------------|-------------------|-------------------------------------------------------|---------------------------------------------------------|
| `block_private` | `bool`            | `true`                                                | Block private/loopback/link-local addresses              |
| `blocklist`     | `HashSet<String>` | `{}`                                                  | Additional hostnames to block                            |
| `allowlist`     | `HashSet<String>` | `{}`                                                  | Hostnames that bypass private-address blocking           |
| `url_keys`      | `Vec<String>`     | `["url", "uri", "endpoint", "base_url", "webhook_url"]` | Argument keys inspected for URLs                        |

## Combining with Other Middleware

The SSRF guard pairs well with other security and reliability middleware:

```rust,ignore
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{
    SsrfGuardMiddleware, SsrfGuardConfig,
    CircuitBreakerMiddleware, CircuitBreakerConfig,
    ToolRetryMiddleware,
};

let options = AgentOptions {
    middleware: vec![
        Arc::new(SsrfGuardMiddleware::new(SsrfGuardConfig::default())),
        Arc::new(CircuitBreakerMiddleware::new(CircuitBreakerConfig::default())),
        Arc::new(ToolRetryMiddleware::new(3)),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

The SSRF guard should generally be listed first so that blocked URLs are rejected before reaching retry or circuit breaker logic.
