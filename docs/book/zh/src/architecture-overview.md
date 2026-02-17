# 架构概览

Synapse 采用分层 crate 架构，将核心 trait 与具体实现分离。这种设计使得每个组件可以独立测试、替换和演进，同时通过 Cargo workspace 统一管理版本和依赖。

## 设计原则

1. **Trait 驱动** -- 所有核心抽象（`ChatModel`、`Tool`、`Embeddings`、`Retriever` 等）都定义为 trait。具体实现在独立的 crate 中提供，消费方只依赖 trait，不依赖具体类型。
2. **关注点分离** -- 每个 crate 只负责一个职责：`synaptic-models` 处理 LLM 适配，`synaptic-memory` 处理会话记忆，`synaptic-graph` 处理状态机编排，以此类推。
3. **零开销抽象** -- 利用 Rust 的泛型和 trait 系统，在编译时消除不需要的间接调用。`BoxRunnable` 提供类型擦除以支持动态组合，但核心路径保持零开销。
4. **异步优先** -- 所有 I/O 操作都是异步的，基于 Tokio 运行时。这使得 Synapse 可以高效处理并发请求，无需线程池开销。

## Crate 依赖关系图

```text
                              synaptic (facade)
                                    |
        ┌───────────┬───────────┬───┴───┬───────────┬───────────┐
        |           |           |       |           |           |
   synaptic-graph synaptic-  synaptic- synaptic- synaptic-  synaptic-
        |       runnables   models   cache    eval       callbacks
        |           |           |       |
        |     synaptic-parsers   |  synaptic-embeddings
        |           |           |       |
        |     synaptic-prompts   |  synaptic-vectorstores
        |           |           |       |
        |           |           |  synaptic-retrieval
        |           |           |       |
        |     synaptic-tools     |  synaptic-splitters
        |           |           |       |
        |           |           |  synaptic-loaders
        |           |           |
        └───────────┴───────────┘
                    |
              synaptic-core
                    |
              synaptic-memory
```

所有 crate 最终依赖 `synaptic-core`，后者定义了共享的 trait 和类型。

## 层级说明

### 核心层（Core Layer）

**`synaptic-core`** -- 定义所有共享的 trait 和类型：

- `ChatModel` trait -- LLM 交互的统一接口（`chat()` 和 `stream_chat()`）
- `Message` 枚举 -- `System`、`Human`、`AI`、`Tool` 四种变体
- `ChatRequest` / `ChatResponse` -- 请求/响应结构
- `Tool` trait -- 工具定义和调用接口
- `ToolChoice` -- 工具选择策略（`Auto`、`Required`、`None`、`Specific`）
- `SynapseError` -- 统一错误类型（19 个变体）
- `RunnableConfig` -- 运行时配置（标签、元数据、并发限制等）

### 实现层（Implementation Layer）

| Crate | 职责 |
|---|---|
| `synaptic-models` | LLM 提供商适配器（OpenAI、Anthropic、Gemini、Ollama）及装饰器（重试、速率限制、结构化输出） |
| `synaptic-memory` | 会话记忆策略（Buffer、Window、Summary、Token Buffer、Summary Buffer） |
| `synaptic-callbacks` | 回调处理器（Recording、Tracing、Composite） |
| `synaptic-prompts` | 提示模板（`PromptTemplate`、`ChatPromptTemplate`、`FewShotChatMessagePromptTemplate`） |
| `synaptic-parsers` | 输出解析器（String、JSON、结构化、列表、枚举等） |
| `synaptic-tools` | 工具注册表和执行器 |
| `synaptic-cache` | LLM 缓存（内存缓存、语义缓存） |

### 组合与检索层（Composition & Retrieval Layer）

| Crate | 职责 |
|---|---|
| `synaptic-runnables` | LCEL 组合原语：`Runnable` trait、管道运算符、并行、分支、回退等 |
| `synaptic-graph` | LangGraph 风格状态机：`StateGraph`、`CompiledGraph`、`ToolNode`、`create_react_agent` |
| `synaptic-loaders` | 文档加载器（文本、JSON、CSV、目录） |
| `synaptic-splitters` | 文本分割器（字符、递归、Markdown Header、Token） |
| `synaptic-embeddings` | 嵌入模型（OpenAI、Ollama、Fake） |
| `synaptic-vectorstores` | 向量存储（内存存储、cosine 相似度） |
| `synaptic-retrieval` | 检索器（BM25、Multi-Query、Ensemble、Compression、Self-Query、Parent Document） |
| `synaptic-eval` | 评估器（精确匹配、正则、JSON 有效性、嵌入距离、LLM Judge） |

### Facade 层

**`synaptic`** -- 统一门面 crate，重新导出所有子 crate：

```rust
use synaptic::core::{ChatModel, Message, ChatRequest};
use synaptic::models::OpenAiChatModel;
use synaptic::runnables::{Runnable, RunnableLambda};
use synaptic::graph::{StateGraph, create_react_agent};
```

只需在 `Cargo.toml` 中添加 `synaptic` 一个依赖，即可使用所有功能。

## Workspace 依赖管理

所有共享依赖通过 `[workspace.dependencies]` 统一管理版本：

- `async-trait`、`serde` / `serde_json`、`thiserror 2.0`
- `tokio`（macros + rt-multi-thread + sync + time）
- `tracing` / `tracing-subscriber`
- `reqwest`（json + stream）、`futures`、`async-stream`

Rust edition 2021，最低支持的 Rust 版本（MSRV）为 1.78。
