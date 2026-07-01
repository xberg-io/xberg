//! `count_pred` head — 2-layer MLP over the pooled prompt embedding.

use candle_core::Tensor;
use candle_nn::{Linear, Module, VarBuilder, linear};

/// Maximum count class index. Output dim is `MAX_COUNT_CLASSES = 20`,
/// so valid argmax results fall in `[0, 19]`.
const MAX_COUNT_CLASSES: usize = 20;

/// `count_pred` — 2-layer MLP that predicts a count class given the
/// pooled prompt embedding.
pub struct CountPred {
    linear_0: Linear,
    linear_2: Linear,
}

impl CountPred {
    /// Construct from a `VarBuilder` rooted at `count_pred`.
    pub fn from_var_builder(vb: &VarBuilder) -> candle_core::Result<Self> {
        let linear_0 = linear(768, 1536, vb.pp("0"))?;
        let linear_2 = linear(1536, MAX_COUNT_CLASSES, vb.pp("2"))?;
        Ok(Self { linear_0, linear_2 })
    }

    /// * `p_emb` — pooled prompt embedding `[1, 768]` (or `[768]`).
    ///
    /// Returns the predicted count as a host-side `usize`, clamped to `[0, 19]`.
    pub fn forward(&self, p_emb: &Tensor) -> candle_core::Result<usize> {
        let p_emb_2d = match p_emb.rank() {
            1 => p_emb.reshape((1, 768))?,
            2 => p_emb.clone(),
            other => {
                return Err(candle_core::Error::Msg(format!(
                    "count_pred::forward: expected p_emb rank 1 or 2, got {other}"
                )));
            }
        };

        let h1 = self.linear_0.forward(&p_emb_2d)?.relu()?;
        let logits = self.linear_2.forward(&h1)?;

        let argmax = logits.argmax(1)?; // [1], dtype u32
        let argmax_scalar = argmax.reshape(())?.to_scalar::<u32>()? as usize;

        Ok(argmax_scalar.min(MAX_COUNT_CLASSES - 1))
    }
}
