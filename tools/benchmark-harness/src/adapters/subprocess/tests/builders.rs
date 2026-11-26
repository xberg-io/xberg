//! Tests for the `SubprocessAdapter` builder methods: format-awareness, native-batch
//! capability, liteparse provenance/batch-args, per-mode command args, configured OCR
//! status, and worker/thread-budget resolution.

use crate::adapter::FrameworkAdapter;
use crate::types::{BatchCapability, BatchEntryPoint, OcrStatus, OutputFormat};
use std::path::{Path, PathBuf};

use super::super::SubprocessAdapter;

#[test]
fn test_format_aware_builder() {
    let adapter =
        SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_format_aware(true);
    assert!(adapter.format_aware);
    assert!(adapter.batch_capability.is_none());
    assert_eq!(
        adapter.supported_output_formats(),
        vec![OutputFormat::Plaintext, OutputFormat::Markdown]
    );
}

#[test]
fn test_native_batch_builder() {
    let adapter = SubprocessAdapter::with_batch_capability(
        "test",
        "echo",
        vec![],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::LiteparseBatchParse,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    )
    .with_format_aware(true);
    assert!(adapter.batch_capability.is_some());
    assert!(adapter.format_aware);
    assert_eq!(
        adapter.batch_capability.map(|capability| capability.entry_point),
        Some(BatchEntryPoint::LiteparseBatchParse)
    );
}

#[test]
fn liteparse_batch_provenance_hashes_normalized_semantic_arguments() {
    let command = PathBuf::from("/opt/liteparse/bin/lit");
    let adapter = SubprocessAdapter::with_batch_capability(
        "liteparse",
        "bash",
        vec!["liteparse_extract.sh".to_string(), "--no-ocr".to_string()],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::LiteparseBatchParse,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    )
    .with_batch_workers(4)
    .with_native_batch_command(command.clone());
    let expected_args = vec![
        "batch-parse".to_string(),
        "<input-dir>".to_string(),
        "<output-dir>".to_string(),
        "--format".to_string(),
        "<output-format>".to_string(),
        "--num-workers".to_string(),
        "4".to_string(),
        "--quiet".to_string(),
        "--ocr-backend".to_string(),
        "tesseract".to_string(),
        "--no-ocr".to_string(),
    ];

    assert_eq!(
        adapter.liteparse_batch_args("<input-dir>", "<output-dir>", "<output-format>", true, None),
        expected_args
    );
    assert_eq!(
        adapter.executable_provenance_for_mode(crate::config::BenchmarkMode::Batch),
        Some(crate::provenance::ExecutableProvenance::from_invocation(
            &command,
            &expected_args
        ))
    );
}

#[test]
fn liteparse_forwards_one_ocr_language_per_native_batch() {
    let adapter = SubprocessAdapter::with_batch_capability(
        "liteparse",
        "bash",
        vec!["liteparse_extract.sh".to_string(), "--ocr".to_string()],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::LiteparseBatchParse,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    )
    .with_ocr_language_arg("--ocr-language")
    .with_ocr_language_policy(crate::adapter::declared_ocr_language_policy("liteparse"));

    assert!(adapter.supports_fixture("pdf", None, Some("eng")));
    assert!(adapter.supports_fixture("pdf", None, Some("deu")));
    assert_eq!(
        adapter.ocr_language_forward_arg(Some("eng")).as_deref(),
        Some("--ocr-language=eng")
    );
    let english = [Some("eng".to_string()), Some("eng".to_string())];
    let language = adapter
        .batch_ocr_language_forward_arg(&english)
        .expect("one language per batch");
    assert_eq!(language.as_deref(), Some("--ocr-language=eng"));
    let args = adapter.liteparse_batch_args("<input-dir>", "<output-dir>", "markdown", false, language);
    assert_eq!(args.last().map(String::as_str), Some("--ocr-language=eng"));
    assert!(
        adapter
            .batch_ocr_language_forward_arg(&[Some("eng".to_string()), Some("deu".to_string())])
            .is_err(),
        "a native batch must not mix OCR languages"
    );
}

#[test]
fn repeated_identical_batch_invocations_receive_distinct_sample_ids() {
    let adapter = SubprocessAdapter::with_batch_capability(
        "docling",
        "python",
        vec![],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::DoclingJobkit,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    )
    .with_batch_workers(4);
    let input = Path::new("/tmp/identical.pdf");
    let paths = [input];

    let first = adapter.batch_sample_id(&paths, false, OutputFormat::Markdown);
    let second = adapter.batch_sample_id(&paths, false, OutputFormat::Markdown);

    assert_ne!(first, second);
}

#[test]
fn generic_batch_builder_preserves_separate_single_file_command() {
    let batch_args = vec!["docling_extract.py".to_string(), "batch".to_string()];
    let single_file_args = vec!["docling_extract.py".to_string(), "sync".to_string()];
    let adapter = SubprocessAdapter::with_batch_capability(
        "docling",
        "python",
        batch_args.clone(),
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::DoclingJobkit,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    )
    .with_single_file_args(single_file_args.clone());

    assert!(adapter.batch_capability.is_some());
    assert_eq!(adapter.args.last().map(String::as_str), Some("batch"));
    assert_eq!(
        adapter
            .single_file_args
            .as_ref()
            .and_then(|args| args.last())
            .map(String::as_str),
        Some("sync")
    );
    assert_eq!(
        adapter.executable_provenance_for_mode(crate::config::BenchmarkMode::SingleFile),
        Some(crate::provenance::ExecutableProvenance::from_invocation(
            Path::new("python"),
            &single_file_args,
        ))
    );
    assert_eq!(
        adapter.executable_provenance_for_mode(crate::config::BenchmarkMode::Batch),
        Some(crate::provenance::ExecutableProvenance::from_invocation(
            Path::new("python"),
            &batch_args,
        ))
    );
}

#[test]
fn liteparse_single_mode_records_wrapper_invocation() {
    let adapter = SubprocessAdapter::with_batch_capability(
        "liteparse",
        "bash",
        vec!["liteparse_extract.sh".to_string()],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::LiteparseBatchParse,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: false,
        },
    );

    let provenance = adapter
        .executable_provenance_for_mode(crate::config::BenchmarkMode::SingleFile)
        .unwrap();
    assert_eq!(provenance.name, "bash");
    assert!(!provenance.invocation_blake3.is_empty());
}

#[test]
fn configured_external_ocr_status_is_used_when_output_has_no_metadata() {
    let enabled =
        SubprocessAdapter::new("docling", "echo", vec![], vec![], vec!["pdf".to_string()]).with_configured_ocr(true);
    let disabled =
        SubprocessAdapter::new("docling", "echo", vec![], vec![], vec!["pdf".to_string()]).with_configured_ocr(false);

    assert_eq!(enabled.resolve_ocr_status(None, false), OcrStatus::Used);
    assert_eq!(disabled.resolve_ocr_status(None, false), OcrStatus::NotUsed);
    assert_eq!(disabled.resolve_ocr_status(None, true), OcrStatus::Used);
}

#[test]
fn batch_worker_builder_uses_requested_nonzero_limit() {
    let requested =
        SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_batch_workers(7);
    let zero = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_batch_workers(0);

    assert_eq!(requested.batch_workers, 7);
    assert_eq!(zero.batch_workers, 1);
}

#[test]
fn xberg_thread_budget_builder_uses_requested_nonzero_limit() {
    let requested =
        SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_xberg_max_threads(9);
    let zero =
        SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_xberg_max_threads(0);

    assert_eq!(requested.xberg_max_threads, Some(9));
    assert_eq!(zero.xberg_max_threads, Some(1));
    assert_eq!(requested.configured_thread_budget(), None);
}

#[test]
fn xberg_thread_budget_defaults_to_batch_workers() {
    let adapter = SubprocessAdapter::new("test", "echo", vec![], vec![], vec!["pdf".to_string()]).with_batch_workers(7);

    assert_eq!(adapter.xberg_max_threads, None);
    assert_eq!(adapter.effective_xberg_max_threads(), 7);
    assert_eq!(adapter.configured_thread_budget(), None);
}

#[test]
fn xberg_worker_provenance_does_not_guess_dynamic_document_concurrency() {
    let adapter = SubprocessAdapter::with_batch_capability(
        "xberg-test",
        "echo",
        vec![],
        vec![],
        vec!["pdf".to_string()],
        BatchCapability {
            entry_point: BatchEntryPoint::XbergCliExtractBatch,
            timing_scope: crate::types::BatchTimingScope::ColdEndToEndSubprocess,
            per_item_timing: true,
        },
    )
    .with_batch_workers(4);

    assert_eq!(adapter.worker_provenance(4), (Some(4), None));
    assert_eq!(adapter.configured_thread_budget(), Some(4));
}
