//! `token_gather`; non-parametric utility head.

use candle_core::{Result, Tensor};

/// Stateless token-gather head. Holds no parameters.
pub struct TokenGather;

impl TokenGather {
    /// `hidden_states[0, word_indices, :]` → `[1, num_words, H]`.
    pub fn forward(&self, hidden_states: &Tensor, word_indices: &Tensor) -> Result<Tensor> {
        let h = hidden_states.squeeze(0)?; // [S, H]
        let gathered = h.index_select(word_indices, 0)?; // [num_words, H]
        gathered.unsqueeze(0) // [1, num_words, H]
    }
}
