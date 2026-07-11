//! The [`StructuredPolicy`] seam: the structured-extraction call-mode decision.
//!
//! The in-core default is [`DefaultStructuredPolicy`], which delegates verbatim
//! to [`heuristics::choose_call_mode`](crate::heuristics::choose_call_mode) over
//! the default [`StructuredThresholds`] and the default [`MergeMode`] — exactly
//! the decision xberg makes today. Alternative policies implement this trait and
//! are injected via
//! [`EngineBuilder::with_structured_policy`](super::super::EngineBuilder::with_structured_policy).

use crate::core::config::MergeMode;
use crate::heuristics::{StructuredCallMode, StructuredInput, StructuredThresholds, choose_call_mode};

/// Decides whether and how a document enters the structured-extraction pipeline.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn StructuredPolicy>`; they may be called concurrently.
#[cfg_attr(alef, alef(skip))]
pub trait StructuredPolicy: Send + Sync + 'static {
    /// Choose the call mode for `input` using this policy's thresholds.
    fn choose_call_mode(&self, input: &StructuredInput) -> StructuredCallMode;

    /// The thresholds this policy applies.
    fn thresholds(&self) -> &StructuredThresholds;

    /// The merge strategy this policy uses for paginated structured output.
    fn merge_mode(&self) -> MergeMode;
}

/// In-core default: the conservative built-in heuristic.
///
/// Holds the default [`StructuredThresholds`] and default [`MergeMode`] and
/// delegates [`choose_call_mode`](StructuredPolicy::choose_call_mode) straight
/// to [`heuristics::choose_call_mode`](crate::heuristics::choose_call_mode), so
/// its decision is byte-identical to calling that free function directly.
#[derive(Debug, Default, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct DefaultStructuredPolicy {
    thresholds: StructuredThresholds,
    merge_mode: MergeMode,
}

impl StructuredPolicy for DefaultStructuredPolicy {
    fn choose_call_mode(&self, input: &StructuredInput) -> StructuredCallMode {
        choose_call_mode(input, &self.thresholds)
    }

    fn thresholds(&self) -> &StructuredThresholds {
        &self.thresholds
    }

    fn merge_mode(&self) -> MergeMode {
        self.merge_mode
    }
}
