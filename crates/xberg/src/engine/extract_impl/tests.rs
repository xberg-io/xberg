use std::fs::File;
use std::io::Write;

use tempfile::tempdir;

use super::*;

#[tokio::test]
async fn extract_bytes_input_returns_envelope() {
    let config = ExtractionConfig::default();
    let output = extract(ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.summary.inputs, 1);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.results[0].content.trim(), "hello");
}

#[tokio::test]
async fn extract_local_uri_returns_envelope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello path").unwrap();

    let config = ExtractionConfig::default();
    let output = extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello path");
}

#[tokio::test]
async fn extract_file_uri_returns_envelope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello file uri").unwrap();

    let config = ExtractionConfig::default();
    let output = extract(ExtractInput::from_uri(format!("file://{}", path.display())), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello file uri");
}

#[tokio::test]
async fn extract_rejects_local_path_when_policy_disallows_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello local policy").unwrap();

    let mut config = ExtractionConfig::default();
    config.url.allow_local_file_inputs = false;
    let error = extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("local filesystem path inputs are disabled"));
}

#[tokio::test]
async fn extract_rejects_non_local_file_uri_host() {
    let config = ExtractionConfig::default();
    let error = extract(ExtractInput::from_uri("file://evilhost/tmp/doc.txt"), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("unsupported non-local file URI host"));
}

#[tokio::test]
async fn extract_file_uri_accepts_localhost_host() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path)
        .unwrap()
        .write_all(b"hello localhost file uri")
        .unwrap();

    let config = ExtractionConfig::default();
    let output = extract(
        ExtractInput::from_uri(format!("file://localhost{}", path.display())),
        &config,
    )
    .await
    .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim(), "hello localhost file uri");
}

#[tokio::test]
async fn extract_rejects_unsupported_scheme() {
    let config = ExtractionConfig::default();
    let error = extract(ExtractInput::from_uri("s3://bucket/file.txt"), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("unsupported URI scheme"));
}

#[tokio::test]
async fn extract_batch_collects_mixed_inputs() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    File::create(&path).unwrap().write_all(b"hello batch path").unwrap();

    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract_batch(
            vec![
                ExtractInput::from_bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::from_uri(path.to_string_lossy()),
            ],
            &config,
        )
        .await
        .unwrap();

    assert_eq!(output.results.len(), 2);
    assert_eq!(output.summary.inputs, 2);
    assert!(output.errors.is_empty());
}

#[tokio::test]
async fn extract_batch_collects_unsupported_scheme_error() {
    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract_batch(
            vec![
                ExtractInput::from_bytes(b"hello batch bytes".to_vec(), "text/plain", None),
                ExtractInput::from_uri("s3://bucket/doc.txt"),
            ],
            &config,
        )
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1);
    assert_eq!(output.errors.len(), 1);
    assert_eq!(output.summary.inputs, 2);
    assert_eq!(output.summary.results, 1);
    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.errors[0].index, 1);
    assert_eq!(output.errors[0].code, 1003);
    assert_eq!(output.errors[0].error_type, "unsupported_format");
}

#[tokio::test]
async fn extract_batch_applies_item_timeout() {
    let item = run_batch_item(
        0,
        "<test>".to_string(),
        std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        Some(1),
        None,
        || async {
            std::future::pending::<()>().await;
            Ok(ExtractionResult::default())
        },
    )
    .await;

    let error = item.result.unwrap_err();
    assert_eq!(error_code(&error), 1004);
    assert_eq!(error_type(&error), "timeout");
}

#[cfg(feature = "url-ingestion")]
#[tokio::test]
async fn url_markdown_page_runs_through_pipeline() {
    let config = ExtractionConfig::default();
    let links = vec![ExtractedUri {
        url: "https://example.com/next".to_string(),
        label: Some("next".to_string()),
        page: None,
        kind: UriKind::Hyperlink,
    }];

    let result = run_url_page_pipeline(
        "alpha beta gamma delta epsilon zeta eta theta".to_string(),
        true,
        "text/html; charset=utf-8",
        links,
        &config,
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/markdown");
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(result.uris.as_ref().map(Vec::len), Some(1));
}

// ── end-to-end: .py file extraction via local URI ──────────────────────────
// Proves that the tree-sitter extractor is selected for .py files end-to-end.
// This covers the extractor-selection half of the fix; the mime-refinement half
// (octet-stream + filename → text/x-source-code) is covered below.

#[cfg(feature = "tree-sitter")]
#[tokio::test]
async fn extract_py_local_uri_returns_source_code_mime() {
    use crate::core::config::TreeSitterConfig;

    let dir = tempdir().unwrap();
    let path = dir.path().join("hello.py");
    File::create(&path)
        .unwrap()
        .write_all(b"def greet(name):\n    return f'Hello, {name}!'\n")
        .unwrap();

    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig::default()),
        ..Default::default()
    };

    let output = extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1, "expected one result");
    assert_eq!(
        output.results[0].mime_type, "text/x-source-code",
        "Python file must extract as text/x-source-code"
    );
    assert!(output.results[0].content.len() >= 5, "content must be non-trivial");
}

// ── refine_downloaded_mime_type unit tests ──────────────────────────────────

#[cfg(feature = "url-ingestion")]
#[test]
fn refine_downloaded_mime_type_passthrough_non_octet_stream() {
    // Explicit MIME types from the server must never be overridden, even when
    // the filename extension suggests something different.
    let refined = refine_downloaded_mime_type("application/pdf", Some("document.py"), "http://example.com/document.py");
    assert_eq!(
        refined, "application/pdf",
        "explicit server MIME type must not be overridden by filename"
    );
}

#[cfg(all(feature = "url-ingestion", feature = "tree-sitter"))]
#[test]
fn refine_downloaded_mime_type_py_extension_resolves_to_source_code() {
    // A .py filename served with Content-Type: application/octet-stream must
    // be refined to text/x-source-code via tree-sitter extension detection.
    let refined = refine_downloaded_mime_type(
        "application/octet-stream",
        Some("hello.py"),
        "http://example.com/code/hello.py",
    );
    assert_eq!(
        refined, "text/x-source-code",
        "octet-stream with .py filename must resolve to text/x-source-code"
    );
}

#[cfg(feature = "url-ingestion")]
#[test]
fn refine_downloaded_mime_type_no_filename_returns_octet_stream() {
    // Without a filename hint, fall back to application/octet-stream so
    // extract_bytes can apply content sniffing.
    let refined = refine_downloaded_mime_type("application/octet-stream", None, "http://example.com/download");
    assert_eq!(
        refined, "application/octet-stream",
        "no filename means no refinement; extract_bytes handles sniffing"
    );
}

/// Regression: a shared-URL batch result that maps to no input slot (e.g.
/// crawlberg drops a panicked task as an empty-URL pair) must NOT cause its
/// input to vanish. The sweep fills every unfilled slot with an error so
/// `results + errors == inputs` always holds.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[test]
fn fill_dropped_shared_slots_reattaches_or_synthesizes_errors() {
    use std::collections::VecDeque;

    let shared_items = vec![
        SharedUrlItem {
            index: 0,
            source: "http://a/".into(),
            uri: "http://a/".into(),
            config: ExtractionConfig::default(),
        },
        SharedUrlItem {
            index: 1,
            source: "http://b/".into(),
            uri: "http://b/".into(),
            config: ExtractionConfig::default(),
        },
        SharedUrlItem {
            index: 2,
            source: "http://c/".into(),
            uri: "http://c/".into(),
            config: ExtractionConfig::default(),
        },
    ];
    // Slots 0 and 2 were written by the batch loop; slot 1 was dropped.
    let mut items: Vec<Option<BatchItemResult>> = vec![
        Some(BatchItemResult {
            index: 0,
            source: "http://a/".into(),
            result: Err(crate::XbergError::Other("a".into())),
        }),
        None,
        Some(BatchItemResult {
            index: 2,
            source: "http://c/".into(),
            result: Err(crate::XbergError::Other("c".into())),
        }),
    ];
    let mut unmatched = VecDeque::new();
    unmatched.push_back(crate::XbergError::Other("task panicked: boom".into()));

    fill_dropped_shared_slots(&shared_items, &mut items, unmatched);

    // No input silently dropped: every slot is filled.
    assert!(items.iter().all(Option::is_some), "every shared slot must be filled");
    let filled = items[1].as_ref().expect("slot 1 filled");
    assert_eq!(filled.index, 1);
    assert_eq!(filled.source, "http://b/");
    // The captured panic error was re-attached rather than discarded.
    match &filled.result {
        Err(crate::XbergError::Other(message)) => {
            assert!(message.contains("task panicked: boom"), "got: {message}");
        }
        _ => panic!("expected the re-attached panic error in slot 1"),
    }
}

/// When no unmatched error was captured, the synthesized error names the URL.
#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[test]
fn fill_dropped_shared_slots_synthesizes_when_no_captured_error() {
    use std::collections::VecDeque;

    let shared_items = vec![SharedUrlItem {
        index: 0,
        source: "http://x/".into(),
        uri: "http://x/".into(),
        config: ExtractionConfig::default(),
    }];
    let mut items: Vec<Option<BatchItemResult>> = vec![None];

    fill_dropped_shared_slots(&shared_items, &mut items, VecDeque::new());

    match &items[0].as_ref().expect("slot 0 filled").result {
        Err(crate::XbergError::Other(message)) => {
            assert!(
                message.contains("http://x/"),
                "synthesized error names the URL, got: {message}"
            );
        }
        _ => panic!("expected a synthesized error naming the URL"),
    }
}
