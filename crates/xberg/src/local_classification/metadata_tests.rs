use serde_json::json;

use super::metadata::validate_model_metadata;
use super::tests::configuration;

fn metadata() -> serde_json::Value {
    json!({"model_type":"bert", "architectures":["BertForSequenceClassification"],
        "id2label":{"0":"contradiction", "1":"neutral", "2":"entailment"}, "max_position_embeddings":512})
}

#[test]
fn declared_three_way_nli_semantics_are_accepted() {
    validate_model_metadata(&metadata(), &configuration().model).unwrap();
}

#[test]
fn a_fixed_taxonomy_head_is_not_treated_as_arbitrary_label_nli() {
    let mut metadata = metadata();
    metadata["id2label"] = json!({"0":"sports", "1":"finance", "2":"other"});
    assert!(validate_model_metadata(&metadata, &configuration().model).is_err());
}

#[test]
fn declared_nli_indices_must_match_the_pinned_model_configuration() {
    let mut model = configuration().model;
    model.entailment_index = 0;
    model.contradiction_index = 2;
    assert!(validate_model_metadata(&metadata(), &model).is_err());
}

#[test]
fn an_unsupported_model_architecture_cannot_silently_use_the_nli_adapter() {
    let mut metadata = metadata();
    metadata["architectures"] = json!(["BertForMaskedLM"]);
    assert!(validate_model_metadata(&metadata, &configuration().model).is_err());
}
