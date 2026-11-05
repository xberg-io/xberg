use super::{CountingCache, ModelContext, PDF};
use crate::core::config::{ExtractInput, ExtractionConfig, OcrConfig};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use image::ImageEncoder;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RevisionBackend {
    models: PathBuf,
    calls: AtomicUsize,
}

impl RevisionBackend {
    fn revision(&self, psm: u8, language: &str) -> String {
        let internal = crate::ocr::types::TesseractConfig {
            psm,
            language: language.into(),
            tessdata_path: Some(self.models.clone()),
            ..Default::default()
        };
        let snapshot = ModelContext::resolve(&internal).unwrap();
        snapshot
            .if_current(|| snapshot.result_key(&internal))
            .expect("each request observes one stable model revision")
    }
}

impl Plugin for RevisionBackend {
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
impl OcrBackend for RevisionBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        assert!(!image.is_empty(), "real source input must reach the controlled backend");
        let psm = config.tesseract_config.as_ref().unwrap().psm.unwrap();
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExtractedDocument {
            content: format!(
                "The model revision for this complete transcript is {}.",
                self.revision(psm as u8, &config.effective_tesseract_language().join("+"))
            ),
            ..Default::default()
        })
    }
}

#[derive(Clone, Copy)]
enum Route {
    Uri,
    Engine,
    Batch,
}

fn source(pdf: bool) -> (Vec<u8>, &'static str, &'static str) {
    if pdf {
        return (PDF.to_vec(), "application/pdf", "pdf");
    }
    let image = image::RgbImage::from_pixel(32, 24, image::Rgb([123, 124, 125]));
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ColorType::Rgb8.into(),
        )
        .unwrap();
    (bytes, "image/png", "png")
}

async fn stable_revisions(route: Route, pdf: bool) {
    for psm in [1, 12] {
        stable_revision_case(route, pdf, psm, false).await;
    }
}

async fn stable_revision_case(route: Route, pdf: bool, psm: i32, pipeline: bool) {
    let root = tempfile::tempdir().unwrap();
    for language in ["eng", "deu", "fra"] {
        std::fs::write(root.path().join(format!("{language}.traineddata")), b"language pack").unwrap();
    }
    let osd = root.path().join("osd.traineddata");
    std::fs::write(&osd, b"revision one").unwrap();
    let backend = Arc::new(RevisionBackend {
        models: root.path().into(),
        calls: AtomicUsize::new(0),
    });
    let config = revision_config(root.path(), psm, pipeline);
    let (bytes, mime, extension) = source(pdf);
    let path = root.path().join(format!("source.{extension}"));
    std::fs::write(&path, &bytes).unwrap();
    let engine = crate::engine::Engine::builder()
        .with_cache_backend(Arc::new(CountingCache::default()))
        .build();
    crate::plugins::registry::with_test_ocr_backend(backend.clone(), async {
        for revision in [b"revision one", b"revision two"] {
            std::fs::write(&osd, revision).unwrap();
            assert_eq!(std::fs::metadata(&osd).unwrap().len(), 12);
            let expected = backend.revision(psm as u8, if pipeline { "fra" } else { "eng" });
            for _ in 0..2 {
                let content = read(route, &engine, &path, &bytes, mime, &config).await;
                assert!(
                    content.contains(&expected),
                    "PSM {psm}; revision {revision:?}; expected {expected}; observed {content}; backend calls {}",
                    backend.calls.load(Ordering::SeqCst)
                );
            }
        }
    })
    .await;
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        2,
        "unchanged revisions hit; each stable model change misses"
    );
}

async fn read(
    route: Route,
    engine: &crate::engine::Engine,
    path: &std::path::Path,
    bytes: &[u8],
    mime: &str,
    config: &ExtractionConfig,
) -> String {
    if matches!(route, Route::Uri) {
        return crate::core::extractor::extract_file(path, Some(mime), config)
            .await
            .unwrap()
            .content;
    }
    let input = ExtractInput::from_bytes(bytes.to_vec(), mime, None);
    let output = match route {
        Route::Engine => engine.extract(input, config).await,
        Route::Batch => engine.extract_batch(vec![input], config).await,
        Route::Uri => unreachable!(),
    }
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    output.results[0].content.clone()
}

#[tokio::test]
async fn stable_osd_change_invalidates_pdf_uri() {
    stable_revisions(Route::Uri, true).await;
}
#[tokio::test]
async fn stable_osd_change_invalidates_pdf_engine() {
    stable_revisions(Route::Engine, true).await;
}
#[tokio::test]
async fn stable_osd_change_invalidates_pdf_batch() {
    stable_revisions(Route::Batch, true).await;
}
#[tokio::test]
async fn stable_osd_change_invalidates_image_uri() {
    stable_revisions(Route::Uri, false).await;
}
#[tokio::test]
async fn stable_osd_change_invalidates_image_engine() {
    stable_revisions(Route::Engine, false).await;
}
#[tokio::test]
async fn stable_osd_change_invalidates_image_batch() {
    stable_revisions(Route::Batch, false).await;
}

async fn zero_mode_rejected_by_engine(batch: bool) {
    let root = tempfile::tempdir().unwrap();
    let backend = Arc::new(RevisionBackend {
        models: root.path().into(),
        calls: AtomicUsize::new(0),
    });
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let engine = crate::engine::Engine::builder().build();
    crate::plugins::registry::with_test_ocr_backend(backend.clone(), async {
        let input = ExtractInput::from_bytes(PDF.to_vec(), "application/pdf", None);
        let result = if batch {
            engine.extract_batch(vec![input], &config).await
        } else {
            engine.extract(input, &config).await
        };
        let error = result.expect_err("OSD-only mode must not become a public text extraction mode");
        assert!(
            error
                .to_string()
                .contains("PSM 0 is orientation and script detection (OSD) only")
        );
    })
    .await;
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn stable_osd_change_preserves_zero_mode_validation_single() {
    zero_mode_rejected_by_engine(false).await;
}
#[tokio::test]
async fn stable_osd_change_preserves_zero_mode_validation_batch() {
    zero_mode_rejected_by_engine(true).await;
}

fn revision_config(root: &std::path::Path, psm: i32, pipeline: bool) -> ExtractionConfig {
    let mut config = ExtractionConfig {
        force_ocr: true,
        use_cache: true,
        enable_quality_processing: false,
        cache_namespace: Some(root.file_name().unwrap().to_str().unwrap().replace('.', "-")),
        ocr: Some(OcrConfig {
            tessdata_path: Some(root.into()),
            tesseract_config: Some(crate::types::TesseractConfig {
                psm: Some(psm),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    if pipeline {
        let ocr = config.ocr.as_mut().unwrap();
        ocr.language = vec!["deu".into()];
        ocr.tesseract_config.as_mut().unwrap().psm = Some(3);
        ocr.pipeline = Some(
            serde_json::from_value(serde_json::json!({
                "stages": [{
                    "backend": "tesseract", "language": ["eng"],
                    "tesseract_config": {"psm": psm, "language": ["fra"]}
                }]
            }))
            .unwrap(),
        );
    }
    config
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_pdf_uri() {
    stage_revisions(Route::Uri, true).await;
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_pdf_engine() {
    stage_revisions(Route::Engine, true).await;
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_pdf_batch() {
    stage_revisions(Route::Batch, true).await;
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_image_uri() {
    stage_revisions(Route::Uri, false).await;
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_image_engine() {
    stage_revisions(Route::Engine, false).await;
}

#[tokio::test]
async fn pipeline_osd_stage_change_invalidates_image_batch() {
    stage_revisions(Route::Batch, false).await;
}

async fn stage_revisions(route: Route, pdf: bool) {
    for psm in [1, 12] {
        stable_revision_case(route, pdf, psm, true).await;
    }
}
