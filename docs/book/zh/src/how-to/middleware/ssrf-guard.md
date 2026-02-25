# SSRF 防护

防止服务端请求伪造 (SSRF) 攻击的中间件。

## 简介

SSRF（Server-Side Request Forgery，服务端请求伪造）是一种攻击手段，攻击者通过控制服务器发出的请求来访问内部网络资源。在 AI Agent 场景中，SSRF 风险尤为突出：Agent 会根据 LLM 的输出自动调用工具，而 LLM 可能被提示注入（prompt injection）引导去访问不安全的内部地址。

`SsrfGuardMiddleware` 通过拦截工具调用中的 URL 参数，阻止对私有网络地址、云平台元数据端点等敏感目标的访问。

## 构造

```rust,ignore
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};

let guard = SsrfGuardMiddleware::new(SsrfGuardConfig::default());
```

默认配置会阻止所有对私有/回环地址的访问，同时允许所有公网 URL。

## SsrfGuardConfig

```rust,ignore
use std::collections::HashSet;
use synaptic::middleware::SsrfGuardConfig;

let config = SsrfGuardConfig {
    block_private: true,
    blocklist: HashSet::from(["evil.com".to_string()]),
    allowlist: HashSet::from(["trusted-internal.local".to_string()]),
    url_keys: vec![
        "url".to_string(),
        "uri".to_string(),
        "endpoint".to_string(),
        "base_url".to_string(),
        "webhook_url".to_string(),
    ],
};
```

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `block_private` | `bool` | `true` | 是否阻止对私有/回环 IP 地址的访问 |
| `blocklist` | `HashSet<String>` | `{}` | 额外需要阻止的主机名 |
| `allowlist` | `HashSet<String>` | `{}` | 始终允许的主机名（覆盖 `block_private`） |
| `url_keys` | `Vec<String>` | `["url", "uri", "endpoint", "base_url", "webhook_url"]` | 工具参数中包含 URL 的键名 |

## 拦截范围

当 `block_private` 为 `true` 时，以下地址会被拦截：

| 类别 | 示例 |
|------|------|
| 回环地址 | `localhost`、`127.0.0.1`、`::1` |
| 私有 IPv4 | `10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16` |
| 链路本地 | `169.254.0.0/16` |
| CGNAT | `100.64.0.0/10` |
| AWS 元数据 | `169.254.169.254` |
| mDNS / 服务发现 | `*.local`、`*.internal` |
| GCP 元数据 | `metadata.google.internal` |
| 广播/未指定 | `0.0.0.0`、`255.255.255.255` |
| 私有 IPv6 | `fc00::/7`（唯一本地）、`fe80::/10`（链路本地） |

中间件还会递归扫描工具参数中的嵌套对象和数组，检查所有以 `http://` 或 `https://` 开头的字符串值。

## 与 `create_agent` 集成

```rust,ignore
use std::sync::Arc;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};
use synaptic::openai::OpenAiChatModel;

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));

let options = AgentOptions {
    middleware: vec![
        Arc::new(SsrfGuardMiddleware::new(SsrfGuardConfig::default())),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

当工具参数中包含被拦截的 URL 时，中间件会返回 `SynapticError::Security` 错误，消息格式为：

```text
SSRF blocked: access to private host 'localhost' is blocked (tool: web_fetch)
```

## Allowlist 示例

在某些场景下，Agent 需要访问内网服务（例如公司内部 API）。可以通过 `allowlist` 精确放行特定主机：

```rust,ignore
use std::collections::HashSet;
use synaptic::middleware::{SsrfGuardMiddleware, SsrfGuardConfig};

let mut config = SsrfGuardConfig::default();
config.allowlist.insert("internal-api.company.local".to_string());
config.allowlist.insert("10.0.1.50".to_string());

let guard = SsrfGuardMiddleware::new(config);
```

此配置允许访问 `internal-api.company.local` 和 `10.0.1.50`，同时继续阻止其他所有私有地址。

也可以结合 `blocklist` 阻止特定的公网域名：

```rust,ignore
let mut config = SsrfGuardConfig::default();
config.blocklist.insert("malicious-site.com".to_string());
config.blocklist.insert("data-exfil.io".to_string());

let guard = SsrfGuardMiddleware::new(config);
```

## 配置参考表

| 配置项 | 说明 | 建议 |
|--------|------|------|
| `block_private: true` | 阻止所有私有网络访问 | 生产环境始终开启 |
| `blocklist` | 额外阻止的公网域名 | 添加已知恶意或不可信的域名 |
| `allowlist` | 允许访问的私有地址 | 仅放行 Agent 确实需要访问的内网服务 |
| `url_keys` | 工具参数中 URL 键名 | 如果自定义工具使用非标准键名，需要添加到此列表 |

**最佳实践：**

- 在生产环境中始终启用 SSRF 防护（`block_private: true`）。
- `allowlist` 应遵循最小权限原则，仅放行必要的内网地址。
- 如果工具参数中使用了自定义的 URL 键名（如 `api_endpoint`），记得添加到 `url_keys` 中。
- 结合其他安全中间件（如 `SecurityMiddleware`）提供多层防护。
