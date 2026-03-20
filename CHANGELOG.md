# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-03-19

### Breaking Changes

- **`AgentMiddleware` trait deleted** — replaced by `Interceptor` trait with `before_model`/`after_model`/`wrap_model_call`/`wrap_tool_call` hooks
- **`MiddlewareChain` deleted** — replaced by `InterceptorChain` with before→wrap→after execution semantics
- **`InterceptorAdapter`, `CallbackAdapter`, `CallbackMiddleware` deleted** — no longer needed with native event emission
- **`EventBusMiddleware` deleted** — event emission now native in graph nodes (`ChatModelNode`, `ToolNode`, `CompiledGraph`)
- **`DeepAgentOptions.middleware`** field removed — use `interceptors: Vec<Arc<dyn Interceptor>>`
- **`DeepAgentOptions.skills_dir`** → `skills_dirs: Vec<String>` (multi-directory skill discovery)
- **`#[before_agent]`/`#[after_agent]` macros deleted** — macros now generate `impl Interceptor` instead of `impl AgentMiddleware`
- **`synaptic-pgvector` renamed** to `synaptic-postgres` (PgVectorStore + PgStore + PgCache + PgCheckpointer)
- **7 OpenAI-compatible wrapper crates deleted** (groq, mistral, deepseek, together, fireworks, xai, perplexity) — consolidated into `synaptic-openai/src/compat/` submodules
- **`ContextAwareTool` removed** — use `RuntimeAwareTool` + `ToolRuntime`
- **Feature flag cleanup** — `embeddings` no longer pulled by `prompts`; `otel`/`langfuse` added to `full`; `agent`/`rag` decoupled from implicit OpenAI dependency (use `agent-openai`/`rag-openai` bundles)

### Crate Consolidation (47 → 18)

**Breaking — Provider crates merged into `synaptic-models`:**
- Deleted: synaptic-openai, synaptic-anthropic, synaptic-gemini, synaptic-ollama, synaptic-bedrock, synaptic-cohere
- New: `synaptic-models` with feature flags: `openai`, `anthropic`, `gemini`, `ollama`, `bedrock`, `cohere`
- OpenAI-compatible providers (groq, deepseek, mistral, together, fireworks, xai, perplexity) via `compat::` submodules

**Breaking — RAG pipeline merged into `synaptic-rag`:**
- Deleted: synaptic-prompts, synaptic-parsers, synaptic-loaders, synaptic-splitters, synaptic-embeddings, synaptic-vectorstores, synaptic-retrieval, synaptic-eval
- New: `synaptic-rag` with sub-features for each module
- Vector store backends (qdrant, pinecone, chroma, elasticsearch, opensearch, milvus, weaviate, lancedb) now feature-gated in synaptic-rag

**Breaking — Storage backends merged into `synaptic-store`:**
- Deleted: synaptic-postgres (was synaptic-pgvector), synaptic-redis, synaptic-sqlite, synaptic-mongodb
- New: `synaptic-store` with `postgres`, `redis`, `sqlite`, `mongodb` features

**Breaking — Config/middleware extensions merged:**
- synaptic-callbacks → `synaptic-events/observers/` (aliased as `callbacks`)
- synaptic-condenser → `synaptic-middleware/condenser/`
- synaptic-secrets → `synaptic-config/secrets`
- synaptic-session → `synaptic-config/session`
- synaptic-cache → `synaptic-config/cache`
- synaptic-plugin → `synaptic-config/plugin/`

**Breaking — Integration services merged into `synaptic-integrations`:**
- Deleted: synaptic-tavily, synaptic-confluence, synaptic-slack, synaptic-voice, synaptic-scheduler, synaptic-langfuse
- New: `synaptic-integrations` with feature flags

**Breaking — Tools merged into `synaptic-tools`:**
- Deleted: synaptic-pdf, synaptic-sqltoolkit, synaptic-e2b, synaptic-browser, synaptic-sandbox

**Breaking — Core types consolidated:**
- synaptic-runnables merged into `synaptic-core/runnable/`
- Embedding providers (jina, voyage, nomic, huggingface, flashrank) moved to `synaptic-rag/embeddings/`

### Added

- **`Interceptor` trait** — LangChain 1.0 hybrid pattern with `before_model`/`after_model` hooks and `InterceptorChain`
- **`EventSubscriber` system** — 29 event kinds (`EventKind`), 5 dispatch modes, `EventBus` with `Event`/`EventAction`/`EmitResult`
- **New crates** — synaptic-events, synaptic-logging, synaptic-config, synaptic-models, synaptic-integrations, synaptic-memory, synaptic-store, synaptic-mcp
- **synaptic-logging** — `LogBuffer`, `MemoryLogLayer`, `RequestID` utilities
- **Node-native event emission** in `ChatModelNode`, `ToolNode`, `CompiledGraph`
- **Plugin system** — `Plugin` trait, `PluginManifest`, `PluginRegistry`
- **Channel adapter traits** — 10 fine-grained channel adapter traits
- **`InputProvenance`/`ProvenanceKind`** for message origin tracking
- **`DeliveryContext`** type for unified cross-channel routing
- **`MemoryProvider` trait** with 7 methods
- **`SessionInfo`** expanded to 25+ fields for OpenClaw alignment
- **Lark** — `LarkCardElement` type, `render_lark_card_elements()` for Card JSON 2.0, `StreamingCardWriter::finish_with_card()`
- **LarkConfig** — `base_url` is now domain root; use `api_url()` for Open API prefix
- **Callbacks** — cost tracking, metrics
- **Middleware** — circuit breaker, SSRF guard
- **MCP** — health checks, OAuth
- **Deep agent** — skill system with multi-dir discovery
- **Store** — semantic search
- **CI** — layered jobs with architecture governance; `publish.sh` rewritten with DAG topological sort

## [0.3.0] - 2026-02-24

### Added

- **synaptic-condenser** — `Condenser` trait with `NoOp`, `Rolling`, `LlmSummarizing`, `TokenBudget`, `Pipeline` impls; `CondenserMiddleware`
- **synaptic-secrets** — `SecretRegistry` (mask/inject), `SecretMaskingMiddleware`
- **synaptic-config** — multi-format config (TOML/JSON/YAML), `ConfigSource` trait, `discover_and_load::<T>()`
- **synaptic-session** — `SessionManager`, `Session` (JSONL transcripts), `SessionCheckpointer`
- **synaptic-core** — `TokenCounter`, `ContextBudget`
- **synaptic-store** — `FileStore` (behind `filesystem` feature)
- **synaptic-graph** — `FileSaver` (behind `filesystem` feature), `StoreCheckpointer`
- **synaptic-middleware** — `SecurityAnalyzer`, `SecurityMiddleware`
- **synaptic-tools** — `ToolFilter` (AllowList/DenyList/StateMachine/Composite)
- **synaptic-deep** — `build_agent_from_config()` (behind `config-builder` feature)
- **synaptic-lark** — Tier 2 integration: `LarkBitableLoader`, `LarkBitableMemoryStore`, `LarkBitableCheckpointer`, `LarkBitableLlmCache`, `LarkSpreadsheetLoader`, `LarkWikiLoader`, `LarkDriveLoader`, AI tools (OCR, translate, ASR, doc_process), `LarkVectorStore`, `LarkEventListener` with HMAC verification, `LarkBotClient` + `LarkLongConnListener` (WebSocket bot mode), API layer + Bitable/Message completions, 5 productivity tools, CardKit API, streaming bot support
- **Redis** — cluster support via `cluster` feature flag
- **Docs** — 9 new how-to pages (EN + ZH), session-memory-store concept page
- **Examples** — condenser, config, file_persistence, secrets, security, session_resume, token_budget, tool_filter, lark_rag

## [0.2.2] - 2026-02-20

### Fixed

- Macro path dynamic resolution, examples use facade imports
- MSRV upgraded to 1.83
- Documentation dark/light theme support

## [0.2.1] - 2026-02-20

### Fixed

- Documentation language switcher paths (EN at root, ZH at `/zh/` subdirectory)
- CI failures — clippy `type_complexity` lint, MSRV raised to 1.82
- Documentation homepage links

## [0.2.0] - 2026-02-19

### Added

- **New crates** — synaptic-store, synaptic-middleware, synaptic-mcp, synaptic-macros, synaptic-deep
- **Rename** — all crates from `synapse-*` to `synaptic-*` for crates.io publishing
- **Documentation** — mdBook site (88 pages EN/ZH), GitHub Pages CI; 19 missing Chinese pages added; zh/SUMMARY.md expanded from 21 to 125 entries
- **Publishing** — `scripts/publish.sh` one-click publish script
- **Unified facade** — `synaptic` crate with feature-gated re-exports

### Changed

- All crate versions unified to 0.2.0

## [0.1.0] - 2026-02-17

### Added

- **Core** — `ChatModel`, `Message`, `Tool`, `MemoryStore`, `CallbackHandler` traits; `ChatRequest`/`ChatResponse`; `SynapticError` (19 variants); `RunnableConfig`
- **Models** — OpenAI, Anthropic, Gemini, Ollama adapters with streaming; `ScriptedChatModel` test double; `RetryChatModel`, `RateLimitedChatModel`, `TokenBucketChatModel` wrappers; `StructuredOutputChatModel<T>`
- **LCEL Runnables** — `Runnable` trait with `invoke`/`batch`/`stream`; pipe operator (`|`); `RunnableLambda`, `RunnableParallel`, `RunnableBranch`, `RunnablePassthrough`, `RunnableWithFallbacks`, `RunnableAssign`, `RunnablePick`; `bind()` for config transforms
- **Prompts** — `PromptTemplate`, `ChatPromptTemplate`, `FewShotChatMessagePromptTemplate`; all implement `Runnable`
- **Parsers** — `StrOutputParser`, `JsonOutputParser`, `StructuredOutputParser<T>`, `ListOutputParser`, `EnumOutputParser`; all implement `Runnable`
- **Tools** — `ToolRegistry`, `SerialToolExecutor`; `tool_choice` control (Auto/Required/None/Specific)
- **Memory** — `InMemoryStore`; `ConversationBufferMemory`, `ConversationWindowMemory`, `ConversationSummaryMemory`, `ConversationTokenBufferMemory`, `ConversationSummaryBufferMemory`; `RunnableWithMessageHistory`
- **Callbacks** — `RecordingCallback`, `TracingCallback`, `CompositeCallback`; `RunEvent` lifecycle events
- **Graph** — LangGraph-style `StateGraph` with conditional edges, `CompiledGraph` with invoke/stream, `ToolNode`, `create_react_agent()`; `Checkpointer` + `MemorySaver`; `interrupt_before`/`interrupt_after`; `StreamMode::Values`/`Updates`
- **Retrieval** — `InMemoryRetriever`, `BM25Retriever`, `MultiQueryRetriever`, `EnsembleRetriever`, `ContextualCompressionRetriever`, `SelfQueryRetriever`, `ParentDocumentRetriever`
- **Loaders** — `TextLoader`, `JsonLoader`, `CsvLoader`, `DirectoryLoader`
- **Splitters** — `CharacterTextSplitter`, `RecursiveCharacterTextSplitter`, `MarkdownHeaderTextSplitter`, `TokenTextSplitter`
- **Embeddings** — `Embeddings` trait; `FakeEmbeddings`, `OpenAiEmbeddings`, `OllamaEmbeddings`
- **Vector Stores** — `VectorStore` trait; `InMemoryVectorStore` (cosine similarity); `VectorStoreRetriever`
- **Caching** — `InMemoryCache` (optional TTL), `SemanticCache`, `CachedChatModel`
- **Evaluation** — `ExactMatchEvaluator`, `JsonValidityEvaluator`, `RegexMatchEvaluator`, `EmbeddingDistanceEvaluator`, `LLMJudgeEvaluator`; `Dataset` + `evaluate()` batch pipeline
- **Facade** — Unified `synaptic` crate with feature-gated re-exports
