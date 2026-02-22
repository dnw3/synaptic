# Pinecone 向量存储

本指南展示如何使用 Synaptic 的 Pinecone 集成进行向量存储和相似性搜索。[Pinecone](https://www.pinecone.io/) 是一个全托管的向量数据库，专为大规模相似性搜索而设计。

## 设置

在 `Cargo.toml` 中添加 `pinecone` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "pinecone"] }
```

你需要在 Pinecone 控制台中创建一个索引，并获取以下信息：

- **API Key** -- 在 Pinecone 控制台的 API Keys 页面获取
- **Host** -- 索引的 URL，格式如 `https://my-index-abc1234.svc.aped-1234-ab12.pinecone.io`

```bash
export PINECONE_API_KEY="your-pinecone-api-key"
```

## 配置

使用 `PineconeConfig` 创建配置：

```rust,ignore
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};

let config = PineconeConfig::new(
    "your-api-key",
    "https://my-index-abc1234.svc.aped-1234-ab12.pinecone.io",
);

let store = PineconeVectorStore::new(config);
```

`host` 参数是 Pinecone 控制台中索引的完整 URL。每个索引都有一个唯一的 URL。

### 自定义命名空间

Pinecone 支持命名空间来隔离数据：

```rust,ignore
let config = PineconeConfig::new("api-key", "https://my-index.pinecone.io")
    .with_namespace("production");
```

## 用法

### 添加文档

`PineconeVectorStore` 实现了 `VectorStore` trait：

```rust,ignore
use synaptic::pinecone::PineconeVectorStore;
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

将 Pinecone 存储桥接到 `Retriever` trait：

```rust,ignore
use std::sync::Arc;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::core::Retriever;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = Arc::new(store);

let retriever = VectorStoreRetriever::new(store, embeddings, 5);
let results = retriever.retrieve("查询内容", 5).await?;
```

## 命名空间隔离

命名空间是构建多租户 RAG 应用的常见模式。每个租户的数据存放在同一索引的不同命名空间中，提供逻辑隔离，无需管理多个索引。

```rust,ignore
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};
use synaptic::core::{VectorStore, Document, Embeddings};
use synaptic::openai::OpenAiEmbeddings;

let api_key = std::env::var("PINECONE_API_KEY")?;
let index_host = "https://my-index-abc123.svc.aped-1234.pinecone.io";

// 为不同租户创建使用不同命名空间的存储
let config_a = PineconeConfig::new(&api_key, index_host)
    .with_namespace("tenant-a");
let config_b = PineconeConfig::new(&api_key, index_host)
    .with_namespace("tenant-b");

let store_a = PineconeVectorStore::new(config_a);
let store_b = PineconeVectorStore::new(config_b);

let embeddings = OpenAiEmbeddings::new("text-embedding-3-small");

// 租户 A 的文档对租户 B 不可见
let docs_a = vec![Document::new("a1", "Tenant A internal report")];
store_a.add_documents(docs_a, &embeddings).await?;

// 在租户 B 的命名空间中搜索不会返回租户 A 的结果
let results = store_b.similarity_search("internal report", 5, &embeddings).await?;
assert!(results.is_empty());
```

这种方式具有良好的可扩展性，因为 Pinecone 在内部处理命名空间级别的分区。你可以在一个命名空间中添加、搜索和删除文档，而不会影响其他命名空间。

## RAG 管道示例

完整的 RAG 管道：加载文档、切分成块、生成嵌入并存入 Pinecone，然后检索相关上下文并生成回答。

```rust,ignore
use synaptic::core::{ChatModel, ChatRequest, Message, Embeddings, VectorStore, Retriever};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::pinecone::{PineconeConfig, PineconeVectorStore};
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

// 2. 存入 Pinecone
let config = PineconeConfig::new(
    std::env::var("PINECONE_API_KEY")?,
    "https://my-index-abc123.svc.aped-1234.pinecone.io",
);
let store = PineconeVectorStore::new(config);
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
| `api_key` | `String` | 必填 | Pinecone API 密钥 |
| `host` | `String` | 必填 | 索引的完整 URL（从 Pinecone 控制台获取） |
| `namespace` | `Option<String>` | `None` | 可选的命名空间 |
