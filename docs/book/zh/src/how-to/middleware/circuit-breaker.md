# 熔断器

实现熔断器模式的中间件，用于工具和模型调用。通过临时阻止对失败服务的调用来防止级联故障。

## 简介

熔断器模式（Circuit Breaker Pattern）是一种容错设计模式，源自电气工程中的断路器概念。当某个外部服务连续失败达到阈值时，熔断器会"断开"，后续对该服务的调用会被立即拒绝，而不是等待超时。经过一段恢复时间后，熔断器允许一个探测请求通过，如果成功则"闭合"恢复正常。

在 AI Agent 场景中，工具可能依赖外部 API（搜索引擎、数据库、第三方服务等），这些服务可能出现间歇性故障。熔断器可以防止 Agent 反复调用已失败的工具，避免浪费 token 和时间。

## 构造

```rust,ignore
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

let cb = CircuitBreakerMiddleware::new(CircuitBreakerConfig::default());
```

使用自定义配置：

```rust,ignore
use std::time::Duration;
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

let config = CircuitBreakerConfig {
    failure_threshold: 3,
    recovery_timeout: Duration::from_secs(30),
};

let cb = CircuitBreakerMiddleware::new(config);
```

## 状态机

熔断器有三个状态，转换关系如下：

```text
        成功
  ┌───────────────┐
  │               │
  ▼               │
Closed ───────► Open ───────► HalfOpen
  ▲   失败>=阈值   │  恢复超时     │
  │               │               │
  └───────────────┴───────────────┘
         成功（探测通过）
```

| 状态 | 行为 | 转换条件 |
|------|------|----------|
| **Closed**（闭合） | 正常放行所有请求 | 连续失败次数 >= `failure_threshold` 时转为 Open |
| **Open**（断开） | 立即拒绝所有请求 | 经过 `recovery_timeout` 时间后转为 HalfOpen |
| **HalfOpen**（半开） | 允许一个探测请求通过 | 探测成功 -> Closed；探测失败 -> Open |

## CircuitBreakerConfig

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `failure_threshold` | `usize` | `5` | 触发熔断的连续失败次数 |
| `recovery_timeout` | `Duration` | `60s` | 从 Open 转为 HalfOpen 的等待时间 |

## 按工具隔离

熔断器为每个工具维护独立的状态追踪器。当某个工具的熔断器打开时，其他工具不受影响。

```rust,ignore
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig, CircuitState};

let cb = CircuitBreakerMiddleware::new(CircuitBreakerConfig {
    failure_threshold: 3,
    ..Default::default()
});

// 工具 A 连续失败 3 次后熔断
// 工具 B 仍然正常工作

// 查询特定工具的熔断状态
let state = cb.state_for("web_search").await;
match state {
    CircuitState::Closed => println!("正常"),
    CircuitState::Open => println!("已熔断"),
    CircuitState::HalfOpen => println!("探测中"),
}
```

这种隔离机制确保单个工具的故障不会影响 Agent 使用其他工具的能力。例如，当搜索 API 不可用时，Agent 仍然可以正常使用计算器、文件读写等其他工具。

## 模型熔断

除了工具调用，熔断器也会包装模型调用。当 LLM API 连续失败时，熔断器会阻止进一步的调用尝试。模型调用使用内部标识 `"__model__"` 作为追踪键。

```rust,ignore
// 模型调用失败时也会触发熔断
// 当 OpenAI API 连续不可用时，熔断器会立即返回错误
// 而不是让每次调用都等待网络超时
```

模型熔断返回 `SynapticError::Model` 错误，消息为：

```text
circuit breaker open for model -- too many consecutive failures
```

## 与 `create_agent` 集成

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use synaptic::graph::{create_agent, AgentOptions};
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};
use synaptic::openai::OpenAiChatModel;

let model = Arc::new(OpenAiChatModel::new("gpt-4o"));

let options = AgentOptions {
    middleware: vec![
        Arc::new(CircuitBreakerMiddleware::new(CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(30),
        })),
    ],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```

熔断器中间件可以与其他中间件组合使用。建议将熔断器放在中间件链的较前位置，使其在工具重试等中间件之前生效：

```rust,ignore
use synaptic::middleware::{
    CircuitBreakerMiddleware, CircuitBreakerConfig,
    ToolRetryMiddleware, SsrfGuardMiddleware, SsrfGuardConfig,
};

let options = AgentOptions {
    middleware: vec![
        // 1. SSRF 防护（最先检查）
        Arc::new(SsrfGuardMiddleware::new(SsrfGuardConfig::default())),
        // 2. 熔断器（在重试之前）
        Arc::new(CircuitBreakerMiddleware::new(CircuitBreakerConfig::default())),
        // 3. 工具重试（最后）
        Arc::new(ToolRetryMiddleware::new(3)),
    ],
    ..Default::default()
};
```

## 用 ScriptedChatModel 测试

无需真实 API 即可测试熔断器行为：

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use synaptic::core::{ChatResponse, Message};
use synaptic::models::ScriptedChatModel;
use synaptic::graph::{create_agent, AgentOptions, MessageState};
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig};

// 验证熔断器状态转换
let config = CircuitBreakerConfig {
    failure_threshold: 2,
    recovery_timeout: Duration::from_millis(100),
};
let cb = CircuitBreakerMiddleware::new(config);

// 初始状态：Closed
assert_eq!(cb.state_for("test_tool").await, CircuitState::Closed);

// 模拟工具失败（通过 Agent 运行触发）
// 连续失败 2 次后，状态变为 Open

// 等待恢复超时后，状态变为 HalfOpen
tokio::time::sleep(Duration::from_millis(150)).await;
assert_eq!(cb.state_for("test_tool").await, CircuitState::HalfOpen);
```

完整的集成测试示例：

```rust,ignore
use std::sync::Arc;
use synaptic::core::{ChatResponse, Message};
use synaptic::models::ScriptedChatModel;
use synaptic::middleware::{CircuitBreakerMiddleware, CircuitBreakerConfig, CircuitState};

let config = CircuitBreakerConfig {
    failure_threshold: 3,
    recovery_timeout: Duration::from_secs(60),
};
let cb = Arc::new(CircuitBreakerMiddleware::new(config));

// ScriptedChatModel 配合 AgentOptions 测试完整的 Agent 流程
let model = Arc::new(ScriptedChatModel::new(vec![
    ChatResponse {
        message: Message::ai("Let me search for that."),
        usage: None,
    },
]));

let options = AgentOptions {
    middleware: vec![cb.clone()],
    ..Default::default()
};

let graph = create_agent(model, tools, options)?;
```
