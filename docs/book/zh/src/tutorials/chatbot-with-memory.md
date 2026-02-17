# 构建带记忆的聊天机器人

本教程将引导你构建一个基于会话的聊天机器人，它能够记住对话历史。你将学习如何使用 `InMemoryStore` 存储和检索消息，通过 session ID 隔离对话，以及选择适合你使用场景的记忆策略。

## 前置条件

在 `Cargo.toml` 中添加所需的 Synaptic crate：

```toml
[dependencies]
synaptic-core = { path = "../crates/synaptic-core" }
synaptic-memory = { path = "../crates/synaptic-memory" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## 第一步：存储和加载消息

每个聊天机器人都需要记住对话内容。Synaptic 提供了 `MemoryStore` trait 来定义这个能力，`InMemoryStore` 是一个基于 `HashMap` 的简单内存实现。

```rust
use synaptic_core::{MemoryStore, Message, SynapticError};
use synaptic_memory::InMemoryStore;

#[tokio::main]
async fn main() -> Result<(), SynapticError> {
    let memory = InMemoryStore::new();
    let session_id = "demo-session";

    // 模拟一段对话
    memory.append(session_id, Message::human("你好，Synaptic")).await?;
    memory.append(session_id, Message::ai("你好！有什么可以帮你的？")).await?;
    memory.append(session_id, Message::human("你能做什么？")).await?;
    memory.append(session_id, Message::ai("我可以帮你完成很多任务！")).await?;

    // 加载对话历史
    let transcript = memory.load(session_id).await?;
    for message in &transcript {
        println!("{}: {}", message.role(), message.content());
    }

    // 完成后清除记忆
    memory.clear(session_id).await?;
    Ok(())
}
```

输出为：

```text
human: 你好，Synaptic
ai: 你好！有什么可以帮你的？
human: 你能做什么？
ai: 我可以帮你完成很多任务！
```

`MemoryStore` trait 定义了三个方法：

- **`append(session_id, message)`** -- 将一条消息追加到某个会话的历史中
- **`load(session_id)`** -- 返回某个会话的所有消息，类型为 `Vec<Message>`
- **`clear(session_id)`** -- 删除某个会话的所有消息

## 第二步：会话隔离

每个 session ID 对应一个独立的对话历史。这就是你将多个用户或对话线程分开的方式：

```rust
use synaptic_core::{MemoryStore, Message, SynapticError};
use synaptic_memory::InMemoryStore;

#[tokio::main]
async fn main() -> Result<(), SynapticError> {
    let memory = InMemoryStore::new();

    // Alice 的对话
    memory.append("alice", Message::human("你好，我是 Alice")).await?;
    memory.append("alice", Message::ai("你好，Alice！")).await?;

    // Bob 的对话（完全独立）
    memory.append("bob", Message::human("你好，我是 Bob")).await?;
    memory.append("bob", Message::ai("你好，Bob！")).await?;

    // 每个会话有自己的历史
    let alice_history = memory.load("alice").await?;
    let bob_history = memory.load("bob").await?;

    assert_eq!(alice_history.len(), 2);
    assert_eq!(bob_history.len(), 2);
    assert_eq!(alice_history[0].content(), "你好，我是 Alice");
    assert_eq!(bob_history[0].content(), "你好，我是 Bob");

    Ok(())
}
```

Session ID 是任意字符串。在 Web 应用中，你通常会使用用户 ID、对话线程 ID 或两者的组合。

## 第三步：选择记忆策略

随着对话增长，将所有消息都发送给 LLM 会变得昂贵，最终会超过上下文窗口的限制。Synaptic 提供了多种记忆策略，它们包装底层的 `MemoryStore` 并控制 `load()` 返回的内容。

### ConversationBufferMemory

保留所有消息。这是最简单的策略——一个直通包装器，让"保留所有内容"的策略变得显式：

```rust
use std::sync::Arc;
use synaptic_core::MemoryStore;
use synaptic_memory::{InMemoryStore, ConversationBufferMemory};

let store = Arc::new(InMemoryStore::new());
let memory = ConversationBufferMemory::new(store);
// memory.load() 返回所有消息
```

最适合：短对话，需要完整历史记录的场景。

### ConversationWindowMemory

只保留最近 **K** 条消息。更早的消息仍然存储，但不会被 `load()` 返回：

```rust
use std::sync::Arc;
use synaptic_core::MemoryStore;
use synaptic_memory::{InMemoryStore, ConversationWindowMemory};

let store = Arc::new(InMemoryStore::new());
let memory = ConversationWindowMemory::new(store, 10); // 保留最近 10 条消息
// memory.load() 最多返回 10 条消息
```

最适合：近期上下文就够用的对话，需要可预测成本的场景。

### ConversationSummaryMemory

使用 LLM 总结较早的消息。当存储的消息数量超过 `buffer_size * 2` 时，较早的部分会被压缩为一条总结，作为系统消息插入开头：

```rust
use std::sync::Arc;
use synaptic_core::{ChatModel, MemoryStore};
use synaptic_memory::{InMemoryStore, ConversationSummaryMemory};

let store = Arc::new(InMemoryStore::new());
let model: Arc<dyn ChatModel> = /* 你的聊天模型 */;
let memory = ConversationSummaryMemory::new(store, model, 6);
// 当消息超过 12 条时，较早的消息会被总结
// memory.load() 返回：[总结系统消息] + [最近 6 条消息]
```

最适合：长时间运行的对话，需要保留较早上下文大意但不需要完整逐字记录的场景。

### ConversationTokenBufferMemory

在 **Token 预算**内保留消息。使用可配置的 token 估算器，当总量超过限制时丢弃最早的消息：

```rust
use std::sync::Arc;
use synaptic_core::MemoryStore;
use synaptic_memory::{InMemoryStore, ConversationTokenBufferMemory};

let store = Arc::new(InMemoryStore::new());
let memory = ConversationTokenBufferMemory::new(store, 4000); // 4000 token 预算
// memory.load() 返回在 4000 token 预算内能容纳的尽可能多的最近消息
```

最适合：需要精确控制 token 数量，确保不超过模型上下文窗口的场景。

### ConversationSummaryBufferMemory

总结和缓冲策略的混合体。保留最近的消息原文，当 token 数量超过阈值时将更早的内容总结：

```rust
use std::sync::Arc;
use synaptic_core::{ChatModel, MemoryStore};
use synaptic_memory::{InMemoryStore, ConversationSummaryBufferMemory};

let store = Arc::new(InMemoryStore::new());
let model: Arc<dyn ChatModel> = /* 你的聊天模型 */;
let memory = ConversationSummaryBufferMemory::new(store, model, 2000);
// 保留最近消息原文；当总 token 超过 2000 时总结较早内容
```

最适合：在成本和上下文质量之间取得平衡——你能获得近期消息的完整细节和较早消息的压缩摘要。

## 第四步：使用 RunnableWithMessageHistory 自动管理历史

在实际的聊天机器人中，你希望历史的加载和保存在每一轮对话中自动发生。`RunnableWithMessageHistory` 包装任何 `Runnable<Vec<Message>, String>` 并为你处理这一切：

1. 从 `RunnableConfig.metadata["session_id"]` 中提取 `session_id`
2. 从内存中加载对话历史
3. 追加用户的新消息
4. 用完整的消息列表调用内部 runnable
5. 将 AI 的回复保存回内存

```rust
use std::sync::Arc;
use std::collections::HashMap;
use synaptic_core::{MemoryStore, RunnableConfig};
use synaptic_memory::{InMemoryStore, RunnableWithMessageHistory};
use synaptic_runnables::Runnable;

// 用自动历史管理包装模型链
let memory = Arc::new(InMemoryStore::new());
let chain = /* 你的模型链 (BoxRunnable<Vec<Message>, String>) */;
let chatbot = RunnableWithMessageHistory::new(chain, memory);

// 每次调用都会自动加载/保存历史
let mut config = RunnableConfig::default();
config.metadata.insert(
    "session_id".to_string(),
    serde_json::Value::String("user-42".to_string()),
);

let response = chatbot.invoke("Rust 是什么？".to_string(), &config).await?;
// 用户消息和 AI 回复现在已经存储到 "user-42" 会话的内存中了
```

这是生产环境聊天机器人的推荐方式，因为它将内存管理逻辑从应用代码中分离出来。

## 整体架构

以下是 Synaptic 记忆系统的心智模型：

```text
                    +-----------------------+
                    |    MemoryStore trait   |
                    |  append / load / clear |
                    +-----------+-----------+
                                |
         +----------------------+----------------------+
         |                      |                      |
  InMemoryStore          (其他存储)              记忆策略
  (原始存储)                                    (包装 MemoryStore)
                                                       |
                                +----------------------+----------------------+
                                |         |         |         |              |
                             Buffer    Window   Summary   TokenBuffer   SummaryBuffer
                            (全部)    (最近K条)  (LLM)    (token预算)     (混合)
```

所有记忆策略本身也实现了 `MemoryStore` trait，因此它们是可组合的——你可以将 `InMemoryStore` 包装在 `ConversationWindowMemory` 中，下游只看到 `MemoryStore` trait。

## 总结

在本教程中你学会了：

- 使用 `InMemoryStore` 存储和检索对话消息
- 通过 session ID 隔离对话
- 根据对话长度和成本需求选择合适的记忆策略
- 使用 `RunnableWithMessageHistory` 自动管理历史

## 下一步

- [构建 RAG 应用](rag-application.md) -- 为聊天机器人添加文档检索能力
- [构建 Graph 工作流](graph-workflow.md) -- 使用状态机构建复杂工作流
- [构建 ReAct Agent](react-agent.md) -- 让 AI 调用工具
