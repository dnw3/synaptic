# Weaviate

[Weaviate](https://weaviate.io/) 是一款云原生、开源的向量数据库，原生支持混合搜索和多租户。`synaptic-weaviate` 使用 Weaviate v1 REST API 实现了 [`VectorStore`] trait。

## 设置

```toml
[dependencies]
synaptic = { version = "0.2", features = ["weaviate"] }
```

使用 Docker 在本地运行 Weaviate：

```bash
docker run -d -p 8080:8080 -p 50051:50051 cr.weaviate.io/semitechnologies/weaviate:latest
```

或使用 [Weaviate Cloud Services](https://console.weaviate.cloud/)。

## 配置

```rust,ignore
use synaptic::weaviate::{WeaviateVectorStore, WeaviateConfig};

// 本地 Weaviate
let config = WeaviateConfig::new("http", "localhost:8080", "Documents");

// Weaviate Cloud Services（WCS）使用 API key
let config = WeaviateConfig::new("https", "my-cluster.weaviate.network", "Documents")
    .with_api_key("wcs-secret-key");

let store = WeaviateVectorStore::new(config);

// 创建 class schema（幂等操作，可多次调用）
store.initialize().await?;
```

### 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `scheme` | `String` | 必填 | `"http"` 或 `"https"` |
| `host` | `String` | 必填 | 主机和端口（如 `localhost:8080`） |
| `class_name` | `String` | 必填 | Weaviate class 名称（必须以大写字母开头） |
| `api_key` | `Option<String>` | `None` | WCS 认证 API key |

## 添加文档

```rust,ignore
use synaptic::weaviate::{WeaviateVectorStore, WeaviateConfig};
use synaptic::core::Document;
use synaptic::openai::OpenAiEmbeddings;
use std::sync::Arc;

let config = WeaviateConfig::new("http", "localhost:8080", "Articles");
let store = WeaviateVectorStore::new(config);
store.initialize().await?;

let embeddings = Arc::new(OpenAiEmbeddings::new(/* config */));

let docs = vec![
    Document::new("1", "Rust 是一种系统编程语言。"),
    Document::new("2", "Weaviate 是一种向量数据库。"),
    Document::new("3", "Synaptic 是一个 Rust Agent 框架。"),
];

let ids = store.add_documents(docs, embeddings.as_ref()).await?;
println!("已添加 {} 个文档", ids.len());
```

## 相似度搜索

```rust,ignore
use synaptic::core::VectorStore;

let results = store.similarity_search("系统编程", 3, embeddings.as_ref()).await?;
for doc in results {
    println!("[{}] {}", doc.id, doc.content);
}
```

## 删除文档

```rust,ignore
store.delete(&["weaviate-uuid-1".to_string(), "weaviate-uuid-2".to_string()]).await?;
```

## RAG 流水线

```rust,ignore
use synaptic::retrieval::VectorStoreRetriever;
use synaptic::core::Retriever;
use std::sync::Arc;

let store = Arc::new(WeaviateVectorStore::new(config));
let retriever = VectorStoreRetriever::new(store, embeddings, 4);

let docs = retriever.get_relevant_documents("Rust 异步编程").await?;
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match store.similarity_search("查询", 5, embeddings.as_ref()).await {
    Ok(docs) => println!("找到 {} 个结果", docs.len()),
    Err(SynapticError::VectorStore(msg)) => eprintln!("Weaviate 错误：{msg}"),
    Err(e) => return Err(e.into()),
}
```

## Class Schema

`initialize()` 会在 class 不存在时创建以下 Weaviate class：

```json
{
  "class": "Documents",
  "vectorizer": "none",
  "properties": [
    { "name": "content",  "dataType": ["text"] },
    { "name": "docId",    "dataType": ["text"] },
    { "name": "metadata", "dataType": ["text"] }
  ]
}
```

向量由 Synaptic 提供（无需 Weaviate vectorizer 模块）。
