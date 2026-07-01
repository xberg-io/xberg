//! `scorer` — non-parametric utility head.
//!
//! `scores[b, p, l, k] = sigmoid(Σ_d span_rep[l, k, d] * struct_proj[b, p, d])`,
//! computed as a single matmul + reshape + sigmoid.

use candle_core::{Result, Tensor};

/// Stateless scorer head. Holds no parameters.
pub struct Scorer;

impl Scorer {
    /// * `span_rep`: `[T, W, H]` (per-sample slice of `[1, T, W, H]`).
    /// * `struct_proj`: `[count, F, H]`.
    /// Returns `[count, F, T, W]` sigmoid scores.
    pub fn forward(&self, span_rep: &Tensor, struct_proj: &Tensor) -> Result<Tensor> {
        let (t, w, h) = span_rep.dims3()?;
        let (count, f, h2) = struct_proj.dims3()?;
        if h != h2 {
            return Err(candle_core::Error::Msg(format!(
                "scorer: hidden mismatch {h} vs {h2}"
            )));
        }

        let span_flat = span_rep.reshape(((), h))?.contiguous()?; // [T*W, H]
        let struct_flat = struct_proj.reshape(((), h))?.contiguous()?; // [count*F, H]
        let scores_flat = struct_flat.matmul(&span_flat.transpose(0, 1)?.contiguous()?)?;
        let scores = scores_flat.reshape((count, f, t, w))?;

        candle_nn::ops::sigmoid(&scores)
    }
}
