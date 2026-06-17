// Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.
//! Subset of `aha::position_embed::rope` covering 1D, 2D, M-RoPE, and XD-RoPE
//! position embeddings used by Qwen2 / Hunyuan-OCR / PaddleOCR-VL 1.5.
//!
//! Vendored symbols:
//! - [`compute_default_rope_parameters`] — inverse-frequency schedule
//! - [`rotate_half`] — split-half rotation helper
//! - [`apply_rotary_pos_emb`] — standard rotate-half RoPE (Qwen2, Hunyuan-OCR)
//! - [`apply_rotary_pos_emb_vision`] — vision-tower RoPE (PaddleOCR-VL vision)
//! - [`apply_rotary_pos_emb_roformer`] — complex-domain RoPE (common modules)
//! - [`RoPE`] — 1-D RoPE with optional repeat-interleave variant
//! - [`Qwen2_5VLTextRotaryEmbedding`] — M-RoPE for PaddleOCR-VL text decoder
//! - [`Qwen2_5VisionRotaryEmbedding`] — 2-D RoPE for PaddleOCR-VL vision tower
//! - [`get_xd_cos_sin`] — XD-RoPE position gather for Hunyuan-OCR
//!
//! The helper functions `index_select_2d` and `split_tensor` from
//! `aha::utils::tensor_utils` are inlined here because they are consumed only
//! internally by [`get_xd_cos_sin`].

use candle_core::{D, DType, Device, IndexOp, Tensor};

use crate::error::{CandleOcrError, Result};

// ---------------------------------------------------------------------------
// Internal tensor helpers (inlined from aha::utils::tensor_utils)
// ---------------------------------------------------------------------------

/// Select rows of a 2-D table `t` (shape `[table_len, dim]`) using a 2-D
/// integer index `index` (shape `[n, k]`), returning `[n, k, dim]`.
fn index_select_2d(t: &Tensor, index: &Tensor) -> Result<Tensor> {
    let n = index.dim(0)?;
    let mut rows: Vec<Tensor> = Vec::with_capacity(n);
    for i in 0..n {
        let idx_i = index.i(i)?;
        rows.push(t.index_select(&idx_i, 0)?);
    }
    Ok(Tensor::stack(&rows, 0)?)
}

/// Split tensor `t` along `dim` into consecutive slices of sizes `splits`.
fn split_tensor(t: &Tensor, splits: &[usize], dim: usize) -> Result<Vec<Tensor>> {
    let mut results: Vec<Tensor> = Vec::with_capacity(splits.len());
    let mut offset = 0;
    for &size in splits {
        results.push(t.narrow(dim, offset, size)?);
        offset += size;
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Inverse-frequency schedule
// ---------------------------------------------------------------------------

/// Compute standard inverse-frequency coefficients for RoPE.
///
/// Returns a `Vec<f32>` of length `dim / 2` where entry `i` is
/// `1 / base^(2i / dim)`.
pub fn compute_default_rope_parameters(dim: usize, base: f32) -> Vec<f32> {
    (0..dim)
        .step_by(2)
        .map(|i| 1.0_f32 / base.powf(i as f32 / dim as f32))
        .collect()
}

// ---------------------------------------------------------------------------
// Rotation helpers
// ---------------------------------------------------------------------------

/// Split-half rotate: `[x1 | x2] -> [-x2 | x1]`.
///
/// Operates on the last dimension; the dimension must be even.
pub fn rotate_half(x: &Tensor) -> Result<Tensor> {
    let half_dim = x.dim(D::Minus1)? / 2;
    let x1 = x.narrow(D::Minus1, 0, half_dim)?;
    let x2 = x.narrow(D::Minus1, half_dim, half_dim)?;
    let x2_neg = x2.affine(-1.0, 0.0)?;
    Ok(Tensor::cat(&[&x2_neg, &x1], D::Minus1)?.contiguous()?)
}

// ---------------------------------------------------------------------------
// Apply-rotary helpers
// ---------------------------------------------------------------------------

/// Standard rotate-half RoPE application.
///
/// Handles `cos`/`sin` of rank 2 `(seq_len, head_dim)`, rank 3
/// `(bs, seq_len, head_dim)`, or rank 4 `(bs, 1, seq_len, head_dim)`.
/// When `tof32` is `true` the computation runs in `f32` and the result is
/// cast back to the original dtype.
pub fn apply_rotary_pos_emb(
    q: &Tensor,
    k: &Tensor,
    cos: &Tensor,
    sin: &Tensor,
    tof32: bool,
) -> Result<(Tensor, Tensor)> {
    let mut cos = cos.clone();
    let mut sin = sin.clone();
    if cos.rank() == 2 {
        cos = cos.unsqueeze(0)?.unsqueeze(0)?;
        sin = sin.unsqueeze(0)?.unsqueeze(0)?;
    }
    if cos.rank() == 3 {
        cos = cos.unsqueeze(1)?;
        sin = sin.unsqueeze(1)?;
    }
    let orig_dtype = q.dtype();
    let q_f = if tof32 { q.to_dtype(DType::F32)? } else { q.clone() };
    let k_f = if tof32 { k.to_dtype(DType::F32)? } else { k.clone() };
    let cos = cos.to_dtype(q_f.dtype())?;
    let sin = sin.to_dtype(q_f.dtype())?;

    let q_embed = q_f
        .broadcast_mul(&cos)?
        .add(&rotate_half(&q_f)?.broadcast_mul(&sin)?)?
        .to_dtype(orig_dtype)?;
    let k_embed = k_f
        .broadcast_mul(&cos)?
        .add(&rotate_half(&k_f)?.broadcast_mul(&sin)?)?
        .to_dtype(orig_dtype)?;
    Ok((q_embed, k_embed))
}

/// Vision-tower RoPE for PaddleOCR-VL.
///
/// `cos`/`sin` are `(seq_len, head_dim)`; a head axis is inserted to give
/// `(seq_len, 1, head_dim)` before broadcast multiplication.
pub fn apply_rotary_pos_emb_vision(q: &Tensor, k: &Tensor, cos: &Tensor, sin: &Tensor) -> Result<(Tensor, Tensor)> {
    let cos = cos.unsqueeze(D::Minus2)?.to_dtype(q.dtype())?;
    let sin = sin.unsqueeze(D::Minus2)?.to_dtype(q.dtype())?;
    let q_embed = q.broadcast_mul(&cos)?.add(&rotate_half(q)?.broadcast_mul(&sin)?)?;
    let k_embed = k.broadcast_mul(&cos)?.add(&rotate_half(k)?.broadcast_mul(&sin)?)?;
    Ok((q_embed, k_embed))
}

/// Complex-domain (RoFormer-style) RoPE application.
///
/// Decomposes the last dimension into real/imaginary interleaved pairs and
/// applies complex multiplication.  Required by the shared `common::modules`
/// layer used by all three VLM-OCR backends.
pub fn apply_rotary_pos_emb_roformer(q: &Tensor, k: &Tensor, cos: &Tensor, sin: &Tensor) -> Result<(Tensor, Tensor)> {
    let ori_dtype = q.dtype();
    let (bs, n_head, seq_len, dim) = q.dims4()?;
    let half_dim = dim / 2;

    let rotr = cos.narrow(D::Minus1, 0, half_dim)?.to_dtype(DType::F32)?;
    let roti = sin.narrow(D::Minus1, 0, half_dim)?.to_dtype(DType::F32)?;

    let q_f = q.reshape((bs, n_head, seq_len, half_dim, 2))?.to_dtype(DType::F32)?;
    let qr = q_f.narrow(D::Minus1, 0, 1)?.squeeze(D::Minus1)?;
    let qi = q_f.narrow(D::Minus1, 1, 1)?.squeeze(D::Minus1)?;

    let k_f = k.reshape((bs, n_head, seq_len, half_dim, 2))?.to_dtype(DType::F32)?;
    let kr = k_f.narrow(D::Minus1, 0, 1)?.squeeze(D::Minus1)?;
    let ki = k_f.narrow(D::Minus1, 1, 1)?.squeeze(D::Minus1)?;

    let qor = qr.broadcast_mul(&rotr)?.sub(&qi.broadcast_mul(&roti)?)?;
    let qoi = qr.broadcast_mul(&roti)?.add(&qi.broadcast_mul(&rotr)?)?;
    let kor = kr.broadcast_mul(&rotr)?.sub(&ki.broadcast_mul(&roti)?)?;
    let koi = kr.broadcast_mul(&roti)?.add(&ki.broadcast_mul(&rotr)?)?;

    let q_out = Tensor::stack(&[qor, qoi], D::Minus1)?
        .reshape((bs, n_head, seq_len, dim))?
        .to_dtype(ori_dtype)?;
    let k_out = Tensor::stack(&[kor, koi], D::Minus1)?
        .reshape((bs, n_head, seq_len, dim))?
        .to_dtype(ori_dtype)?;
    Ok((q_out, k_out))
}

// ---------------------------------------------------------------------------
// 1-D RoPE struct
// ---------------------------------------------------------------------------

/// Standard 1-D RoPE embedding.
///
/// Used by Qwen2 (`RoPE::forward`), DeepSeek-OCR, and Hunyuan-OCR text
/// decoder.
#[derive(Debug, Clone)]
pub struct RoPE {
    /// Inverse-frequency buffer: shape `(1, dim / 2)`.
    inv_freq: Tensor,
}

impl RoPE {
    /// Construct a `RoPE` for the given `dim` and `theta_base`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError::Candle`] if tensor allocation fails.
    pub fn new(dim: usize, theta_base: f32, device: &Device) -> Result<Self> {
        let inv_freq = compute_default_rope_parameters(dim, theta_base);
        let inv_freq = Tensor::from_slice(&inv_freq, (1, inv_freq.len()), device)?;
        Ok(Self { inv_freq })
    }

    /// Compute `(cos, sin)` of shape `(seq_len, dim)` for positions
    /// `seqlen_offset .. seqlen_offset + seq_len`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError::Candle`] on tensor operation failure.
    pub fn forward(&self, seqlen_offset: usize, seq_len: usize, device: &Device) -> Result<(Tensor, Tensor)> {
        let positions = Tensor::arange(
            seqlen_offset as f32,
            (seqlen_offset + seq_len) as f32,
            self.inv_freq.device(),
        )?
        .reshape((seq_len, 1))?;
        let freqs = positions.matmul(&self.inv_freq)?;
        let emb = Tensor::cat(&[&freqs, &freqs], D::Minus1)?
            .contiguous()?
            .to_device(device)?;
        Ok((emb.cos()?, emb.sin()?))
    }

    /// Variant that uses `repeat_interleave(2)` layout instead of the
    /// `cat(freqs, freqs)` layout — required by some interleaved attention
    /// kernels.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError::Candle`] on tensor operation failure.
    pub fn forward_repeat_interleave(
        &self,
        seqlen_offset: usize,
        seq_len: usize,
        device: &Device,
    ) -> Result<(Tensor, Tensor)> {
        let positions = Tensor::arange(
            seqlen_offset as f32,
            (seqlen_offset + seq_len) as f32,
            self.inv_freq.device(),
        )?
        .reshape((seq_len, 1))?;
        let freqs = positions.matmul(&self.inv_freq)?;
        let cos = freqs
            .cos()?
            .unsqueeze(D::Minus1)?
            .repeat((1, 1, 2))?
            .flatten_from(D::Minus2)?
            .contiguous()?
            .to_device(device)?;
        let sin = freqs
            .sin()?
            .unsqueeze(D::Minus1)?
            .repeat((1, 1, 2))?
            .flatten_from(D::Minus2)?
            .contiguous()?
            .to_device(device)?;
        Ok((cos, sin))
    }
}

// ---------------------------------------------------------------------------
// Qwen2.5-VL text-decoder M-RoPE
// ---------------------------------------------------------------------------

/// M-RoPE embedding for the Qwen2.5-VL **text decoder**, used by
/// PaddleOCR-VL 1.5.
///
/// `mrope_section` partitions `head_dim / 2` into three segments (height,
/// width, time/text).  Each segment receives an independent 1-D positional
/// signal drawn from the corresponding slice of `position_ids[0..3]`.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub struct Qwen2_5VLTextRotaryEmbedding {
    inv_freq: Vec<f32>,
}

#[allow(non_camel_case_types)]
impl Qwen2_5VLTextRotaryEmbedding {
    /// Create an embedding for the given `dim` and `theta_base`.
    pub fn new(dim: usize, theta_base: f32) -> Self {
        Self {
            inv_freq: compute_default_rope_parameters(dim, theta_base),
        }
    }

    /// Compute `(cos, sin)` of shape `(bs, 1, seq_len, head_dim)`.
    ///
    /// `position_ids` — shape `(3, bs, seq_len)`.
    /// `mrope_section` — length-3 `Vec` whose entries sum to `head_dim / 2`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError::Candle`] on tensor operation failure.
    pub fn forward(&self, position_ids: &Tensor, dtype: DType, mrope_section: Vec<usize>) -> Result<(Tensor, Tensor)> {
        // (3, bs, seq_len) -> (3, bs, 1, seq_len)
        let position_ids_expanded = position_ids.unsqueeze(D::Minus2)?.to_dtype(DType::F32)?.contiguous()?;

        // inv_freq -> (1, 1, head_dim/2, 1) -> broadcast (3, bs, head_dim/2, 1)
        let bs = position_ids.dim(1)?;
        let inv_freq_expanded = Tensor::from_vec(
            self.inv_freq.clone(),
            (1, 1, self.inv_freq.len(), 1),
            position_ids.device(),
        )?
        .broadcast_as((3, bs, self.inv_freq.len(), 1))?
        .to_dtype(DType::F32)?
        .contiguous()?;

        // (3, bs, head_dim/2, 1) @ (3, bs, 1, seq_len) -> (3, bs, seq_len, head_dim/2)
        let freqs = inv_freq_expanded.matmul(&position_ids_expanded)?.transpose(2, 3)?;

        // (3, bs, seq_len, head_dim/2) -> (3, bs, seq_len, head_dim)
        let emb = Tensor::cat(&[&freqs, &freqs], D::Minus1)?.contiguous()?;
        let cos_full = emb.cos()?;
        let sin_full = emb.sin()?;

        // Double section sizes because emb is head_dim (not head_dim/2)
        let section_doubled: Vec<usize> = mrope_section.iter().chain(mrope_section.iter()).copied().collect();

        // For each of the 2*N sections, index dim-0 (the "3" axis) by i % 3
        let last_dim_full = cos_full.rank() - 1;
        let cos_select: Vec<Tensor> = split_tensor(&cos_full, &section_doubled, last_dim_full)?
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.i(i % 3).map_err(CandleOcrError::from))
            .collect::<Result<Vec<_>>>()?;
        let cos = Tensor::cat(&cos_select, D::Minus1)?.unsqueeze(1)?.contiguous()?;

        let sin_select: Vec<Tensor> = split_tensor(&sin_full, &section_doubled, last_dim_full)?
            .into_iter()
            .enumerate()
            .map(|(i, m)| m.i(i % 3).map_err(CandleOcrError::from))
            .collect::<Result<Vec<_>>>()?;
        let sin = Tensor::cat(&sin_select, D::Minus1)?.unsqueeze(1)?.contiguous()?;

        Ok((cos.to_dtype(dtype)?, sin.to_dtype(dtype)?))
    }
}

// ---------------------------------------------------------------------------
// Qwen2.5-VL vision-tower 2-D RoPE
// ---------------------------------------------------------------------------

/// 2-D RoPE for the Qwen2.5-VL **vision tower**, used by PaddleOCR-VL 1.5.
///
/// Produces a frequency tensor for a 1-D grid; callers concatenate height
/// and width slices and compute `cos`/`sin` themselves.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub struct Qwen2_5VisionRotaryEmbedding {
    inv_freq: Vec<f32>,
}

#[allow(non_camel_case_types)]
impl Qwen2_5VisionRotaryEmbedding {
    /// Create an embedding for the given `dim` (per-axis; total head_dim is
    /// `2 * dim`) and optional `theta_base` (defaults to `10000.0`).
    pub fn new(dim: usize, theta_base: Option<f32>) -> Self {
        let theta_base = theta_base.unwrap_or(10_000.0_f32);
        Self {
            inv_freq: compute_default_rope_parameters(dim, theta_base),
        }
    }

    /// Compute a `(seqlen, head_dim/2)` frequency tensor for positions
    /// `0..seqlen`.
    ///
    /// # Errors
    ///
    /// Returns [`CandleOcrError::Candle`] on tensor operation failure.
    pub fn forward(&self, seqlen: usize, device: &Device) -> Result<Tensor> {
        let seq = Tensor::arange(0.0_f32, seqlen as f32, device)?.reshape((seqlen, 1))?;
        let inv_freq = Tensor::from_vec(self.inv_freq.clone(), (1, self.inv_freq.len()), device)?;
        Ok(seq.matmul(&inv_freq)?)
    }
}

// ---------------------------------------------------------------------------
// XD-RoPE for Hunyuan-OCR
// ---------------------------------------------------------------------------

/// Gather per-position XD-RoPE cosine/sine slices for Hunyuan-OCR.
///
/// This function is the entry point for Hunyuan's
/// `forward_step_with_position_ids` XD-RoPE path.
///
/// # Arguments
///
/// * `cos`/`sin` — shape `(max_seq_len, head_dim)` precomputed buffers from
///   [`RoPE::forward`].
/// * `position_ids` — shape `(bs, x_dim, seq_len)` integer index tensor where
///   `x_dim == xdrope_section.len()`.
/// * `xdrope_section` — per-dimension head_dim split; entries sum to
///   `head_dim / 2`.
///
/// # Returns
///
/// `(cos, sin)` of shape `(bs, seq_len, head_dim)`.
///
/// # Errors
///
/// Returns [`CandleOcrError::Candle`] on tensor operation failure.
pub fn get_xd_cos_sin(
    cos: &Tensor,
    sin: &Tensor,
    position_ids: &Tensor,
    xdrope_section: Vec<usize>,
) -> Result<(Tensor, Tensor)> {
    let x_dim = xdrope_section.len();
    let bs = position_ids.dim(0)?;

    let mut cos_vec: Vec<Tensor> = Vec::with_capacity(bs);
    let mut sin_vec: Vec<Tensor> = Vec::with_capacity(bs);
    for i in 0..bs {
        let pos_i = position_ids.i(i)?;
        cos_vec.push(index_select_2d(cos, &pos_i)?);
        sin_vec.push(index_select_2d(sin, &pos_i)?);
    }

    // (bs, x_dim, seq_len, dim) -> (bs, seq_len, x_dim, dim)
    let cos_stacked = Tensor::stack(&cos_vec, 0)?.permute((0, 2, 1, 3))?.contiguous()?;
    let sin_stacked = Tensor::stack(&sin_vec, 0)?.permute((0, 2, 1, 3))?.contiguous()?;

    // Double section sizes because cos/sin carry head_dim not head_dim/2
    let section_doubled: Vec<usize> = xdrope_section.iter().map(|&s| s * 2).collect();
    let last_dim = cos_stacked.rank() - 1;

    let cos_select: Vec<Tensor> = split_tensor(&cos_stacked, &section_doubled, last_dim)?
        .into_iter()
        .enumerate()
        .map(|(i, m)| m.i((.., .., i % x_dim)).map_err(CandleOcrError::from))
        .collect::<Result<Vec<_>>>()?;

    let sin_select: Vec<Tensor> = split_tensor(&sin_stacked, &section_doubled, last_dim)?
        .into_iter()
        .enumerate()
        .map(|(i, m)| m.i((.., .., i % x_dim)).map_err(CandleOcrError::from))
        .collect::<Result<Vec<_>>>()?;

    Ok((
        Tensor::cat(&cos_select, D::Minus1)?,
        Tensor::cat(&sin_select, D::Minus1)?,
    ))
}
