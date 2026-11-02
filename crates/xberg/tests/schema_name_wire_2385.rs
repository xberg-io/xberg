//! Keep provider identifiers safe without rewriting caller schema or prompt data.
#![cfg(feature = "structured")]

use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};
use xberg::llm::structured::complete_with_json_schema;
use xberg::{ExtractInput, ExtractionConfig, LlmConfig, StructuredExtractionConfig, extract};

const DOCUMENT: &str = "payload admission control";
const SCHEMA_NAME_LIMIT: usize = 64;

#[derive(Clone, Copy)]
enum EntryPoint {
    SharedHelper,
    PublicPipeline,
}

fn schema(name: &str) -> Value {
    json!({
        "type": "object",
        "title": name,
        "properties": { "value": { "$ref": "#/$defs/entry" } },
        "$defs": { "entry": { "type": "string", "title": name } },
        "required": ["value"],
        "additionalProperties": false
    })
}

fn completion() -> Value {
    json!({
        "id": "chatcmpl-schema-name", "object": "chat.completion", "created": 0,
        "model": "gpt-4o-mini",
        "choices": [{ "index": 0, "message": { "role": "assistant", "content": "{\"value\":\"kept\"}" },
            "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 }
    })
}

fn llm(server: &MockServer) -> LlmConfig {
    LlmConfig {
        model: "openai/gpt-4o-mini".to_owned(),
        api_key: Some("test-key".to_owned()),
        base_url: Some(server.uri()),
        timeout_secs: Some(10),
        max_retries: Some(0),
        ..Default::default()
    }
}

async fn call_shared(config: &LlmConfig, original: &str, original_schema: &Value) {
    let prompt = format!("NAME={original};DOC={DOCUMENT}");
    let (value, usage) = complete_with_json_schema(config, &prompt, original, original_schema, "schema_name_control")
        .await
        .expect("real shared helper must complete through the local provider");
    assert_eq!(value, json!({ "value": "kept" }));
    assert_eq!(usage.expect("actual provider usage").source, "schema_name_control");
}

async fn call_pipeline(config: LlmConfig, original: &str, original_schema: &Value) {
    let extraction = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            schema: original_schema.clone(),
            schema_name: original.to_owned(),
            schema_description: Some("Original description".to_owned()),
            strict: true,
            prompt: Some("NAME={{ schema_name }};DOC={{ content }}".to_owned()),
            llm: config,
        }),
        ..Default::default()
    };
    let before = serde_json::to_value(&extraction).expect("original public config");
    let output = extract(
        ExtractInput::from_bytes(
            DOCUMENT.as_bytes().to_vec(),
            "text/plain",
            Some("source.txt".to_owned()),
        ),
        &extraction,
    )
    .await
    .expect("actual public extraction must succeed");
    assert_eq!(output.summary.inputs, 1);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.summary.errors, 0);
    assert!(output.errors.is_empty());
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content, DOCUMENT);
    assert_eq!(output.results[0].structured_output, Some(json!({ "value": "kept" })));
    assert!(output.results[0].processing_warnings.is_empty());
    assert_eq!(serde_json::to_value(&extraction).expect("final public config"), before);
}

async fn assert_provider_name(entry: EntryPoint, original: &str, expected: &str) {
    assert!(!expected.is_empty() && expected.len() <= SCHEMA_NAME_LIMIT);
    assert!(
        expected
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(completion()))
        .expect(1)
        .mount(&server)
        .await;
    let original_schema = schema(original);
    match entry {
        EntryPoint::SharedHelper => call_shared(&llm(&server), original, &original_schema).await,
        EntryPoint::PublicPipeline => call_pipeline(llm(&server), original, &original_schema).await,
    }
    let requests = server
        .received_requests()
        .await
        .expect("captured real provider requests");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].url.path().ends_with("/chat/completions"));
    let payload: Value = serde_json::from_slice(&requests[0].body).expect("actual JSON provider payload");
    assert_eq!(payload["response_format"]["type"], "json_schema");
    assert_eq!(payload["response_format"]["json_schema"]["schema"], original_schema);
    assert_eq!(
        payload["messages"][0]["content"],
        format!("NAME={original};DOC={DOCUMENT}")
    );
    assert_eq!(
        payload["response_format"]["json_schema"]["strict"],
        matches!(entry, EntryPoint::PublicPipeline)
    );
    assert_eq!(payload["response_format"]["json_schema"]["name"], expected);
}

macro_rules! both_entrypoints {
    ($shared:ident, $pipeline:ident, $original:expr, $expected:expr) => {
        #[tokio::test]
        async fn $shared() {
            assert_provider_name(EntryPoint::SharedHelper, &$original, &$expected).await;
        }

        #[tokio::test]
        async fn $pipeline() {
            assert_provider_name(EntryPoint::PublicPipeline, &$original, &$expected).await;
        }
    };
}

both_entrypoints!(
    shared_display_name_is_safe_on_wire,
    pipeline_display_name_is_safe_on_wire,
    "Contract Extraction",
    "Contract_Extraction"
);
both_entrypoints!(
    shared_invalid_runs_collapse_and_boundaries_trim,
    pipeline_invalid_runs_collapse_and_boundaries_trim,
    " \tOrder !!! / receipt??? ",
    "Order_receipt"
);
both_entrypoints!(
    shared_unicode_suffix_is_trimmed,
    pipeline_unicode_suffix_is_trimmed,
    "Japanese Receipt (領収書)",
    "Japanese_Receipt"
);
both_entrypoints!(
    shared_unicode_only_name_uses_default,
    pipeline_unicode_only_name_uses_default,
    "領収書",
    "extraction"
);
both_entrypoints!(
    shared_empty_name_uses_default,
    pipeline_empty_name_uses_default,
    "",
    "extraction"
);
both_entrypoints!(
    shared_punctuation_only_name_uses_default,
    pipeline_punctuation_only_name_uses_default,
    "!!! / ..",
    "extraction"
);
both_entrypoints!(
    shared_legal_boundary_underscores_are_preserved,
    pipeline_legal_boundary_underscores_are_preserved,
    "__Invoice-Data__",
    "__Invoice-Data__"
);
both_entrypoints!(
    shared_exact_limit_name_is_preserved,
    pipeline_exact_limit_name_is_preserved,
    "a".repeat(SCHEMA_NAME_LIMIT),
    "a".repeat(SCHEMA_NAME_LIMIT)
);
both_entrypoints!(
    shared_long_name_is_truncated,
    pipeline_long_name_is_truncated,
    "a".repeat(SCHEMA_NAME_LIMIT + 1),
    "a".repeat(SCHEMA_NAME_LIMIT)
);
both_entrypoints!(
    shared_utf8_is_replaced_before_ascii_truncation,
    pipeline_utf8_is_replaced_before_ascii_truncation,
    format!("{}éb", "a".repeat(SCHEMA_NAME_LIMIT - 1)),
    format!("{}_", "a".repeat(SCHEMA_NAME_LIMIT - 1))
);
both_entrypoints!(
    shared_existing_underscore_is_not_an_invalid_run,
    pipeline_existing_underscore_is_not_an_invalid_run,
    "good_?name",
    "good__name"
);
both_entrypoints!(
    shared_only_introduced_boundary_separators_are_trimmed,
    pipeline_only_introduced_boundary_separators_are_trimmed,
    " ?__name--__? ",
    "__name--__"
);
both_entrypoints!(
    shared_one_character_name_is_preserved,
    pipeline_one_character_name_is_preserved,
    "x",
    "x"
);
