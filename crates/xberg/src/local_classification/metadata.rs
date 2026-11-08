use super::LocalNliModel;

pub(super) fn validate_model_metadata(metadata: &serde_json::Value, model: &LocalNliModel) -> crate::Result<()> {
    use super::policy::invalid;
    let supported = [
        ("bert", "BertForSequenceClassification"),
        ("roberta", "RobertaForSequenceClassification"),
        ("xlm-roberta", "XLMRobertaForSequenceClassification"),
    ];
    let architecture = metadata["architectures"].as_array();
    if !supported.iter().any(|(kind, name)| {
        metadata["model_type"].as_str() == Some(*kind)
            && architecture.is_some_and(|values| values.len() == 1 && values[0].as_str() == Some(*name))
    }) {
        return Err(invalid("Unsupported local NLI model architecture"));
    }
    let labels = metadata["id2label"]
        .as_object()
        .filter(|labels| labels.len() == 3)
        .ok_or_else(|| invalid("Local NLI metadata must declare three semantic labels"))?;
    for index in 0..3u32 {
        let expected = if index == model.entailment_index {
            "entailment"
        } else if index == model.contradiction_index {
            "contradiction"
        } else {
            "neutral"
        };
        if !labels
            .get(&index.to_string())
            .and_then(serde_json::Value::as_str)
            .is_some_and(|label| label.eq_ignore_ascii_case(expected))
        {
            return Err(invalid("Local NLI label indices conflict with model metadata"));
        }
    }
    Ok(())
}
