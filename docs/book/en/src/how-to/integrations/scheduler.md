# Scheduler

Cron and interval-based job scheduling for Synaptic agents. The `synaptic-scheduler` crate provides a `Scheduler` trait and a Tokio-backed implementation for running periodic tasks such as health checks, data syncs, or scheduled agent invocations.

## Setup

Add the `scheduler` feature to your `Cargo.toml`:

```toml
[dependencies]
synaptic = { version = "0.4", features = ["scheduler"] }
```

## SchedulerTask Trait

Every scheduled job must implement the `SchedulerTask` trait:

```rust,ignore
use async_trait::async_trait;
use synaptic::scheduler::SchedulerTask;

struct MyTask {
    label: String,
}

#[async_trait]
impl SchedulerTask for MyTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("[{}] Running scheduled task", self.label);
        // Perform your work here
        Ok(())
    }
}
```

The `run` method is called each time the scheduler fires the job. If it returns an error, the scheduler logs it and continues scheduling future runs.

## TokioScheduler

`TokioScheduler` is the built-in implementation backed by Tokio tasks and timers. Each job runs in its own spawned task.

```rust,ignore
use synaptic::scheduler::{TokioScheduler, Scheduler};

let scheduler = TokioScheduler::new();
```

### Scheduling with a Cron Expression

```rust,ignore
use synaptic::scheduler::{TokioScheduler, Scheduler, SchedulerTask};

let scheduler = TokioScheduler::new();

let task = Box::new(MyTask { label: "cron-job".to_string() });
let job_id = scheduler.schedule_cron("*/5 * * * *", "every-5-min", task).await?;
println!("Scheduled job: {}", job_id);
```

### Scheduling with a Fixed Interval

```rust,ignore
use std::time::Duration;
use synaptic::scheduler::{TokioScheduler, Scheduler};

let scheduler = TokioScheduler::new();

let task = Box::new(MyTask { label: "interval-job".to_string() });
let job_id = scheduler
    .schedule_interval(Duration::from_secs(30), "every-30s", task)
    .await?;
```

### Managing Jobs

```rust,ignore
// List all registered jobs
let jobs = scheduler.list_jobs().await;
for job in &jobs {
    println!("{}: {} (runs: {})", job.id, job.name, job.run_count);
}

// Cancel a specific job
scheduler.cancel(&job_id).await?;

// Shut down all jobs
scheduler.shutdown().await;
```

## Cron Expression Reference

The built-in cron parser supports a subset of standard 5-field expressions:

| Expression      | Description                  |
|-----------------|------------------------------|
| `*/5 * * * *`   | Every 5 minutes              |
| `*/15 * * * *`  | Every 15 minutes             |
| `0 * * * *`     | Every hour (at minute 0)     |
| `0 0 * * *`     | Daily at midnight            |
| `0 0 * * 0`     | Weekly on Sunday at midnight |
| `0 0 * * 1`     | Weekly on Monday at midnight |

The fields are: `minute hour day-of-month month day-of-week`. Unsupported or complex expressions (e.g. ranges, lists) will return a `SynapticError::Config` error at scheduling time.

## JobInfo

The `list_jobs()` method returns `Vec<JobInfo>` with metadata about each registered job:

| Field       | Type                   | Description                       |
|-------------|------------------------|-----------------------------------|
| `id`        | `JobId`                | Unique identifier (UUID)          |
| `name`      | `String`               | Human-readable job name           |
| `schedule`  | `ScheduleKind`         | `Cron(expr)` or `Interval(dur)`   |
| `next_run`  | `Option<Instant>`      | Estimated next execution time     |
| `run_count` | `u64`                  | Number of completed runs          |

## Example: Periodic Health Check Agent

A complete example that runs a health-check agent every 5 minutes:

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use synaptic::core::{ChatModel, ChatRequest, Message};
use synaptic::openai::OpenAiChatModel;
use synaptic::scheduler::{TokioScheduler, Scheduler, SchedulerTask};

struct HealthCheckTask {
    model: Arc<dyn ChatModel>,
}

#[async_trait]
impl SchedulerTask for HealthCheckTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let request = ChatRequest::new(vec![
            Message::system("You are a health check assistant. Summarize system status."),
            Message::human("Check that all services are operational."),
        ]);

        let response = self.model.chat(&request).await?;
        println!("Health check result: {}", response.message.content());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model: Arc<dyn ChatModel> = Arc::new(OpenAiChatModel::new("gpt-4o-mini"));
    let scheduler = TokioScheduler::new();

    let task = Box::new(HealthCheckTask { model });
    scheduler.schedule_cron("*/5 * * * *", "health-check", task).await?;

    println!("Scheduler running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    scheduler.shutdown().await;
    Ok(())
}
```

## Example: Multiple Jobs

```rust,ignore
use std::time::Duration;
use synaptic::scheduler::{TokioScheduler, Scheduler};

let scheduler = TokioScheduler::new();

// Fast polling job
let fast_task = Box::new(MyTask { label: "fast-poll".to_string() });
scheduler
    .schedule_interval(Duration::from_secs(10), "fast-poll", fast_task)
    .await?;

// Hourly summary job
let hourly_task = Box::new(MyTask { label: "hourly-summary".to_string() });
scheduler
    .schedule_cron("0 * * * *", "hourly-summary", hourly_task)
    .await?;

// Check what is running
let jobs = scheduler.list_jobs().await;
assert_eq!(jobs.len(), 2);
```
