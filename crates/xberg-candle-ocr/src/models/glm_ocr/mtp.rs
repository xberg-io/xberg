//! Multi-Token Prediction decoding loop for GLM-OCR.
//!
//! Phase 1 stub. GLM-OCR's decoder predicts multiple tokens per forward pass
//! to improve throughput. The loop consumes the assembled vision-prefix
//! `input_embeds` and the GLM-4 decoder, sampling tokens until EOS or
//! `max_new_tokens`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MtpConfig {
    /// Number of tokens predicted per decoder forward pass. `1` reduces to
    /// vanilla autoregressive decoding.
    pub num_tokens_per_step: usize,
    /// Greedy when `false`; nucleus sampling when `true`.
    pub sample: bool,
    pub top_p: f32,
    pub temperature: f32,
    pub repetition_penalty: f32,
}

impl Default for MtpConfig {
    fn default() -> Self {
        Self {
            num_tokens_per_step: 4,
            sample: false,
            top_p: 0.9,
            temperature: 0.1,
            repetition_penalty: 1.1,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use candle_core::{Device, Tensor};

    use super::super::decoder::Glm4Decoder;
    use super::MtpConfig;
    use crate::CandleOcrError;
    use crate::error::Result;

    /// Build a `(3, 1, 1)` M-RoPE position tensor where all three axes share the
    /// same scalar value `pos`. Used for per-step autoregressive decoding once
    /// the vision region has been consumed.
    fn make_text_step_positions(pos: u32, dev: &Device) -> Result<Tensor> {
        let buf = vec![pos, pos, pos];
        let tensor = Tensor::from_vec(buf, (3, 1, 1), dev)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Step position tensor: {}", e)))?;
        Ok(tensor)
    }

    /// Run the decoding loop with explicit M-RoPE position_ids for the prefill
    /// pass and an incrementing `(t, h, w) = (next, next, next)` triple for
    /// each generated text token.
    ///
    /// `prefill_position_ids` must be shape `(3, 1, prefix_len)` — built by the
    /// engine to encode the vision-prefixed sequence's per-token positions.
    /// `next_text_pos_start` is the position assigned to the first decoded
    /// token (== `max_position_in_prefill + 1`, computed by the engine).
    pub fn generate_mrope(
        decoder: &mut Glm4Decoder,
        input_embeds: &Tensor,
        prefill_position_ids: &Tensor,
        next_text_pos_start: u32,
        config: &MtpConfig,
        max_new_tokens: usize,
        eos_token_ids: &[u32],
    ) -> Result<Vec<u32>> {
        decoder.clear_kv_cache();
        let mut output_ids = Vec::new();

        // Prefill: forward the whole vision-prefixed sequence with proper
        // (3, 1, seq) M-RoPE positions.
        let mut logits = decoder
            .forward_embeds(input_embeds, prefill_position_ids)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Prefill forward: {}", e)))?;
        super::super::glm_debug_tensor("prefill_logits", &logits);

        let mut next_text_pos = next_text_pos_start;
        let dev = input_embeds.device().clone();

        while output_ids.len() < max_new_tokens {
            let last_logits = logits
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?;

            let penalized_logits = if config.repetition_penalty != 1.0 && !output_ids.is_empty() {
                apply_repetition_penalty(&last_logits, &output_ids, config.repetition_penalty)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Repetition penalty: {}", e)))?
            } else {
                last_logits.clone()
            };

            let token_id = if config.sample {
                sample_nucleus(&penalized_logits, config.top_p, config.temperature)
            } else {
                sample_greedy(&penalized_logits)
            }
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Sampling: {}", e)))?;

            if output_ids.len() < 5 && std::env::var_os("XBERG_GLM_DEBUG").is_some() {
                super::super::glm_debug_tensor(&format!("logits_step{}", output_ids.len()), &penalized_logits);
                eprintln!(
                    "[glm-debug] step{}: token_id={} is_eos={}",
                    output_ids.len(),
                    token_id,
                    eos_token_ids.contains(&token_id)
                );
            }

            output_ids.push(token_id);

            if eos_token_ids.contains(&token_id) {
                return Ok(output_ids);
            }

            // Embed the new token and forward with a (3, 1, 1) position tensor
            // (t = h = w = next_text_pos).
            let token_tensor = Tensor::new(&[token_id as i64], &dev)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add batch: {}", e)))?;

            let token_embeds = decoder
                .embed_tokens(&token_tensor)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Embed tokens: {}", e)))?;

            let step_positions = make_text_step_positions(next_text_pos, &dev)?;

            logits = decoder
                .forward_embeds(&token_embeds, &step_positions)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Decode forward: {}", e)))?;

            next_text_pos += 1;
        }

        Ok(output_ids)
    }

    /// Run the MTP decoding loop and return generated token IDs (excluding the prefix).
    ///
    /// Algorithm:
    /// 1. Prefill: forward `input_embeds` at seqlen_offset=0 to seed KV cache
    /// 2. Per-token: sample using greedy or nucleus sampling
    /// 3. For each token: embed and forward at the current seqlen_offset
    /// 4. Stop on EOS or `max_new_tokens`
    ///
    /// Note: num_tokens_per_step is configured but not yet used for multi-token
    /// prediction. Currently set to 1 for autoregressive decoding; future optimization.
    pub fn generate(
        decoder: &mut Glm4Decoder,
        input_embeds: &Tensor,
        config: &MtpConfig,
        max_new_tokens: usize,
        eos_token_ids: &[u32],
    ) -> Result<Vec<u32>> {
        decoder.clear_kv_cache();
        let mut output_ids = Vec::new();
        let prefix_len = input_embeds.dim(1)?;

        // Prefill: seed the KV cache with the vision-prefix embeddings at offset=0.
        let mut logits = decoder
            .forward_embeds_with_offset(input_embeds, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Prefill forward: {}", e)))?;

        let mut seqlen_offset = prefix_len;

        // Decoding loop: generate tokens until EOS or max_new_tokens.
        while output_ids.len() < max_new_tokens {
            // Get logits for the last position: logits is (B=1, vocab)
            let last_logits = logits
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?;

            // Apply repetition penalty before sampling
            let penalized_logits = if config.repetition_penalty != 1.0 && !output_ids.is_empty() {
                apply_repetition_penalty(&last_logits, &output_ids, config.repetition_penalty)
                    .map_err(|e| CandleOcrError::InferenceFailed(format!("Repetition penalty: {}", e)))?
            } else {
                last_logits.clone()
            };

            let token_id = if config.sample {
                sample_nucleus(&penalized_logits, config.top_p, config.temperature)
            } else {
                sample_greedy(&penalized_logits)
            }
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Sampling: {}", e)))?;

            output_ids.push(token_id);

            // Stop on any EOS token.
            if eos_token_ids.contains(&token_id) {
                return Ok(output_ids);
            }

            // Embed the generated token and forward for next prediction.
            let token_tensor = Tensor::new(&[token_id as i64], logits.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add batch: {}", e)))?;

            let token_embeds = decoder
                .embed_tokens(&token_tensor)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Embed tokens: {}", e)))?;

            // Forward with new embedding at current seqlen_offset (before incrementing)
            logits = decoder
                .forward_embeds_with_offset(&token_embeds, seqlen_offset)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Decode forward: {}", e)))?;

            seqlen_offset += 1;
        }

        Ok(output_ids)
    }

    /// Apply repetition penalty to logits: reduce scores for tokens already in output.
    /// Penalty > 1 suppresses repetition; < 1 encourages it.
    ///
    // HF canonical form: positive logits shrink toward 0 (divide), negative logits push further
    // negative (multiply). Both reduce the post-softmax probability.
    pub(crate) fn apply_repetition_penalty(
        logits: &Tensor,
        output_ids: &[u32],
        penalty: f32,
    ) -> candle_core::Result<Tensor> {
        let mut logits_vec = logits.to_vec1::<f32>()?;
        for &token_id in output_ids {
            let idx = token_id as usize;
            if idx < logits_vec.len() {
                if logits_vec[idx] >= 0.0 {
                    logits_vec[idx] /= penalty;
                } else {
                    logits_vec[idx] *= penalty;
                }
            }
        }
        Tensor::from_vec(logits_vec, logits.dims(), logits.device())
    }

    /// Greedy decoding: return argmax of logits.
    fn sample_greedy(logits: &Tensor) -> Result<u32> {
        let argmax = logits
            .argmax(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Argmax: {}", e)))?;
        let token_id = argmax
            .to_scalar::<u32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Scalar: {}", e)))?;
        Ok(token_id)
    }

    /// Nucleus (top-p) sampling with temperature scaling.
    pub(crate) fn sample_nucleus(logits: &Tensor, top_p: f32, temperature: f32) -> Result<u32> {
        // Guard against invalid temperature.
        if temperature <= 0.0 {
            return sample_greedy(logits);
        }

        // Temperature scaling.
        let scaled = if (temperature - 1.0).abs() > 1e-5 {
            logits
                .affine(1.0 / temperature as f64, 0.0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Scale temp: {}", e)))?
        } else {
            logits.clone()
        };

        // Softmax to get probabilities.
        let probs = candle_nn::ops::softmax(&scaled, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Softmax: {}", e)))?;

        let probs_vec = probs
            .squeeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?
            .to_vec1::<f32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec: {}", e)))?;

        // Sort by probability (descending) and find top-p cutoff.
        // Filter out NaN/inf values before sorting for numerical safety.
        let mut indexed: Vec<(usize, f32)> = probs_vec
            .iter()
            .enumerate()
            .filter(|&(_, &p)| p.is_finite())
            .map(|(i, &p)| (i, p))
            .collect();
        // NaN-safe sort: treat NaN as less than any number
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut cumsum = 0.0;
        let mut valid_indices = Vec::new();
        for (idx, prob) in indexed {
            cumsum += prob;
            valid_indices.push((idx as u32, prob));
            if cumsum >= top_p {
                break;
            }
        }

        // Renormalize and sample from valid tokens.
        if valid_indices.is_empty() {
            return sample_greedy(logits);
        }

        let total_prob: f32 = valid_indices.iter().map(|(_, p)| p).sum();
        if total_prob <= 0.0 {
            return sample_greedy(logits);
        }

        // Use a simple deterministic approach: sample by cumulative sum.
        // In Phase 1.b, we use seeded sampling for reproducibility.
        use std::cell::RefCell;
        thread_local! {
            static RNG: RefCell<u64> = const { RefCell::new(0xDEADBEEF) };
        }

        RNG.with(|rng| {
            let mut state = rng.borrow_mut();
            // Linear congruential generator for reproducibility.
            *state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let sample_val = (*state % 1_000_000) as f32 / 1_000_000.0 * total_prob;

            let mut cumsum = 0.0;
            for (idx, prob) in &valid_indices {
                cumsum += prob;
                if sample_val <= cumsum {
                    return Ok(*idx);
                }
            }
            valid_indices
                .last()
                .map(|(idx, _)| *idx)
                .ok_or_else(|| CandleOcrError::InferenceFailed("Empty valid indices".to_string()))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::{generate, generate_mrope};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::imp::{apply_repetition_penalty, sample_nucleus};
    use crate::error::Result;
    use candle_core::{Device, Tensor};

    #[test]
    fn test_apply_repetition_penalty_reduces_both_signs() {
        // Create a small logits tensor
        let logits = vec![0.5f32, -0.3, 1.0, -0.8];
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(logits.clone(), (4,), &device).unwrap();

        // Apply penalty of 1.1 to token 0 and 1
        let output_ids = vec![0, 1];
        let result = apply_repetition_penalty(&logits_tensor, &output_ids, 1.1).unwrap();
        let result_vec = result.to_vec1::<f32>().unwrap();

        // Token 0: 0.5 / 1.1 ≈ 0.454 (positive: shrinks toward 0)
        assert!((result_vec[0] - 0.5 / 1.1).abs() < 0.01);
        // Token 1: -0.3 * 1.1 = -0.33 (negative logits get more negative under penalty,
        // reducing post-softmax probability)
        assert!((result_vec[1] - (-0.3 * 1.1)).abs() < 0.01);
        // Token 2, 3: unchanged
        assert!((result_vec[2] - 1.0).abs() < 0.01);
        assert!((result_vec[3] - (-0.8)).abs() < 0.01);
    }

    #[test]
    fn test_sample_nucleus_handles_nan() -> Result<()> {
        // Create logits with a NaN and regular values
        let logits = vec![0.5f32, f32::NAN, 1.0, -0.8];
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(logits, (4,), &device).unwrap();

        // Should not panic and return a valid token ID
        let result = sample_nucleus(&logits_tensor, 0.9, 1.0)?;
        assert!(result < 4);
        Ok(())
    }
}
