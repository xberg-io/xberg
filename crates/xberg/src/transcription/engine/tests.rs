use super::*;

#[test]
fn whisper_engine_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<WhisperEngine>();
}

#[test]
fn greedy_argmax_last_picks_highest() {
    let data = Array3::from_shape_vec((1, 2, 4), vec![0.1_f32, 0.9, 0.2, 0.0, 0.1, 0.2, 0.3, 0.8]).unwrap();
    let val: Value = Value::from_array(data).unwrap().into();
    let tok = greedy_argmax_last(&val).unwrap();
    assert_eq!(tok, 3, "expected argmax at index 3 (last position)");
}

#[test]
fn greedy_argmax_last_single_position() {
    let data = Array3::from_shape_vec((1, 1, 5), vec![0.0_f32, 0.0, 100.0, 0.0, 0.0]).unwrap();
    let val: Value = Value::from_array(data).unwrap().into();
    let tok = greedy_argmax_last(&val).unwrap();
    assert_eq!(tok, 2);
}

#[test]
fn greedy_argmax_last_preserves_last_tie_and_rank_errors() {
    let tied = Array3::from_shape_vec((1, 1, 4), vec![0.0_f32, 2.0, 2.0, 1.0]).unwrap();
    let logits: Value = Value::from_array(tied).unwrap().into();
    assert_eq!(greedy_argmax_last(&logits).unwrap(), 2);

    let rank_two = Array2::from_shape_vec((1, 4), vec![0.0_f32; 4]).unwrap();
    let logits: Value = Value::from_array(rank_two).unwrap().into();
    let error = greedy_argmax_last(&logits).unwrap_err();
    assert_eq!(
        error.to_string(),
        "tensor shape error: Expected logits tensor rank 3, got rank 2"
    );
}

#[test]
fn cloned_decoder_state_preserves_tensor_shape_and_values() {
    let values = vec![1.0_f32, -2.0, 3.0, 4.0];
    let tensor = Array3::from_shape_vec((1, 2, 2), values.clone()).unwrap();
    let original: Value = Value::from_array(tensor).unwrap().into();
    let cloned = clone_value_f32(&original).unwrap();

    let observed = cloned.try_extract_array::<f32>().unwrap();
    assert_eq!(observed.shape(), &[1, 2, 2]);
    assert_eq!(observed.iter().copied().collect::<Vec<_>>(), values);
    assert_eq!(original.try_extract_array::<f32>().unwrap(), observed);
}

#[test]
fn special_tokens_resolve_from_tiny_vocab() {
    use crate::core::config::transcription::WhisperModel;
    use crate::transcription::model::ensure_whisper_model;

    let paths = match ensure_whisper_model(WhisperModel::Tiny, None, false, false) {
        Ok(p) => p,
        Err(_) => return,
    };

    let tokenizer = Tokenizer::from_file(&paths.tokenizer).expect("tokenizer load");
    let st = SpecialTokens::resolve(&tokenizer).expect("special token resolution");

    assert!(st.end_of_text > 0, "end_of_text token should be a valid non-zero ID");
    assert!(
        st.language_ids.contains_key("en"),
        "English language token must be present"
    );
}

/// The decoder's position-embedding table has [`WHISPER_MAX_TOKENS`] rows,
/// so the prompt must be subtracted from the budget. Regression for
/// GH#1944: an unsubtracted budget let a repetition loop feed a token at
/// position 448, slicing an empty position range and crashing Reshape.
#[test]
fn decoder_token_budget_reserves_prompt_positions() {
    assert_eq!(decoder_token_budget(4), WHISPER_MAX_TOKENS - 4);
    assert_eq!(decoder_token_budget(WHISPER_MAX_TOKENS), 0);
    assert!(decoder_token_budget(WHISPER_MAX_TOKENS + 1) == 0, "never underflows");
    assert!(4 + decoder_token_budget(4) <= WHISPER_MAX_TOKENS);
}

/// One undecodable window must not discard the others (GH#1944).
#[test]
fn a_failing_window_is_skipped_and_the_rest_are_kept() {
    let samples = vec![0.0_f32; WHISPER_CHUNK_SAMPLES * 2 + 1];
    let ms_per_sample = 1000.0 / 16_000.0;
    let mut calls = 0usize;

    let (segments, warnings, first_error, any_success) =
        transcribe_chunks_with_isolation(&samples, ms_per_sample, |_chunk, _duration| {
            let index = calls;
            calls += 1;
            if index == 1 {
                Err(TranscriptionError::Shape("decoder rejected window 1".into()))
            } else {
                Ok(vec![(0, 30_000, format!("chunk-{index}"))])
            }
        });

    assert!(any_success, "chunks 0 and 2 succeeded");
    assert_eq!(calls, 3, "expected three 30-second windows");
    assert_eq!(warnings.len(), 1, "exactly the middle window must warn");
    assert_eq!(warnings[0].start_ms, 30_000);
    assert_eq!(warnings[0].end_ms, 60_000);
    assert!(warnings[0].message.contains("window 1"));
    assert!(first_error.is_some());
    let texts: Vec<&str> = segments.iter().map(|(_, _, text)| text.as_str()).collect();
    assert_eq!(texts, vec!["chunk-0", "chunk-2"]);
    assert_eq!(segments[1].0, 60_000, "third chunk keeps its absolute offset");
}

/// A systemic decoder failure must still surface as an error rather than an
/// empty transcript.
#[test]
fn every_window_failing_reports_the_first_error() {
    let samples = vec![0.0_f32; WHISPER_CHUNK_SAMPLES + 1];
    let ms_per_sample = 1000.0 / 16_000.0;

    let (segments, warnings, first_error, any_success) =
        transcribe_chunks_with_isolation(&samples, ms_per_sample, |_chunk, _duration| {
            Err(TranscriptionError::Shape("boom".into()))
        });

    assert!(!any_success, "no window succeeded");
    assert!(segments.is_empty(), "no partial text to return");
    assert_eq!(warnings.len(), 2, "both windows must warn");
    assert!(first_error.is_some(), "the caller must be able to fail the whole call");
}
