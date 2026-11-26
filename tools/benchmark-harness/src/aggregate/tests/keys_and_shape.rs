//! Aggregate-key shape, format-support matrix, and basic `aggregate_new_format` grouping tests.

use super::support::create_test_result;
use super::*;
use crate::stats::percentile_r7;
use crate::types::OcrStatus;

#[test]
fn format_support_matrix_marks_declared_unsupported_pairs() {
    // Corpus spans pdf/docx/html/rtf; xberg reads all four, liteparse reads pdf and the
    // LibreOffice-converted docx/rtf but not html, and docling reads pdf/docx/html but not rtf.
    let results = vec![
        create_test_result("xberg-markdown-baseline", "pdf", OcrStatus::NotUsed, 10, 1.0, 1024),
        create_test_result("xberg-markdown-baseline", "docx", OcrStatus::NotUsed, 10, 1.0, 1024),
        create_test_result("xberg-markdown-baseline", "html", OcrStatus::NotUsed, 10, 1.0, 1024),
        create_test_result("xberg-markdown-baseline", "rtf", OcrStatus::NotUsed, 10, 1.0, 1024),
        create_test_result("liteparse", "pdf", OcrStatus::NotUsed, 10, 1.0, 1024),
        create_test_result("docling", "pdf", OcrStatus::NotUsed, 10, 1.0, 1024),
    ];

    let aggregated = aggregate_new_format(&results);
    let support = &aggregated.format_support;

    assert_eq!(
        support.file_types,
        vec![
            "docx".to_string(),
            "html".to_string(),
            "pdf".to_string(),
            "rtf".to_string()
        ],
        "file_types must be the sorted, de-duplicated corpus extension set"
    );
    assert!(
        !support.unsupported.contains_key("xberg"),
        "xberg is the subject under test and supports the whole corpus; it must never be marked unsupported"
    );
    assert_eq!(
        support.unsupported.get("liteparse"),
        Some(&vec!["html".to_string()]),
        "liteparse declares docx/rtf through LibreOffice but has no html reader"
    );
    assert_eq!(
        support.unsupported.get("docling"),
        Some(&vec!["rtf".to_string()]),
        "docling reads pdf/docx/html but not rtf"
    );
}

#[test]
fn test_extract_framework_and_mode() {
    assert_eq!(
        extract_framework_and_mode("xberg-markdown-baseline"),
        ("xberg-markdown-baseline", "single")
    );
    assert_eq!(
        extract_framework_and_mode("xberg-plaintext-paddle-ocr"),
        ("xberg-plaintext-paddle-ocr", "single")
    );
    assert_eq!(
        extract_framework_and_mode("xberg-markdown-baseline-batch"),
        ("xberg-markdown-baseline", "batch")
    );

    assert_eq!(extract_framework_and_mode("xberg-sync"), ("xberg", "single"));
    assert_eq!(extract_framework_and_mode("xberg-async"), ("xberg", "single"));

    assert_eq!(extract_framework_and_mode("xberg-batch"), ("xberg", "batch"));
    assert_eq!(extract_framework_and_mode("python-batch"), ("python", "batch"));

    assert_eq!(extract_framework_and_mode("xberg"), ("xberg", "single"));
    assert_eq!(extract_framework_and_mode("docling"), ("docling", "single"));
}

#[test]
fn test_make_aggregate_key_xberg_family() {
    assert_eq!(
        make_aggregate_key("xberg-markdown-baseline", OutputFormat::Markdown, "single"),
        "xberg-markdown-baseline:single"
    );
    assert_eq!(
        make_aggregate_key("xberg-plaintext-layout", OutputFormat::Plaintext, "batch"),
        "xberg-plaintext-layout:batch"
    );
}

#[test]
fn test_make_aggregate_key_competitors() {
    assert_eq!(
        make_aggregate_key("docling", OutputFormat::Markdown, "single"),
        "docling:markdown:single"
    );
    assert_eq!(
        make_aggregate_key("unstructured", OutputFormat::Plaintext, "batch"),
        "unstructured:plaintext:batch"
    );
}

/// Defensive regression test for the aggregate-key design documented on
/// [`make_aggregate_key`]: xberg keys omit `output_format` because the format is already
/// baked into the framework name (`xberg-markdown-baseline` vs `xberg-plaintext-baseline`).
/// This is not a live bug — real xberg framework names never collide — but pins the current
/// safe behavior instead of changing the key format, which would break `bench_matrix`'s
/// pinned exact-key-string tests and the published release-contract keys downstream
/// consumers already depend on.
#[test]
fn xberg_aggregate_keys_never_collide_across_real_framework_name_variants() {
    let xberg_variants = [
        ("xberg-markdown-baseline", OutputFormat::Markdown),
        ("xberg-markdown-layout", OutputFormat::Markdown),
        ("xberg-plaintext-baseline", OutputFormat::Plaintext),
        ("xberg-plaintext-layout", OutputFormat::Plaintext),
        ("xberg-markdown-paddle-ocr", OutputFormat::Markdown),
        ("xberg-plaintext-paddle-ocr", OutputFormat::Plaintext),
    ];

    let mut keys = std::collections::HashSet::new();
    for (framework, format) in xberg_variants {
        for mode in ["single", "batch"] {
            let key = make_aggregate_key(framework, format, mode);
            assert!(keys.insert(key.clone()), "duplicate aggregate key: {key}");
        }
    }
}

/// A same-name-different-format xberg pair *would* collide under the current key format
/// (`{framework}:{mode}`, no format component). This cannot happen with real xberg naming
/// (format is always baked into the name), but pinning the mechanism here makes a future
/// change to it deliberate rather than accidental.
#[test]
fn hypothetical_same_name_different_format_xberg_pair_collides_by_design() {
    let markdown_key = make_aggregate_key("xberg-shared-name", OutputFormat::Markdown, "single");
    let plaintext_key = make_aggregate_key("xberg-shared-name", OutputFormat::Plaintext, "single");
    assert_eq!(
        markdown_key, plaintext_key,
        "xberg keys intentionally omit output_format; real xberg framework names never share \
         a name across formats, so this collision is theoretical, not a live bug"
    );
}

#[test]
fn test_aggregate_new_format_xberg_key_shape() {
    let results = vec![
        create_test_result(
            "xberg-markdown-baseline",
            "pdf",
            OcrStatus::NotUsed,
            100,
            1_000_000.0,
            10_000_000,
        ),
        create_test_result(
            "xberg-markdown-baseline-batch",
            "pdf",
            OcrStatus::NotUsed,
            80,
            1_000_000.0,
            10_000_000,
        ),
    ];

    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.by_framework_mode.len(), 2);
    assert!(
        aggregated
            .by_framework_mode
            .contains_key("xberg-markdown-baseline:single")
    );
    assert!(
        aggregated
            .by_framework_mode
            .contains_key("xberg-markdown-baseline:batch")
    );

    let single_agg = &aggregated.by_framework_mode["xberg-markdown-baseline:single"];
    assert_eq!(single_agg.framework, "xberg-markdown-baseline");
    assert_eq!(single_agg.mode, "single");
}

#[test]
fn test_percentile_r7() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(percentile_r7(&values, 0.0), 1.0);
    assert_eq!(percentile_r7(&values, 0.5), 3.0);
    assert_eq!(percentile_r7(&values, 1.0), 5.0);
    assert_eq!(percentile_r7(&[], 0.5), 0.0);
}

#[test]
fn test_aggregate_new_format() {
    let results = vec![
        create_test_result("xberg-sync", "pdf", OcrStatus::NotUsed, 100, 1_000_000.0, 10_000_000),
        create_test_result("xberg-sync", "pdf", OcrStatus::Used, 200, 500_000.0, 20_000_000),
        create_test_result("xberg-batch", "docx", OcrStatus::NotUsed, 150, 750_000.0, 15_000_000),
    ];

    let aggregated = aggregate_new_format(&results);

    assert_eq!(aggregated.by_framework_mode.len(), 2);
    assert!(aggregated.by_framework_mode.contains_key("xberg:markdown:single"));
    assert!(aggregated.by_framework_mode.contains_key("xberg:markdown:batch"));

    let single_agg = &aggregated.by_framework_mode["xberg:markdown:single"];
    assert_eq!(single_agg.framework, "xberg");
    assert_eq!(single_agg.mode, "single");
    assert!(single_agg.cold_start.is_some());

    let pdf_agg = &single_agg.by_file_type["pdf"];
    assert!(pdf_agg.no_ocr.is_some());
    assert!(pdf_agg.with_ocr.is_some());

    assert_eq!(pdf_agg.no_ocr.as_ref().unwrap().successful_sample_count, 1);
    assert_eq!(pdf_agg.with_ocr.as_ref().unwrap().successful_sample_count, 1);
}
