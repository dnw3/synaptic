# 加载器 & 向量库

## LarkDocLoader

将飞书文档和知识库页面加载为 Synaptic [`Document`]，可直接用于 RAG 管道。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDocLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");

// 加载指定文档 token
let loader = LarkDocLoader::new(config.clone())
    .with_doc_tokens(vec!["doxcnAbcXxx".to_string()]);

// 或遍历整个 Wiki 空间
let loader = LarkDocLoader::new(config)
    .with_wiki_space_id("spcXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("标题: {}", doc.metadata["title"]);
    println!("URL:  {}", doc.metadata["url"]);
    println!("长度: {} 字符", doc.content.len());
}
```

### 文档 Metadata 字段

| 字段 | 说明 |
|------|------|
| `doc_id` | 飞书文档 token |
| `title` | 文档标题 |
| `source` | `lark:doc:<token>` |
| `url` | 飞书文档直链 |
| `doc_type` | 固定为 `"docx"` |

### 构建器选项

| 方法 | 说明 |
|------|------|
| `with_doc_tokens(tokens)` | 加载指定文档 token 列表 |
| `with_wiki_space_id(id)` | 遍历 Wiki 空间内的所有文档 |

---

## LarkWikiLoader

递归加载飞书知识库空间中的所有页面，生成 `Document` 列表。通过 `with_space_id` 指定目标空间，`with_max_depth` 控制递归深度。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkWikiLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkWikiLoader::new(config)
    .with_space_id("spcXxx")
    .with_max_depth(3);

let docs = loader.load().await?;
println!("共加载 {} 个知识库页面", docs.len());
```

---

## LarkDriveLoader

从飞书云盘文件夹加载文件，根据文件类型自动分派到对应的子加载器（文档、电子表格等）进行处理。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkDriveLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkDriveLoader::new(config, "fldcnXxx");

let docs = loader.load().await?;
for doc in &docs {
    println!("{}: {} 字符", doc.metadata["file_name"], doc.content.len());
}
```

---

## LarkSpreadsheetLoader

将飞书电子表格的行加载为 Synaptic [`Document`]。每一行生成一个文档，列标题作为 metadata 的键存储。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkSpreadsheetLoader};
use synaptic::core::Loader;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let loader = LarkSpreadsheetLoader::new(config, "shtcnXxx", "0");

let docs = loader.load().await?;
for doc in &docs {
    println!("行内容: {}", doc.content);
    println!("表格 ID: {}", doc.metadata["sheet_id"]);
}
```

---

## LarkVectorStore

以飞书搜索 API 作为向量存储后端，由飞书平台负责服务端向量化，文档在 Lark 中建立索引后即可通过语义搜索检索。

```rust,ignore
use synaptic::lark::{LarkConfig, LarkVectorStore};
use synaptic::core::VectorStore;

let config = LarkConfig::new("cli_xxx", "secret_xxx");
let store = LarkVectorStore::new(config, "data_source_id_xxx");

// 建立文档索引
store.add_documents(docs).await?;

// 语义搜索——向量化由飞书平台处理
let results = store.similarity_search("季度营收", 5).await?;
for doc in &results {
    println!("{}", doc.content);
}
```
