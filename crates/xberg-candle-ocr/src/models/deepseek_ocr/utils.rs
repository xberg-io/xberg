//! Utility functions for DeepSeek-OCR model.
//!
//! Vendored from jhqxxx/aha (Apache-2.0). See repo-root ATTRIBUTIONS.md § jhqxxx/aha.

use candle_core::{DType, Device, IndexOp, Tensor};

use crate::error::{CandleOcrError, Result};

/// Linear 1D interpolation for tensors.
pub fn interpolate_linear_1d(input: &Tensor, target_size: usize, _align_corner: Option<bool>) -> Result<Tensor> {
    let src_size = input.dim(2)?;
    if src_size == target_size {
        return Ok(input.clone());
    }
    let ratio = (src_size - 1) as f32 / (target_size - 1).max(1) as f32;
    let mut output = Vec::new();
    for i in 0..target_size {
        let src_i = i as f32 * ratio;
        let src_i_floor = src_i.floor() as usize;
        let src_i_ceil = (src_i_floor + 1).min(src_size - 1);
        let weight = src_i - src_i_floor as f32;
        let val_floor = input.i((.., .., src_i_floor))?;
        let val_ceil = input.i((.., .., src_i_ceil))?;
        let val = val_floor
            .mul(&Tensor::new(&[1.0 - weight], input.device())?)?
            .add(&val_ceil.mul(&Tensor::new(&[weight], input.device())?)?)?;
        output.push(val);
    }
    Tensor::cat(&output, 2).map_err(|e| CandleOcrError::InferenceFailed(format!("cat failed: {e}")))
}

/// Bicubic interpolation for spatial tensors.
pub fn interpolate_bicubic(
    input: &Tensor,
    target_size: (usize, usize),
    _align_corner: Option<bool>,
    _half_pixel: Option<bool>,
) -> Result<Tensor> {
    // Simplified bicubic: use bilinear fallback
    // Full bicubic interpolation is complex; use linear approximation
    let (_, _, h, w) = input.dims4()?;
    let (target_h, target_w) = target_size;

    if h == target_h && w == target_w {
        return Ok(input.clone());
    }

    // Simple nearest-neighbor resize for now (can be improved)
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
    let mut result = Vec::with_capacity(ih * iw);
    for i in 0..ih {
        for j in 0..iw {
            let idx = (index.i((i, j))?.to_scalar::<u32>()? as usize).min(num - 1);
            result.push(t.i(idx)?);
        }
    }
    Tensor::stack(&result, 0)
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
    let mut mask = Tensor::ones((seq_len, seq_len), DType::U32, device)?;
    for i in 0..seq_len {
        for j in (i + 1)..seq_len {
            mask = mask.slice_assign(&[(i..i + 1), (j..j + 1)], &Tensor::zeros((1, 1), DType::U32, device)?)?;
        }
    }
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
/// dimension are dropped and Tensor::stack sees mixed ranks.
pub fn masked_scatter_dim0(dst: &Tensor, src: &Tensor, mask: &Tensor) -> Result<Tensor> {
    let batched = dst.rank() == 3;
    let dst = if batched { dst.squeeze(0)? } else { dst.clone() };
    let mut output_rows = Vec::new();
    let mask_vec = mask.flatten_all()?.to_vec1::<u32>()?;
    let mut src_idx = 0;
    for (i, &m) in mask_vec.iter().enumerate() {
        if m != 0 && src_idx < src.dim(0)? {
            let src_row = src.i(src_idx)?;
            output_rows.push(src_row);
            src_idx += 1;
        } else if m == 0 && i < dst.dim(0)? {
            let dst_row = dst.i(i)?;
            output_rows.push(dst_row);
        }
    }
    if output_rows.is_empty() {
        return Ok(if batched { dst.unsqueeze(0)? } else { dst });
    }
    let out = Tensor::stack(&output_rows, 0)
        .map_err(|e| CandleOcrError::InferenceFailed(format!("stack failed: {e}")))?;
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

    for b in 0..batch_size {
        let row = flattened.i(b)?;
        let mut items: Vec<(usize, f32)> = Vec::new();
        for i in 0..num_items {
            let val = row.i(i)?.to_scalar::<f32>()?;
            items.push((i, val));
        }
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

    #[test]
    fn topk_returns_indices_then_weights() {
        let dev = Device::Cpu;
        let input = Tensor::new(&[[0.1f32, 0.7, 0.2]], &dev).expect("input");
        let (indices, weights) = topk(&input, 2).expect("topk");
        assert_eq!(indices.to_vec2::<u32>().expect("idx"), vec![vec![1, 2]]);
        assert_eq!(weights.to_vec2::<f32>().expect("w"), vec![vec![0.7, 0.2]]);
    }
}
