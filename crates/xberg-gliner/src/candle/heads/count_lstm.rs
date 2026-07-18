//! `count_embed` head; `CountLSTM` ported to Candle (M5a).
//!
//! Despite the `count_lstm` filename (kept for the M3-stub call sites) and
//! the `counting_layer: "count_lstm"` config field, the actual module in
//! `fastino/gliner2-multi-v1` is a single-layer **GRU** plus a positional
//! embedding and a 2-layer projection MLP. See
//! `docs/dev-notes/gliner2-multi-v1-safetensors-keys.md` ("count_embed")
//! and `docs/dev-notes/gliner2-multi-v1-forward-pass.md` §3.
//!
//! Forward (notation matches the dev-note):
//!
//! ```text
//!   L = min(pred_count, MAX_COUNT = 20)
//!   if L == 0: return zeros([0, F, 768])
//!   pos_seq = pos_embedding[arange(L)]                 # [L, 768]
//!   pos_seq_fbcast = pos_seq.unsqueeze(1).expand(L, F, 768)
//!   h_0 = field_embs                                   # [F, 768]
//!   for t in 0..L:
//!       h_t = GRU_step(pos_seq_fbcast[t], h_{t-1})     # [F, 768]
//!   out_h = stack(h_0..h_{L-1}, dim=0)                 # [L, F, 768]
//!   field_emb_b = field_embs.unsqueeze(0).expand(L, F, 768)
//!   cat = concat([out_h, field_emb_b], dim=-1)         # [L, F, 1536]
//!   struct_proj = projector_2(relu(projector_0(cat)))  # [L, F, 768]
//! ```
//!
//! `candle_nn::rnn::GRU` already implements PyTorch GRU semantics with
//! (reset, update, new) gate ordering (matches `nn.GRU`'s `weight_ih_l0` /
//! `weight_hh_l0` / `bias_{ih,hh}_l0` layout exactly), so the safetensors
//! weights load directly with no permutation.

use candle_core::{Device, IndexOp, Tensor};
use candle_nn::rnn::{GRU, GRUConfig, GRUState, RNN, gru};
use candle_nn::{Embedding, Linear, Module, VarBuilder, embedding, linear};

/// Maximum supported count value (matches `pos_embedding.weight.shape[0]`
/// and the `count_pred` output dimension).
pub const MAX_COUNT: usize = 20;

/// Hidden / input size for the GRU and the field/pos embeddings.
const HIDDEN: usize = 768;

/// `CountLSTM` (misnomer; actually a GRU). Builds a per-(instance, field)
/// "structure" embedding tensor of shape `[count, num_fields, 768]`.
pub struct CountLstmFixed {
    gru: GRU,
    pos_embedding: Embedding,
    /// projector.0 : Linear 1536 → 3072
    projector_0: Linear,
    /// projector.2 : Linear 3072 → 768
    projector_2: Linear,
    device: Device,
}

impl CountLstmFixed {
    /// Construct from a `VarBuilder` rooted at `count_embed`.
    pub fn from_var_builder(vb: &VarBuilder, device: &Device) -> candle_core::Result<Self> {
        let gru = gru(HIDDEN, HIDDEN, GRUConfig::default(), vb.pp("gru"))?;
        let pos_embedding = embedding(MAX_COUNT, HIDDEN, vb.pp("pos_embedding"))?;
        let projector_0 = linear(1536, 3072, vb.pp("projector.0"))?;
        let projector_2 = linear(3072, HIDDEN, vb.pp("projector.2"))?;

        Ok(Self {
            gru,
            pos_embedding,
            projector_0,
            projector_2,
            device: device.clone(),
        })
    }

    /// Forward pass.
    pub fn forward(&self, field_embs: &Tensor, pred_count: usize, device: &Device) -> candle_core::Result<Tensor> {
        let _ = device;
        let (f, h) = field_embs.dims2()?;
        debug_assert_eq!(h, HIDDEN);

        let l = pred_count.min(MAX_COUNT);

        if l == 0 {
            return Tensor::zeros((0, f, HIDDEN), field_embs.dtype(), &self.device);
        }

        let pos_ids = Tensor::arange(0u32, l as u32, &self.device)?;
        let pos_seq = self.pos_embedding.forward(&pos_ids)?;

        let mut state = GRUState {
            h: field_embs.contiguous()?,
        };
        let mut hidden_states: Vec<Tensor> = Vec::with_capacity(l);

        for t in 0..l {
            let pos_t = pos_seq.i(t)?;
            let input_t = pos_t.broadcast_as((f, HIDDEN))?.contiguous()?;
            state = self.gru.step(&input_t, &state)?;
            hidden_states.push(state.h.clone());
        }

        let out_h = Tensor::stack(&hidden_states, 0)?;

        let field_b = field_embs.unsqueeze(0)?.broadcast_as((l, f, HIDDEN))?.contiguous()?;

        let cat = Tensor::cat(&[&out_h, &field_b], 2)?;

        let h2 = self.projector_0.forward(&cat)?.relu()?;
        let struct_proj = self.projector_2.forward(&h2)?;

        Ok(struct_proj)
    }
}
