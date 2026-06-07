//! Telemetry and observability for kreuzberg.
//!
//! This module provides:
//! - **Semantic conventions** ([`conventions`]) — constant attribute and metric
//!   names following the `kreuzberg.*` namespace.
//! - **Span helpers** (`spans`) — functions to create properly-attributed
//!   tracing spans (requires `otel` feature).
//! - **Metrics instruments** (`metrics`) — counters, histograms, and gauges
//!   for monitoring extraction operations (requires `otel` feature).
//!
//! The `conventions` module is always available (it's just string constants).
//! The `spans` and `metrics` modules are gated behind the `otel` feature.

pub mod conventions;

#[cfg(feature = "otel")]
pub mod metrics;

#[cfg(feature = "otel")]
pub mod spans;
