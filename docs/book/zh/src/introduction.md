# 简介

**Synaptic 是一个基于 Rust 的 AI Agent 框架，兼容 LangChain 架构。**

使用与 LangChain 相同的思维模型，在 Rust 中构建生产级 AI Agent、链式调用和检索管道 -- 同时享受编译时类型安全、零开销抽象和原生异步高性能。

## 为什么选择 Synaptic？

- **类型安全** -- 消息类型、工具定义和 Runnable 管道在编译时进行检查，不会在运行时因 schema 不匹配而出错。
- **异步原生** -- 基于 Tokio 和 `async-trait` 从底层构建。所有 trait 方法均为异步，流式处理（Streaming）通过 `Stream` trait 作为一等公民支持。
- **可组合** -- LCEL 风格的管道运算符（`|`）、并行分支、条件路由和回退链，让你可以用简单的组件构建复杂的工作流。
- **LangChain 兼容** -- 熟悉的概念可以直接映射：`ChatPromptTemplate`、`StateGraph`、`create_react_agent`、`ToolNode`、`VectorStoreRetriever` 等。

## 功能一览

| 领域 | 提供的能力 |
|---|---|
| Chat Models | OpenAI、Anthropic、Gemini、Ollama 适配器，支持流式处理、重试、速率限制和缓存 |
| Messages | 带有工厂方法、过滤、裁剪和合并工具的类型化消息枚举 |
| Prompts | 模板插值、聊天提示模板、少样本提示 |
| Output Parsers | String、JSON、结构化、列表、枚举、布尔、XML 解析器 |
| Runnables (LCEL) | 管道运算符、并行、分支、assign/pick、bind、回退、重试 |
| Tools | Tool trait、注册表、串行/并行执行、tool choice |
| Memory | Buffer、Window、Summary、Token Buffer、Summary Buffer 策略 |
| Graph | LangGraph 风格的状态机，支持 Checkpointing、流式处理和人机交互（Human-in-the-Loop） |
| Retrieval | 加载器、分割器、嵌入、向量存储、BM25、Multi-Query、Ensemble 检索器 |
| Evaluation | 精确匹配、正则、JSON 有效性、嵌入距离、LLM Judge 评估器 |
| Callbacks | Recording、Tracing、Composite 回调处理器 |

## 快速链接

- [什么是 Synaptic？](what-is-synaptic.md) -- 从 LangChain Python 到 Synaptic Rust 的概念映射
- [架构概览](architecture-overview.md) -- 分层 crate 设计和依赖关系图
- [安装](installation.md) -- 将 Synaptic 添加到你的项目
- [快速开始](quickstart.md) -- 用 30 行代码编写你的第一个 Synaptic 程序
- [教程](tutorials/simple-llm-app.md) -- 常见用例的分步指南
- [API 参考](api-reference.md) -- 完整 API 文档
