use ndarray::Array4;

use super::*;
use crate::decode::{decode_logits, greedy_search};
use crate::preprocess::{EncodedInput, PromptInput, TokenizedInput};
use crate::session::validate_schema_names;
use crate::splitter::{DEFAULT_SPLITTER_REGEX, RegexSplitter, Splitter};
use crate::tensor::make_span_tensors;
use crate::tokenizer::Tokenizer;

struct FakeTokenizer;

impl Tokenizer for FakeTokenizer {
    fn encode(&self, input: &str) -> Result<Vec<u32>> {
        let tokens = match input {
            "<<ENT>>" => vec![128_002],
            "<<SEP>>" => vec![128_003],
            "movie character" => vec![1421, 1470],
            "vehicle" => vec![1508],
            "1a" => vec![16, 64],
            other => vec![stable_token_id(other)],
        };
        Ok(tokens)
    }
}

fn stable_token_id(input: &str) -> u32 {
    input
        .bytes()
        .fold(100u32, |accumulator, byte| accumulator.wrapping_add(u32::from(byte)))
}

fn fake_encoded(texts: &[&str], entities: &[&str]) -> EncodedInput {
    let input = TextInput::from_str(texts, entities).expect("valid input");
    let splitter = RegexSplitter::new(DEFAULT_SPLITTER_REGEX).expect("valid regex");
    let tokenized = TokenizedInput::from(input, &splitter, None).expect("tokenized");
    let prompt = PromptInput::from(tokenized);
    EncodedInput::from(prompt, &FakeTokenizer).expect("encoded")
}

#[test]
fn rejects_empty_input() {
    assert!(TextInput::new(Vec::new(), vec!["person".to_string()]).is_err());
    assert!(TextInput::new(vec!["Ada".to_string()], Vec::new()).is_err());
    assert!(TextInput::new(vec!["  ".to_string()], vec!["person".to_string()]).is_err());
    assert!(TextInput::new(vec!["Ada".to_string()], vec!["".to_string()]).is_err());
    assert!(TextInput::new(vec!["Ada".to_string()], vec!["<<SEP>>".to_string()]).is_err());
}

#[test]
fn rejects_invalid_parameters() {
    assert!(Parameters::default().with_threshold(-0.1).validate().is_err());
    assert!(Parameters::default().with_threshold(f32::NAN).validate().is_err());
    assert!(Parameters::default().with_max_width(0).validate().is_err());
    assert!(Parameters::default().with_max_length(Some(0)).validate().is_err());
    Parameters::default().validate().expect("default parameters");
}

#[test]
fn regex_splitter_handles_default_unicode_and_limit() {
    let splitter = RegexSplitter::new(DEFAULT_SPLITTER_REGEX).expect("valid regex");
    let tokens = splitter.split("This is an oh-yeah test", None).expect("split");
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[3].text(), "oh-yeah");
    assert_eq!(tokens[3].start(), 11);
    assert_eq!(tokens[3].end(), 18);

    let unicode = splitter
        .split("Word with accents: éàèèçîù foo bar", None)
        .expect("unicode split");
    assert_eq!(unicode.len(), 7);

    let limited = splitter.split("w1 w2 w3 w4 w5 w6 w7", Some(5)).expect("limited split");
    assert_eq!(limited.len(), 5);
    assert_eq!(limited[4].text(), "w5");
}

#[test]
fn prompt_builder_uses_gliner_marker_layout() {
    let input =
        TextInput::from_str(&["This is a text !", "This is a longer one."], &["Person", "Place"]).expect("valid input");
    let splitter = RegexSplitter::new(DEFAULT_SPLITTER_REGEX).expect("valid regex");
    let tokenized = TokenizedInput::from(input, &splitter, None).expect("tokenized");
    let prepared = PromptInput::from(tokenized);

    assert_eq!(prepared.prompts.len(), 2);
    assert_eq!(prepared.prompts[0].tokens.len(), 10);
    assert_eq!(prepared.prompts[1].tokens.len(), 11);
    assert_eq!(prepared.text_lengths, vec![5, 6]);
    assert_eq!(prepared.prompts[0].tokens[0], "<<ENT>>");
    assert_eq!(prepared.prompts[0].tokens[1], "Person");
    assert_eq!(prepared.prompts[0].tokens[2], "<<ENT>>");
    assert_eq!(prepared.prompts[0].tokens[3], "Place");
    assert_eq!(prepared.prompts[0].tokens[4], "<<SEP>>");
    assert_eq!(prepared.prompts[1].tokens[5], "This");
    assert_eq!(prepared.num_words, 6);
}

#[test]
fn encoding_builds_expected_masks_for_multi_token_words() {
    let encoded = fake_encoded(&["1a John Doe"], &["movie character", "vehicle"]);

    assert_eq!(encoded.input_ids.shape(), &[1, 12]);
    assert_eq!(
        encoded.input_ids.row(0).to_vec(),
        vec![1, 128_002, 1421, 1470, 128_002, 1508, 128_003, 16, 64, 499, 380, 2]
    );
    assert_eq!(
        encoded.attention_masks.row(0).to_vec(),
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    );
    assert_eq!(
        encoded.word_masks.row(0).to_vec(),
        vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 3, 0]
    );
    assert_eq!(encoded.text_lengths.row(0).to_vec(), vec![3]);
}

#[test]
fn span_tensors_include_valid_span_indices_and_masks() {
    let encoded = fake_encoded(&["My name is Bond"], &["person"]);
    let (span_idx, span_mask) = make_span_tensors(&encoded, 3).expect("span tensors");

    assert_eq!(span_idx.shape(), &[1, 12, 2]);
    assert_eq!(span_mask.shape(), &[1, 12]);
    assert_eq!(span_idx[[0, 0, 0]], 0);
    assert_eq!(span_idx[[0, 0, 1]], 0);
    assert!(span_mask[[0, 0]]);
    assert_eq!(span_idx[[0, 1, 0]], 0);
    assert_eq!(span_idx[[0, 1, 1]], 1);
    assert!(span_mask[[0, 1]]);
    assert_eq!(span_idx[[0, 9, 0]], 3);
    assert_eq!(span_idx[[0, 9, 1]], 3);
    assert!(span_mask[[0, 9]]);
    assert!(!span_mask[[0, 10]]);
    assert!(!span_mask[[0, 11]]);
}

#[test]
fn decode_logits_returns_spans_above_threshold() {
    let encoded = fake_encoded(&["Ada works at Xberg"], &["person", "organization"]);
    let context = EntityContext {
        texts: encoded.texts,
        tokens: encoded.tokens,
        entities: encoded.entities,
        num_words: encoded.num_words,
    };
    let mut logits = Array4::<f32>::from_elem((1, 4, 2, 2), -10.0);
    logits[[0, 0, 0, 0]] = 10.0;
    logits[[0, 3, 0, 1]] = 9.0;

    let output = decode_logits(logits.view().into_dyn(), context, 0.5, 2, true, false, false).expect("decoded");

    assert_eq!(output.spans[0].len(), 2);
    assert_eq!(output.spans[0][0].text(), "Ada");
    assert_eq!(output.spans[0][0].class(), "person");
    assert_eq!(output.spans[0][1].text(), "Xberg");
    assert_eq!(output.spans[0][1].class(), "organization");
}

#[test]
fn decode_logits_rejects_unexpected_shape() {
    let encoded = fake_encoded(&["Ada"], &["person"]);
    let context = EntityContext {
        texts: encoded.texts,
        tokens: encoded.tokens,
        entities: encoded.entities,
        num_words: encoded.num_words,
    };
    let logits = Array4::<f32>::zeros((1, 2, 2, 1));
    let error =
        decode_logits(logits.view().into_dyn(), context, 0.5, 2, true, false, false).expect_err("shape mismatch");

    assert!(matches!(error, GlinerError::UnexpectedLogitsShape { .. }));
}

#[test]
fn greedy_search_keeps_adjacent_spans_and_filters_overlaps() {
    let spans = vec![
        Span::new(0, 0, 3, "Ada".to_string(), "person".to_string(), 0.9).expect("span"),
        Span::new(0, 3, 8, "Xberg".to_string(), "organization".to_string(), 0.8).expect("span"),
        Span::new(0, 0, 8, "Ada Xberg".to_string(), "organization".to_string(), 0.7).expect("span"),
    ];

    let selected = greedy_search(&spans, true, false, false);
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].text(), "Ada");
    assert_eq!(selected[1].text(), "Xberg");
}

#[test]
fn validates_model_schema_names() {
    let inputs = INPUT_NAMES.map(str::to_string);
    let outputs = OUTPUT_NAMES.map(str::to_string);

    validate_schema_names("input", &INPUT_NAMES, &inputs).expect("valid inputs");
    validate_schema_names("output", &OUTPUT_NAMES, &outputs).expect("valid outputs");

    let error = validate_schema_names("input", &INPUT_NAMES, &["input_ids".to_string()]).expect_err("missing inputs");
    assert!(matches!(
        error,
        GlinerError::UnexpectedModelSchema { kind: "input", .. }
    ));

    let mut extra_inputs = inputs.to_vec();
    extra_inputs.push("extra_required_input".to_string());
    assert!(validate_schema_names("input", &INPUT_NAMES, &extra_inputs).is_err());
}
