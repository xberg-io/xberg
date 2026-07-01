//! GLiNER2 inference heads (Candle).
//!
//! `token_gather`, `schema_gather`, `scorer` are parameter-free utilities.
//! `span_rep`, `count_pred`, `count_lstm` are parametric (Task 5b). The
//! `classifier` head from anno is intentionally NOT ported — this crate
//! ships `extract_ner` parity only (see plan Global Constraints).

pub mod count_lstm;
pub mod count_pred;
pub mod schema_gather;
pub mod scorer;
pub mod span_rep;
pub mod token_gather;

/// Maximum span width baked into the v2 Candle heads' trained weights
/// (`span_rep`'s reshape, `scorer`'s axis sizing). Model-architecture-fixed —
/// see Global Constraints.
pub(crate) const MAX_WIDTH: usize = 8;
