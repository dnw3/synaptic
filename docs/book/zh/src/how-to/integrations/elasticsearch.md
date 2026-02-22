# Elasticsearch 向量存储

本指南展示如何使用 Synaptic 的 Elasticsearch 集成进行向量存储和 kNN 相似性搜索。[Elasticsearch](https://www.elastic.co/elasticsearch) 从 8.0 版本开始支持原生的 dense vector 字段和 kNN 搜索。

## 设置

在 `Cargo.toml` 中添加 `elasticsearch` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["openai", "elasticsearch"] }
```

启动 Elasticsearch 实例（例如通过 Docker）：

```bash
docker run -p 9200:9200 \
  -e "discovery.type=single-node" \
  -e "xpack.security.enabled=false" \
  elasticsearch:8.12.0
```

## 配置

使用 `ElasticsearchConfig` 创建配置：

```rust,ignore
use synaptic::elasticsearch::{ElasticsearchConfig, ElasticsearchVectorStore};

let config = ElasticsearchConfig::new(
    "http://localhost:9200",   // Elasticsearch URL
    "my_index",                // 索引名称
    1536,                      // 向量维度
);

let store = ElasticsearchVectorStore::new(config);
```

### 认证

如果 Elasticsearch 启用了安全认证，使用 `with_credentials()` 设置用户名和密码：

```rust,ignore
let config = ElasticsearchConfig::new("https://my-es-cluster:9200", "docs", 1536)
    .with_credentials("elastic", "your-password");
```

### 自定义字段名称

默认的内容字段为 `"content"`，嵌入字段为 `"embedding"`。如需自定义：

```rust,ignore
let config = ElasticsearchConfig::new("http://localhost:9200", "my_index", 1536)
    .with_content_field("text")
    .with_embedding_field("vector");
```

## 创建索引

调用 `ensure_index()` 创建带 kNN 映射的索引。如果索引已存在，则不会重复创建：

```rust,ignore
store.ensure_index().await?;
```

此操作会创建一个包含以下映射的索引：

- `content` 字段（`text` 类型）
- `embedding` 字段（`dense_vector` 类型，指定维度，kNN 索引）
- `metadata` 字段（`object` 类型）

## 用法

### 添加文档

`ElasticsearchVectorStore` 实现了 `VectorStore` trait：

```rust,ignore
use synaptic::elasticsearch::ElasticsearchVectorStore;
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

## 索引映射配置

虽然 `ensure_index()` 会自动创建默认映射，但在生产环境中你可能需要完全控制索引映射。以下是推荐的 Elasticsearch 向量搜索映射：

```json
{
  "mappings": {
    "properties": {
      "embedding": {
        "type": "dense_vector",
        "dims": 1536,
        "index": true,
        "similarity": "cosine"
      },
      "content": { "type": "text" },
      "metadata": { "type": "object", "enabled": true }
    }
  }
}
```

### 通过 REST API 创建索引

你可以使用 Elasticsearch REST API 创建带有自定义映射的索引：

```bash
curl -X PUT "http://localhost:9200/my-index" \
  -H "Content-Type: application/json" \
  -d '{
    "mappings": {
      "properties": {
        "embedding": {
          "type": "dense_vector",
          "dims": 1536,
          "index": true,
          "similarity": "cosine"
        },
        "content": { "type": "text" },
        "metadata": { "type": "object", "enabled": true }
      }
    }
  }'
```

### 关键映射字段说明

- **`type: "dense_vector"`** -- 告诉 Elasticsearch 该字段存储固定长度的浮点数组，用于向量运算。
- **`dims`** -- 必须与你的嵌入模型的输出维度一致（例如 `text-embedding-3-small` 为 1536，许多开源模型为 768）。
- **`index: true`** -- 启用 kNN 搜索数据结构。如果不设置此项，你可以存储向量但无法执行高效的近似最近邻查询。生产环境请设置为 `true`。
- **`similarity`** -- 决定 kNN 搜索使用的距离函数：
  - `"cosine"`（默认）-- 余弦相似度，适用于大多数嵌入模型。
  - `"dot_product"` -- 点积，适用于单位长度归一化的向量。
  - `"l2_norm"` -- 欧氏距离。

### 元数据过滤映射

如果你计划按元数据字段过滤搜索结果，需要为这些字段添加显式映射：

```json
{
  "mappings": {
    "properties": {
      "embedding": {
        "type": "dense_vector",
        "dims": 1536,
        "index": true,
        "similarity": "cosine"
      },
      "content": { "type": "text" },
      "metadata": {
        "properties": {
          "source": { "type": "keyword" },
          "category": { "type": "keyword" },
          "created_at": { "type": "date" }
        }
      }
    }
  }
}
```

对元数据字段使用 `keyword` 类型可以在 kNN 查询中启用精确匹配过滤。

## RAG 管道示例

以下是一个完整的检索增强生成（RAG）管道，它加载文档、分割文本、生成嵌入并存储到 Elasticsearch，然后检索相关上下文来回答问题。

```rust,ignore
use std::sync::Arc;
use synaptic::core::{
    ChatModel, ChatRequest, Document, Embeddings, Message, Retriever, VectorStore,
};
use synaptic::elasticsearch::{ElasticsearchConfig, ElasticsearchVectorStore};
use synaptic::openai::{OpenAiChatModel, OpenAiEmbeddings};
use synaptic::splitters::RecursiveCharacterTextSplitter;
use synaptic::vectorstores::VectorStoreRetriever;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 配置嵌入模型和 LLM
    let embeddings = Arc::new(OpenAiEmbeddings::new("text-embedding-3-small"));
    let llm = OpenAiChatModel::new("gpt-4o-mini");

    // 2. 连接 Elasticsearch 并创建索引
    let config = ElasticsearchConfig::new("http://localhost:9200", "rag_documents", 1536);
    let store = ElasticsearchVectorStore::new(config);
    store.ensure_index().await?;

    // 3. 加载并分割文档
    let raw_docs = vec![
        Document::new("doc1", "Rust 是一门多范式通用编程语言，强调性能、类型安全和并发性。\
            它在不使用垃圾回收器的情况下保证内存安全。"),
        Document::new("doc2", "Elasticsearch 是一个分布式的 RESTful 搜索和分析引擎。\
            它通过 dense_vector 字段和近似 kNN 查询支持向量搜索，\
            适用于语义搜索和 RAG 应用场景。"),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(500, 50);
    let chunks = splitter.split_documents(&raw_docs);

    // 4. 嵌入并存储到 Elasticsearch
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
| `url` | `String` | 必填 | Elasticsearch URL（例如 `http://localhost:9200`） |
| `index_name` | `String` | 必填 | 索引名称 |
| `dims` | `u32` | 必填 | 向量维度 |
| `username` | `Option<String>` | `None` | 认证用户名 |
| `password` | `Option<String>` | `None` | 认证密码 |
| `content_field` | `String` | `"content"` | 文档内容字段名称 |
| `embedding_field` | `String` | `"embedding"` | 嵌入向量字段名称 |
