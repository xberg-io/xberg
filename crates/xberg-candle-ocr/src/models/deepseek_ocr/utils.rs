//! Utility functions for DeepSeek-OCR model.
//!
//! Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

use candle_core::{DType, Device, IndexOp, Tensor};

use crate::error::{CandleOcrError, Result};

/// Linear 1D interpolation along the last axis of a `(batch, channels, length)` tensor, with
/// PyTorch's `align_corners=False` (half-pixel) sample placement — the mode the reference
/// SAM `get_rel_pos` uses when it resizes the relative-position table to the tile size.
///
/// A thin adapter over candle's bilinear resize: it takes a rank-4 tensor, so the length axis is
/// carried as the width of a one-row image.
pub fn interpolate_linear_1d(input: &Tensor, target_size: usize, _align_corner: Option<bool>) -> Result<Tensor> {
    let src_size = input.dim(2)?;
    if src_size == target_size {
        return Ok(input.clone());
    }
    // ~keep: candle owns this. `upsample_bilinear2d` with `align_corners=false` is PyTorch's
    // half-pixel sampling, and over a height of one it degenerates to `F.interpolate(mode=
    // "linear")` exactly: the height weight is always 1.0 on row 0. Sampling the axis a position
    // at a time in Rust cost `target_size` narrows, affines and a `cat` of `target_size` tensors,
    // per resize, per attention layer, per crop, and it is the second defect this hand-rolled
    // resize produced after the scalar-`mul` shape mismatch of GH#1701 and #1702 (GH#1719).
    input
        .unsqueeze(2)?
        .upsample_bilinear2d(1, target_size, false)
        .and_then(|resized| resized.squeeze(2))
        .map_err(|e| CandleOcrError::InferenceFailed(format!("interpolate_linear_1d: {e}")))
}

/// Bicubic interpolation for spatial tensors.
pub fn interpolate_bicubic(
    input: &Tensor,
    target_size: (usize, usize),
    _align_corner: Option<bool>,
    _half_pixel: Option<bool>,
) -> Result<Tensor> {
    let (_, _, h, w) = input.dims4()?;
    let (target_h, target_w) = target_size;

    if h == target_h && w == target_w {
        return Ok(input.clone());
    }

    let h_ratio = (h - 1) as f32 / (target_h - 1).max(1) as f32;
    let w_ratio = (w - 1) as f32 / (target_w - 1).max(1) as f32;

    let mut rows = Vec::new();
    for i in 0..target_h {
        let src_i = (i as f32 * h_ratio).round() as usize;
        let src_i = src_i.min(h - 1);
        let row = input.i((.., .., src_i, ..))?;

        let mut cols = Vec::new();
        for j in 0..target_w {
            let src_j = (j as f32 * w_ratio).round() as usize;
            let src_j = src_j.min(w - 1);
            let col = row.i((.., .., src_j))?;
            cols.push(col);
        }
        let resized_row = Tensor::stack(&cols, 2)?;
        rows.push(resized_row);
    }
    Tensor::stack(&rows, 2).map_err(|e| CandleOcrError::InferenceFailed(format!("stack failed: {e}")))
}

/// Advanced 2D gather: `t` is a `[num, dim]` lookup table and `index` is an
/// `[ih, iw]` grid of row indices; returns `t[index]` shaped `[ih, iw, dim]`.
///
/// This is the SAM decomposed relative-position lookup (`rel_pos[coords]`): the
/// caller passes the rank-2 `[2*max-1, head_dim]` rel-pos table, not a rank-3
/// tensor, so gather rows and restore the `[ih, iw]` grid on the result.
pub fn index_select_2d(t: &Tensor, index: &Tensor) -> Result<Tensor> {
    let (num, dim) = t.dims2()?;
    if num == 0 {
        return Err(CandleOcrError::InferenceFailed(
            "index_select_2d: lookup table has no rows".to_string(),
        ));
    }
    let (ih, iw) = index.dims2()?;
    // ~keep: one device-to-host transfer for the whole index grid, then one on-device gather.
    // Reading the grid a scalar at a time stalled the SAM attention on ih * iw round trips per
    // rel-pos lookup, per attention layer, per crop (GH#1714). to_vec2 returns logical row-major
    // order for a strided layout as well as a contiguous one, so the flattened order still
    // matches the [i][j] grid the scalar loop walked.
    let rows: Vec<u32> = index
        .to_vec2::<u32>()?
        .into_iter()
        .flatten()
        .map(|idx| (idx as usize).min(num - 1) as u32)
        .collect();
    let flat = Tensor::from_vec(rows, (ih * iw,), t.device())?;
    t.index_select(&flat, 0)
        .and_then(|r| r.reshape((ih, iw, dim)))
        .map_err(|e| CandleOcrError::InferenceFailed(format!("index_select_2d: {e}")))
}

/// One-hot encoding.
pub fn onehot(indices: &Tensor, num_classes: usize) -> Result<Tensor> {
    let shape = indices.shape().dims();
    let mut total: usize = 1;
    for &d in shape {
        total *= d;
    }
    let indices_flat = indices.reshape((total,))?;
    let indices_vec = indices_flat.to_vec1::<u32>()?;
    let device = indices.device();
    let mut output = vec![vec![0u8; num_classes]; total];
    for (i, &idx) in indices_vec.iter().enumerate() {
        if (idx as usize) < num_classes {
            output[i][idx as usize] = 1;
        }
    }
    let flat: Vec<u8> = output.into_iter().flatten().collect();
    let mut new_shape = shape.to_vec();
    new_shape.push(num_classes);
    Tensor::new(flat.as_slice(), device)?
        .reshape(new_shape.as_slice())
        .map_err(|e| CandleOcrError::InferenceFailed(format!("reshape failed: {e}")))
}

/// Find non-zero indices.
pub fn nonzero(t: &Tensor) -> Result<(Vec<usize>, Vec<usize>)> {
    let (_rows, _cols) = t.dims2()?;
    let t_vec = t.to_vec2::<u32>()?;
    let mut row_indices = Vec::new();
    let mut col_indices = Vec::new();
    for (i, row) in t_vec.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if val != 0 {
                row_indices.push(i);
                col_indices.push(j);
            }
        }
    }
    Ok((row_indices, col_indices))
}

/// Prepare causal attention mask.
pub fn prepare_causal_attention_mask(bs: usize, seq_len: usize, _offset: usize, device: &Device) -> Result<Tensor> {
    // ~keep: the caller (`eager_attention_forward`) broadcast-adds this mask straight onto the
    // raw attention logits before softmax, so it must already be additive (0 visible / -inf
    // blocked). A 0/1 keep-mask cast to the logits' dtype and added would only ever nudge scores
    // by 1.0 instead of hard-blocking future positions -- see GH#1701.
    let mut data = vec![0f32; seq_len * seq_len];
    for i in 0..seq_len {
        for j in (i + 1)..seq_len {
            data[i * seq_len + j] = f32::NEG_INFINITY;
        }
    }
    let mask = Tensor::from_vec(data, (seq_len, seq_len), device)?
        .unsqueeze(0)?
        .unsqueeze(0)?;
    Ok(mask.expand((bs, 1, seq_len, seq_len))?)
}

/// Masked fill for attention.
pub fn attn_masked_fill(on_true: &Tensor, mask: &Tensor, on_false: f32) -> Result<Tensor> {
    let (_mask_rows, _mask_cols) = mask.dims2()?;
    let on_false_tensor = Tensor::new(&[on_false], on_true.device())?;
    let mask_expanded = mask.unsqueeze(0)?.unsqueeze(0)?;
    let _mask_bool = mask_expanded.broadcast_as(on_true.shape())?.to_dtype(DType::U8)?;
    let result = on_true.where_cond(&mask_expanded, &on_false_tensor)?;
    Ok(result)
}

/// Masked scatter on dimension 0.
///
/// Replaces the rows of `dst` where `mask` is set with successive rows of `src`.
/// `dst` arrives batched as `[1, seq, hidden]` from the language embeddings, so the
/// batch dim is dropped for the row scatter and restored afterwards; `mask` is
/// flattened to a rank-1 `[seq]`. Without this the sequence rows past the batch
/// dimension are dropped and the row gather sees mixed ranks.
pub fn masked_scatter_dim0(dst: &Tensor, src: &Tensor, mask: &Tensor) -> Result<Tensor> {
    let batched = dst.rank() == 3;
    let dst = if batched { dst.squeeze(0)? } else { dst.clone() };
    let dst_row_count = dst.dim(0)?;
    let src_row_count = src.dim(0)?;
    let mask_vec = mask.flatten_all()?.to_vec1::<u32>()?;
    // ~keep: the row schedule follows from the mask alone, and the mask already arrives in one
    // transfer, so the schedule is built on the host and spent as two gathers and one select.
    // Narrowing a row at a time and stacking the result cost one narrow per sequence position and
    // a `stack` of thousands of tensors, once per page during prefill (GH#1719). A masked position
    // past the end of `src`, and an unmasked position past the end of `dst`, still contribute no
    // row at all, which is what the per-row loop did and what the shorter output records.
    let mut src_positions: Vec<u32> = Vec::with_capacity(mask_vec.len());
    let mut dst_positions: Vec<u32> = Vec::with_capacity(mask_vec.len());
    let mut takes_src: Vec<u8> = Vec::with_capacity(mask_vec.len());
    let mut next_src = 0u32;
    for (i, &m) in mask_vec.iter().enumerate() {
        if m != 0 && (next_src as usize) < src_row_count {
            src_positions.push(next_src);
            dst_positions.push(0);
            takes_src.push(1);
            next_src += 1;
        } else if m == 0 && i < dst_row_count {
            src_positions.push(0);
            dst_positions.push(i as u32);
            takes_src.push(0);
        }
    }
    let out_row_count = takes_src.len();
    let any_from_src = takes_src.contains(&1);
    let any_from_dst = takes_src.contains(&0);
    let dst = if dst.is_contiguous() { dst } else { dst.contiguous()? };
    let src = if src.is_contiguous() {
        src.clone()
    } else {
        src.contiguous()?
    };
    let from_src = if any_from_src {
        let index = Tensor::from_vec(src_positions, (out_row_count,), src.device())?;
        Some(src.index_select(&index, 0)?)
    } else {
        None
    };
    let from_dst = if any_from_dst {
        let index = Tensor::from_vec(dst_positions, (out_row_count,), dst.device())?;
        Some(dst.index_select(&index, 0)?)
    } else {
        None
    };
    let out = match (from_src, from_dst) {
        (Some(from_src), Some(from_dst)) => {
            let select = Tensor::from_vec(takes_src, (out_row_count, 1), dst.device())?
                .broadcast_as(from_dst.shape())?
                .contiguous()?;
            select
                .where_cond(&from_src, &from_dst)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("masked_scatter_dim0: {e}")))?
        }
        (Some(rows), None) | (None, Some(rows)) => rows,
        (None, None) => return Ok(if batched { dst.unsqueeze(0)? } else { dst }),
    };
    if batched { Ok(out.unsqueeze(0)?) } else { Ok(out) }
}

/// Top-k selection.
///
/// Returns `(indices, weights)` in that order: U32 positions of the k largest
/// values per row, then the F32 values themselves.
pub fn topk(input: &Tensor, k: usize) -> Result<(Tensor, Tensor)> {
    let shape = input.shape().dims();
    let flattened = input.reshape((shape[0], ()))?;
    let (batch_size, num_items) = flattened.dims2()?;

    let mut top_weights_vec = vec![vec![0.0f32; k]; batch_size];
    let mut top_indices_vec = vec![vec![0u32; k]; batch_size];

    // ~keep: one device-to-host transfer for the whole score matrix. Reading it a scalar at a
    // time stalled the decode loop on `num_items` round trips per row per MoE layer per token,
    // which is what left the GPU idle for most of a page (GH#1711).
    let scores = flattened.to_vec2::<f32>()?;

    for (b, row) in scores.iter().enumerate().take(batch_size) {
        let mut items: Vec<(usize, f32)> = row.iter().copied().enumerate().take(num_items).collect();
        items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for (idx, (item_idx, val)) in items.iter().take(k).enumerate() {
            top_weights_vec[b][idx] = *val;
            top_indices_vec[b][idx] = *item_idx as u32;
        }
    }

    let top_weights_flat: Vec<f32> = top_weights_vec.into_iter().flatten().collect();
    let top_indices_flat: Vec<u32> = top_indices_vec.into_iter().flatten().collect();

    let weights = Tensor::new(top_weights_flat.as_slice(), input.device())?.reshape((batch_size, k))?;
    let indices = Tensor::new(top_indices_flat.as_slice(), input.device())?.reshape((batch_size, k))?;

    Ok((indices, weights))
}

#[cfg(test)]
mod tests {
    use candle_core::Device;

    use super::*;

    /// The shape the SAM encoder hits on a 640 px local crop: a (1, 64, 127) rel-pos table
    /// resized to 79 positions. The old implementation failed here with a candle shape
    /// mismatch instead of interpolating (GH#1701).
    #[test]
    fn interpolate_linear_1d_resizes_a_rel_pos_table_to_the_crop_length() {
        let dev = Device::Cpu;
        let input = Tensor::arange(0f32, 64.0 * 127.0, &dev)
            .and_then(|t| t.reshape((1, 64, 127)))
            .expect("input");

        let resized = interpolate_linear_1d(&input, 79, None).expect("interpolate");

        assert_eq!(resized.dims(), &[1, 64, 79]);
    }

    /// Values follow PyTorch `F.interpolate(mode="linear")` with `align_corners=False`:
    /// `[0, 1, 2, 3]` upsampled to 8 samples is `[0, .25, .75, 1.25, 1.75, 2.25, 2.75, 3]`.
    #[test]
    fn interpolate_linear_1d_matches_half_pixel_linear_sampling() {
        let dev = Device::Cpu;
        let input = Tensor::new(&[[[0f32, 1.0, 2.0, 3.0]]], &dev).expect("input");

        let resized = interpolate_linear_1d(&input, 8, None).expect("interpolate");

        let values = resized.flatten_all().and_then(|t| t.to_vec1::<f32>()).expect("read");
        assert_eq!(values, vec![0.0, 0.25, 0.75, 1.25, 1.75, 2.25, 2.75, 3.0]);
    }

    /// `get_rel_pos` transposes the table before it resizes it, so the resize must read a strided
    /// tensor as correctly as a packed one. The per-position narrow did; candle's resize reads
    /// through the layout's strides, which this pins.
    #[test]
    fn interpolate_linear_1d_accepts_a_strided_input() {
        let dev = Device::Cpu;
        let strided = Tensor::new(&[[[0f32, 1.0], [2.0, 3.0], [4.0, 5.0], [6.0, 7.0]]], &dev)
            .and_then(|t| t.transpose(1, 2))
            .expect("strided input");
        assert!(
            !strided.is_contiguous(),
            "input must be strided for this test to have power"
        );

        let resized = interpolate_linear_1d(&strided, 8, None).expect("interpolate");

        assert_eq!(resized.dims(), &[1, 2, 8]);
        let values = resized
            .flatten_all()
            .and_then(|t| t.to_vec1::<f32>())
            .expect("read")
            .into_iter()
            .take(8)
            .collect::<Vec<_>>();
        assert_eq!(values, vec![0.0, 0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.0]);
    }

    #[test]
    fn prepare_causal_attention_mask_hides_future_positions() {
        let dev = Device::Cpu;
        let mask = prepare_causal_attention_mask(1, 4, 0, &dev).expect("mask");
        assert_eq!(mask.dims(), &[1, 1, 4, 4], "mask should be (batch, 1, seq, seq)");

        let rows = mask
            .reshape((4, 4))
            .and_then(|t| t.to_vec2::<f32>())
            .expect("read mask");
        for (i, row) in rows.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                if j > i {
                    assert!(value == f32::NEG_INFINITY, "future position ({i},{j}) should be masked");
                } else {
                    assert_eq!(value, 0.0, "visible position ({i},{j}) should be unmasked");
                }
            }
        }
    }

    #[test]
    fn prepare_causal_attention_mask_blocks_future_logits_after_broadcast_add() {
        // ~keep Regression for GH#1701: the mask must be additive so that adding it to raw
        // attention logits drives future positions to -inf before softmax, not just
        // nudge visible positions by +1 relative to future ones.
        let dev = Device::Cpu;
        let mask = prepare_causal_attention_mask(1, 3, 0, &dev).expect("mask");
        let logits = Tensor::new(&[[[[5.0f32, 5.0, 5.0], [5.0, 5.0, 5.0], [5.0, 5.0, 5.0]]]], &dev).expect("logits");
        let combined = logits.broadcast_add(&mask).expect("add mask");
        let row0 = combined
            .reshape((3, 3))
            .and_then(|t| t.to_vec2::<f32>())
            .expect("read combined");
        assert_eq!(row0[0][0], 5.0, "current position keeps its logit");
        assert!(
            row0[0][1].is_infinite() && row0[0][1].is_sign_negative(),
            "future position must become -inf, not stay finite"
        );
        assert!(
            row0[0][2].is_infinite() && row0[0][2].is_sign_negative(),
            "future position must become -inf, not stay finite"
        );
    }

    #[test]
    fn index_select_2d_gathers_rows_into_grid() {
        let dev = Device::Cpu;
        let table = Tensor::new(&[[0f32, 1.], [10., 11.], [20., 21.]], &dev).expect("table");
        let index = Tensor::new(&[[2u32, 0], [1, 2]], &dev).expect("index");
        let out = index_select_2d(&table, &index).expect("gather");
        assert_eq!(out.dims(), &[2, 2, 2]);
        let rows = out.reshape((4, 2)).and_then(|t| t.to_vec2::<f32>()).expect("read");
        assert_eq!(rows, vec![vec![20., 21.], vec![0., 1.], vec![10., 11.], vec![20., 21.]]);
    }

    #[test]
    fn index_select_2d_rejects_empty_table() {
        let dev = Device::Cpu;
        let table = Tensor::zeros((0usize, 4usize), DType::F32, &dev).expect("empty table");
        let index = Tensor::new(&[[0u32]], &dev).expect("index");
        assert!(index_select_2d(&table, &index).is_err());
    }

    #[test]
    fn masked_scatter_dim0_replaces_masked_rows_and_keeps_batch_dim() {
        let dev = Device::Cpu;
        let dst = Tensor::new(&[[[0f32, 0.], [1., 1.], [2., 2.], [3., 3.]]], &dev).expect("dst");
        let src = Tensor::new(&[[10f32, 10.], [20., 20.]], &dev).expect("src");
        let mask = Tensor::new(&[[0u32, 1, 1, 0]], &dev).expect("mask");
        let out = masked_scatter_dim0(&dst, &src, &mask).expect("scatter");
        assert_eq!(out.dims(), &[1, 4, 2]);
        let rows = out.reshape((4, 2)).and_then(|t| t.to_vec2::<f32>()).expect("read");
        assert_eq!(rows, vec![vec![0., 0.], vec![10., 10.], vec![20., 20.], vec![3., 3.]]);
    }

    /// A mask that sets more rows than `src` has, and reaches past the end of `dst`, drops the
    /// positions it cannot fill. The gathers must reproduce that shorter output, not pad it.
    #[test]
    fn masked_scatter_dim0_drops_positions_neither_tensor_can_fill() {
        let dev = Device::Cpu;
        let dst = Tensor::new(&[[0f32, 0.], [1., 1.]], &dev).expect("dst");
        let src = Tensor::new(&[[10f32, 10.]], &dev).expect("src");
        let mask = Tensor::new(&[1u32, 0, 1, 0], &dev).expect("mask");

        let out = masked_scatter_dim0(&dst, &src, &mask).expect("scatter");

        assert_eq!(out.dims(), &[2, 2], "the two unfillable positions contribute no row");
        let rows = out.to_vec2::<f32>().expect("read");
        assert_eq!(rows, vec![vec![10., 10.], vec![1., 1.]]);
    }

    /// Every position is masked, so no row comes from `dst` at all and there is nothing to select
    /// between. The result is the gathered `src` rows in mask order.
    #[test]
    fn masked_scatter_dim0_takes_every_row_from_src_when_the_mask_is_full() {
        let dev = Device::Cpu;
        let dst = Tensor::new(&[[[0f32, 0.], [1., 1.]]], &dev).expect("dst");
        let src = Tensor::new(&[[10f32, 10.], [20., 20.]], &dev).expect("src");
        let mask = Tensor::new(&[[1u32, 1]], &dev).expect("mask");

        let out = masked_scatter_dim0(&dst, &src, &mask).expect("scatter");

        assert_eq!(out.dims(), &[1, 2, 2]);
        let rows = out
            .reshape((2usize, 2usize))
            .and_then(|t| t.to_vec2::<f32>())
            .expect("read");
        assert_eq!(rows, vec![vec![10., 10.], vec![20., 20.]]);
    }

    #[test]
    fn topk_returns_indices_then_weights() {
        let dev = Device::Cpu;
        let input = Tensor::new(&[[0.1f32, 0.7, 0.2]], &dev).expect("input");
        let (indices, weights) = topk(&input, 2).expect("topk");
        assert_eq!(indices.to_vec2::<u32>().expect("idx"), vec![vec![1, 2]]);
        assert_eq!(weights.to_vec2::<f32>().expect("w"), vec![vec![0.7, 0.2]]);
    }

    /// The per-position sampler this crate shipped before GH#1719, kept as the reference the
    /// candle-backed resize is measured against: `src = max((i + 0.5) * scale - 0.5, 0)`, floor
    /// and ceil clamped to the table, and a linear blend in the tensor's own dtype.
    fn reference_interpolate_linear_1d(input: &Tensor, target_size: usize) -> Tensor {
        const HALF_PIXEL: f64 = 0.5;
        let src_size = input.dim(2).expect("rank 3");
        let scale = src_size as f64 / target_size as f64;
        let mut output = Vec::with_capacity(target_size);
        for i in 0..target_size {
            let src_i = ((i as f64 + HALF_PIXEL) * scale - HALF_PIXEL).max(0.0);
            let src_i_floor = (src_i.floor() as usize).min(src_size - 1);
            let src_i_ceil = (src_i_floor + 1).min(src_size - 1);
            let weight = src_i - src_i_floor as f64;
            let val_floor = input.i((.., .., src_i_floor)).and_then(|t| t.affine(1.0 - weight, 0.0));
            let val_ceil = input.i((.., .., src_i_ceil)).and_then(|t| t.affine(weight, 0.0));
            let blended = val_floor
                .and_then(|floor| floor.add(&val_ceil.expect("ceil")))
                .and_then(|t| t.unsqueeze(2))
                .expect("blend");
            output.push(blended);
        }
        Tensor::cat(&output, 2).expect("cat")
    }

    /// Deterministic values in `[-1, 1]` so an absolute tolerance is meaningful.
    fn pseudo_random_tensor(shape: (usize, usize, usize), seed: u64, dev: &Device) -> Tensor {
        const LCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
        const LCG_INCREMENT: u64 = 1_442_695_040_888_963_407;
        let count = shape.0 * shape.1 * shape.2;
        let mut state = seed;
        let values: Vec<f32> = (0..count)
            .map(|_| {
                state = state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
                ((state >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
            })
            .collect();
        Tensor::from_vec(values, shape, dev).expect("tensor")
    }

    /// candle's resize accumulates in f64 and rounds once where the deleted loop blended in f32,
    /// so the two agree to a few f32 ulps, not bit-for-bit. The cases are the real model shapes:
    /// the (1, 64, 127) pretrained table onto a 640 px crop's 79 positions and back, the windowed
    /// 27-position table, and the transposed (strided) layout `get_rel_pos` hands over.
    #[test]
    fn interpolate_linear_1d_matches_the_per_position_reference_within_tolerance() {
        const TOLERANCE: f32 = 1e-6;
        let dev = Device::Cpu;
        let cases: [(usize, usize, usize, usize); 6] =
            [(1, 64, 127, 79), (1, 64, 79, 127), (1, 64, 27, 127), (2, 3, 5, 12), (1, 64, 127, 2), (1, 1, 2, 9)];
        for &(batch, channels, src_len, target) in &cases {
            for strided in [false, true] {
                let input = if strided {
                    pseudo_random_tensor((batch, src_len, channels), 7, &dev)
                        .transpose(1, 2)
                        .expect("transpose")
                } else {
                    pseudo_random_tensor((batch, channels, src_len), 7, &dev)
                };
                if strided && channels > 1 {
                    assert!(!input.is_contiguous(), "input must be strided for this case to have power");
                }

                let expected = reference_interpolate_linear_1d(&input, target);
                let actual = interpolate_linear_1d(&input, target, None).expect("interpolate");

                assert_eq!(actual.dims(), expected.dims(), "shape for {batch}x{channels}x{src_len}->{target}");
                let expected = expected.flatten_all().and_then(|t| t.to_vec1::<f32>()).expect("read");
                let actual = actual.flatten_all().and_then(|t| t.to_vec1::<f32>()).expect("read");
                let worst = expected
                    .iter()
                    .zip(&actual)
                    .map(|(e, a)| (e - a).abs())
                    .fold(0f32, f32::max);
                assert!(
                    worst <= TOLERANCE,
                    "{batch}x{channels}x{src_len}->{target} strided={strided}: worst |diff| {worst} > {TOLERANCE}"
                );
            }
        }
    }

    /// The row-at-a-time scatter this crate shipped before GH#1719, kept as the reference: a set
    /// mask position takes the next `src` row while any remain, a clear one takes its own `dst`
    /// row while one exists, and every other position contributes nothing.
    fn reference_masked_scatter_dim0(dst: &Tensor, src: &Tensor, mask: &[u32]) -> Tensor {
        let mut output_rows = Vec::new();
        let mut src_idx = 0;
        for (i, &m) in mask.iter().enumerate() {
            if m != 0 && src_idx < src.dim(0).expect("src rows") {
                output_rows.push(src.i(src_idx).expect("src row"));
                src_idx += 1;
            } else if m == 0 && i < dst.dim(0).expect("dst rows") {
                output_rows.push(dst.i(i).expect("dst row"));
            }
        }
        Tensor::stack(&output_rows, 0).expect("stack")
    }

    /// The gathers select rows, they never blend them, so the result must be bit-identical to the
    /// row-at-a-time reference on every mask shape prefill produces: image tokens in one block,
    /// split across blocks, at the sequence's edges, and a mask longer than `src` can serve.
    #[test]
    fn masked_scatter_dim0_is_bit_identical_to_the_row_at_a_time_reference() {
        let dev = Device::Cpu;
        let hidden = 3;
        let dst = pseudo_random_tensor((1, 9, hidden), 11, &dev);
        let src = pseudo_random_tensor((1, 4, hidden), 13, &dev).squeeze(0).expect("src");
        let masks: [Vec<u32>; 5] = [
            vec![0, 1, 1, 1, 1, 0, 0, 0, 0],
            vec![1, 1, 0, 0, 0, 0, 0, 1, 1],
            vec![1, 1, 1, 1, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 1, 1, 1, 1],
            vec![1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
        ];
        for mask in &masks {
            let mask_tensor = Tensor::from_vec(mask.clone(), (1, mask.len()), &dev).expect("mask");
            let expected = reference_masked_scatter_dim0(&dst.squeeze(0).expect("dst"), &src, mask);
            let actual = masked_scatter_dim0(&dst, &src, &mask_tensor).expect("scatter");
            assert_eq!(actual.dims(), &[1, expected.dim(0).expect("rows"), hidden], "shape for {mask:?}");
            let expected = expected.to_vec2::<f32>().expect("read");
            let actual = actual.squeeze(0).and_then(|t| t.to_vec2::<f32>()).expect("read");
            assert_eq!(actual, expected, "rows for {mask:?}");
        }
    }
}
