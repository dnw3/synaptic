# Chroma 向量存储

本指南展示如何使用 Synaptic 的 Chroma 集成进行向量存储和相似性搜索。[Chroma](https://www.trychroma.com/) 是一个开源的嵌入数据库，支持本地部署和云端托管。

## 设置

在 `Cargo.toml` 中添加 `chroma` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "chroma"] }
```

启动 Chroma 实例（例如通过 Docker）：

```bash
docker run -p 8000:8000 chromadb/chroma
```

默认端口为 8000。

## 配置

使用 `ChromaConfig` 创建配置：

```rust,ignore
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};

let config = ChromaConfig::new(
    "http://localhost:8000",   // Chroma 服务器 URL
    "my_collection",           // Collection 名称
);

let store = ChromaVectorStore::new(config);
```

### 默认 URL

如果 Chroma 运行在默认地址 `http://localhost:8000`，URL 参数可以使用此默认值。

### 自定义 Collection 元数据

```rust,ignore
let config = ChromaConfig::new("http://localhost:8000", "my_collection")
    .with_metadata(serde_json::json!({
        "hnsw:space": "cosine"
    }));
```

## 创建 Collection

调用 `ensure_collection()` 确保 collection 存在。如果不存在会自动创建：

```rust,ignore
store.ensure_collection().await?;
```

此操作是幂等的，可以在每次启动时安全调用。

## 用法

### 添加文档

`ChromaVectorStore` 实现了 `VectorStore` trait：

```rust,ignore
use synaptic::chroma::ChromaVectorStore;
use synaptic::core::{VectorStore, Document, Embeddings};
use synaptic::openai::OpenAiEmbeddings;

let embeddings = OpenAiEmbeddings::new("text-embedding-3-small");

let docs = vec![
    Document::new("1", "Rust 是一门系统编程语言"),
    Document::new("2", "Python 适合数据科学"),
    Document::new("3", "Go 擅长并发编程"),
];

let ids = store.add_documents(docs, &embeddings).await?;
```

### 相似性搜索

```rust,ignore
let results = store.similarity_search("系统编程", 3, &embeddings).await?;
for doc in &results {
    println!("{}: {}", doc.id, doc.content);
}
```

### 带分数搜索

```rust,ignore
let scored = store.similarity_search_with_score("并发", 3, &embeddings).await?;
for (doc, score) in &scored {
    println!("{} (score: {:.3}): {}", doc.id, score, doc.content);
}
```

### 删除文档

```rust,ignore
store.delete(&["1", "3"]).await?;
```

## 与 Retriever 配合使用

将 Chroma 存储桥接到 `Retriever` trait：

```rust,ignore
use std::sync::Arc;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::core::Retriever;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = Arc::new(store);

let retriever = VectorStoreRetriever::new(store, embeddings, 5);
let results = retriever.retrieve("查询内容", 5).await?;
```

## Docker 部署

Chroma 可以通过 Docker 轻松部署，适用于开发和生产环境。

**快速启动** -- 使用默认配置运行 Chroma 服务器：

```bash
# 在端口 8000 启动 Chroma
docker run -p 8000:8000 chromadb/chroma:latest
```

**持久化存储** -- 挂载卷以确保数据在容器重启后不会丢失：

```bash
docker run -p 8000:8000 -v ./chroma-data:/chroma/chroma chromadb/chroma:latest
```

**Docker Compose** -- 用于生产部署，使用 `docker-compose.yml`：

```yaml
version: "3.8"
services:
  chroma:
    image: chromadb/chroma:latest
    ports:
      - "8000:8000"
    volumes:
      - chroma-data:/chroma/chroma
    restart: unless-stopped

volumes:
  chroma-data:
```

然后从 Synaptic 连接：

```rust,ignore
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};

let config = ChromaConfig::new("http://localhost:8000", "my_collection");
let store = ChromaVectorStore::new(config);
store.ensure_collection().await?;
```

对于远程或需要认证的部署，使用 `with_auth_token()`：

```rust,ignore
let config = ChromaConfig::new("https://chroma.example.com", "my_collection")
    .with_auth_token("your-token");
```

## RAG 管道示例

完整的 RAG 管道：加载文档、切分成块、生成嵌入并存入 Chroma，然后检索相关上下文并生成回答。

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, Embeddings, VectorStore, Retriever};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::chroma::{ChromaConfig, ChromaVectorStore};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::loaders::TextLoader;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::models::HttpBackend;
use std::sync::Arc;

let backend = Arc::new(HttpBackend::new());
let embeddings = Arc::new(OpenAiEmbeddings::new(
    OpenAiEmbeddings::config("text-embedding-3-small"),
    backend.clone(),
));

// 1. 加载并切分文档
let loader = TextLoader::new("docs/knowledge-base.txt");
let docs = loader.load().await?;
let splitter = RecursiveCharacterTextSplitter::new(500, 50);
let chunks = splitter.split_documents(&docs)?;

// 2. 存入 Chroma
let config = ChromaConfig::new("http://localhost:8000", "my_collection");
let store = ChromaVectorStore::new(config);
store.ensure_collection().await?;
store.add_documents(chunks, embeddings.as_ref()).await?;

// 3. 检索并回答
let store = Arc::new(store);
let retriever = VectorStoreRetriever::new(store, embeddings.clone(), 5);
let relevant = retriever.retrieve("What is Synaptic?", 5).await?;
let context = relevant.iter().map(|d| d.content.as_str()).collect::<Vec<_>>().join("\n\n");

let model = OpenAiChatModel::new(/* config */);
let request = ChatRequest::new(vec![
    Message::system(&format!("Answer based on context:\n{context}")),
    Message::human("What is Synaptic?"),
]);
let response = model.chat(&request).await?;
```

## 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `url` | `String` | 必填 | Chroma 服务器 URL（例如 `http://localhost:8000`） |
| `collection_name` | `String` | 必填 | Collection 名称 |
| `metadata` | `Option<Value>` | `None` | 可选的 collection 元数据 |
