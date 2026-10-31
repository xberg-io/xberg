use super::{Outcome, validate_and_report};
use crate::core::config::MergeMode;
use serde_json::{Value, json};

fn person_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "relationship": {"type": "string"}
        },
        "required": ["name", "relationship"]
    })
}

#[test]
fn report_retains_correct_fields_without_declaring_a_null_required_field_valid() {
    let response = json!({"name": "Alice", "relationship": null});
    let report = validate_and_report(vec![response.clone()], &person_schema(), MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert!(report.validated.error_message.is_some());
    assert_eq!(report.batches.len(), 1);
    assert_eq!(report.batches[0].batch_index, 0);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert_eq!(report.batches[0].unvalidated_response["name"], "Alice");
    assert_eq!(report.batches[0].errors.len(), 1);
    assert_eq!(
        report.batches[0].errors[0].instance_pointer.as_deref(),
        Some("/relationship")
    );
    assert_eq!(
        report.batches[0].errors[0].schema_pointer.as_deref(),
        Some("/properties/relationship/type")
    );
    assert!(!report.batches[0].errors[0].message.is_empty());

    let wrapped = Value::String(response.to_string());
    let report = validate_and_report(vec![wrapped.clone()], &person_schema(), MergeMode::ObjectMerge);
    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.batches[0].unvalidated_response, wrapped);
    assert_eq!(
        report.batches[0].errors[0].instance_pointer.as_deref(),
        Some("/relationship")
    );
}

#[test]
fn report_keeps_a_missing_required_field_as_a_root_validation_error() {
    let response = json!({"name": "Alice"});
    let report = validate_and_report(vec![response.clone()], &person_schema(), MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert_eq!(report.batches[0].errors.len(), 1);
    assert_eq!(report.batches[0].errors[0].instance_pointer.as_deref(), Some(""));
    assert_eq!(report.batches[0].errors[0].schema_pointer.as_deref(), Some("/required"));
}

#[test]
fn report_collects_every_error_and_escapes_json_pointer_components() {
    let schema = json!({
        "type": "object",
        "properties": {"a/b~c": {"type": "string"}, "age": {"type": "integer"}},
        "required": ["city"]
    });
    let response = json!({"a/b~c": 7, "age": "unknown"});
    let report = validate_and_report(vec![response.clone()], &schema, MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.batches[0].unvalidated_response, response);
    let mut pointers = report.batches[0]
        .errors
        .iter()
        .map(|error| (error.instance_pointer.as_deref(), error.schema_pointer.as_deref()))
        .collect::<Vec<_>>();
    pointers.sort_unstable();
    assert_eq!(
        pointers,
        vec![
            (Some(""), Some("/required")),
            (Some("/age"), Some("/properties/age/type")),
            (Some("/a~1b~0c"), Some("/properties/a~1b~0c/type")),
        ]
    );
}

#[test]
fn report_preserves_table_rows_and_reports_the_original_array_position() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {"id": {"type": "integer"}, "label": {"type": "string"}},
            "required": ["id", "label"]
        }
    });
    let response = json!([{"id": 1, "label": "first"}, {"id": 2, "label": null}]);
    let report = validate_and_report(vec![response.clone()], &schema, MergeMode::ArrayConcat);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert_eq!(report.batches[0].errors.len(), 1);
    assert_eq!(
        report.batches[0].errors[0].instance_pointer.as_deref(),
        Some("/1/label")
    );
    assert_eq!(
        report.batches[0].errors[0].schema_pointer.as_deref(),
        Some("/items/properties/label/type")
    );
}

#[test]
fn report_keeps_existing_partial_success_limited_to_whole_valid_batches() {
    let valid = json!({"name": "Alice", "relationship": "parent"});
    let invalid = json!({"name": "Bob", "relationship": null});
    let report = validate_and_report(
        vec![valid.clone(), invalid.clone()],
        &person_schema(),
        MergeMode::ObjectMerge,
    );

    assert_eq!(report.validated.outcome, Outcome::PartialSuccess);
    assert_eq!(report.validated.merged, valid);
    assert_eq!(report.validated.per_batch_errors.len(), 1);
    assert_eq!(report.batches.len(), 2);
    assert_eq!(report.batches[0].batch_index, 0);
    assert_eq!(report.batches[0].unvalidated_response, valid);
    assert!(report.batches[0].errors.is_empty());
    assert_eq!(report.batches[1].batch_index, 1);
    assert_eq!(report.batches[1].unvalidated_response, invalid);
    assert_eq!(report.batches[1].errors.len(), 1);
}

#[test]
fn report_does_not_treat_fields_as_independently_valid_under_one_of_constraints() {
    let schema = json!({
        "type": "object",
        "oneOf": [
            {"properties": {"choice": {"const": "A"}}, "required": ["choice", "alpha"]},
            {"properties": {"choice": {"const": "B"}}, "required": ["choice", "beta"]}
        ]
    });
    let response = json!({"choice": "A", "beta": "present but not a valid alternative"});
    let report = validate_and_report(vec![response.clone()], &schema, MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert!(report.batches[0].errors.iter().any(|error| {
        error.instance_pointer.as_deref() == Some("") && error.schema_pointer.as_deref() == Some("/oneOf")
    }));
}

#[test]
fn report_honors_local_references_without_relaxing_the_referenced_schema() {
    let schema = json!({
        "$defs": {"name": {"type": "string"}},
        "type": "object",
        "properties": {"name": {"$ref": "#/$defs/name"}},
        "required": ["name"]
    });
    let response = json!({"name": null});
    let report = validate_and_report(vec![response.clone()], &schema, MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert_eq!(report.batches[0].errors.len(), 1);
    assert_eq!(report.batches[0].errors[0].instance_pointer.as_deref(), Some("/name"));
    assert_eq!(
        report.batches[0].errors[0].schema_pointer.as_deref(),
        Some("/$defs/name/type")
    );
}

#[test]
fn report_retains_malformed_json_with_parse_errors_without_invented_pointers() {
    let response = Value::String("{not valid JSON".to_string());
    let report = validate_and_report(vec![response.clone()], &person_schema(), MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::SchemaInvalid);
    assert_eq!(report.validated.merged, Value::Null);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert_eq!(report.batches[0].errors.len(), 1);
    assert_eq!(report.batches[0].errors[0].instance_pointer, None);
    assert_eq!(report.batches[0].errors[0].schema_pointer, None);
    assert!(
        report.batches[0].errors[0]
            .message
            .contains("failed to parse string as JSON")
    );
}

#[test]
fn report_rejects_an_offline_remote_reference_and_keeps_original_candidates_unvalidated() {
    let schema = json!({"$ref": "https://example.invalid/private-schema.json"});
    let response = json!({"name": "Alice"});
    let report = validate_and_report(vec![response.clone()], &schema, MergeMode::ObjectMerge);

    assert_eq!(report.validated.outcome, Outcome::Error);
    assert_eq!(report.validated.merged, Value::Null);
    assert!(report.validated.error_message.is_some());
    assert_eq!(report.batches.len(), 1);
    assert_eq!(report.batches[0].unvalidated_response, response);
    assert!(!report.batches[0].errors.is_empty());
}
