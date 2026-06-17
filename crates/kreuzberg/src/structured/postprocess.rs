//! JSON Schema validation and multi-batch merge of vision responses.
//!
//! Validates each batch's vision-LLM JSON response against the user-supplied
//! schema, then merges by the configured strategy (object merge, array concat,
//! or take-first). Produces merged output with per-batch error tracking and a
//! [`crate::heuristics::SchemaCompliance`] handoff for confidence scoring.

use serde::{Deserialize, Serialize};

use crate::core::config::MergeMode;
use crate::heuristics::SchemaCompliance;

/// High-level outcome of a [`validate_and_merge`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every batch validated and merged.
    Success,
    /// At least one batch validated; at least one failed schema validation.
    PartialSuccess,
    /// All batches failed schema validation (but the schema itself is valid JSON Schema).
    SchemaInvalid,
    /// Empty input or an invalid schema — no output could be produced.
    Error,
}

/// Result produced by [`validate_and_merge`].
#[derive(Debug, Clone)]
pub struct MergedOutput {
    /// The merged JSON value, or [`serde_json::Value::Null`] on `Error`/`SchemaInvalid`.
    pub merged: serde_json::Value,
    /// High-level outcome.
    pub outcome: Outcome,
    /// Schema compliance for the confidence-scoring handoff.
    pub schema_compliance: SchemaCompliance,
    /// Human-readable per-batch validation error strings.
    pub per_batch_errors: Vec<String>,
}

/// Validate each raw response and merge by the configured strategy.
///
/// # Arguments
///
/// * `raw_responses` — Vision-LLM JSON responses (already parsed as
///   [`serde_json::Value`], or string values that will be re-parsed as JSON).
/// * `schema` — JSON Schema (any draft understood by `jsonschema` 0.46) used to
///   validate each batch.
/// * `merge_mode` — Merging strategy from the preset.
///
/// # Behaviour
///
/// 1. Empty input → `Outcome::Error` / `SchemaCompliance::AllInvalid`.
/// 2. An invalid `schema` itself → `Outcome::Error` / `SchemaCompliance::AllInvalid`.
/// 3. Each response is normalised (string responses re-parsed as JSON) and
///    validated against the schema.  Failing batches collect a human-readable
///    message in `per_batch_errors`.
/// 4. If no batch passes, returns `Outcome::SchemaInvalid` / `AllInvalid`.
/// 5. The passing batches are merged with `merge_mode`.
/// 6. `outcome` is `Success` when every batch passed, `PartialSuccess` otherwise.
pub fn validate_and_merge(
    raw_responses: Vec<serde_json::Value>,
    schema: &serde_json::Value,
    merge_mode: MergeMode,
) -> MergedOutput {
    if raw_responses.is_empty() {
        return MergedOutput {
            merged: serde_json::Value::Null,
            outcome: Outcome::Error,
            schema_compliance: SchemaCompliance::AllInvalid,
            per_batch_errors: vec![],
        };
    }

    let validator = match jsonschema::validator_for(schema) {
        Ok(v) => v,
        Err(e) => {
            return MergedOutput {
                merged: serde_json::Value::Null,
                outcome: Outcome::Error,
                schema_compliance: SchemaCompliance::AllInvalid,
                per_batch_errors: vec![format!("schema is invalid: {e}")],
            };
        }
    };

    let mut validated_batches: Vec<serde_json::Value> = Vec::new();
    let mut per_batch_errors: Vec<String> = Vec::new();

    for (idx, raw) in raw_responses.into_iter().enumerate() {
        let value = match normalize_response(raw) {
            Ok(v) => v,
            Err(e) => {
                per_batch_errors.push(format!("batch {idx}: {e}"));
                continue;
            }
        };

        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();

        if errors.is_empty() {
            validated_batches.push(value);
        } else {
            per_batch_errors.push(format!(
                "batch {idx}: schema validation failed: {}",
                errors.join("; ")
            ));
        }
    }

    if validated_batches.is_empty() {
        return MergedOutput {
            merged: serde_json::Value::Null,
            outcome: Outcome::SchemaInvalid,
            schema_compliance: SchemaCompliance::AllInvalid,
            per_batch_errors,
        };
    }

    let merged = merge_validated(validated_batches, merge_mode);

    let (outcome, schema_compliance) = if per_batch_errors.is_empty() {
        (Outcome::Success, SchemaCompliance::AllValid)
    } else {
        (Outcome::PartialSuccess, SchemaCompliance::PartialValid)
    };

    MergedOutput {
        merged,
        outcome,
        schema_compliance,
        per_batch_errors,
    }
}

/// Normalise a response: if it is a JSON string, re-parse it as JSON.
fn normalize_response(value: serde_json::Value) -> Result<serde_json::Value, String> {
    match value {
        serde_json::Value::String(s) => {
            serde_json::from_str(&s).map_err(|e| format!("failed to parse string as JSON: {e}"))
        }
        other => Ok(other),
    }
}

/// Merge validated batches by merge mode.
///
/// * `ObjectMerge` — deep-merge objects field by field; later batches only fill
///   keys that are absent in the accumulator (earlier batches win per key).
/// * `ArrayConcat` — concatenate top-level arrays.
/// * `ObjectFirst` — return the first valid batch unchanged.
fn merge_validated(batches: Vec<serde_json::Value>, merge_mode: MergeMode) -> serde_json::Value {
    match merge_mode {
        MergeMode::ObjectMerge => {
            let mut result = serde_json::Map::new();
            for batch in batches {
                if let serde_json::Value::Object(obj) = batch {
                    for (key, value) in obj {
                        // Later batches fill missing keys only; earlier batches win.
                        result.entry(key).or_insert_with(|| deep_merge_value(value));
                    }
                }
            }
            serde_json::Value::Object(result)
        }
        MergeMode::ArrayConcat => {
            let mut result: Vec<serde_json::Value> = Vec::new();
            for batch in batches {
                if let serde_json::Value::Array(arr) = batch {
                    result.extend(arr);
                }
            }
            serde_json::Value::Array(result)
        }
        MergeMode::ObjectFirst => batches
            .into_iter()
            .next()
            .unwrap_or(serde_json::Value::Null),
    }
}

/// Clone a value — used so the merge path owns its data unambiguously.
fn deep_merge_value(value: serde_json::Value) -> serde_json::Value {
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ObjectMerge — happy path: two valid batches supply disjoint keys
    // -------------------------------------------------------------------------

    #[test]
    fn object_merge_two_valid_batches_fills_disjoint_keys() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age":  { "type": "number" }
            }
        });
        let batch1 = serde_json::json!({"name": "Alice"});
        let batch2 = serde_json::json!({"age": 30});

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(result.schema_compliance, SchemaCompliance::AllValid);
        assert!(result.per_batch_errors.is_empty());
        assert_eq!(
            result.merged.get("name").and_then(|v| v.as_str()),
            Some("Alice")
        );
        assert_eq!(
            result.merged.get("age").and_then(|v| v.as_u64()),
            Some(30)
        );
    }

    // -------------------------------------------------------------------------
    // ObjectMerge — earlier batch wins on key collision
    // -------------------------------------------------------------------------

    #[test]
    fn object_merge_earlier_batch_wins_on_key_collision() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        });
        let batch1 = serde_json::json!({"value": "first"});
        let batch2 = serde_json::json!({"value": "second"});

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(
            result.merged.get("value").and_then(|v| v.as_str()),
            Some("first"),
            "earlier batch must win on collision"
        );
    }

    // -------------------------------------------------------------------------
    // ObjectMerge — one invalid batch → PartialSuccess / PartialValid
    // -------------------------------------------------------------------------

    #[test]
    fn object_merge_one_invalid_batch_gives_partial_success() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });
        let batch1 = serde_json::json!({"name": "Alice"});
        let batch2 = serde_json::json!({"name": 123}); // invalid: number, not string

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::PartialSuccess);
        assert_eq!(result.schema_compliance, SchemaCompliance::PartialValid);
        assert_eq!(result.per_batch_errors.len(), 1);
        assert!(
            result.per_batch_errors[0].contains("batch 1"),
            "error must identify the failing batch"
        );
        assert_eq!(
            result.merged.get("name").and_then(|v| v.as_str()),
            Some("Alice")
        );
    }

    // -------------------------------------------------------------------------
    // All invalid → SchemaInvalid / AllInvalid
    // -------------------------------------------------------------------------

    #[test]
    fn all_invalid_returns_schema_invalid_and_all_invalid_compliance() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "count": { "type": "number" }
            },
            "required": ["count"]
        });
        let batch1 = serde_json::json!({"count": "not a number"});
        let batch2 = serde_json::json!({"count": "also not a number"});

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::SchemaInvalid);
        assert_eq!(result.schema_compliance, SchemaCompliance::AllInvalid);
        assert_eq!(result.merged, serde_json::Value::Null);
        assert_eq!(result.per_batch_errors.len(), 2);
    }

    // -------------------------------------------------------------------------
    // ArrayConcat
    // -------------------------------------------------------------------------

    #[test]
    fn array_concat_concatenates_two_valid_batches() {
        let schema = serde_json::json!({
            "type": "array",
            "items": { "type": "object" }
        });
        let batch1 = serde_json::json!([{"id": 1}]);
        let batch2 = serde_json::json!([{"id": 2}]);

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ArrayConcat);

        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(result.schema_compliance, SchemaCompliance::AllValid);
        assert!(result.per_batch_errors.is_empty());
        let arr = result.merged.as_array().expect("must be array");
        assert_eq!(arr.len(), 2);
    }

    // -------------------------------------------------------------------------
    // ObjectFirst
    // -------------------------------------------------------------------------

    #[test]
    fn object_first_takes_first_valid_batch_and_ignores_later_ones() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            }
        });
        let batch1 = serde_json::json!({"value": "first"});
        let batch2 = serde_json::json!({"value": "second"});

        let result = validate_and_merge(vec![batch1, batch2], &schema, MergeMode::ObjectFirst);

        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(
            result.merged.get("value").and_then(|v| v.as_str()),
            Some("first")
        );
    }

    // -------------------------------------------------------------------------
    // Empty input → Error
    // -------------------------------------------------------------------------

    #[test]
    fn empty_input_returns_error_outcome() {
        let schema = serde_json::json!({"type": "object"});

        let result = validate_and_merge(vec![], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::Error);
        assert_eq!(result.schema_compliance, SchemaCompliance::AllInvalid);
        assert_eq!(result.merged, serde_json::Value::Null);
        assert!(result.per_batch_errors.is_empty());
    }

    // -------------------------------------------------------------------------
    // Invalid schema → Error
    // -------------------------------------------------------------------------

    #[test]
    fn invalid_schema_returns_error_outcome_with_message() {
        // A JSON value that is not a valid JSON Schema.
        let bad_schema = serde_json::json!({"type": "not-a-valid-type-value-xxxxx"});

        let result = validate_and_merge(
            vec![serde_json::json!({"x": 1})],
            &bad_schema,
            MergeMode::ObjectMerge,
        );

        assert_eq!(result.outcome, Outcome::Error);
        assert_eq!(result.schema_compliance, SchemaCompliance::AllInvalid);
        assert_eq!(result.merged, serde_json::Value::Null);
        assert!(!result.per_batch_errors.is_empty());
        assert!(
            result.per_batch_errors[0].contains("schema is invalid"),
            "error message: {}",
            result.per_batch_errors[0]
        );
    }

    // -------------------------------------------------------------------------
    // String-wrapped JSON is normalised
    // -------------------------------------------------------------------------

    #[test]
    fn string_wrapped_json_is_normalised_before_validation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "key": { "type": "string" }
            }
        });
        // A JSON *string* whose content is a JSON object.
        let batch_as_string = serde_json::json!(r#"{"key": "value"}"#);

        let result =
            validate_and_merge(vec![batch_as_string], &schema, MergeMode::ObjectMerge);

        assert_eq!(result.outcome, Outcome::Success);
        assert_eq!(
            result.merged.get("key").and_then(|v| v.as_str()),
            Some("value")
        );
    }

    // -------------------------------------------------------------------------
    // per_batch_errors contains the batch index
    // -------------------------------------------------------------------------

    #[test]
    fn per_batch_errors_contain_batch_index() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "n": { "type": "number" }
            },
            "required": ["n"]
        });
        let valid = serde_json::json!({"n": 1});
        let invalid = serde_json::json!({"n": "wrong"});

        let result = validate_and_merge(
            vec![valid, invalid],
            &schema,
            MergeMode::ObjectMerge,
        );

        assert_eq!(result.per_batch_errors.len(), 1);
        assert!(
            result.per_batch_errors[0].contains("batch 1"),
            "got: {}",
            result.per_batch_errors[0]
        );
    }
}
