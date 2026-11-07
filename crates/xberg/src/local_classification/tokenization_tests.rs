use super::tests::configuration;
fn encode_pair(
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
    label: &str,
    config: &super::LocalClassificationConfig,
) -> crate::Result<tokenizers::Encoding> {
    let verifier = super::tokenization::source_tokenizer(tokenizer);
    super::tokenization::encode_pair(tokenizer, &verifier, text, label, config)
}

pub(super) fn tokenizer() -> tokenizers::Tokenizer {
    tokenizers::Tokenizer::from_bytes(
        br#"{
      "version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,
      "pre_tokenizer":{"type":"Whitespace"},"decoder":null,
      "post_processor":{"type":"BertProcessing","sep":["[SEP]",3],"cls":["[CLS]",2]},
      "model":{"type":"WordLevel","vocab":{"[UNK]":0,"[PAD]":1,"[CLS]":2,"[SEP]":3,
      "alpha":4,"beta":5,"topic":6,"science":7,"\u79d1\u5b66":8},"unk_token":"[UNK]"}
    }"#,
    )
    .unwrap()
}

#[test]
fn complete_premise_and_hypothesis_include_pair_special_tokens() {
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    config.limits.max_tokens = 7;
    let encoding = encode_pair(&tokenizer(), "alpha beta", "science", &config).unwrap();
    assert_eq!(encoding.get_ids(), &[2, 4, 5, 3, 6, 7, 3]);
    assert_eq!(encoding.get_type_ids(), &[0, 0, 0, 0, 1, 1, 1]);
}

#[test]
fn the_complete_pair_is_rejected_when_special_tokens_exceed_the_limit() {
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    config.limits.max_tokens = 6;
    assert!(encode_pair(&tokenizer(), "alpha beta", "science", &config).is_err());
}

#[test]
fn configured_longest_first_truncation_cannot_discard_hypothesis_tokens() {
    let mut tokenizer = tokenizer();
    tokenizer
        .with_truncation(Some(tokenizers::TruncationParams {
            max_length: 5,
            ..Default::default()
        }))
        .unwrap();
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    assert!(encode_pair(&tokenizer, "alpha beta", "science", &config).is_err());
}

#[test]
fn a_multibyte_label_stays_in_the_hypothesis_sequence() {
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    let encoding = encode_pair(&tokenizer(), "alpha", "科学", &config).unwrap();
    assert_eq!(
        encoding.get_tokens(),
        &["[CLS]", "alpha", "[SEP]", "topic", "科学", "[SEP]"]
    );
    assert_eq!(encoding.get_type_ids(), &[0, 0, 0, 1, 1, 1]);
}

fn serialized_pair_template(pair: serde_json::Value) -> tokenizers::Tokenizer {
    let mut serialized: serde_json::Value = serde_json::from_str(&tokenizer().to_string(false).unwrap()).unwrap();
    serialized["post_processor"] = serde_json::json!({
        "type": "TemplateProcessing",
        "single": [{"Sequence": {"id": "A", "type_id": 0}}],
        "pair": pair,
        "special_tokens": {},
    });
    tokenizers::Tokenizer::from_bytes(serde_json::to_vec(&serialized).unwrap()).unwrap()
}

fn pair_config() -> super::LocalClassificationConfig {
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    config
}

#[test]
fn a_serialized_a_only_pair_cannot_discard_the_complete_hypothesis() {
    let tokenizer = serialized_pair_template(serde_json::json!([
        {"Sequence": {"id": "A", "type_id": 1}},
    ]));
    let raw = tokenizer.encode(("alpha beta", "topic science"), true).unwrap();
    assert_eq!(raw.get_ids(), &[4, 5]);
    assert_eq!(raw.get_type_ids(), &[1, 1]);
    assert_eq!(raw.get_sequence_ids(), vec![Some(0), Some(0)]);
    let error = encode_pair(&tokenizer, "alpha beta", "science", &pair_config()).unwrap_err();
    assert!(error.to_string().contains("sequence"), "{error}");
}

#[test]
fn a_serialized_b_only_pair_cannot_discard_the_complete_premise() {
    let tokenizer = serialized_pair_template(serde_json::json!([
        {"Sequence": {"id": "B", "type_id": 0}},
    ]));
    let raw = tokenizer.encode(("alpha beta", "topic science"), true).unwrap();
    assert_eq!(raw.get_ids(), &[6, 7]);
    assert_eq!(raw.get_type_ids(), &[0, 0]);
    assert_eq!(raw.get_sequence_ids(), vec![Some(0), Some(0)]);
    let error = encode_pair(&tokenizer, "alpha beta", "science", &pair_config()).unwrap_err();
    assert!(error.to_string().contains("sequence"), "{error}");
}

#[test]
fn a_complete_serialized_pair_can_use_zero_type_ids_for_both_sequences() {
    let tokenizer = serialized_pair_template(serde_json::json!([
        {"Sequence": {"id": "A", "type_id": 0}},
        {"Sequence": {"id": "B", "type_id": 0}},
    ]));
    let encoding = encode_pair(&tokenizer, "alpha beta", "science", &pair_config()).unwrap();
    assert_eq!(encoding.get_ids(), &[4, 5, 6, 7]);
    assert_eq!(encoding.get_type_ids(), &[0, 0, 0, 0]);
    assert_eq!(encoding.get_sequence_ids(), vec![Some(0), Some(0), Some(1), Some(1)]);
    assert_eq!(encoding.get_special_tokens_mask(), &[0, 0, 0, 0]);
    assert_eq!(encoding.get_offsets(), &[(0, 5), (6, 10), (0, 5), (6, 13)]);
}

#[test]
fn a_serialized_pair_cannot_duplicate_non_special_source_tokens() {
    let tokenizer = serialized_pair_template(serde_json::json!([
        {"Sequence": {"id": "A", "type_id": 0}},
        {"Sequence": {"id": "B", "type_id": 1}},
        {"Sequence": {"id": "B", "type_id": 1}},
    ]));
    let raw = tokenizer.encode(("alpha beta", "topic science"), true).unwrap();
    assert_eq!(raw.get_ids(), &[4, 5, 6, 7, 6, 7]);
    assert_eq!(raw.get_special_tokens_mask(), &[0, 0, 0, 0, 0, 0]);
    let error = encode_pair(&tokenizer, "alpha beta", "science", &pair_config()).unwrap_err();
    assert!(error.to_string().contains("sequence"), "{error}");
}

#[test]
fn reused_source_verifier_preserves_pair_encoding_for_multiple_labels() {
    let tokenizer = tokenizer();
    let verifier = super::tokenization::source_tokenizer(&tokenizer);
    let config = pair_config();
    for (label, expected) in [("science", [2, 4, 5, 3, 6, 7, 3]), ("科学", [2, 4, 5, 3, 6, 8, 3])] {
        let repeated = super::tokenization::encode_pair(&tokenizer, &verifier, "alpha beta", label, &config).unwrap();
        let baseline = tokenizer
            .encode(("alpha beta", format!("topic {label}").as_str()), true)
            .unwrap();
        assert_eq!(repeated.get_ids(), &expected);
        assert_eq!(repeated.get_ids(), baseline.get_ids());
        assert_eq!(repeated.get_tokens(), baseline.get_tokens());
        assert_eq!(repeated.get_sequence_ids(), baseline.get_sequence_ids());
        assert_eq!(repeated.get_offsets(), baseline.get_offsets());
        assert_eq!(repeated.get_type_ids(), baseline.get_type_ids());
    }
}
