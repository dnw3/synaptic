# Prometheus 指标

以 Prometheus 文本格式导出 Agent 指标。`synaptic-metrics` crate 封装 `MetricsCallback`，提供 `/metrics` HTTP 端点供采集。

## 安装

```toml
[dependencies]
synaptic = { version = "0.3", features = ["metrics", "callbacks"] }
```

## 快速开始

```rust,ignore
use std::sync::Arc;
use synaptic::callbacks::MetricsCallback;
use synaptic::metrics::PrometheusExporter;

// 创建 MetricsCallback（附加到 agent/模型）
let metrics = Arc::new(MetricsCallback::new());

// 创建 exporter 并启动服务
let exporter = PrometheusExporter::new(metrics.clone());
let handle = exporter.serve("0.0.0.0:9090").await?;

println!("Prometheus 指标地址: http://{}/metrics", handle.addr());

// ... 运行你的 agent ...

// 完成后停止服务器
handle.stop().await;
```

## 导出的指标

exporter 渲染以下指标（前缀默认为 `synaptic`）：

| 指标 | 类型 | 标签 | 说明 |
|------|------|------|------|
| `synaptic_model_calls_total` | counter | -- | LLM API 调用总次数 |
| `synaptic_model_latency_seconds` | gauge | -- | 平均模型调用延迟 |
| `synaptic_model_errors_total` | counter | -- | 模型调用错误总次数 |
| `synaptic_tokens_input_total` | counter | -- | 输入 token 消耗总量 |
| `synaptic_tokens_output_total` | counter | -- | 输出 token 消耗总量 |
| `synaptic_tool_calls_total` | counter | `tool` | 每个工具的调用次数 |
| `synaptic_tool_latency_seconds` | gauge | `tool` | 每个工具的平均延迟 |
| `synaptic_tool_errors_total` | counter | `tool` | 每个工具的错误次数 |

## 自定义前缀

```rust,ignore
let exporter = PrometheusExporter::new(metrics.clone())
    .with_prefix("myapp");
// 指标将命名为: myapp_model_calls_total 等
```

## 不启动服务器直接渲染

如果你希望集成到已有的 HTTP 框架，可以直接调用 `render()`：

```rust,ignore
let exporter = PrometheusExporter::new(metrics.clone());
let text = exporter.render().await;
// text 为 Prometheus 文本格式，例如：
// # HELP synaptic_model_calls_total Total model calls
// # TYPE synaptic_model_calls_total counter
// synaptic_model_calls_total 42
```

## Prometheus scrape_config 示例

```yaml
scrape_configs:
  - job_name: 'synaptic-agent'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```
