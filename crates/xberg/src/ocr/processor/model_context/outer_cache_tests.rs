use super::ModelContext;
use crate::core::config::{ExtractInput, ExtractionConfig, OcrConfig};
use crate::engine::seams::CacheBackend;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const PDF: &[u8] = include_bytes!("../../../../tests/fixtures/ocr/rotated_financial_table.pdf");
const UNCERTAIN: &str = "This valid seeded transcript was produced while the model revision changed.";
const STABLE: &str = "This valid seeded transcript was produced by a new stable model operation.";

#[derive(Clone, Copy)]
enum Mutation {
    DuringUse,
    BeforeStart,
    #[cfg(unix)]
    ReadError,
}

struct ChangingBackend {
    models: std::path::PathBuf,
    mutation: Mutation,
    calls: AtomicUsize,
}

impl Plugin for ChangingBackend {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for ChangingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        assert!(
            !image.is_empty(),
            "the actual PDF page must reach the controlled backend"
        );
        let first = self.calls.fetch_add(1, Ordering::SeqCst) == 0;
        if first {
            self.observe_change();
        }
        Ok(ExtractedDocument {
            content: if first { UNCERTAIN } else { STABLE }.into(),
            ..Default::default()
        })
    }
}

impl ChangingBackend {
    fn observe_change(&self) {
        let osd = self.models.join("osd.traineddata");
        #[cfg(unix)]
        if matches!(self.mutation, Mutation::ReadError) {
            std::fs::remove_file(&osd).unwrap();
            std::os::unix::fs::symlink(&osd, &osd).unwrap();
            assert!(ModelContext::for_path(self.models.to_str().unwrap().into(), 1).is_err());
            std::fs::remove_file(&osd).unwrap();
            std::fs::write(&osd, b"revision one").unwrap();
            return;
        }
        let starts_changed = matches!(self.mutation, Mutation::BeforeStart);
        if starts_changed {
            std::fs::write(&osd, b"revision two").unwrap();
        }
        let snapshot = ModelContext::for_path(self.models.to_str().unwrap().into(), 1).unwrap();
        if !starts_changed {
            std::fs::write(&osd, b"revision two").unwrap();
            assert!(snapshot.if_current(|| ()).is_none(), "the inner guard must observe B");
        } else {
            let _ = snapshot.if_current(|| ());
        }
        std::fs::write(&osd, b"revision one").unwrap();
    }
}

#[derive(Default)]
struct CountingCache {
    entries: Mutex<HashMap<String, Vec<u8>>>,
    puts: AtomicUsize,
}

#[async_trait::async_trait]
impl CacheBackend for CountingCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.entries.lock().unwrap().get(key).cloned()
    }
    async fn put(&self, key: &str, value: Vec<u8>, _: Option<std::time::Duration>) {
        self.entries.lock().unwrap().insert(key.into(), value);
        self.puts.fetch_add(1, Ordering::SeqCst);
    }
}

fn setup(mutation: Mutation) -> (tempfile::TempDir, Arc<ChangingBackend>, ExtractionConfig) {
    let root = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        ("eng", b"language pack".as_slice()),
        ("osd", b"revision one".as_slice()),
    ] {
        std::fs::write(root.path().join(format!("{name}.traineddata")), bytes).unwrap();
    }
    let backend = Arc::new(ChangingBackend {
        models: root.path().into(),
        mutation,
        calls: AtomicUsize::new(0),
    });
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: true,
        enable_quality_processing: false,
        cache_namespace: Some(root.path().file_name().unwrap().to_str().unwrap().replace('.', "-")),
        ocr: Some(OcrConfig {
            backend: backend.name().into(),
            tessdata_path: Some(root.path().into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    (root, backend, config)
}

async fn assert_engine_skips_observed_mutation(batch: bool, mutation: Mutation) {
    let (_root, backend, config) = setup(mutation);
    let cache = Arc::new(CountingCache::default());
    let engine = crate::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    crate::plugins::registry::with_test_ocr_backend(backend.clone(), async {
        for expected in [UNCERTAIN, STABLE, STABLE] {
            let input = ExtractInput::from_bytes(PDF.to_vec(), "application/pdf", None);
            let output = if batch {
                engine.extract_batch(vec![input], &config).await
            } else {
                engine.extract(input, &config).await
            }
            .unwrap();
            assert!(output.errors.is_empty(), "{:?}", output.errors);
            assert_eq!(output.results.len(), 1);
            assert!(
                output.results[0].content.contains(expected),
                "{}",
                output.results[0].content
            );
        }
    })
    .await;
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "uncertain output must not become an outer hit"
    );
    assert_eq!(
        cache.puts.load(Ordering::SeqCst),
        1,
        "only the fresh stable operation may publish"
    );
}

#[tokio::test]
async fn observed_mutation_then_rollback_does_not_publish_engine_single() {
    assert_engine_skips_observed_mutation(false, Mutation::DuringUse).await;
}
#[tokio::test]
async fn observed_mutation_then_rollback_does_not_publish_engine_batch() {
    assert_engine_skips_observed_mutation(true, Mutation::DuringUse).await;
}
#[tokio::test]
async fn inner_starting_at_b_does_not_publish_under_outer_a_single() {
    assert_engine_skips_observed_mutation(false, Mutation::BeforeStart).await;
}
#[tokio::test]
async fn inner_starting_at_b_does_not_publish_under_outer_a_batch() {
    assert_engine_skips_observed_mutation(true, Mutation::BeforeStart).await;
}

async fn assert_uri_skips_observed_mutation(mutation: Mutation) {
    let (root, backend, config) = setup(mutation);
    let path = root.path().join("source.pdf");
    std::fs::write(&path, PDF).unwrap();
    crate::plugins::registry::with_test_ocr_backend(backend.clone(), async {
        for expected in [UNCERTAIN, STABLE, STABLE] {
            let output = crate::core::extractor::extract_file(&path, Some("application/pdf"), &config)
                .await
                .unwrap();
            assert!(output.content.contains(expected), "{}", output.content);
        }
    })
    .await;
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "file cache must not retain uncertain output"
    );
}
#[tokio::test]
async fn observed_mutation_then_rollback_does_not_publish_uri() {
    assert_uri_skips_observed_mutation(Mutation::DuringUse).await;
}
#[tokio::test]
async fn inner_starting_at_b_does_not_publish_under_outer_a_uri() {
    assert_uri_skips_observed_mutation(Mutation::BeforeStart).await;
}

#[cfg(unix)]
#[tokio::test]
async fn inner_read_error_then_restoration_does_not_publish_engine_single() {
    assert_engine_skips_observed_mutation(false, Mutation::ReadError).await;
}
#[cfg(unix)]
#[tokio::test]
async fn inner_read_error_then_restoration_does_not_publish_engine_batch() {
    assert_engine_skips_observed_mutation(true, Mutation::ReadError).await;
}
#[cfg(unix)]
#[tokio::test]
async fn inner_read_error_then_restoration_does_not_publish_uri() {
    assert_uri_skips_observed_mutation(Mutation::ReadError).await;
}

#[path = "stable_revision_tests.rs"]
mod stable_revision_tests;
