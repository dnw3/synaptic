//! Job scheduling (cron + interval) for the Synaptic AI agent framework.
//!
//! Provides a [`Scheduler`] trait and a [`TokioScheduler`] implementation
//! for cron-based and interval-based job scheduling.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use synaptic_core::SynapticError;
use tokio::sync::RwLock;
use tokio::time::Instant;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Unique identifier for a scheduled job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub String);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Describes how a job is scheduled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleKind {
    /// Cron expression (e.g. `"*/5 * * * *"`).
    Cron(String),
    /// Fixed interval between runs.
    Interval(Duration),
}

/// Metadata about a registered job.
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: JobId,
    pub name: String,
    pub schedule: ScheduleKind,
    pub next_run: Option<Instant>,
    pub run_count: u64,
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// A unit of work that can be executed by the scheduler.
#[async_trait]
pub trait SchedulerTask: Send + Sync + 'static {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Scheduler interface for registering, cancelling, and managing jobs.
#[async_trait]
pub trait Scheduler: Send + Sync {
    /// Schedule a task using a cron expression.
    ///
    /// Supported patterns:
    /// - `"*/N * * * *"` — every N minutes
    /// - `"0 * * * *"` — every hour
    /// - `"0 0 * * *"` — daily at midnight
    /// - `"0 0 * * 0"` — weekly on Sunday at midnight
    async fn schedule_cron(
        &self,
        cron_expr: &str,
        name: &str,
        task: Box<dyn SchedulerTask>,
    ) -> Result<JobId, SynapticError>;

    /// Schedule a task with a fixed interval between runs.
    async fn schedule_interval(
        &self,
        interval: Duration,
        name: &str,
        task: Box<dyn SchedulerTask>,
    ) -> Result<JobId, SynapticError>;

    /// Cancel a previously scheduled job.
    async fn cancel(&self, job_id: &JobId) -> Result<(), SynapticError>;

    /// List all registered jobs.
    async fn list_jobs(&self) -> Vec<JobInfo>;

    /// Stop all jobs and shut down the scheduler.
    async fn shutdown(&self);
}

// ---------------------------------------------------------------------------
// Cron helpers (minimal built-in parser)
// ---------------------------------------------------------------------------

/// Parsed representation of simple cron patterns.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum CronSchedule {
    /// Run every N minutes.
    EveryNMinutes(u64),
    /// Run once per hour at minute 0.
    Hourly,
    /// Run once per day at 00:00.
    Daily,
    /// Run once per week on a given weekday (0 = Sunday) at 00:00.
    Weekly(u32),
}

impl CronSchedule {
    /// Parse a subset of cron expressions.
    fn parse(expr: &str) -> Result<Self, SynapticError> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(SynapticError::Config(format!(
                "unsupported cron expression: {expr}"
            )));
        }

        let (minute, hour, _dom, _month, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

        // "*/N * * * *" — every N minutes
        if minute.starts_with("*/") && hour == "*" && dow == "*" {
            let n: u64 = minute[2..].parse().map_err(|_| {
                SynapticError::Config(format!("unsupported cron expression: {expr}"))
            })?;
            if n == 0 {
                return Err(SynapticError::Config(format!(
                    "unsupported cron expression: {expr}"
                )));
            }
            return Ok(CronSchedule::EveryNMinutes(n));
        }

        // "0 * * * *" — hourly
        if minute == "0" && hour == "*" && dow == "*" {
            return Ok(CronSchedule::Hourly);
        }

        // "0 0 * * *" — daily
        if minute == "0" && hour == "0" && dow == "*" {
            return Ok(CronSchedule::Daily);
        }

        // "0 0 * * N" — weekly on day N
        if minute == "0" && hour == "0" {
            let day: u32 = dow.parse().map_err(|_| {
                SynapticError::Config(format!("unsupported cron expression: {expr}"))
            })?;
            if day > 6 {
                return Err(SynapticError::Config(format!(
                    "unsupported cron expression: {expr}"
                )));
            }
            return Ok(CronSchedule::Weekly(day));
        }

        Err(SynapticError::Config(format!(
            "unsupported cron expression: {expr}"
        )))
    }

    /// Return the [`Duration`] until the next run from *now*.
    fn next_duration(&self) -> Duration {
        match self {
            CronSchedule::EveryNMinutes(n) => Duration::from_secs(n * 60),
            CronSchedule::Hourly => Duration::from_secs(3600),
            CronSchedule::Daily => Duration::from_secs(86400),
            CronSchedule::Weekly(_) => Duration::from_secs(604800),
        }
    }
}

// ---------------------------------------------------------------------------
// TokioScheduler
// ---------------------------------------------------------------------------

/// Internal handle for a running job.
struct JobHandle {
    abort_handle: tokio::task::AbortHandle,
    info: JobInfo,
}

/// A scheduler implementation backed by Tokio tasks and timers.
///
/// Each job runs in its own spawned task. Interval jobs sleep for the
/// specified duration between runs. Cron jobs compute the next run
/// duration from a simple built-in parser.
pub struct TokioScheduler {
    jobs: Arc<RwLock<HashMap<JobId, JobHandle>>>,
}

impl TokioScheduler {
    /// Create a new, empty scheduler.
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for TokioScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scheduler for TokioScheduler {
    async fn schedule_cron(
        &self,
        cron_expr: &str,
        name: &str,
        task: Box<dyn SchedulerTask>,
    ) -> Result<JobId, SynapticError> {
        let schedule = CronSchedule::parse(cron_expr)?;
        let id = JobId(uuid::Uuid::new_v4().to_string());
        let interval = schedule.next_duration();

        let jobs = Arc::clone(&self.jobs);
        let job_id = id.clone();
        let task: Arc<dyn SchedulerTask> = Arc::from(task);

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let _ = task.run().await;

                // Increment run_count
                let mut map = jobs.write().await;
                if let Some(handle) = map.get_mut(&job_id) {
                    handle.info.run_count += 1;
                    handle.info.next_run = Some(Instant::now() + interval);
                } else {
                    // Job was removed — exit the loop.
                    break;
                }
            }
        });

        let info = JobInfo {
            id: id.clone(),
            name: name.to_string(),
            schedule: ScheduleKind::Cron(cron_expr.to_string()),
            next_run: Some(Instant::now() + interval),
            run_count: 0,
        };

        let handle = JobHandle {
            abort_handle: join_handle.abort_handle(),
            info,
        };

        self.jobs.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    async fn schedule_interval(
        &self,
        interval: Duration,
        name: &str,
        task: Box<dyn SchedulerTask>,
    ) -> Result<JobId, SynapticError> {
        let id = JobId(uuid::Uuid::new_v4().to_string());

        let jobs = Arc::clone(&self.jobs);
        let job_id = id.clone();
        let task: Arc<dyn SchedulerTask> = Arc::from(task);

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let _ = task.run().await;

                // Increment run_count
                let mut map = jobs.write().await;
                if let Some(handle) = map.get_mut(&job_id) {
                    handle.info.run_count += 1;
                    handle.info.next_run = Some(Instant::now() + interval);
                } else {
                    // Job was removed — exit the loop.
                    break;
                }
            }
        });

        let info = JobInfo {
            id: id.clone(),
            name: name.to_string(),
            schedule: ScheduleKind::Interval(interval),
            next_run: Some(Instant::now() + interval),
            run_count: 0,
        };

        let handle = JobHandle {
            abort_handle: join_handle.abort_handle(),
            info,
        };

        self.jobs.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    async fn cancel(&self, job_id: &JobId) -> Result<(), SynapticError> {
        let mut map = self.jobs.write().await;
        if let Some(handle) = map.remove(job_id) {
            handle.abort_handle.abort();
            Ok(())
        } else {
            Err(SynapticError::Config(format!(
                "job not found: {}",
                job_id.0
            )))
        }
    }

    async fn list_jobs(&self) -> Vec<JobInfo> {
        let map = self.jobs.read().await;
        map.values().map(|h| h.info.clone()).collect()
    }

    async fn shutdown(&self) {
        let mut map = self.jobs.write().await;
        for (_, handle) in map.drain() {
            handle.abort_handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A simple counter task for testing.
    struct CounterTask {
        count: Arc<AtomicU64>,
    }

    impl CounterTask {
        fn new() -> (Self, Arc<AtomicU64>) {
            let count = Arc::new(AtomicU64::new(0));
            (
                Self {
                    count: Arc::clone(&count),
                },
                count,
            )
        }
    }

    #[async_trait]
    impl SchedulerTask for CounterTask {
        async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_schedule_interval() {
        let scheduler = TokioScheduler::new();
        let (task, count) = CounterTask::new();

        let _id = scheduler
            .schedule_interval(Duration::from_millis(100), "counter", Box::new(task))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(350)).await;

        let runs = count.load(Ordering::SeqCst);
        assert!(runs >= 3, "expected >= 3 runs, got {runs}");

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let scheduler = TokioScheduler::new();
        let (task, _count) = CounterTask::new();

        let id = scheduler
            .schedule_interval(Duration::from_millis(100), "to_cancel", Box::new(task))
            .await
            .unwrap();

        assert_eq!(scheduler.list_jobs().await.len(), 1);

        scheduler.cancel(&id).await.unwrap();

        assert!(
            scheduler.list_jobs().await.is_empty(),
            "job list should be empty after cancel"
        );
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let scheduler = TokioScheduler::new();
        let (task1, _) = CounterTask::new();
        let (task2, _) = CounterTask::new();

        scheduler
            .schedule_interval(Duration::from_secs(60), "job_a", Box::new(task1))
            .await
            .unwrap();

        scheduler
            .schedule_interval(Duration::from_secs(60), "job_b", Box::new(task2))
            .await
            .unwrap();

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 2);

        let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
        assert!(names.contains(&"job_a"));
        assert!(names.contains(&"job_b"));

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_shutdown() {
        let scheduler = TokioScheduler::new();
        let (task1, _) = CounterTask::new();
        let (task2, _) = CounterTask::new();

        scheduler
            .schedule_interval(Duration::from_secs(60), "s1", Box::new(task1))
            .await
            .unwrap();

        scheduler
            .schedule_interval(Duration::from_secs(60), "s2", Box::new(task2))
            .await
            .unwrap();

        assert_eq!(scheduler.list_jobs().await.len(), 2);

        scheduler.shutdown().await;

        assert!(
            scheduler.list_jobs().await.is_empty(),
            "all jobs should be removed after shutdown"
        );
    }

    #[tokio::test]
    async fn test_cron_parse_every_n_minutes() {
        let scheduler = TokioScheduler::new();
        let (task, _) = CounterTask::new();

        let result = scheduler
            .schedule_cron("*/5 * * * *", "every_5_min", Box::new(task))
            .await;
        assert!(result.is_ok());

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_cron_parse_hourly() {
        let scheduler = TokioScheduler::new();
        let (task, _) = CounterTask::new();

        let result = scheduler
            .schedule_cron("0 * * * *", "hourly", Box::new(task))
            .await;
        assert!(result.is_ok());

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_cron_parse_daily() {
        let scheduler = TokioScheduler::new();
        let (task, _) = CounterTask::new();

        let result = scheduler
            .schedule_cron("0 0 * * *", "daily", Box::new(task))
            .await;
        assert!(result.is_ok());

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_cron_parse_weekly() {
        let scheduler = TokioScheduler::new();
        let (task, _) = CounterTask::new();

        let result = scheduler
            .schedule_cron("0 0 * * 0", "weekly_sunday", Box::new(task))
            .await;
        assert!(result.is_ok());

        scheduler.shutdown().await;
    }

    #[tokio::test]
    async fn test_cron_parse_unsupported() {
        let scheduler = TokioScheduler::new();
        let (task, _) = CounterTask::new();

        let result = scheduler
            .schedule_cron("30 4 1,15 * 5", "complex", Box::new(task))
            .await;
        assert!(result.is_err());

        if let Err(SynapticError::Config(msg)) = result {
            assert!(msg.contains("unsupported cron expression"));
        } else {
            panic!("expected SynapticError::Config");
        }
    }
}
