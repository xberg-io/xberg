//! Tokenizer wrapper resolving GLM-OCR special tokens and assembling the
//! input prompt that matches the upstream chat template.
//!
//! GLM-OCR is a chat model. The processor template (cached at
//! `/tmp/glm-ocr-audit/chat_template.jinja`) renders OCR requests as:
//!
//! ```text
//! [gMASK]<sop><|user|>
//! <|begin_of_image|><|image|><|end_of_image|>Text Recognition:<|assistant|>
//! ```
//!
//! The single `<|image|>` in the template is expanded to `num_image_tokens`
//! placeholders before the language model is invoked. We pre-expand here so the
//! engine can splice in the vision-projected embeddings at fixed positions.

/// Special-token IDs cached at engine construction. EOS, `<|begin_of_image|>`,
/// `<|end_of_image|>`, and `<|image|>` come from upstream `config.json` (with
/// runtime resolution against the tokenizer as a sanity check); `<|user|>` and
/// `<|assistant|>` are resolved from the tokenizer.
#[derive(Debug, Clone)]
pub struct SpecialTokens {
    /// Primary EOS token (`<|endoftext|>` = 59246).
    pub eos: u32,
    /// All EOS token IDs. The upstream `config.json` declares
    /// `"eos_token_id": [59246, 59253]` where 59246 is `<|endoftext|>` and
    /// 59253 is `<|user|>`. Generation stops on any token in this list.
    pub eos_token_ids: Vec<u32>,
    /// `<|begin_of_image|>` = 59256.
    pub image_start: u32,
    /// `<|end_of_image|>` = 59257.
    pub image_end: u32,
    /// `<|image|>` placeholder = 59280. Repeated `num_image_tokens` times in
    /// the input sequence and replaced by vision-projected embeddings via
    /// engine-side splicing.
    pub image_token: u32,
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use tokenizers::Tokenizer;

    use super::SpecialTokens;
    use crate::CandleOcrError;
    use crate::error::Result;

    /// Resolve the special-token IDs used by GLM-OCR. Hardcoded fallback IDs
    /// match the upstream `config.json` so the engine still works if the
    /// tokenizer's reverse-lookup fails (e.g. a fine-tune renames tokens).
    ///
    /// The upstream `config.json` declares `"eos_token_id": [59246, 59253]`:
    /// - 59246 = `<|endoftext|>` — natural end-of-generation
    /// - 59253 = `<|user|>` — turn boundary; generation must not cross it
    ///
    /// Both are collected into `eos_token_ids` so the generation loop stops on
    /// either without running on to produce trailing repetition artefacts.
    pub fn resolve_special_tokens(tokenizer: &Tokenizer) -> Result<SpecialTokens> {
        let eos = tokenizer
            .token_to_id("<|endoftext|>")
            .ok_or_else(|| CandleOcrError::Tokenizer("<|endoftext|> not in vocab".to_string()))?;
        let image_start = tokenizer
            .token_to_id("<|begin_of_image|>")
            .ok_or_else(|| CandleOcrError::Tokenizer("<|begin_of_image|> not in vocab".to_string()))?;
        let image_end = tokenizer
            .token_to_id("<|end_of_image|>")
            .ok_or_else(|| CandleOcrError::Tokenizer("<|end_of_image|> not in vocab".to_string()))?;
        let image_token = tokenizer
            .token_to_id("<|image|>")
            .ok_or_else(|| CandleOcrError::Tokenizer("<|image|> not in vocab".to_string()))?;

        // Collect all EOS tokens. Primary is <|endoftext|>; the secondary is
        // <|user|> (= 59253 in upstream vocab), which marks a turn boundary the
        // decoder must never cross during generation.
        let mut eos_token_ids = vec![eos];
        if let Some(user_token) = tokenizer.token_to_id("<|user|>") {
            if !eos_token_ids.contains(&user_token) {
                eos_token_ids.push(user_token);
            }
        } else {
            // Hardcoded fallback matching upstream config.json eos_token_id list.
            let fallback_user_token: u32 = 59253;
            if !eos_token_ids.contains(&fallback_user_token) {
                eos_token_ids.push(fallback_user_token);
            }
        }

        Ok(SpecialTokens {
            eos,
            eos_token_ids,
            image_start,
            image_end,
            image_token,
        })
    }

    /// Build the input token sequence following the upstream chat template.
    ///
    /// Returns `(token_ids, image_tokens_start)` where `image_tokens_start` is
    /// the index of the first `<|image|>` placeholder; the engine replaces the
    /// `num_image_tokens` placeholders starting there with vision embeddings.
    ///
    /// The trailing `<|assistant|>\n` is the generation cue — the model
    /// continues from there. No EOS is appended; that would tell the model
    /// generation is already complete.
    pub fn build_input_ids(
        special: &SpecialTokens,
        tokenizer: &Tokenizer,
        task_prompt: &str,
        num_image_tokens: usize,
    ) -> Result<(Vec<u32>, usize)> {
        let prompt_string = format!(
            "[gMASK]<sop><|user|>\n<|begin_of_image|>{placeholders}<|end_of_image|>{task}<|assistant|>\n",
            placeholders = "<|image|>".repeat(num_image_tokens),
            task = task_prompt,
        );

        let encoding = tokenizer
            .encode(prompt_string, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Encode prompt: {}", e)))?;

        let ids: Vec<u32> = encoding.get_ids().to_vec();

        let image_tokens_start = ids
            .iter()
            .position(|&id| id == special.image_token)
            .ok_or_else(|| CandleOcrError::Tokenizer("No <|image|> placeholder in encoded prompt".to_string()))?;

        // Defensive: ensure the encoder produced exactly num_image_tokens consecutive placeholders.
        let observed = ids[image_tokens_start..]
            .iter()
            .take_while(|&&id| id == special.image_token)
            .count();
        if observed != num_image_tokens {
            return Err(CandleOcrError::Tokenizer(format!(
                "Expected {} <|image|> placeholders, encoded prompt has {}",
                num_image_tokens, observed
            )));
        }

        Ok((ids, image_tokens_start))
    }

    /// Decode generated token IDs and strip residual special markers.
    pub fn decode_output(tokenizer: &Tokenizer, ids: &[u32]) -> Result<String> {
        let text = tokenizer
            .decode(ids, false)
            .map_err(|e| CandleOcrError::Tokenizer(format!("Decode error: {}", e)))?;

        let cleaned = text
            .replace("<|endoftext|>", "")
            .replace("<|user|>", "")
            .replace("<|assistant|>", "")
            .replace("<|begin_of_image|>", "")
            .replace("<|end_of_image|>", "")
            .replace("<|image|>", "")
            .replace("[gMASK]", "")
            .replace("<sop>", "")
            .trim()
            .to_string();

        Ok(cleaned)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::{build_input_ids, decode_output, resolve_special_tokens};
