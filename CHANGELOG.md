# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-17

### Added

- **Core** — `ChatModel`, `Message`, `Tool`, `MemoryStore`, `CallbackHandler` traits; `ChatRequest`/`ChatResponse`; `SynapseError` (19 variants); `RunnableConfig`
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
- **Facade** — Unified `synapse` crate with feature-gated re-exports
