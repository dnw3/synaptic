# Introduction

**Synapse is a Rust agent framework with LangChain-compatible architecture.**

Build production-grade AI agents, chains, and retrieval pipelines in Rust with the same mental model you know from LangChain -- but with compile-time safety, zero-cost abstractions, and native async performance.

## Why Synapse?

- **Type-safe** -- Message types, tool definitions, and runnable pipelines are checked at compile time. No runtime surprises from mismatched schemas.
- **Async-native** -- Built on Tokio and `async-trait` from the ground up. Every trait method is async, and streaming is a first-class citizen via `Stream`.
- **Composable** -- LCEL-style pipe operator (`|`), parallel branches, conditional routing, and fallback chains let you build complex workflows from simple parts.
- **LangChain-compatible** -- Familiar concepts map directly: `ChatPromptTemplate`, `StateGraph`, `create_react_agent`, `ToolNode`, `VectorStoreRetriever`, and more.

## Features at a Glance

| Area | What you get |
|---|---|
| [Chat Models](how-to/chat-models/index.md) | OpenAI, Anthropic, Gemini, Ollama adapters with streaming, retry, rate limiting, and caching |
| [Messages](how-to/messages/index.md) | Typed message enum with factory methods, filtering, trimming, and merge utilities |
| [Prompts](how-to/prompts/index.md) | Template interpolation, chat prompt templates, few-shot prompting |
| [Output Parsers](how-to/output-parsers/index.md) | String, JSON, structured, list, enum, boolean, XML parsers |
| [Runnables (LCEL)](how-to/runnables/index.md) | Pipe operator, parallel, branch, assign/pick, bind, fallbacks, retry |
| [Tools](how-to/tools/index.md) | Tool trait, registry, serial/parallel execution, tool choice |
| [Memory](how-to/memory/index.md) | Buffer, window, summary, token buffer, summary buffer strategies |
| [Graph](how-to/graph/index.md) | LangGraph-style state machines with checkpointing, streaming, and human-in-the-loop |
| [Retrieval](how-to/retrieval/index.md) | Loaders, splitters, embeddings, vector stores, BM25, multi-query, ensemble retrievers |
| [Evaluation](how-to/evaluation/index.md) | Exact match, regex, JSON validity, embedding distance, LLM judge evaluators |
| [Callbacks](how-to/callbacks/index.md) | Recording, tracing, composite callback handlers |

## Quick Links

- [What is Synapse?](what-is-synapse.md) -- Concept mapping from LangChain Python to Synapse Rust
- [Architecture Overview](architecture-overview.md) -- Layered crate design and dependency graph
- [Installation](installation.md) -- Add Synapse to your project
- [Quickstart](quickstart.md) -- Your first Synapse program in 30 lines
- [Tutorials](tutorials/simple-llm-app.md) -- Step-by-step guides for common use cases
- [API Reference](api-reference.md) -- Full API documentation
