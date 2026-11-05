//! Serde and construction contracts for additive per-input layout options.

use serde_json::{Value, json};
use xberg::{ExtractInput, ExtractInputWithOptions, FileExtractionConfig};

fn input() -> ExtractInput {
    ExtractInput::from_bytes(b"fixture".to_vec(), "text/plain", None)
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn batch_json_accepts_repeated_inputs_and_rejects_non_boolean_options() {
    let config = xberg::ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let item = json!({"input": input(), "use_layout_for_markdown": false});
    let batch = json!([item.clone(), item]).to_string();
    let result = xberg::extract_batch_with_input_options_json(&batch, &config)
        .await
        .unwrap();
    assert!(result.errors.is_empty());
    assert_eq!(result.results.len(), 2);
    assert_eq!(result.results[0].content.trim(), "fixture");
    assert_eq!(result.results[1].content.trim(), "fixture");

    let malformed = json!([{"input": input(), "use_layout_for_markdown": "false"}]).to_string();
    let error = xberg::extract_batch_with_input_options_json(&malformed, &config)
        .await
        .unwrap_err();
    assert!(matches!(error, xberg::XbergError::Validation { .. }));
    assert!(error.to_string().contains("boolean"), "{error}");
}

#[test]
fn constructor_and_default_keep_layout_unspecified() {
    let item = ExtractInputWithOptions::new(input());
    assert!(item.use_layout_for_markdown.is_none());
    assert_eq!(
        serde_json::to_value(&item.input).unwrap(),
        serde_json::to_value(input()).unwrap()
    );
    assert!(ExtractInputWithOptions::default().use_layout_for_markdown.is_none());
}

fn round_trip(setting: Option<bool>) {
    let mut item = ExtractInputWithOptions::new(input());
    item.use_layout_for_markdown = setting;
    let encoded = serde_json::to_value(&item).unwrap();
    let expected = match setting {
        Some(value) => json!({"input":input(),"use_layout_for_markdown":value}),
        None => json!({"input":input()}),
    };
    assert_eq!(encoded, expected);
    assert!(encoded.get("_private").is_none());
    let decoded: ExtractInputWithOptions = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.use_layout_for_markdown, setting);
    assert_eq!(
        serde_json::to_value(decoded.input).unwrap(),
        serde_json::to_value(input()).unwrap()
    );
}

#[test]
fn omitted_layout_round_trip_preserves_omission() {
    round_trip(None);
}

#[test]
fn explicit_false_round_trip_preserves_false() {
    round_trip(Some(false));
}

#[test]
fn explicit_true_round_trip_preserves_true() {
    round_trip(Some(true));
}

#[test]
fn null_layout_is_unspecified_and_canonicalizes_to_omission() {
    let decoded: ExtractInputWithOptions =
        serde_json::from_value(json!({"input":input(),"use_layout_for_markdown":null})).unwrap();
    assert!(decoded.use_layout_for_markdown.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), json!({"input":input()}));
}

#[test]
fn input_is_required_even_though_rust_default_exists() {
    for value in [
        json!({}),
        json!({"use_layout_for_markdown":true}),
        json!({"input":null}),
    ] {
        assert!(serde_json::from_value::<ExtractInputWithOptions>(value).is_err());
    }
}

#[test]
fn malformed_booleans_are_rejected_without_coercion() {
    let invalid = [json!("false"), json!(0), json!(1), json!([]), json!({})];
    assert_eq!(invalid.len(), 5);
    for setting in invalid {
        let error = serde_json::from_value::<ExtractInputWithOptions>(
            json!({"input":input(),"use_layout_for_markdown":setting}),
        )
        .unwrap_err();
        assert!(error.to_string().contains("boolean"), "{error}");
    }
}

#[test]
fn new_wrapper_rejects_unknown_and_duplicate_option_keys() {
    let error = serde_json::from_value::<ExtractInputWithOptions>(json!({"input":input(),"unknown":true})).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
    let source = format!(
        "{{\"input\":{},\"use_layout_for_markdown\":true,\"use_layout_for_markdown\":false}}",
        serde_json::to_string(&input()).unwrap()
    );
    let error = serde_json::from_str::<ExtractInputWithOptions>(&source).unwrap_err();
    assert!(error.to_string().contains("duplicate field"));
}

#[test]
fn legacy_file_overrides_remain_strict_and_unchanged() {
    assert_eq!(
        serde_json::to_value(FileExtractionConfig::default()).unwrap(),
        json!({})
    );
    let error = serde_json::from_value::<FileExtractionConfig>(json!({"use_layout_for_markdown":false})).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
    let encoded: Value = serde_json::to_value(ExtractInputWithOptions::new(input())).unwrap();
    assert!(encoded["input"].get("config").is_none());
}
