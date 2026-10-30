use super::{CitationInputs, FinalizedOutput, finalize_with_recovery, merge_and_cite};
use crate::core::config::MergeMode;
use crate::engine::structured::citations::{CitationSource, CitedField};
use crate::engine::structured::schema::Outcome;
use serde_json::{Value, json};

fn person_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"name": {"type": "string"}, "relationship": {"type": "string"}},
        "required": ["name", "relationship"]
    })
}

fn citation_inputs(emit_citations: bool, ocr_elements: &[Value]) -> CitationInputs<'_> {
    CitationInputs {
        ocr_elements,
        element_metadata: &[],
        emit_citations,
        match_threshold: 0.8,
        fused_confidence: 0.95,
    }
}

fn assert_strict_equal(actual: &FinalizedOutput, expected: &FinalizedOutput) {
    assert_eq!(actual.structured_output, expected.structured_output);
    assert_eq!(actual.structured_output_flat, expected.structured_output_flat);
    assert_eq!(actual.outcome, expected.outcome);
    assert_eq!(actual.error_message, expected.error_message);
    assert_eq!(actual.per_batch_errors, expected.per_batch_errors);
}

#[test]
fn opt_in_finalization_recovers_usable_fields_when_a_required_string_cell_is_null() {
    let response = json!({"name": "Alice", "relationship": null});
    let strict = merge_and_cite(
        vec![response.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );
    assert_eq!(strict.outcome, Outcome::SchemaInvalid);
    assert_eq!(strict.structured_output, Value::Null);

    let recovery = finalize_with_recovery(
        vec![response.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );
    assert_strict_equal(&recovery.strict, &strict);
    assert_eq!(recovery.unvalidated_candidate, Some(response));
    let candidate = recovery
        .unvalidated_candidate
        .as_ref()
        .expect("usable candidate is retained");
    assert_eq!(candidate["name"], "Alice");
    assert_eq!(candidate["relationship"], Value::Null);
    assert_eq!(recovery.batches.len(), 1);
    assert_eq!(recovery.batches[0].batch_index, 0);
    assert_eq!(
        recovery.batches[0].errors[0].instance_pointer.as_deref(),
        Some("/relationship")
    );
}

#[test]
fn recovery_candidate_is_never_citation_fused_even_when_strict_citations_are_enabled() {
    let response = json!({"name": "Alice", "relationship": null});
    let ocr = [json!({"text": "Alice", "page_number": 1, "bbox": [10.0, 20.0, 100.0, 30.0]})];
    let recovery = finalize_with_recovery(
        vec![response.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(true, &ocr),
    );

    assert_eq!(recovery.strict.outcome, Outcome::SchemaInvalid);
    let cited: CitedField = serde_json::from_value(recovery.strict.structured_output).expect("legacy null envelope");
    assert_eq!(cited.value, Value::Null);
    assert_eq!(cited.source, CitationSource::None);
    assert_eq!(recovery.unvalidated_candidate, Some(response));
}

#[test]
fn object_recovery_merges_original_fields_without_overwriting_the_strict_validated_result() {
    let valid = json!({"name": "Alice", "relationship": "parent"});
    let invalid = json!({"name": "Bob", "relationship": null, "note": "retained"});
    let recovery = finalize_with_recovery(
        vec![valid.clone(), invalid.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::PartialSuccess);
    assert_eq!(recovery.strict.structured_output, valid);
    assert_eq!(recovery.unvalidated_candidate, Some(invalid));
    assert_eq!(recovery.batches[1].batch_index, 1);
    assert_eq!(recovery.batches[1].errors.len(), 1);
    assert_eq!(
        recovery.batches[1].errors[0].schema_pointer.as_deref(),
        Some("/properties/relationship/type")
    );
}

#[test]
fn array_recovery_retains_rows_and_keeps_error_pointers_local_to_the_original_batch() {
    let schema = json!({"type": "array", "items": person_schema()});
    let valid = json!({"name": "Alice", "relationship": "parent"});
    let invalid = json!({"name": "Bob", "relationship": null});
    let recovery = finalize_with_recovery(
        vec![json!([valid.clone()]), json!([invalid.clone()])],
        &schema,
        MergeMode::ArrayConcat,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::PartialSuccess);
    assert_eq!(recovery.strict.structured_output, json!([valid.clone()]));
    assert_eq!(recovery.unvalidated_candidate, Some(json!([valid, invalid])));
    assert_eq!(recovery.batches[1].batch_index, 1);
    assert_eq!(
        recovery.batches[1].errors[0].instance_pointer.as_deref(),
        Some("/0/relationship")
    );
}

#[test]
fn first_response_recovery_normalizes_json_strings_without_selecting_the_first_valid_batch() {
    let invalid = json!({"name": "Alice", "relationship": null});
    let valid = json!({"name": "Bob", "relationship": "parent"});
    let wrapped = Value::String(invalid.to_string());
    let recovery = finalize_with_recovery(
        vec![wrapped.clone(), valid.clone()],
        &person_schema(),
        MergeMode::ObjectFirst,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::PartialSuccess);
    assert_eq!(recovery.strict.structured_output, valid);
    assert_eq!(recovery.unvalidated_candidate, Some(invalid));
    assert_eq!(recovery.batches[0].unvalidated_response, wrapped);
    assert_eq!(
        recovery.batches[0].errors[0].instance_pointer.as_deref(),
        Some("/relationship")
    );
}

#[test]
fn malformed_json_is_reported_and_never_inserted_into_the_merged_candidate() {
    let malformed = Value::String("{not JSON".to_string());
    let valid = json!({"name": "Alice", "relationship": "parent"});
    let recovery = finalize_with_recovery(
        vec![malformed.clone(), valid.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::PartialSuccess);
    assert_eq!(recovery.unvalidated_candidate, Some(valid));
    assert_eq!(recovery.batches[0].unvalidated_response, malformed);
    assert_eq!(recovery.batches[0].errors.len(), 1);
    assert_eq!(recovery.batches[0].errors[0].instance_pointer, None);
    assert_eq!(recovery.batches[0].errors[0].schema_pointer, None);
}

#[test]
fn recovery_does_not_prune_or_relax_cross_field_constraints() {
    let schema = json!({
        "type": "object",
        "oneOf": [
            {"properties": {"choice": {"const": "A"}}, "required": ["choice", "alpha"]},
            {"properties": {"choice": {"const": "B"}}, "required": ["choice", "beta"]}
        ]
    });
    let response = json!({"choice": "A", "beta": "not a valid alternative"});
    let recovery = finalize_with_recovery(
        vec![response.clone()],
        &schema,
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::SchemaInvalid);
    assert_eq!(recovery.strict.structured_output, Value::Null);
    assert_eq!(recovery.unvalidated_candidate, Some(response));
    assert!(
        recovery.batches[0]
            .errors
            .iter()
            .any(|error| error.schema_pointer.as_deref() == Some("/oneOf"))
    );
}

#[test]
fn invalid_offline_schema_remains_an_error_and_never_validates_the_candidate() {
    let schema = json!({"$ref": "https://example.invalid/private-schema.json"});
    let response = json!({"name": "Alice"});
    let recovery = finalize_with_recovery(
        vec![response.clone()],
        &schema,
        MergeMode::ObjectMerge,
        citation_inputs(false, &[]),
    );

    assert_eq!(recovery.strict.outcome, Outcome::Error);
    assert_eq!(recovery.strict.structured_output, Value::Null);
    assert!(recovery.strict.error_message.is_some());
    assert_eq!(recovery.unvalidated_candidate, Some(response));
    assert!(!recovery.batches[0].errors.is_empty());
}

#[test]
fn absent_parseable_responses_and_an_explicit_null_response_are_distinct() {
    for raw in [vec![], vec![Value::String("{not JSON".to_string())]] {
        let recovery = finalize_with_recovery(
            raw,
            &person_schema(),
            MergeMode::ObjectFirst,
            citation_inputs(false, &[]),
        );
        assert_ne!(recovery.strict.outcome, Outcome::Success);
        assert_eq!(recovery.unvalidated_candidate, None);
    }
    let recovery = finalize_with_recovery(
        vec![Value::Null],
        &person_schema(),
        MergeMode::ObjectFirst,
        citation_inputs(false, &[]),
    );
    assert_eq!(recovery.strict.outcome, Outcome::SchemaInvalid);
    assert_eq!(recovery.unvalidated_candidate, Some(Value::Null));
    assert_eq!(recovery.batches[0].errors[0].instance_pointer.as_deref(), Some(""));
}

#[test]
fn fully_valid_recovery_preserves_the_existing_cited_and_flattened_outputs() {
    let response = json!({"name": "Alice", "relationship": "parent"});
    let ocr = [json!({"text": "Alice", "page_number": 1, "bbox": [10.0, 20.0, 100.0, 30.0]})];
    let strict = merge_and_cite(
        vec![response.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(true, &ocr),
    );
    let recovery = finalize_with_recovery(
        vec![response.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
        citation_inputs(true, &ocr),
    );

    assert_eq!(recovery.strict.outcome, Outcome::Success);
    assert_strict_equal(&recovery.strict, &strict);
    assert_eq!(recovery.unvalidated_candidate, Some(response));
    assert!(recovery.batches[0].errors.is_empty());
}
