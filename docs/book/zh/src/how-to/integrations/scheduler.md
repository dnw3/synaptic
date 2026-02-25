# 任务调度

基于 cron 和固定间隔的定时任务调度。

## 简介

`synaptic-scheduler` crate 提供了一个轻量级的任务调度系统，包含：

- **`SchedulerTask`** trait -- 定义可调度的工作单元。
- **`Scheduler`** trait -- 调度器接口，支持 cron 表达式和固定间隔两种调度方式。
- **`TokioScheduler`** -- 基于 Tokio 任务和定时器的调度器实现。

每个注册的任务在独立的 Tokio 任务中运行，支持取消、列出和批量关闭。

## 安装

在 `Cargo.toml` 中添加 `scheduler` feature：

```toml
[dependencies]
synaptic = { version = "0.3", features = ["scheduler"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## SchedulerTask Trait

实现 `SchedulerTask` trait 来定义可调度的任务：

```rust,ignore
use async_trait::async_trait;
use synaptic::scheduler::SchedulerTask;

struct HealthCheckTask {
    endpoint: String,
}

#[async_trait]
impl SchedulerTask for HealthCheckTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let resp = client.get(&self.endpoint).send().await?;
        println!("健康检查 {}: {}", self.endpoint, resp.status());
        Ok(())
    }
}
```

`SchedulerTask` 要求 `Send + Sync + 'static`，因此任务可以安全地在异步运行时中跨线程共享。

## TokioScheduler

`TokioScheduler` 是 `Scheduler` trait 的默认实现。每个任务通过 `tokio::spawn` 在独立的异步任务中运行。

### 创建调度器

```rust,ignore
use synaptic::scheduler::TokioScheduler;

let scheduler = TokioScheduler::new();
```

### schedule_cron -- Cron 调度

使用 cron 表达式注册定时任务：

```rust,ignore
use synaptic::scheduler::Scheduler;

let job_id = scheduler.schedule_cron(
    "*/5 * * * *",          // 每 5 分钟
    "health_check",         // 任务名称
    Box::new(HealthCheckTask {
        endpoint: "https://api.example.com/health".to_string(),
    }),
).await?;

println!("已注册任务: {}", job_id);
```

### schedule_interval -- 固定间隔调度

使用 `Duration` 指定任务执行间隔：

```rust,ignore
use std::time::Duration;
use synaptic::scheduler::Scheduler;

let job_id = scheduler.schedule_interval(
    Duration::from_secs(30),    // 每 30 秒
    "metrics_collector",        // 任务名称
    Box::new(MetricsTask {}),
).await?;
```

### cancel -- 取消任务

```rust,ignore
scheduler.cancel(&job_id).await?;
```

### list_jobs -- 列出所有任务

```rust,ignore
let jobs = scheduler.list_jobs().await;
for job in &jobs {
    println!("任务 {}: {} ({:?})", job.id, job.name, job.schedule);
    println!("  已执行次数: {}", job.run_count);
}
```

`JobInfo` 包含以下字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | `JobId` | 唯一标识符 |
| `name` | `String` | 任务名称 |
| `schedule` | `ScheduleKind` | `Cron(String)` 或 `Interval(Duration)` |
| `next_run` | `Option<Instant>` | 下次执行时间 |
| `run_count` | `u64` | 已执行次数 |

### shutdown -- 关闭调度器

停止所有任务并清理资源：

```rust,ignore
scheduler.shutdown().await;
assert!(scheduler.list_jobs().await.is_empty());
```

## Cron 表达式参考

调度器内置了一个简化的 cron 解析器，支持以下常用模式：

| 表达式 | 含义 |
|--------|------|
| `*/N * * * *` | 每 N 分钟执行一次 |
| `0 * * * *` | 每小时整点执行 |
| `0 0 * * *` | 每天午夜执行 |
| `0 0 * * 0` | 每周日午夜执行 |
| `0 0 * * 1` | 每周一午夜执行 |

Cron 表达式格式为 5 个字段：`分钟 小时 日 月 星期`，其中星期 0 = 周日，6 = 周六。

> **注意：** 内置解析器仅支持上述常用模式。对于更复杂的 cron 表达式，建议结合 `schedule_interval` 和自定义逻辑实现。

## 示例 -- 每 5 分钟执行健康检查

以下是一个完整的示例，展示如何设置定时健康检查并在一段时间后优雅关闭：

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use synaptic::scheduler::{Scheduler, SchedulerTask, TokioScheduler};

struct HealthCheckTask {
    service_name: String,
    endpoint: String,
}

#[async_trait]
impl SchedulerTask for HealthCheckTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let resp = client.get(&self.endpoint).send().await?;

        if resp.status().is_success() {
            println!("[{}] 健康检查通过", self.service_name);
        } else {
            eprintln!("[{}] 健康检查失败: {}", self.service_name, resp.status());
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = TokioScheduler::new();

    // 注册多个健康检查任务
    scheduler.schedule_cron(
        "*/5 * * * *",
        "api_health",
        Box::new(HealthCheckTask {
            service_name: "API".to_string(),
            endpoint: "https://api.example.com/health".to_string(),
        }),
    ).await?;

    scheduler.schedule_interval(
        Duration::from_secs(120),
        "db_health",
        Box::new(HealthCheckTask {
            service_name: "Database".to_string(),
            endpoint: "https://db.example.com/ping".to_string(),
        }),
    ).await?;

    // 列出已注册的任务
    let jobs = scheduler.list_jobs().await;
    println!("已注册 {} 个调度任务", jobs.len());

    // 运行一段时间后关闭
    tokio::time::sleep(Duration::from_secs(3600)).await;
    scheduler.shutdown().await;

    Ok(())
}
```
