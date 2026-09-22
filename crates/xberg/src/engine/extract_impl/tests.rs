use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;

use tempfile::tempdir;

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::core::config::concurrency::LayoutBatchWorkload;

#[test]
fn extraction_cache_namespaces_invalidate_pre_f32_pdf_results() {
    assert_eq!(CACHE_KEY_NAMESPACE, b"xberg-engine-extract-v2");
    assert_eq!(BATCH_CACHE_KEY_NAMESPACE, b"xberg-engine-extract-batch-v2");
}

#[tokio::test]
async fn extract_bytes_input_returns_envelope() {
    let config = ExtractionConfig::default();
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None), &config)
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
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
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
    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(format!("file://{}", path.display())), &config)
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
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("local filesystem path inputs are disabled"));
}

#[tokio::test]
async fn extract_rejects_non_local_file_uri_host() {
    let config = ExtractionConfig::default();
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri("file://evilhost/tmp/doc.txt"), &config)
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
    let output = crate::engine::Engine::new_default()
        .extract(
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
    let error = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri("s3://bucket/file.txt"), &config)
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
    assert_eq!(output.errors[0].code, 1010);
    assert_eq!(output.errors[0].error_type, "unsupported_format");
}

#[tokio::test]
async fn extract_batch_applies_item_timeout() {
    let item = run_batch_item(0, "<test>".to_string(), Some(1), None, || async {
        std::future::pending::<()>().await;
        Ok(ExtractionResult::default())
    })
    .await;

    let error = item.result.unwrap_err();
    assert_eq!(error_code(&error), 1014);
    assert_eq!(error_type(&error), "timeout");
}

#[test]
fn should_map_every_variant_to_its_canonical_code_and_type() {
    let cases: [(XbergError, u32, &str); 18] = [
        (XbergError::Io(std::io::Error::other("t")), 1000, "io"),
        (XbergError::parsing("t"), 1001, "parsing"),
        (XbergError::ocr("t"), 1002, "ocr"),
        (XbergError::validation("t"), 1003, "validation"),
        (XbergError::cache("t"), 1004, "cache"),
        (XbergError::image_processing("t"), 1005, "image_processing"),
        (XbergError::serialization("t"), 1006, "serialization"),
        (
            XbergError::MissingDependency("t".to_string()),
            1007,
            "missing_dependency",
        ),
        (
            XbergError::Plugin {
                message: "t".to_string(),
                plugin_name: "p".to_string(),
            },
            1008,
            "plugin",
        ),
        (XbergError::LockPoisoned("t".to_string()), 1009, "lock_poisoned"),
        (
            XbergError::UnsupportedFormat("t/mime".to_string()),
            1010,
            "unsupported_format",
        ),
        (XbergError::embedding("t"), 1011, "embedding"),
        (XbergError::reranking("t"), 1012, "reranking"),
        (XbergError::transcription("t"), 1013, "transcription"),
        (
            XbergError::Timeout {
                elapsed_ms: 1,
                limit_ms: 2,
            },
            1014,
            "timeout",
        ),
        (XbergError::Cancelled, 1015, "cancelled"),
        (XbergError::security("t"), 1016, "security"),
        (XbergError::Other("t".to_string()), 1017, "other"),
    ];

    for (error, expected_code, expected_type) in cases {
        assert_eq!(error_code(&error), expected_code);
        assert_eq!(error_type(&error), expected_type);
    }
}

#[tokio::test]
async fn batch_scheduler_prioritizes_larger_inputs() {
    let directory = tempdir().unwrap();
    let large_path = directory.path().join("large.pdf");
    let mut large_file = File::create(&large_path).unwrap();
    large_file.write_all(&[0; 32]).unwrap();

    let pending = [
        (0, ExtractInput::from_bytes([0], "application/pdf", None), "small"),
        (1, ExtractInput::from_uri(large_path.to_string_lossy()), "large"),
        (2, ExtractInput::from_bytes([0; 8], "application/pdf", None), "medium"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [1, 2, 0]
    );
}

#[tokio::test]
async fn batch_scheduler_respects_per_input_local_policy() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("large.pdf");
    File::create(&path).unwrap().write_all(&[0; 32]).unwrap();
    let mut denied = ExtractInput::from_uri(path.to_string_lossy());
    denied.config = Some(crate::core::config::FileExtractionConfig {
        url: Some(crate::core::config::UrlExtractionConfig {
            allow_local_file_inputs: false,
            ..Default::default()
        }),
        ..Default::default()
    });
    let pending = [
        (0, ExtractInput::from_bytes([0], "application/pdf", None), "small"),
        (1, denied, "denied"),
        (2, ExtractInput::from_bytes([0; 8], "application/pdf", None), "medium"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [2, 1, 0]
    );
}

#[tokio::test]
async fn batch_scheduler_preserves_tie_order_and_remote_slots() {
    let pending = [
        (0, ExtractInput::from_bytes([0; 8], "application/pdf", None), "first"),
        (1, ExtractInput::from_uri("https://example.com/a.pdf"), "remote"),
        (2, ExtractInput::from_bytes([0; 32], "application/pdf", None), "large"),
        (3, ExtractInput::from_bytes([0; 8], "application/pdf", None), "second"),
    ]
    .into_iter()
    .map(|(index, input, source)| (index, input, source.to_string()))
    .collect();

    let scheduled = prioritize_pending_batch_items(pending, &ExtractionConfig::default()).await;

    assert_eq!(
        scheduled.iter().map(|(index, _, _)| *index).collect::<Vec<_>>(),
        [2, 1, 0, 3]
    );
}

#[test]
fn batch_scheduler_prioritizes_only_when_work_will_queue() {
    assert!(!should_prioritize_pending_batch_items(4, 1));
    assert!(!should_prioritize_pending_batch_items(4, 4));
    assert!(should_prioritize_pending_batch_items(5, 4));
}

#[test]
fn batch_scheduler_does_not_probe_disallowed_local_inputs() {
    let bare = ExtractInput::from_uri("/private/automount/doc.pdf");
    let file_uri = ExtractInput::from_uri("file:///private/automount/doc.pdf");
    let mut config = ExtractionConfig::default();
    config.url.allow_local_file_inputs = false;
    config.url.allow_file_uris = false;

    assert_eq!(local_batch_path(&bare, &config), None);
    assert_eq!(local_batch_path(&file_uri, &config), None);
}

#[tokio::test]
async fn batch_scheduler_restores_public_result_order_after_prioritizing() {
    let directory = tempdir().unwrap();
    let contents = ["small".to_string(), "large ".repeat(32), "medium medium".to_string()];
    let mut inputs = Vec::new();
    for (index, content) in contents.iter().enumerate() {
        let path = directory.path().join(format!("{index}.txt"));
        File::create(&path).unwrap().write_all(content.as_bytes()).unwrap();
        inputs.push(ExtractInput::from_uri(path.to_string_lossy()));
    }

    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(2),
            max_concurrent_ocr: None,
        }),
        max_concurrent_extractions: Some(2),
        ..Default::default()
    };
    let output = crate::engine::Engine::new_default()
        .extract_batch(inputs, &config)
        .await
        .unwrap();

    assert!(output.errors.is_empty());
    assert_eq!(
        output
            .results
            .iter()
            .map(|document| document.content.trim())
            .collect::<Vec<_>>(),
        contents.iter().map(|content| content.trim()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn bounded_batch_scheduler_caps_in_flight_tasks() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let pending = (0..8).collect::<VecDeque<_>>();
    let completed = run_bounded_batch_tasks(pending, 2, {
        let active = Arc::clone(&active);
        let peak = Arc::clone(&peak);
        move |index| {
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            async move {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                BatchItemResult {
                    index,
                    source: index.to_string(),
                    result: Ok(ExtractionResult::default()),
                }
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(completed.len(), 8);
    assert_eq!(peak.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn bounded_batch_scheduler_preserves_completion_and_error_indices() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let stage = Arc::new(AtomicUsize::new(0));
    let pending = (0..3).collect::<VecDeque<_>>();
    let completed = run_bounded_batch_tasks(pending, 3, {
        let stage = Arc::clone(&stage);
        move |index| {
            let stage = Arc::clone(&stage);
            async move {
                let prerequisite = match index {
                    0 => 2,
                    2 => 1,
                    _ => 0,
                };
                while stage.load(Ordering::SeqCst) < prerequisite {
                    tokio::task::yield_now().await;
                }
                stage.fetch_add(1, Ordering::SeqCst);
                let result = if index == 1 {
                    Err(XbergError::Other("indexed failure".to_string()))
                } else {
                    Ok(ExtractionResult::default())
                };
                BatchItemResult {
                    index,
                    source: index.to_string(),
                    result,
                }
            }
        }
    })
    .await
    .unwrap();

    assert_eq!(completed.iter().map(|item| item.index).collect::<Vec<_>>(), [1, 2, 0]);
    assert!(completed[0].result.is_err());
    assert_eq!(completed[0].source, "1");
}

#[test]
#[cfg(layout_detection)]
fn engine_batch_execution_plan_matches_layout_aware_resolution() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let non_layout = resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::None, 8);
    assert_eq!(non_layout.workers, 4);
    assert_eq!(non_layout.thread_budget, 1);
    let layout = resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::All, 8);
    assert_eq!(layout.workers, 1);
    assert_eq!(layout.thread_budget, 4);

    let explicit = ExtractionConfig {
        max_concurrent_extractions: Some(2),
        ..config
    };
    let layout_explicit = resolve_engine_batch_execution_plan_for(&explicit, LayoutBatchWorkload::All, 8);
    assert_eq!(layout_explicit.workers, 1);
    assert_eq!(layout_explicit.thread_budget, 4);
    let non_layout_explicit = resolve_engine_batch_execution_plan_for(&explicit, LayoutBatchWorkload::None, 8);
    assert_eq!(non_layout_explicit.workers, 2);
    assert_eq!(non_layout_explicit.thread_budget, 2);
}

#[test]
fn engine_batch_base_config_applies_plan_budget_once() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    });

    let adjusted = resolve_batch_base_config(&base, 2);
    assert_eq!(
        adjusted.concurrency.as_ref().and_then(|config| config.max_threads),
        Some(2)
    );
    assert!(!Arc::ptr_eq(&base, &adjusted));

    let reused = resolve_batch_base_config(&adjusted, 2);
    assert!(Arc::ptr_eq(&adjusted, &reused));
}

/// Only the thread budget is divided across batch workers. `max_concurrent_ocr`
/// is a bound on host memory, which every worker shares, so dividing it would be
/// wrong and dropping it silently returns the caller to the automatic limit --
/// the exact setting they reached for `max_concurrent_ocr` to escape.
#[test]
fn engine_batch_base_config_carries_the_configured_recognition_limit() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });

    let adjusted = resolve_batch_base_config(&base, 2);
    let concurrency = adjusted
        .concurrency
        .as_ref()
        .expect("batch config keeps a concurrency block");
    assert_eq!(concurrency.max_threads, Some(2), "the thread budget is the divided one");
    assert_eq!(
        concurrency.max_concurrent_ocr,
        Some(3),
        "the caller's recognition limit survives the budget rewrite"
    );
}

/// Regression test for task #709: `resolve_input_config` is the single choke point
/// both `extract_one` (the single-input `extract`/`extract_batch_sequential` path) and
/// the shared-URL-group construction in `extract_batch_concurrent` go through.
/// Installing the internal cancellation token here — before either `run_batch_item`'s
/// or `finalize_shared_item`'s own timeout wrapper races the extraction — guarantees
/// `token.cancel()` at those call sites has the SAME token this config's extractor
/// checkpoints observe, not a disconnected one.
#[test]
fn resolve_input_config_installs_a_cancel_token_when_a_timeout_is_configured() {
    let base = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: None,
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_input_config(&input, &base);

    assert!(
        resolved.cancel_token.is_some(),
        "resolve_input_config must install an internal token when a timeout is configured"
    );
}

/// A caller-supplied token (the REST cancel path, `DELETE /jobs/{id}`) must survive
/// `resolve_input_config` unchanged, so it keeps working exactly as before.
#[test]
fn resolve_input_config_preserves_a_caller_supplied_cancel_token() {
    let supplied = crate::cancellation::CancellationToken::new();
    let base = ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: Some(supplied.clone()),
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_input_config(&input, &base);

    supplied.cancel();
    assert!(
        resolved.cancel_token.expect("token must survive").is_cancelled(),
        "resolve_input_config must keep the caller's own token (a clone of the same Arc), \
         not swap in an unrelated one that never observes the caller's cancel() call"
    );
}

/// Companion to `resolve_input_config`'s tests above, for the concurrent-batch path:
/// `resolve_batch_input_config` feeds both `run_batch_item`'s cancel_token argument and
/// the `resolved_config` passed to `extract_one_resolved`, so it must install the same
/// guarantee without breaking the existing Arc-reuse fast path when nothing changed.
#[test]
fn resolve_batch_input_config_shares_the_arc_when_nothing_needs_installing() {
    let base = Arc::new(ExtractionConfig {
        extraction_timeout_secs: None,
        cancel_token: None,
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(base.concurrency.as_ref());

    let resolved = resolve_batch_input_config(&input, &base, thread_budget);

    assert!(
        Arc::ptr_eq(&resolved, &base),
        "no timeout, no cancel_token need, and no thread-budget change must reuse the \
         SAME Arc, not clone the config"
    );
}

#[test]
fn resolve_batch_input_config_installs_a_cancel_token_when_a_timeout_is_configured() {
    let base = Arc::new(ExtractionConfig {
        extraction_timeout_secs: Some(30),
        cancel_token: None,
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    let thread_budget = crate::core::config::concurrency::resolve_thread_budget(base.concurrency.as_ref());

    let resolved = resolve_batch_input_config(&input, &base, thread_budget);

    assert!(
        resolved.cancel_token.is_some(),
        "resolve_batch_input_config must install an internal token when a timeout is \
         configured, even though nothing else forced a clone"
    );
}

/// The per-input companion to `engine_batch_base_config_carries_the_configured_recognition_limit`.
/// Both resolvers rewrite the concurrency block when the batch divides the thread
/// budget, and `resolve_batch_input_config` is the one every batch item goes
/// through, so a caller's `max_concurrent_ocr` has to survive the rewrite here
/// too. Dropping it returns recognition to the automatic limit, which is the
/// setting the caller reached for the field to escape.
#[test]
fn resolve_batch_input_config_carries_the_configured_recognition_limit() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });
    let input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);

    let resolved = resolve_batch_input_config(&input, &base, 2);

    let concurrency = resolved
        .concurrency
        .as_ref()
        .expect("batch config keeps a concurrency block");
    assert_eq!(concurrency.max_threads, Some(2), "the thread budget is the divided one");
    assert_eq!(
        concurrency.max_concurrent_ocr,
        Some(3),
        "the caller's recognition limit survives the budget rewrite"
    );
}

/// An input that carries its own overrides takes the clone-and-merge path rather
/// than the `Arc::clone` fast path, and the budget rewrite then runs on the merged
/// config. `FileExtractionConfig` has no concurrency block of its own, so the
/// base's recognition limit is the one that must come out the far side.
#[test]
fn resolve_batch_input_config_carries_the_recognition_limit_through_a_file_override() {
    let base = Arc::new(ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: Some(3),
        }),
        ..Default::default()
    });
    let mut input = ExtractInput::from_bytes(b"hello".to_vec(), "text/plain", None);
    input.config = Some(crate::core::config::FileExtractionConfig {
        force_ocr: Some(true),
        ..Default::default()
    });

    let resolved = resolve_batch_input_config(&input, &base, 2);

    assert!(resolved.force_ocr, "the per-input override is applied");
    assert_eq!(
        resolved.concurrency.as_ref().and_then(|c| c.max_concurrent_ocr),
        Some(3),
        "the recognition limit survives the override merge and the budget rewrite"
    );
}

#[test]
fn engine_batch_execution_plan_clamps_explicit_zero_to_one() {
    let config = ExtractionConfig {
        max_concurrent_extractions: Some(0),
        ..Default::default()
    };

    assert_eq!(
        resolve_engine_batch_execution_plan_for(&config, LayoutBatchWorkload::None, 8).workers,
        1
    );
}

#[test]
fn engine_batch_execution_plan_without_layout_respects_input_count() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput::default()];

    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 1);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_all_markdown_pdfs_for_single_layout_worker() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: true,
        disable_ocr: true,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.pdf"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::All);
    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 1);
    assert_eq!(plan.thread_budget, 8);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_disabled_layout_as_none_when_ocr_is_disabled() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: false,
        disable_ocr: true,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.pdf"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::None);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 4);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_partial_input_layout_override_as_mixed() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        use_layout_for_markdown: true,
        disable_ocr: true,
        ..Default::default()
    };
    let layout_input = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..ExtractInput::from_uri("layout.pdf")
    };
    let inputs = vec![
        layout_input,
        ExtractInput::from_uri("plain.pdf"),
        ExtractInput::from_uri("plain.pdf"),
        ExtractInput::from_uri("plain.pdf"),
    ];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::Mixed);
    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 2);
    assert_eq!(plan.thread_budget, 4);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_ocr_capable_layout_as_mixed() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        layout: Some(Default::default()),
        use_layout_for_markdown: false,
        disable_ocr: false,
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("image.png"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::Mixed);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 2);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_classifies_ordinary_batch_as_non_layout() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput::from_uri("document.txt"); 4];

    assert_eq!(classify_layout_batch(&config, &inputs), LayoutBatchWorkload::None);
    assert_eq!(resolve_engine_batch_execution_plan(&config, &inputs).workers, 4);
}

#[cfg(all(layout_detection, feature = "url-ingestion"))]
#[test]
fn engine_batch_plan_ignores_shared_url_count_and_layout_overrides() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(8),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let shared = ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..ExtractInput::from_uri("https://example.com/document.pdf")
    };
    assert!(shared_group_uri(&shared).is_some());

    let local = ExtractInput::from_uri("local.pdf");
    let all_plan = resolve_engine_batch_execution_plan(&config, &[shared, local.clone()]);
    assert_eq!(all_plan.workers, 2);
    assert_eq!(all_plan.thread_budget, 4);

    let pending = VecDeque::from([(1, local, "local.pdf".to_string())]);
    let pending_plan = resolve_pending_batch_execution_plan(&config, &pending);
    assert_eq!(pending_plan.workers, 1);
    assert_eq!(pending_plan.thread_budget, 8);
}

#[cfg(layout_detection)]
#[test]
fn engine_batch_concurrency_detects_per_input_layout_override() {
    let config = ExtractionConfig {
        concurrency: Some(crate::core::config::ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        }),
        ..Default::default()
    };
    let inputs = vec![ExtractInput {
        config: Some(crate::core::config::FileExtractionConfig {
            layout: Some(Default::default()),
            ..Default::default()
        }),
        ..Default::default()
    }];

    let plan = resolve_engine_batch_execution_plan(&config, &inputs);
    assert_eq!(plan.workers, 1);
    assert_eq!(plan.thread_budget, 4);
}

#[cfg(feature = "url-ingestion")]
#[tokio::test]
async fn url_markdown_page_runs_through_pipeline_and_preserves_source_mime() {
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
        "",
        links,
        &config,
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(result.uris.as_ref().map(Vec::len), Some(1));
}

#[cfg(feature = "url-ingestion")]
#[tokio::test]
async fn url_page_rejects_untrusted_content_type_as_public_mime() {
    let result = run_url_page_pipeline(
        "safe content".to_string(),
        true,
        "text/html\r\nx-injected: value",
        "",
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
}

/// GH CI E2E `test_metadata_access`: a crawled page is restamped `text/html`, so
/// `metadata.format.html` must be populated even though the extraction itself ran over
/// crawlberg's pre-rendered markdown and never touched the HTML extractor.
#[cfg(all(feature = "url-ingestion", feature = "html"))]
#[tokio::test]
async fn url_html_page_recovers_format_metadata_from_source_html_when_content_is_markdown() {
    let source_html = "<html><head><title>Simple Table Test</title></head><body><h1>Heading</h1></body></html>";

    let result = run_url_page_pipeline(
        "# Heading\n\nalpha beta gamma".to_string(),
        true,
        "text/html",
        source_html,
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert_eq!(result.mime_type, "text/html");
    let Some(crate::types::FormatMetadata::Html(html_metadata)) = result.metadata.format else {
        panic!("expected FormatMetadata::Html; got {:?}", result.metadata.format);
    };
    assert_eq!(html_metadata.title.as_deref(), Some("Simple Table Test"));
}

/// Negative control for the test above: with no source HTML to recover from, the format field
/// stays `None` rather than being invented.
#[cfg(all(feature = "url-ingestion", feature = "html"))]
#[tokio::test]
async fn url_page_without_source_html_leaves_format_metadata_unset() {
    let result = run_url_page_pipeline(
        "alpha beta gamma".to_string(),
        true,
        "text/html",
        "",
        Vec::new(),
        &ExtractionConfig::default(),
    )
    .await
    .unwrap();

    assert!(
        result.metadata.format.is_none(),
        "format must stay None with no HTML to read; got {:?}",
        result.metadata.format
    );
}

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

    let output = crate::engine::Engine::new_default()
        .extract(ExtractInput::from_uri(path.to_string_lossy()), &config)
        .await
        .unwrap();

    assert_eq!(output.results.len(), 1, "expected one result");
    assert_eq!(
        output.results[0].mime_type, "text/x-source-code",
        "Python file must extract as text/x-source-code"
    );
    assert!(output.results[0].content.len() >= 5, "content must be non-trivial");
}

#[test]
fn downloaded_specific_http_mime_remains_authoritative_under_every_policy() {
    for policy in [
        crate::MimeDetectionPolicy::PreferContent,
        crate::MimeDetectionPolicy::TrustExtension,
        crate::MimeDetectionPolicy::ContentOnly,
    ] {
        let config = ExtractionConfig {
            mime_detection_policy: policy,
            ..Default::default()
        };
        let resolved = resolve_bytes_mime_type(
            Some("application/pdf"),
            Some("document.txt"),
            br#"{"kind":"json"}"#,
            &config,
        )
        .unwrap();
        assert_eq!(resolved, "application/pdf", "unexpected MIME for {policy:?}");
    }
}

#[test]
fn downloaded_octet_stream_uses_policy_with_derived_filename() {
    let cases = [
        (crate::MimeDetectionPolicy::PreferContent, "application/json"),
        (crate::MimeDetectionPolicy::TrustExtension, "text/plain"),
        (crate::MimeDetectionPolicy::ContentOnly, "application/json"),
    ];
    for (policy, expected) in cases {
        let config = ExtractionConfig {
            mime_detection_policy: policy,
            ..Default::default()
        };
        let resolved = resolve_bytes_mime_type(
            Some("application/octet-stream"),
            Some("download.txt"),
            br#"{"kind":"json"}"#,
            &config,
        )
        .unwrap();
        assert_eq!(resolved, expected, "unexpected MIME for {policy:?}");
    }
}

#[test]
fn prefer_content_bytes_falls_back_from_unsupported_specialized_extension_to_plain_text() {
    let config = ExtractionConfig {
        mime_detection_policy: crate::MimeDetectionPolicy::PreferContent,
        ..Default::default()
    };

    let resolved = resolve_bytes_mime_type(None, Some("feed.atom"), b"ordinary prose without markup", &config).unwrap();

    assert_eq!(resolved, "text/plain");
}

#[test]
fn content_only_bytes_ignores_a_supported_filename_extension() {
    let config = ExtractionConfig {
        mime_detection_policy: crate::MimeDetectionPolicy::ContentOnly,
        ..Default::default()
    };

    let resolved = resolve_bytes_mime_type(None, Some("document.txt"), br#"{"kind":"content"}"#, &config).unwrap();

    assert_eq!(resolved, "application/json");
}

#[cfg(feature = "tree-sitter")]
#[test]
fn generic_text_keeps_tree_sitter_content_detection_without_bypassing_trusted_extensions() {
    use crate::core::config::TreeSitterConfig;

    let source = b"#!/usr/bin/env python3\nprint('hello')\n";
    let mut config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig::default()),
        ..Default::default()
    };
    let detected = resolve_bytes_mime_type(None, Some("script.unknown"), source, &config).unwrap();
    assert_eq!(detected, "text/x-source-code");

    config.mime_detection_policy = crate::MimeDetectionPolicy::TrustExtension;
    let trusted = resolve_bytes_mime_type(None, Some("script.txt"), source, &config).unwrap();
    assert_eq!(trusted, "text/plain");
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

    assert!(items.iter().all(Option::is_some), "every shared slot must be filled");
    let filled = items[1].as_ref().expect("slot 1 filled");
    assert_eq!(filled.index, 1);
    assert_eq!(filled.source, "http://b/");
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

#[cfg(all(feature = "tokio-runtime", feature = "url-ingestion"))]
#[tokio::test]
async fn shared_url_duration_includes_fetch_without_extending_conversion_timeout() {
    let config = ExtractionConfig {
        extraction_timeout_secs: Some(1),
        ..ExtractionConfig::default()
    };
    let shared = SharedUrlItem {
        index: 0,
        source: "http://example.com/".into(),
        uri: "http://example.com/".into(),
        config,
    };
    let batch_started = Instant::now() - std::time::Duration::from_millis(25);
    let conversion = async { Ok(ExtractionResult::single(ExtractedDocument::default())) };

    let item = finalize_shared_item(&shared, batch_started, conversion).await;

    let output = item.result.expect("immediate conversion remains within its timeout");
    assert_eq!(output.results.len(), 1);
    assert!(
        output.results[0].metadata.extraction_duration_ms.unwrap_or_default() >= 25,
        "duration must include time before conversion began"
    );
}
