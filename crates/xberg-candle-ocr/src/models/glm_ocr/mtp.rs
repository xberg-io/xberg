//! Token decoding loop for GLM-OCR.
//!
//! Consumes the assembled vision-prefix `input_embeds` and the GLM-4 decoder: prefills the KV
//! cache, then samples one token per forward pass — greedy or nucleus, with an optional
//! repetition penalty — until an EOS token or `max_new_tokens`. `generate_mrope` threads
//! explicit M-RoPE position ids through prefill and each decode step; `generate` uses plain
//! sequence-length offsets.
//!
//! Despite the `MtpConfig` name, no multi-token prediction happens — see `num_tokens_per_step`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MtpConfig {
    /// Intended number of tokens predicted per decoder forward pass. Currently inert: neither
    /// `generate` nor `generate_mrope` reads it, and decoding is always one token per pass.
    pub num_tokens_per_step: usize,
    /// Greedy when `false`; nucleus sampling when `true`.
    pub sample: bool,
    pub top_p: f32,
    pub temperature: f32,
    /// Score scaling applied to previously generated tokens before sampling (`>1.0` suppresses
    /// repetition, `<1.0` encourages it, `1.0` is a no-op).
    ///
    // ~keep: defaults to 1.0 because upstream's generation_config.json (zai-org/GLM-OCR, revision
    // ca5d8b3e287e52589e37c28385d9655ee4372f9d) carries no `repetition_penalty` key at all -- just
    // `do_sample: false` and two `eos_token_id`s. GH#1675: an earlier default of 1.1 here, combined
    // with `apply_repetition_penalty` scaling PER OCCURRENCE over the whole decode history
    // (`1.1^k` for a token seen k times), exponentially suppressed hex digits and `-` inside long
    // identifiers (UUIDs) until argmax drifted onto a token that had never been penalised.
    pub repetition_penalty: f32,
}

impl Default for MtpConfig {
    fn default() -> Self {
        Self {
            num_tokens_per_step: 4,
            sample: false,
            top_p: 0.9,
            temperature: 0.1,
            repetition_penalty: 1.0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::collections::BTreeSet;

    use candle_core::{Device, Tensor};

    use super::super::decoder::Glm4Decoder;
    use super::MtpConfig;
    use crate::CandleOcrError;
    use crate::error::Result;

    /// Trailing token history considered by the repetition penalty.
    ///
    // ~keep: bounding the window keeps a token from many steps ago (an earlier page's heading,
    // a table row already closed out) from being penalised for the rest of decoding -- GH#1675's
    // fix removes the per-occurrence exponential blowup, but an unbounded history would still
    // eventually penalise every token in the vocabulary that has appeared even once.
    pub(crate) const REPETITION_PENALTY_WINDOW: usize = 64;

    /// Trailing slice of `ids`, at most [`REPETITION_PENALTY_WINDOW`] tokens.
    pub(crate) fn repetition_penalty_window(ids: &[u32]) -> &[u32] {
        let start = ids.len().saturating_sub(REPETITION_PENALTY_WINDOW);
        &ids[start..]
    }

    /// Build a `(3, 1, 1)` M-RoPE position tensor where all three axes share the
    /// same scalar value `pos`. Used for per-step autoregressive decoding once
    /// the vision region has been consumed.
    fn make_text_step_positions(pos: u32, dev: &Device) -> Result<Tensor> {
        let buf = vec![pos, pos, pos];
        let tensor = Tensor::from_vec(buf, (3, 1, 1), dev)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Step position tensor: {}", e)))?;
        Ok(tensor)
    }

    /// Generation stop criteria and the M-RoPE position at which decoding resumes, bundled so
    /// [`generate_mrope`] stays under the workspace parameter-count limit. ~keep
    pub struct GenerationLimits<'a> {
        /// Position assigned to the first decoded token (== `max_position_in_prefill + 1`,
        /// computed by the engine).
        pub next_text_pos_start: u32,
        pub max_new_tokens: usize,
        pub eos_token_ids: &'a [u32],
    }

    /// Compute the next token from `logits`, applying the repetition penalty (if configured)
    /// over the trailing token window, then greedy or nucleus sampling. Split out of
    /// [`generate_mrope`] to keep that function under the workspace line-count limit. ~keep
    fn sample_next_token(
        logits: &Tensor,
        output_ids: &[u32],
        config: &MtpConfig,
        eos_token_ids: &[u32],
    ) -> Result<u32> {
        let last_logits = logits
            .squeeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?;

        let windowed_ids = repetition_penalty_window(output_ids);
        let penalized_logits = if config.repetition_penalty != 1.0 && !output_ids.is_empty() {
            apply_repetition_penalty(&last_logits, windowed_ids, config.repetition_penalty)
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

        if output_ids.len() < 5 && tracing::enabled!(tracing::Level::TRACE) {
            super::super::glm_debug_tensor(&format!("logits_step{}", output_ids.len()), &penalized_logits);
            tracing::trace!(
                "[glm-debug] step{}: token_id={} is_eos={}",
                output_ids.len(),
                token_id,
                eos_token_ids.contains(&token_id)
            );
        }

        Ok(token_id)
    }

    /// Run the decoding loop with explicit M-RoPE position_ids for the prefill
    /// pass and an incrementing `(t, h, w) = (next, next, next)` triple for
    /// each generated text token.
    ///
    /// `prefill_position_ids` must be shape `(3, 1, prefix_len)` — built by the
    /// engine to encode the vision-prefixed sequence's per-token positions.
    pub fn generate_mrope(
        decoder: &mut Glm4Decoder,
        input_embeds: &Tensor,
        prefill_position_ids: &Tensor,
        config: &MtpConfig,
        limits: GenerationLimits,
    ) -> Result<Vec<u32>> {
        let GenerationLimits {
            next_text_pos_start,
            max_new_tokens,
            eos_token_ids,
        } = limits;
        decoder.clear_kv_cache();
        let mut output_ids = Vec::new();

        let mut logits = decoder
            .forward_embeds(input_embeds, prefill_position_ids)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Prefill forward: {}", e)))?;
        super::super::glm_debug_tensor("prefill_logits", &logits);

        let mut next_text_pos = next_text_pos_start;
        let dev = input_embeds.device().clone();

        while output_ids.len() < max_new_tokens {
            let token_id = sample_next_token(&logits, &output_ids, config, eos_token_ids)?;

            output_ids.push(token_id);

            if eos_token_ids.contains(&token_id) {
                return Ok(output_ids);
            }

            if let Some(period) = crate::generation::stop_if_degenerate(&mut output_ids) {
                tracing::warn!(
                    period,
                    step = output_ids.len(),
                    "GLM-OCR: degenerate repetition, stopping"
                );
                break;
            }

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
    /// Note: `config.num_tokens_per_step` is never read. Every forward pass emits exactly one
    /// token regardless of the configured value (default 4), so throughput is that of plain
    /// autoregressive decoding, not multi-token prediction.
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

        let mut logits = decoder
            .forward_embeds_with_offset(input_embeds, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Prefill forward: {}", e)))?;

        let mut seqlen_offset = prefix_len;

        while output_ids.len() < max_new_tokens {
            let last_logits = logits
                .squeeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze batch: {}", e)))?;

            let windowed_ids = repetition_penalty_window(&output_ids);
            let penalized_logits = if config.repetition_penalty != 1.0 && !output_ids.is_empty() {
                apply_repetition_penalty(&last_logits, windowed_ids, config.repetition_penalty)
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

            if eos_token_ids.contains(&token_id) {
                return Ok(output_ids);
            }

            if let Some(period) = crate::generation::stop_if_degenerate(&mut output_ids) {
                tracing::warn!(
                    period,
                    step = output_ids.len(),
                    "GLM-OCR: degenerate repetition, stopping"
                );
                break;
            }

            let token_tensor = Tensor::new(&[token_id as i64], logits.device())
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Token tensor: {}", e)))?
                .unsqueeze(0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Add batch: {}", e)))?;

            let token_embeds = decoder
                .embed_tokens(&token_tensor)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Embed tokens: {}", e)))?;

            logits = decoder
                .forward_embeds_with_offset(&token_embeds, seqlen_offset)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Decode forward: {}", e)))?;

            seqlen_offset += 1;
        }

        Ok(output_ids)
    }

    /// Apply repetition penalty to logits: reduce scores for tokens already in `recent_ids`.
    /// Penalty > 1 suppresses repetition; < 1 encourages it.
    ///
    // ~keep: GH#1675 -- `recent_ids` is deduplicated (`BTreeSet`) so each token is penalised
    // exactly ONCE regardless of how many times it occurs in the window. The earlier
    // implementation scaled once per occurrence (`penalty.powi(k)` for a token seen k times),
    // which drove hex digits and `-` inside a UUID toward zero probability after a handful of
    // repeats and made argmax drift onto a token that had never been penalised. Runs entirely on
    // `logits`'s device via `index_select` / `where_cond` / `scatter` -- no per-step `to_vec1`
    // copy of the full vocabulary row to the host.
    pub(crate) fn apply_repetition_penalty(
        logits: &Tensor,
        recent_ids: &[u32],
        penalty: f32,
    ) -> candle_core::Result<Tensor> {
        let vocab_size = logits.dims()[0] as u32;
        let unique_ids: Vec<u32> = recent_ids
            .iter()
            .copied()
            .filter(|&id| id < vocab_size)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if unique_ids.is_empty() {
            return Ok(logits.clone());
        }

        let device = logits.device();
        let ids_len = unique_ids.len();
        let ids_tensor = Tensor::from_vec(unique_ids, (ids_len,), device)?;
        let selected = logits.index_select(&ids_tensor, 0)?;
        let divided = selected.affine(1.0 / penalty as f64, 0.0)?;
        let multiplied = selected.affine(penalty as f64, 0.0)?;
        let is_non_negative = selected.ge(0f32)?;
        let penalized_selected = is_non_negative.where_cond(&divided, &multiplied)?;
        logits.scatter(&ids_tensor, &penalized_selected, 0)
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
        if temperature <= 0.0 {
            return sample_greedy(logits);
        }

        let scaled = if (temperature - 1.0).abs() > 1e-5 {
            logits
                .affine(1.0 / temperature as f64, 0.0)
                .map_err(|e| CandleOcrError::InferenceFailed(format!("Scale temp: {}", e)))?
        } else {
            logits.clone()
        };

        let probs = candle_nn::ops::softmax(&scaled, 0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Softmax: {}", e)))?;

        let probs_vec = probs
            .squeeze(0)
            .map_err(|e| CandleOcrError::InferenceFailed(format!("Squeeze: {}", e)))?
            .to_vec1::<f32>()
            .map_err(|e| CandleOcrError::InferenceFailed(format!("To vec: {}", e)))?;

        let mut indexed: Vec<(usize, f32)> = probs_vec
            .iter()
            .enumerate()
            .filter(|&(_, &p)| p.is_finite())
            .map(|(i, &p)| (i, p))
            .collect();
        indexed.sort_by(|a, b| b.1.total_cmp(&a.1));

        let mut cumsum = 0.0;
        let mut valid_indices = Vec::new();
        for (idx, prob) in indexed {
            cumsum += prob;
            valid_indices.push((idx as u32, prob));
            if cumsum >= top_p {
                break;
            }
        }

        if valid_indices.is_empty() {
            return sample_greedy(logits);
        }

        let total_prob: f32 = valid_indices.iter().map(|(_, p)| p).sum();
        if total_prob <= 0.0 {
            return sample_greedy(logits);
        }

        use std::cell::RefCell;
        thread_local! {
            static RNG: RefCell<u64> = const { RefCell::new(0xDEADBEEF) };
        }

        RNG.with(|rng| {
            let mut state = rng.borrow_mut();
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
pub use imp::{GenerationLimits, generate, generate_mrope};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::imp::{REPETITION_PENALTY_WINDOW, apply_repetition_penalty, repetition_penalty_window, sample_nucleus};
    use crate::error::Result;
    use candle_core::{Device, Tensor};

    #[test]
    fn test_apply_repetition_penalty_reduces_both_signs() {
        let logits = vec![0.5f32, -0.3, 1.0, -0.8];
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(logits.clone(), (4,), &device).unwrap();

        let output_ids = vec![0, 1];
        let result = apply_repetition_penalty(&logits_tensor, &output_ids, 1.1).unwrap();
        let result_vec = result.to_vec1::<f32>().unwrap();

        assert!((result_vec[0] - 0.5 / 1.1).abs() < 0.01);
        assert!((result_vec[1] - (-0.3 * 1.1)).abs() < 0.01);
        assert!((result_vec[2] - 1.0).abs() < 0.01);
        assert!((result_vec[3] - (-0.8)).abs() < 0.01);
    }

    /// GH#1675: a token seen 3 times in the history must be penalised by a single factor of
    /// `penalty`, not `penalty^3` -- the exponential-per-occurrence blowup that drove hex digits
    /// in a UUID toward zero probability.
    #[test]
    fn repeated_token_is_penalised_once_not_per_occurrence() {
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(vec![0.5f32, 1.0], (2,), &device).unwrap();

        let output_ids = vec![0u32, 0, 0];
        let result = apply_repetition_penalty(&logits_tensor, &output_ids, 1.1).unwrap();
        let result_vec = result.to_vec1::<f32>().unwrap();

        assert!(
            (result_vec[0] - 0.5 / 1.1).abs() < 1e-4,
            "expected a single application of the penalty (0.5 / 1.1 = {}), got {}",
            0.5 / 1.1,
            result_vec[0]
        );
        let per_occurrence = 0.5 / 1.1f32.powi(3);
        assert!(
            (result_vec[0] - per_occurrence).abs() > 1e-3,
            "must not scale per occurrence (1.1^3 = {})",
            per_occurrence
        );
    }

    /// A token that fell outside the trailing [`REPETITION_PENALTY_WINDOW`] must not be
    /// penalised -- the window bounds how far back the penalty looks, so a token from many
    /// steps ago cannot keep suppressing itself for the rest of decoding.
    #[test]
    fn window_keeps_only_the_trailing_tokens() {
        let short: Vec<u32> = (0..10).collect();
        assert_eq!(repetition_penalty_window(&short), &short[..]);

        let long: Vec<u32> = (0..(REPETITION_PENALTY_WINDOW as u32 + 5)).collect();
        let window = repetition_penalty_window(&long);
        assert_eq!(window.len(), REPETITION_PENALTY_WINDOW);
        assert_eq!(window[0], 5);
        assert_eq!(
            window[REPETITION_PENALTY_WINDOW - 1],
            REPETITION_PENALTY_WINDOW as u32 + 4
        );

        assert_eq!(repetition_penalty_window(&[]), &[] as &[u32]);
    }

    #[test]
    fn tokens_outside_window_are_not_penalised() {
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(vec![0.5f32, 0.5], (2,), &device).unwrap();

        // Token 0 occurs once, far enough back that a window of REPETITION_PENALTY_WINDOW
        // tokens no longer contains it; token 1 fills the rest of the window.
        let mut output_ids = vec![0u32];
        output_ids.extend(std::iter::repeat_n(1u32, REPETITION_PENALTY_WINDOW));
        let start = output_ids.len() - REPETITION_PENALTY_WINDOW;
        let windowed = &output_ids[start..];
        assert!(
            !windowed.contains(&0),
            "test fixture must push token 0 out of the window"
        );

        let result = apply_repetition_penalty(&logits_tensor, windowed, 1.1).unwrap();
        let result_vec = result.to_vec1::<f32>().unwrap();

        assert!(
            (result_vec[0] - 0.5).abs() < 1e-6,
            "token outside the window must be left unpenalised, got {}",
            result_vec[0]
        );
        assert!(
            (result_vec[1] - 0.5 / 1.1).abs() < 1e-4,
            "token inside the window must still be penalised, got {}",
            result_vec[1]
        );
    }

    /// A penalty of 1.0 must be an exact identity -- the decode loops rely on this to skip
    /// calling the penalty at all when `config.repetition_penalty == 1.0`.
    #[test]
    fn penalty_of_one_is_identity() {
        let logits = vec![0.5f32, -0.3, 1.0, -0.8];
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(logits.clone(), (4,), &device).unwrap();

        let output_ids = vec![0u32, 1, 1, 2];
        let result = apply_repetition_penalty(&logits_tensor, &output_ids, 1.0).unwrap();
        let result_vec = result.to_vec1::<f32>().unwrap();

        assert_eq!(result_vec, logits);
    }

    /// GH#1675: upstream (`zai-org/GLM-OCR`, revision
    /// ca5d8b3e287e52589e37c28385d9655ee4372f9d) `generation_config.json` carries no
    /// `repetition_penalty` key -- fetched and confirmed to contain only `do_sample: false`
    /// and `eos_token_id: [59246, 59253]`. Our default must match that absence (1.0 = no-op).
    #[test]
    fn default_config_matches_upstream_generation_config() {
        let config = super::MtpConfig::default();
        assert_eq!(config.repetition_penalty, 1.0);
    }

    #[test]
    fn test_sample_nucleus_handles_nan() -> Result<()> {
        let logits = vec![0.5f32, f32::NAN, 1.0, -0.8];
        let device = Device::Cpu;
        let logits_tensor = Tensor::from_vec(logits, (4,), &device).unwrap();

        let result = sample_nucleus(&logits_tensor, 0.9, 1.0)?;
        assert!(result < 4);
        Ok(())
    }
}
