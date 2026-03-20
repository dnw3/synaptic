//! Built-in event subscribers (observers).
//!
//! These were previously in `synaptic-callbacks`; they now live here as
//! first-party observers shipped with the event system.

mod cost_tracking;
mod metrics;
mod recording;
mod stdout;
mod tracing_cb;

pub use cost_tracking::{
    default_pricing, CostTrackingCallback, ModelPricing, ModelUsage, UsageSnapshot,
};
pub use metrics::{MetricsCallback, MetricsSnapshot, ToolMetrics};
pub use recording::RecordingCallback;
pub use stdout::StdOutCallbackHandler;
pub use tracing_cb::TracingCallback;

#[cfg(feature = "otel")]
mod opentelemetry_cb;
#[cfg(feature = "otel")]
pub use opentelemetry_cb::OpenTelemetryCallback;
