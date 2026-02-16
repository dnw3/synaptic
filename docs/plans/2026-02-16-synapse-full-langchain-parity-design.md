# Synapse — Full LangChain Parity Design

## Overview

Synapse is a Rust agent framework that mirrors LangChain's conceptual architecture with Rust-idiomatic implementation. This document defines the complete 12-phase roadmap to achieve full LangChain feature parity.

**Architecture approach:** Rust-idiomatic — LangChain concepts and naming, but traits/generics/Stream-based implementation. No dynamic typing hacks.

## Crate Structure

```
synapse-core          → Core traits, messages, Runnable protocol, RunnableConfig
synapse-models        → ChatModel providers (OpenAI, Anthropic, Gemini, Ollama)
synapse-embeddings    → Embedding providers (OpenAI, Cohere, Ollama, HuggingFace)
synapse-prompts       → Prompt templates (ChatPromptTemplate, FewShot, Pipeline)
synapse-parsers       → Output parsers (JSON, Structured, List, Enum, OutputFixer)
synapse-tools         → Tool trait, registry, executor, ToolDefinition schema
synapse-loaders       → Document loaders (Text, PDF, HTML, CSV, JSON, Web, Directory)
synapse-splitters     → Text splitters (Recursive, Token, Markdown, Code)
synapse-vectorstores  → Vector store trait + backends (Qdrant, Pinecone, PGVector, Redis, Milvus, Chroma, SQLite, Weaviate)
synapse-retrieval     → Retriever trait + advanced strategies (MultiQuery, SelfQuery, Ensemble, Compression, BM25)
synapse-memory        → Memory strategies (Buffer, Window, Summary, Token, VectorStore) + history backends (Redis, PG, SQLite, Mongo)
synapse-agents        → Agent executors (ReAct, ToolCalling) — legacy style
synapse-graph         → StateGraph, CompiledGraph, nodes, edges, checkpointing (LangGraph equivalent)
synapse-callbacks     → Callback traits, tracing integration, event system, OpenTelemetry export
synapse-eval          → Evaluation (datasets, evaluators, metrics, LLM-as-judge)
synapse-guardrails    → Validation guardrails (JSON, schema, content filtering)
synapse-chains        → LCEL composition (RunnableSequence, Parallel, Branch, Lambda, Passthrough, Fallbacks, Retry)
synapse-cache         → LLM caching (InMemory, Redis, SQLite, SemanticCache)
synapse              → Unified facade crate (re-exports all sub-crates)
```

## Phase 1: Core Refactor + Compilation Fix

**Goal:** Fix existing compilation issues, redesign core traits for extensibility.

- Fix `react_basic` and `synapse-agents` `usage` field compilation errors
- Refactor `Message` to enum variants (`SystemMessage`, `HumanMessage`, `AIMessage`, `ToolMessage`) with multimodal content support
- `AIMessage` carries `tool_calls: Vec<ToolCall>` and `usage: Option<TokenUsage>`
- `ToolMessage` carries `tool_call_id: String`
- `AIMessageChunk` for streaming with `+` concatenation
- `RunnableConfig` struct: callbacks, tags, metadata, max_concurrency, recursion_limit
- Expand `SynapseError` with variants for all subsystems
- Clean up Phase 2 placeholder tests so `cargo test --workspace` passes

## Phase 2: Model Providers + Streaming

**Goal:** Connect to real LLMs with streaming support.

Core trait:
```rust
#[async_trait]
pub trait ChatModel: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, SynapseError>;
    fn stream(&self, request: ChatRequest) -> BoxStream<Result<AIMessageChunk, SynapseError>>;
    fn bind_tools(&self, tools: Vec<ToolDefinition>) -> Box<dyn ChatModel>;
}
```

Implementations:
- `OpenAIChatModel` (GPT-4o, GPT-4, o1) — reqwest + SSE
- `AnthropicChatModel` (Claude) — Messages API
- `GeminiChatModel` (Gemini) — Google AI API
- `OllamaChatModel` (local) — Ollama REST API
- `ScriptedChatModel` preserved for testing
- Unified tool calling protocol (provider-specific format differences handled in adapter layer)
- Token usage tracking
- `RetryPolicy` + `RateLimiter`

## Phase 3: LCEL — Runnable Composition Protocol

**Goal:** The universal composition system; all components composable via `|` pipe.

Core trait:
```rust
pub trait Runnable: Send + Sync {
    type Input: Send;
    type Output: Send;
    async fn invoke(&self, input: Self::Input, config: &RunnableConfig) -> Result<Self::Output, SynapseError>;
    fn stream(&self, input: Self::Input, config: &RunnableConfig) -> BoxStream<Result<Self::Output, SynapseError>>;
    async fn batch(&self, inputs: Vec<Self::Input>, config: &RunnableConfig) -> Vec<Result<Self::Output, SynapseError>>;
}
```

Composition types:
- `RunnableSequence<A, B>` — pipe via `BitOr` operator overload
- `RunnableParallel` — concurrent execution, merge to HashMap
- `RunnableLambda` — wrap closures
- `RunnableBranch` — conditional routing
- `RunnablePassthrough` — pass-through (critical for RAG)
- `RunnableWithFallbacks` — failure fallback chain
- `RunnableRetry` — retry with policy
- `RunnableMap` — apply to each element

## Phase 4: Prompt Templates + Output Parsers

**Goal:** Rich prompt templating and structured output parsing.

Prompt Templates:
- `ChatPromptTemplate` — produces `Vec<Message>`, `from_messages()` factory
- `MessagesPlaceholder` — inject dynamic message history
- `SystemMessagePromptTemplate` / `HumanMessagePromptTemplate` / `AIMessagePromptTemplate`
- `FewShotChatMessagePromptTemplate` — few-shot example injection
- `PipelinePromptTemplate` — multi-template composition
- `{variable}` and `{{ jinja2 }}` template formats
- All templates implement `Runnable`

Output Parsers:
- `StrOutputParser` — extract string content
- `JsonOutputParser` — parse JSON, streaming partial JSON
- `StructuredOutputParser<T: DeserializeOwned>` — serde deserialization to any struct
- `ListOutputParser` — parse lists
- `EnumOutputParser` — validate enum values
- `OutputFixingParser` — LLM-assisted fix on parse failure
- All parsers implement `Runnable`

## Phase 5: Document Pipeline (Loaders + Splitters)

**Goal:** Complete document loading and splitting.

Loader trait:
```rust
#[async_trait]
pub trait Loader: Send + Sync {
    async fn load(&self) -> Result<Vec<Document>, SynapseError>;
    fn lazy_load(&self) -> BoxStream<Result<Document, SynapseError>>;
}
```

Implementations: `TextLoader`, `JsonLoader`, `CsvLoader`, `PdfLoader` (pdf-extract/lopdf), `HtmlLoader` (scraper), `MarkdownLoader`, `WebLoader` (reqwest+scraper), `DirectoryLoader`

Splitter trait:
```rust
pub trait TextSplitter: Send + Sync {
    fn split_text(&self, text: &str) -> Vec<String>;
    fn split_documents(&self, docs: Vec<Document>) -> Vec<Document>;
}
```

Implementations: `RecursiveCharacterTextSplitter`, `TokenTextSplitter` (tiktoken-rs), `MarkdownHeaderTextSplitter`, `CodeTextSplitter` (language-aware)

Common parameters: `chunk_size`, `chunk_overlap`, `length_function`

## Phase 6: Embeddings + Vector Stores

**Goal:** Vectorization and vector storage — RAG core infrastructure.

Embeddings trait:
```rust
#[async_trait]
pub trait Embeddings: Send + Sync {
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SynapseError>;
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, SynapseError>;
}
```

Implementations: `OpenAIEmbeddings`, `CohereEmbeddings`, `OllamaEmbeddings`, `FakeEmbeddings`, `CachedEmbeddings`

VectorStore trait:
```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn add_documents(&self, docs: Vec<Document>, embeddings: &dyn Embeddings) -> Result<Vec<String>, SynapseError>;
    async fn similarity_search(&self, query: &str, k: usize) -> Result<Vec<Document>, SynapseError>;
    async fn similarity_search_with_score(&self, query: &str, k: usize) -> Result<Vec<(Document, f32)>, SynapseError>;
    async fn delete(&self, ids: &[&str]) -> Result<(), SynapseError>;
    fn as_retriever(&self, k: usize) -> Box<dyn Retriever>;
}
```

Implementations: `InMemoryVectorStore`, `QdrantVectorStore`, `PineconeVectorStore`, `PGVectorStore`, `WeaviateVectorStore`, `RedisVectorStore`, `MilvusVectorStore`, `ChromaVectorStore`, `SqliteVectorStore`

## Phase 7: Advanced Retrieval

**Goal:** Advanced retrieval strategies for full RAG capability.

Retriever trait (implements Runnable<String, Vec<Document>>):
```rust
#[async_trait]
pub trait Retriever: Send + Sync {
    async fn get_relevant_documents(&self, query: &str) -> Result<Vec<Document>, SynapseError>;
}
```

Implementations:
- `VectorStoreRetriever` — basic vector retrieval
- `MultiQueryRetriever` — LLM generates query variants, merges results
- `SelfQueryRetriever` — LLM converts natural language to structured query + filters
- `ContextualCompressionRetriever` — compress/filter results to relevant parts
- `ParentDocumentRetriever` — index small chunks, return parent document
- `EnsembleRetriever` — weighted Reciprocal Rank Fusion
- `BM25Retriever` — classic keyword retrieval

Document Compressors: `LLMChainExtractor`, `EmbeddingsFilter`, `CohereReranker`

RAG chain constructors:
- `create_stuff_documents_chain(model, prompt)`
- `create_retrieval_chain(retriever, combine_chain)`
- `create_history_aware_retriever(model, retriever, prompt)`

## Phase 8: Graph Agent Orchestration (LangGraph Equivalent)

**Goal:** Graph-based state machine for complex agent workflows.

```rust
pub struct StateGraph<S: State> {
    nodes: HashMap<String, Box<dyn Node<S>>>,
    edges: Vec<Edge>,
    conditional_edges: Vec<ConditionalEdge<S>>,
}
```

Core capabilities:
- `StateGraph` builder — nodes are functions/Runnables, edges are fixed/conditional
- `CompiledGraph` — executable graph, implements Runnable
- `State` trait — state definition + reducer merge strategy
- `ToolNode` — prebuilt node for tool_calls execution
- `create_react_agent(model, tools)` — prebuilt ReAct agent graph

Checkpointing:
```rust
#[async_trait]
pub trait Checkpointer: Send + Sync {
    async fn put(&self, config: &CheckpointConfig, checkpoint: &Checkpoint) -> Result<(), SynapseError>;
    async fn get(&self, config: &CheckpointConfig) -> Result<Option<Checkpoint>, SynapseError>;
    async fn list(&self, config: &CheckpointConfig) -> Result<Vec<Checkpoint>, SynapseError>;
}
```

Implementations: `MemorySaver`, `SqliteSaver`, `PostgresSaver`

Human-in-the-Loop: `interrupt_before`/`interrupt_after`, `update_state()`, subgraph support

## Phase 9: Memory Strategies + Persistence

**Goal:** Rich conversation memory strategies.

Chat History backends: `InMemoryChatHistory`, `RedisChatHistory`, `PostgresChatHistory`, `SqliteChatHistory`, `MongoChatHistory`

Memory strategies:
- `ConversationBufferMemory` — full conversation
- `ConversationWindowMemory` — last K turns
- `ConversationSummaryMemory` — LLM summarization
- `ConversationSummaryBufferMemory` — recent verbatim + older summarized
- `ConversationTokenBufferMemory` — token limit buffer
- `VectorStoreMemory` — vector retrieval memory

`RunnableWithMessageHistory` — wraps any Runnable with auto load/save history.

## Phase 10: Caching, Rate Limiting, Reliability

**Goal:** Production-grade reliability.

LLM Cache trait + implementations: `InMemoryCache`, `RedisCache`, `SqliteCache`, `SemanticCache`

Rate Limiting: `TokenBucketRateLimiter`, per-provider independent policies

Retry & Fallback: `RetryPolicy` (exponential backoff), `RunnableWithFallbacks`

## Phase 11: Observability + Evaluation

**Goal:** Complete observability and evaluation system.

Tracing (built on `tracing` crate ecosystem):
- Auto Span per Runnable invocation
- Run Tree with parent-child tracking
- Event types: `on_llm_start/end/error/token`, `on_chain_start/end`, `on_tool_start/end`, `on_retriever_start/end`
- `astream_events()` — real-time event stream
- OpenTelemetry export (Jaeger/Zipkin/Grafana compatible)

Evaluation:
```rust
pub trait Evaluator: Send + Sync {
    async fn evaluate(&self, prediction: &str, reference: &str, input: &str) -> Result<EvalResult, SynapseError>;
}
```

Implementations: `ExactMatchEvaluator`, `EmbeddingDistanceEvaluator`, `LLMJudgeEvaluator`, `JsonValidityEvaluator`, `RegexMatchEvaluator`

`Dataset` + `evaluate()` function for batch evaluation pipeline.

## Phase 12: Full LangChain Alignment + Ecosystem

**Goal:** Long-tail capabilities and LangChain feature equivalence.

- More document loaders: S3, Google Drive, Notion, Confluence, Arxiv, etc.
- More vector stores: FAISS bindings, LanceDB, DuckDB, etc.
- Structured Output: `with_structured_output(schema)` via serde + JSON Schema
- Serialization: chain/graph serialization/deserialization
- Prebuilt agents: SQL Agent, Web Research Agent, etc.
- API Server: HTTP serving wrapper (LangServe/LangGraph Platform equivalent)
- CLI tooling: project scaffolding, dev server
- `synapse` unified facade crate: re-exports all sub-crates
