# HuggingFace 嵌入向量

通过 HuggingFace Inference API，访问数千个开源句子迁移模型来生成文本嵌入向量。

## 设置

在 `Cargo.toml` 中添加 `huggingface` feature：

```toml
[dependencies]
synaptic = { version = "0.2", features = ["huggingface"] }
```

可选设置 HuggingFace API 密鑰：

```bash
export HF_API_KEY="hf_..."
```

## 配置

```rust,ignore
use synaptic::huggingface::{HuggingFaceEmbeddings, HuggingFaceEmbeddingsConfig};

let config = HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5")
    .with_api_key("hf_...");
let embeddings = HuggingFaceEmbeddings::new(config);
```

## 常用模型

| 模型 | 维度 | 适用场景 |
|-------|------|----------|
| BAAI/bge-small-en-v1.5 | 384 | 快速英文检索 |
| BAAI/bge-large-en-v1.5 | 1024 | 高质量英文检索 |
| sentence-transformers/all-MiniLM-L6-v2 | 384 | 通用，流行 |
| intfloat/multilingual-e5-large | 1024 | 多语言检索 |
| BAAI/bge-m3 | 1024 | 多语言，长文本 |

## 用法

### 单条查询嵌入

```rust,ignore
use synaptic::core::Embeddings;

let vector = embeddings.embed_query("什么是 Rust？").await?;
println!("嵌入维度: {}", vector.len());
```

### 批量文档嵌入

```rust,ignore
use synaptic::core::Embeddings;

let docs = ["Rust 确保内存安全", "Python 是解释性语言"];
let vecs = embeddings.embed_documents(&docs).await?;
```

## RAG 流水线

将 HuggingFace 嵌入与 InMemoryVectorStore 结合，构建 RAG 检索流水线：

```rust,ignore
use synaptic::huggingface::{HuggingFaceEmbeddings, HuggingFaceEmbeddingsConfig};
use synaptic::vectorstores::InMemoryVectorStore;

let embeddings = std::sync::Arc::new(HuggingFaceEmbeddings::new(
    HuggingFaceEmbeddingsConfig::new("BAAI/bge-small-en-v1.5").with_api_key("hf_..."),
));
let store = std::sync::Arc::new(InMemoryVectorStore::new());
store.add_documents(&docs, embeddings.as_ref()).await?;
let results = retriever.retrieve("查询").await?;
```

## API 密鑰

在 https://huggingface.co/settings/tokens 获取 HuggingFace API Token。免费号对公开模型进行访问，付费号可以解锁高频限制和私有模型访问权限。

## 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|-------|------|--------|------|
| `model` | String | 必填 | HuggingFace 模型 ID |
| `api_key` | Option | None | API 认证令牌 |
| `base_url` | String | https://api-inference.huggingface.co/models | API 基础 URL |
| `wait_for_model` | bool | true | 等待模型加载 |
