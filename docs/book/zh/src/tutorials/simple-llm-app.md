# 构建一个简单的 LLM 应用

本教程将引导你构建一个完整的 LLM 应用，使用 Prompt Template 格式化输入，通过 Chat Model 生成回复，并使用 Output Parser 提取结果。这是 Synaptic 中最基础的链式调用模式。

## 你将学到什么

- 使用 `ChatPromptTemplate` 构建可复用的提示模板
- 使用 `StrOutputParser` 将 AI 消息转换为字符串
- 使用 LCEL 管道运算符（`|`）将组件串联

## 完整代码

```rust
use synaptic::core::{ChatModel, ChatRequest, ChatResponse, Message, SynapticError};
use synaptic::models::ScriptedChatModel;
use synaptic::prompts::{ChatPromptTemplate, MessageTemplate};
use synaptic::parsers::StrOutputParser;
use synaptic::runnables::Runnable;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), SynapticError> {
    // 1. 创建提示模板
    let prompt = ChatPromptTemplate::new(vec![
        MessageTemplate::system("You are a helpful translator. Translate the following text to {{ language }}."),
        MessageTemplate::human("{{ text }}"),
    ]);

    // 2. 创建模型（使用脚本化模型演示）
    let model = ScriptedChatModel::new(vec![
        ChatResponse {
            message: Message::ai("Bonjour le monde!"),
            usage: None,
        },
    ]);

    // 3. 准备输入变量
    let mut variables = HashMap::new();
    variables.insert("language".to_string(), "French".to_string());
    variables.insert("text".to_string(), "Hello world!".to_string());

    // 4. 渲染提示模板为消息列表
    let messages = prompt.invoke(variables).await?;

    // 5. 发送请求并获取响应
    let request = ChatRequest::new(messages);
    let response = model.chat(request).await?;

    // 6. 解析输出
    let parser = StrOutputParser;
    let result = parser.invoke(response.message).await?;

    println!("翻译结果: {}", result);
    Ok(())
}
```

运行后输出：

```
翻译结果: Bonjour le monde!
```

## 逐步解析

### 1. 创建提示模板

```rust
let prompt = ChatPromptTemplate::new(vec![
    MessageTemplate::system("You are a helpful translator. Translate the following text to {{ language }}."),
    MessageTemplate::human("{{ text }}"),
]);
```

`ChatPromptTemplate` 接受一个 `MessageTemplate` 列表。每个模板使用 `{{ variable }}` 语法标记待替换的变量。调用 `invoke()` 时，变量会被实际值替换，生成 `Vec<Message>`。

### 2. 发送请求

```rust
let messages = prompt.invoke(variables).await?;
let request = ChatRequest::new(messages);
let response = model.chat(request).await?;
```

将渲染后的消息列表包装为 `ChatRequest`，然后通过 `ChatModel::chat()` 发送。所有操作都是异步的。

### 3. 解析输出

```rust
let parser = StrOutputParser;
let result = parser.invoke(response.message).await?;
```

`StrOutputParser` 实现了 `Runnable` trait，它从 `Message` 中提取文本内容并返回 `String`。

## 使用 LCEL 管道组合

上面的代码分步执行了每个组件。在实际应用中，你可以使用 LCEL 管道运算符将它们串联成一个链：

```rust
use synaptic::runnables::BoxRunnable;

// 将组件转换为 BoxRunnable 并用 | 连接
let chain = prompt.boxed() | model.boxed() | parser.boxed();

// 一次调用执行整个链
let result = chain.invoke(variables).await?;
```

管道运算符 `|` 将前一个组件的输出作为后一个组件的输入，自动串联调用。这就是 LCEL（LangChain Expression Language）在 Synaptic 中的体现。

## 核心概念回顾

| 组件 | 作用 | 输入 | 输出 |
|---|---|---|---|
| `ChatPromptTemplate` | 将变量渲染为消息列表 | `HashMap<String, String>` | `Vec<Message>` |
| `ChatModel` | 将消息发送给 LLM | `ChatRequest` | `ChatResponse` |
| `StrOutputParser` | 从消息中提取文本 | `Message` | `String` |

## 下一步

- [构建带记忆的聊天机器人](chatbot-with-memory.md) -- 让 AI 记住对话历史
- [构建 ReAct Agent](react-agent.md) -- 赋予 AI 使用工具的能力
- [Runnables 与 LCEL](../concepts/runnables-lcel.md) -- 深入了解管道组合机制
