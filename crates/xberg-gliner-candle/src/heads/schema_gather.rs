//! `schema_gather` — non-parametric utility head.

use candle_core::{Result, Tensor};

/// Stateless schema-gather head. Holds no parameters.
pub struct SchemaGather;

/// Result of [`SchemaGather::forward`].
pub struct SchemaGatherOutput {
    /// `[1, H]` — the `[P]` token's hidden state (prompt context).
    pub pc_emb: Tensor,
    /// `[F, H]` — per-field / per-label embeddings.
    pub field_embs: Tensor,
}

impl SchemaGather {
    /// `schema_indices` includes the `[P]` index first, followed by all
    /// per-field `[E]` indices — matches `schema_positions` order from
    /// `xberg_gliner::encode_v2`.
    pub fn forward(
        &self,
        hidden_states: &Tensor,  // [1, S, H]
        schema_indices: &Tensor, // [num_special]
    ) -> Result<SchemaGatherOutput> {
        let h = hidden_states.squeeze(0)?; // [S, H]
        let all = h.index_select(schema_indices, 0)?; // [num_special, H]

        let pc_emb = all.narrow(0, 0, 1)?; // [1, H]
        let n = all.dim(0)?;
        let hidden_dim = all.dim(1)?;
        let field_embs = if n > 1 {
            all.narrow(0, 1, n - 1)? // [F, H]
        } else {
            Tensor::zeros((0, hidden_dim), all.dtype(), all.device())?
        };

        Ok(SchemaGatherOutput { pc_emb, field_embs })
    }
}
