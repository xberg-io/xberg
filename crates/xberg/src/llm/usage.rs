//! Helpers for extracting LLM usage metadata from liter-llm responses.

use crate::types::LlmUsage;

/// Extract usage metadata from a chat completion response.
pub(crate) fn extract_usage_from_chat(response: &liter_llm::ChatCompletionResponse, source: &str) -> Option<LlmUsage> {
    Some(LlmUsage {
        model: response.model.clone(),
        source: source.to_string(),
        input_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
        total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
        estimated_cost: response.estimated_cost(),
        finish_reason: response
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|fr| format!("{fr:?}").to_lowercase()),
    })
}

/// Extract usage metadata from an embedding response.
// Matches `embed_via_llm`'s own gate (its only caller): `tokio-runtime` because
// `liter-llm` alone doesn't imply it (unlike `embeddings`), and `not(wasm32)`
// since wasm32 has no LLM-hosted embedding path yet.
#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(crate) fn extract_usage_from_embedding(response: &liter_llm::EmbeddingResponse, source: &str) -> Option<LlmUsage> {
    Some(LlmUsage {
        model: response.model.clone(),
        source: source.to_string(),
        input_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
        total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
        estimated_cost: response.estimated_cost(),
        finish_reason: None,
    })
}

/// Append an `LlmUsage` entry to an `ExtractedDocument`, lazily initializing the vec.
pub(crate) fn push_llm_usage(result: &mut crate::ExtractedDocument, usage: Option<LlmUsage>) {
    if let Some(u) = usage {
        result.llm_usage.get_or_insert_with(Vec::new).push(u);
    }
}
