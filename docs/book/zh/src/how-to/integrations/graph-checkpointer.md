# 图持久化检查点

默认情况下，`synaptic-graph` 使用 [`MemorySaver`]，将图状态存储在进程内存中。这意味着进程重启后状态会丢失——不适合生产环境。

Synaptic 提供两种持久化检查点后端：

| 后端 | Crate | Feature flag |
|------|-------|--------------|
| Redis | `synaptic-redis` | `redis` + `checkpointer` |
| PostgreSQL | `synaptic-pgvector` | `pgvector` + `checkpointer` |

## 设置

在 `Cargo.toml` 中添加相应的 feature flag：

```toml
# Redis 检查点
[dependencies]
synaptic = { version = "0.2", features = ["agent", "redis"] }
synaptic-redis = { version = "0.2", features = ["checkpointer"] }

# 或 PostgreSQL 检查点
synaptic = { version = "0.2", features = ["agent", "pgvector"] }
synaptic-pgvector = { version = "0.2", features = ["checkpointer"] }
```

## Redis 检查点

### 快速开始

```rust,ignore
use synaptic_redis::checkpointer::{RedisCheckpointer, RedisCheckpointerConfig};
use synaptic::graph::{create_react_agent, MessageState};
use std::sync::Arc;

// 连接到 Redis
let checkpointer = RedisCheckpointer::from_url("redis://127.0.0.1/").await?;

// 使用持久化检查点构建图
let graph = create_react_agent(model, tools)?
    .with_checkpointer(Arc::new(checkpointer));

// 使用 thread_id 运行以实现持久化
let state = MessageState { messages: vec![Message::human("你好")] };
let config = RunnableConfig::default().with_metadata("thread_id", "user-123");
let result = graph.invoke_with_config(state, config).await?;
```

### 配置

```rust,ignore
use synaptic_redis::checkpointer::RedisCheckpointerConfig;

let config = RedisCheckpointerConfig::new("redis://127.0.0.1/")
    .with_ttl(86400)          // 检查点 24 小时后过期
    .with_prefix("myapp");    // 自定义键前缀（默认："synaptic"）

let checkpointer = RedisCheckpointer::new(config).await?;
```

### 配置参考

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `url` | `String` | 必填 | Redis 连接 URL |
| `ttl` | `Option<u64>` | `None` | 检查点键的 TTL（秒） |
| `prefix` | `String` | `"synaptic"` | 所有检查点键的前缀 |

### 键方案

Redis 使用以下键方案存储检查点：

- **检查点数据**：`{prefix}:checkpoint:{thread_id}:{checkpoint_id}` — JSON 序列化的 `Checkpoint`
- **线程索引**：`{prefix}:idx:{thread_id}` — 按时间顺序排列的检查点 ID Redis LIST

## PostgreSQL 检查点

### 快速开始

```rust,ignore
use sqlx::postgres::PgPoolOptions;
use synaptic_pgvector::checkpointer::PgCheckpointer;
use synaptic::graph::{create_react_agent, MessageState};
use std::sync::Arc;

// 创建连接池
let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect("postgres://user:pass@localhost/mydb")
    .await?;

// 创建并初始化检查点（若表不存在则自动创建）
let checkpointer = PgCheckpointer::new(pool);
checkpointer.initialize().await?;

// 构建图
let graph = create_react_agent(model, tools)?
    .with_checkpointer(Arc::new(checkpointer));
```

### 数据库 Schema

`initialize()` 会在表不存在时创建以下表：

```sql
CREATE TABLE IF NOT EXISTS synaptic_checkpoints (
    thread_id     TEXT        NOT NULL,
    checkpoint_id TEXT        NOT NULL,
    state         JSONB       NOT NULL,
    next_node     TEXT,
    parent_id     TEXT,
    metadata      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (thread_id, checkpoint_id)
);
```

### 自定义表名

```rust,ignore
let checkpointer = PgCheckpointer::new(pool)
    .with_table("my_custom_checkpoints");
checkpointer.initialize().await?;
```

## 结合人工干预的持久化工作流

持久化检查点支持有状态的人工干预工作流：

```rust,ignore
use synaptic::graph::{StateGraph, MessageState, StreamMode};
use synaptic_redis::checkpointer::RedisCheckpointer;

let checkpointer = Arc::new(RedisCheckpointer::from_url("redis://localhost/").await?);

// 编译图，在 "human_review" 节点前中断
let graph = builder
    .interrupt_before(vec!["human_review"])
    .compile_with_checkpointer(checkpointer)?;

// 第一次调用——图在 "human_review" 前暂停
let config = RunnableConfig::default().with_metadata("thread_id", "session-42");
let result = graph.invoke_with_config(initial_state, config.clone()).await?;

// 注入人工反馈并恢复执行
let updated = graph.update_state(config.clone(), feedback_state).await?;
let final_result = graph.invoke_with_config(updated, config).await?;
```

## 时间旅行调试

通过检查点 ID 检索任意历史检查点，用于调试或重放：

```rust,ignore
use synaptic_graph::checkpoint::{CheckpointConfig, Checkpointer};

let config = CheckpointConfig::with_checkpoint_id("thread-123", "specific-checkpoint-id");
if let Some(checkpoint) = checkpointer.get(&config).await? {
    println!("检查点状态：{:?}", checkpoint.state);
}

// 列出线程的所有检查点
let all = checkpointer.list(&CheckpointConfig::new("thread-123")).await?;
println!("总检查点数：{}", all.len());
```

## 错误处理

```rust,ignore
use synaptic::core::SynapticError;

match checkpointer.get(&config).await {
    Ok(Some(cp)) => println!("已加载检查点：{}", cp.id),
    Ok(None) => println!("未找到检查点"),
    Err(SynapticError::Store(msg)) => eprintln!("存储错误：{msg}"),
    Err(e) => return Err(e.into()),
}
```
