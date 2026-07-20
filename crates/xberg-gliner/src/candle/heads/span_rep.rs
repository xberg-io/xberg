//! `span_rep` head; three 2-layer MLPs (`project_start`, `project_end`,
//! `out_project`) with ReLU between layers.

use candle_core::{IndexOp, Tensor};
use candle_nn::{Linear, Module, VarBuilder, linear};

use super::MAX_WIDTH;

/// SpanMarkerV0; builds per-(start, end) span representations from
/// per-token hidden states.
pub struct SpanRep {
    project_start_0: Linear,
    project_start_3: Linear,
    project_end_0: Linear,
    project_end_3: Linear,
    out_project_0: Linear,
    out_project_3: Linear,
}

impl SpanRep {
    /// Construct from a `VarBuilder` rooted at `span_rep.span_rep_layer`.
    pub fn from_var_builder(vb: &VarBuilder) -> candle_core::Result<Self> {
        let project_start_0 = linear(768, 3072, vb.pp("project_start.0"))?;
        let project_start_3 = linear(3072, 768, vb.pp("project_start.3"))?;
        let project_end_0 = linear(768, 3072, vb.pp("project_end.0"))?;
        let project_end_3 = linear(3072, 768, vb.pp("project_end.3"))?;
        let out_project_0 = linear(1536, 3072, vb.pp("out_project.0"))?;
        let out_project_3 = linear(3072, 768, vb.pp("out_project.3"))?;

        Ok(Self {
            project_start_0,
            project_start_3,
            project_end_0,
            project_end_3,
            out_project_0,
            out_project_3,
        })
    }

    /// * `text_emb`; `[1, T, 768]` per-word pooled hidden states.
    /// * `span_idx`; `[1, T*MAX_WIDTH, 2]` int64 (start, end) indices.
    ///
    /// Returns `[1, T, MAX_WIDTH, 768]`.
    pub fn forward(&self, text_emb: &Tensor, span_idx: &Tensor) -> candle_core::Result<Tensor> {
        let (b, t, _h) = text_emb.dims3()?;
        debug_assert_eq!(b, 1, "SpanRep currently assumes batch=1");

        let start_rep = self
            .project_start_3
            .forward(&self.project_start_0.forward(text_emb)?.relu()?)?;
        let end_rep = self
            .project_end_3
            .forward(&self.project_end_0.forward(text_emb)?.relu()?)?;

        let start_idx = span_idx.i((0, .., 0))?.contiguous()?;
        let end_idx = span_idx.i((0, .., 1))?.contiguous()?;

        let start_rep_2d = start_rep.squeeze(0)?;
        let end_rep_2d = end_rep.squeeze(0)?;

        let start_at = start_rep_2d.index_select(&start_idx, 0)?;
        let end_at = end_rep_2d.index_select(&end_idx, 0)?;

        let cat = Tensor::cat(&[&start_at, &end_at], 1)?.relu()?;

        let out_2d = self.out_project_3.forward(&self.out_project_0.forward(&cat)?.relu()?)?;

        out_2d.reshape((1, t, MAX_WIDTH, 768))
    }
}
