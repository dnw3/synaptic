# MongoDB Atlas 向量搜索

本指南展示如何使用 Synaptic 的 MongoDB 集成，利用 [MongoDB Atlas](https://www.mongodb.com/atlas) 的向量搜索功能进行相似性检索。MongoDB Atlas 提供原生的向量搜索索引，可以在现有 MongoDB 部署上启用向量检索能力。

## 设置

在 `Cargo.toml` 中添加 `mongodb` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "mongodb"] }
```

### 前置条件

1. 一个 MongoDB Atlas 集群（M10 或以上，向量搜索需要 Atlas 专用集群）
2. 预先创建 Atlas Search 索引（带向量字段映射）

在 Atlas 控制台中为你的集合创建搜索索引，索引定义示例：

```json
{
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

## 配置

使用 `MongoVectorConfig` 创建配置：

```rust,ignore
use synaptic::mongodb::{MongoVectorConfig, MongoVectorStore};

let config = MongoVectorConfig::new(
    "my_database",       // 数据库名称
    "my_collection",     // 集合名称
    "vector_index",      // Atlas Search 索引名称
    1536,                // 向量维度
);
```

### 从 URI 创建

使用 MongoDB 连接字符串创建存储实例：

```rust,ignore
let store = MongoVectorStore::from_uri(
    "mongodb+srv://user:pass@cluster.mongodb.net",
    config,
).await?;
```

### 自定义字段名称

默认的内容字段为 `"content"`，嵌入字段为 `"embedding"`。如需自定义：

```rust,ignore
let config = MongoVectorConfig::new("db", "collection", "index", 1536)
    .with_content_field("text")
    .with_embedding_field("vector");
```

## 用法

### 添加文档

`MongoVectorStore` 实现了 `VectorStore` trait：

```rust,ignore
use synaptic::mongodb::MongoVectorStore;
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

```rust,ignore
use std::sync::Arc;
use synaptic::vectorstores::VectorStoreRetriever;
use synaptic::core::Retriever;

let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
let store = Arc::new(store);

let retriever = VectorStoreRetriever::new(store, embeddings, 5);
let results = retriever.retrieve("查询内容", 5).await?;
```

## Atlas Search 索引设置

在执行相似性搜索之前，你必须在 MongoDB Atlas 集合上创建一个**向量搜索索引**。这需要 **M10 或更高规格**的专用集群（向量搜索在免费/共享集群上不可用）。

### 通过 Atlas UI 创建索引

1. 在 [MongoDB Atlas 控制台](https://cloud.mongodb.com/) 中进入你的集群。
2. 进入 **Search** > **Create Search Index**。
3. 选择 **JSON Editor**，然后选择目标数据库和集合。
4. 粘贴以下索引定义：

```json
{
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

5. 为索引命名（例如 `vector_index`），然后点击 **Create Search Index**。

> **注意：** `path` 字段必须与 `MongoVectorConfig` 中配置的 `embedding_field` 一致。如果你使用 `.with_embedding_field("vector")` 自定义了字段名，则索引定义中需要设置 `"path": "vector"`。同样，`numDimensions` 需要与你的嵌入模型输出维度一致。

### 通过 Atlas CLI 创建索引

你也可以使用 [MongoDB Atlas CLI](https://www.mongodb.com/docs/atlas/cli/) 以编程方式创建索引。

首先，将索引定义保存到 `index.json` 文件中：

```json
{
  "fields": [
    {
      "type": "vector",
      "path": "embedding",
      "numDimensions": 1536,
      "similarity": "cosine"
    }
  ]
}
```

然后执行：

```bash
atlas clusters search indexes create \
  --clusterName my-cluster \
  --db my_database \
  --collection my_collection \
  --file index.json
```

索引构建是异步进行的。你可以通过以下命令检查状态：

```bash
atlas clusters search indexes list \
  --clusterName my-cluster \
  --db my_database \
  --collection my_collection
```

等待状态显示 **READY** 后，即可执行相似性搜索。

### 相似度选项

索引定义中的 `similarity` 字段控制向量比较方式：

| 值 | 说明 |
|----|------|
| `cosine` | 余弦相似度（默认，适用于归一化的嵌入向量） |
| `euclidean` | 欧氏（L2）距离 |
| `dotProduct` | 点积（适用于单位长度向量） |

## RAG 管道示例

以下是一个完整的检索增强生成（RAG）管道，它加载文档、分割文本、生成嵌入并存储到 MongoDB Atlas，然后检索相关上下文来回答问题。

```rust,ignore
use std::sync::Arc;
use synaptic::core::{
    ChatModel, ChatRequest, Document, Embeddings, Message, Retriever, VectorStore,
};
use synaptic::mongodb::{MongoVectorConfig, MongoVectorStore};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::vectorstores::VectorStoreRetriever;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 配置嵌入模型和 LLM
    let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
    let llm = OpenAiChatModel::new("gpt-4o-mini");

    // 2. 连接 MongoDB Atlas
    let config = MongoVectorConfig::new("my_database", "documents", "vector_index", 1536);
    let store = MongoVectorStore::from_uri(
        "mongodb+srv://user:pass@cluster.mongodb.net/",
        config,
    )
    .await?;

    // 3. 加载并分割文档
    let raw_docs = vec![
        Document::new("doc1", "Rust 是一门多范式通用编程语言，强调性能、类型安全和并发性。\
            它在不使用垃圾回收器的情况下保证内存安全。"),
        Document::new("doc2", "MongoDB Atlas 是一个全托管的云数据库服务。它为 AI 应用提供\
            内置的向量搜索功能，支持余弦、欧氏和点积相似度指标。"),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(500, 50);
    let chunks = splitter.split_documents(&raw_docs);

    // 4. 嵌入并存储到 MongoDB
    store.add_documents(chunks, embeddings.as_ref()).await?;

    // 5. 创建检索器
    let store = Arc::new(store);
    let retriever = VectorStoreRetriever::new(store, embeddings, 3);

    // 6. 检索相关上下文
    let query = "什么是 Rust？";
    let relevant_docs = retriever.retrieve(query, 3).await?;

    let context = relevant_docs
        .iter()
        .map(|doc| doc.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    // 7. 使用检索到的上下文生成回答
    let messages = vec![
        Message::system("根据以下上下文回答用户的问题。\
            如果上下文中没有相关信息，请说明。\n\n\
            上下文：\n{context}".replace("{context}", &context)),
        Message::human(query),
    ];

    let response = llm.chat(ChatRequest::new(messages)).await?;
    println!("回答：{}", response.message.content());

    Ok(())
}
```

## 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `database` | `String` | 必填 | MongoDB 数据库名称 |
| `collection` | `String` | 必填 | MongoDB 集合名称 |
| `index_name` | `String` | 必填 | Atlas Search 索引名称 |
| `dims` | `u32` | 必填 | 向量维度 |
| `content_field` | `String` | `"content"` | 文档内容字段名称 |
| `embedding_field` | `String` | `"embedding"` | 嵌入向量字段名称 |
