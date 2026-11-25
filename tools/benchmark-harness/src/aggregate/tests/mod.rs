//! Tests for [`super`], split by what they exercise. [`support`] holds the shared fixture
//! builders; every other submodule is a group of `#[test]` functions moved verbatim out of the
//! former single `aggregate.rs` test module.

use super::*;

mod support;

mod derived_metrics;
mod extraction_duration;
mod keys_and_shape;
mod paired_deltas;
mod percentiles_and_batching;
mod quality_and_failures;
mod ranking_pareto;
mod ranking_segments;
