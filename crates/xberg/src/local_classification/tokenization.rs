use super::LocalClassificationConfig;
use super::policy::invalid;

pub(super) fn encode_pair(
    tokenizer: &tokenizers::Tokenizer,
    verifier: &tokenizers::Tokenizer,
    text: &str,
    label: &str,
    config: &LocalClassificationConfig,
) -> crate::Result<tokenizers::Encoding> {
    if tokenizer.get_truncation().is_some() {
        return Err(invalid("Local classification requires an untruncated tokenizer"));
    }
    let hypothesis = config.hypothesis_template.replace("{label}", label);
    let encoding = tokenizer
        .encode((text, hypothesis.as_str()), true)
        .map_err(|error| invalid(format!("Local classification tokenization failed: {error}")))?;
    if encoding.len() > config.limits.max_tokens as usize {
        return Err(invalid("Complete premise and hypothesis exceed the token limit"));
    }
    validate_source_coverage(verifier, text, &hypothesis, &encoding)?;
    Ok(encoding)
}

pub(super) fn source_tokenizer(tokenizer: &tokenizers::Tokenizer) -> tokenizers::Tokenizer {
    let mut verifier = tokenizer.clone();
    verifier.with_post_processor(None::<tokenizers::PostProcessorWrapper>);
    verifier.with_padding(None);
    verifier
}

fn source_tokens(encoding: &tokenizers::Encoding) -> Vec<(u32, String)> {
    encoding
        .get_ids()
        .iter()
        .copied()
        .zip(encoding.get_tokens().iter().cloned())
        .collect()
}

fn validate_source_coverage(
    verifier: &tokenizers::Tokenizer,
    text: &str,
    hypothesis: &str,
    encoding: &tokenizers::Encoding,
) -> crate::Result<()> {
    let mut actual = [Vec::new(), Vec::new()];
    let sequences = encoding.get_sequence_ids();
    if sequences.len() != encoding.len() {
        return Err(invalid("Local NLI sequence coverage has an invalid length"));
    }
    for (index, sequence) in sequences.iter().enumerate() {
        if encoding.get_special_tokens_mask()[index] != 0 {
            continue;
        }
        let sequence = sequence
            .filter(|sequence| *sequence < actual.len())
            .ok_or_else(|| invalid("Local NLI sequence contains unassigned non-special source tokens"))?;
        // RoBERTa legitimately trims offsets during postprocessing; model-visible
        // token IDs and token spellings must still match each complete source. ~keep
        actual[sequence].push((encoding.get_ids()[index], encoding.get_tokens()[index].clone()));
    }
    for (index, source) in [text, hypothesis].into_iter().enumerate() {
        let expected = verifier
            .encode(source, false)
            .map_err(|error| invalid(format!("Local NLI source sequence tokenization: {error}")))?;
        if expected.is_empty() || actual[index] != source_tokens(&expected) {
            return Err(invalid(
                "Local NLI sequence must preserve every premise and hypothesis token exactly once",
            ));
        }
    }
    Ok(())
}
