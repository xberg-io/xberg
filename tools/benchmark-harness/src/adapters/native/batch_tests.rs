use super::*;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn warm_native_batch_uses_engine_batch_and_emits_one_process_sample() {
    let adapter = NativeAdapter::new();
    let temp_dir = TempDir::new().unwrap();
    let first = temp_dir.path().join("first.txt");
    let second = temp_dir.path().join("second.txt");
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();

    let results = adapter
        .extract_batch(
            &[first.as_path(), second.as_path()],
            Duration::from_secs(10),
            &[false, false],
            &[None, None],
            crate::types::OutputFormat::Plaintext,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].extracted_text.as_deref(), Some("first"));
    assert_eq!(results[1].extracted_text.as_deref(), Some("second"));
    assert_eq!(results[0].framework_capabilities.batch_performance_sample, Some(true));
    assert_eq!(results[1].framework_capabilities.batch_performance_sample, Some(false));
    assert_eq!(
        results[0].framework_capabilities.batch_sample_id,
        results[1].framework_capabilities.batch_sample_id
    );
    assert_eq!(results[0].duration, results[1].duration);
}

#[test]
fn native_batch_rejects_multiple_results_without_honest_child_accounting() {
    let resource_stats = ResourceStats::default();
    let context = BatchRowContext {
        framework: "xberg-test",
        output_format: crate::types::OutputFormat::Plaintext,
        avg_duration_per_file: Duration::from_millis(1),
        resource_stats: &resource_stats,
    };
    let first = tempfile::NamedTempFile::new().unwrap();
    let second = tempfile::NamedTempFile::new().unwrap();
    let mut first_child = ExtractedDocument::default();
    first_child.content = "page one".to_string();
    first_child.metadata.additional.insert("source_index".into(), 0.into());
    let mut second_child = first_child.clone();
    second_child.content = "page two".to_string();
    let mut other = ExtractedDocument::default();
    other.content = "other".to_string();
    other.metadata.additional.insert("source_index".into(), 1.into());
    let output = xberg::ExtractionResult {
        results: vec![first_child, second_child, other],
        ..Default::default()
    };

    let error = assemble_batch_rows(
        &context,
        &[first.path(), second.path()],
        &output,
        false,
        &ExtractionConfig::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("multiple results for input index 0"));
    assert!(
        error
            .to_string()
            .contains("cannot aggregate per-child timing and metadata")
    );
}

#[tokio::test]
async fn warm_native_batch_timeout_is_fatal_for_retained_engine() {
    let adapter = NativeAdapter::new();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "timeout probe").unwrap();

    let error = adapter
        .extract_batch(
            &[file.path()],
            Duration::ZERO,
            &[false],
            &[None],
            crate::types::OutputFormat::Plaintext,
        )
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Timeout(_)));
    assert!(error.to_string().contains("retained Engine is no longer reusable"));
}
