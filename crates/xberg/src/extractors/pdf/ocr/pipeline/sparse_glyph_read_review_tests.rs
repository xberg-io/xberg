use super::*;
use crate::core::config::{LlmConfig, OcrQualityThresholds, PageConfig, VlmFallbackPolicy};
use crate::plugins::{OcrBackendType, Plugin};

const FOLIO: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pdf/separation-overprint-2817/folio-black.pdf"
));

async fn assert_normal_tesseract_raster(configure: impl FnOnce(&mut OcrConfig)) {
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(
        tessdata.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let mut ocr = OcrConfig {
        tessdata_path: Some(tessdata.path().to_path_buf()),
        quality_thresholds: Some(OcrQualityThresholds {
            pipeline_min_quality: 0.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    configure(&mut ocr);
    let config = test_config(ocr);
    let output = crate::core::extractor::extract_bytes(FOLIO, "application/pdf", &config)
        .await
        .unwrap();
    let [page] = output.pages.as_deref().unwrap() else {
        panic!("expected one OCR page");
    };
    let raster = page
        .image_preprocessing
        .as_ref()
        .expect("normal Tesseract raster metadata");
    assert_eq!(
        (raster.original_dimensions.width, raster.original_dimensions.height),
        (1275, 1650)
    );
    assert!(page.content.contains("quiet queue"));
}

fn test_config(ocr: OcrConfig) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(ocr),
        ..Default::default()
    }
}

#[test]
fn explicit_tesseract_paddle_pipeline_keeps_the_normal_raster() {
    super::super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_normal_tesseract_raster(|ocr| {
            let pipeline = ocr.effective_pipeline().expect("default Paddle-enabled stages");
            assert!(matches!(pipeline.stages.as_slice(), [first, second]
                if first.backend == "tesseract" && second.backend == "paddleocr"));
            ocr.pipeline = Some(pipeline);
        })
    });
}

#[test]
fn explicit_vlm_fallback_keeps_the_normal_raster() {
    super::super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_normal_tesseract_raster(|ocr| {
            ocr.vlm_config = Some(LlmConfig {
                model: "openai/gpt-4o".to_owned(),
                ..Default::default()
            });
            ocr.vlm_fallback = VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.0 };
            let pipeline = ocr.effective_pipeline().expect("explicit VLM fallback stages");
            assert!(matches!(pipeline.stages.as_slice(), [first, second]
                if first.backend == "tesseract" && second.backend == "vlm"));
        })
    });
}

#[derive(Default)]
struct PaddleRasterProbe(std::sync::Mutex<Vec<(u32, u32)>>);

impl Plugin for PaddleRasterProbe {
    fn name(&self) -> &str {
        "paddleocr"
    }
}

#[async_trait::async_trait]
impl OcrBackend for PaddleRasterProbe {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::PaddleOCR
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, bytes: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let image = image::load_from_memory(bytes).unwrap();
        self.0.lock().unwrap().push((image.width(), image.height()));
        Ok(ExtractedDocument {
            content: "The committee met on Tuesday to review the library planning notes.".to_owned(),
            ..Default::default()
        })
    }
}

#[test]
fn explicit_paddle_backend_receives_the_normal_raster() {
    super::super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || async {
        let paddle = Arc::new(PaddleRasterProbe::default());
        let config = test_config(OcrConfig {
            backend: "paddleocr".to_owned(),
            ..Default::default()
        });
        crate::plugins::registry::with_test_ocr_backend(paddle.clone(), async {
            crate::core::extractor::extract_bytes(FOLIO, "application/pdf", &config)
                .await
                .unwrap();
        })
        .await;
        assert_eq!(*paddle.0.lock().unwrap(), [(1275, 1650)]);
    });
}

#[test]
fn sparse_preview_preserves_the_detached_recovery_context() {
    use super::super::super::{number_recovery, orientation_retry, read_reuse};

    super::super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || async {
        let (pdf, _, rotations) = super::super::super::rendering::open_pdf_for_page_ocr(FOLIO).unwrap();
        assert_eq!(rotations, [0]);
        let media = upright_media_box(&pdf, 0).unwrap();
        let pdf = Arc::new(pdf);
        let config = ExtractionConfig::default();
        let number_run = Arc::new(number_recovery::Run::default());
        let probes = Arc::new(read_reuse::ProbeReads::new(1024));
        let orientation = Arc::new(orientation_retry::OrientationRetry::new(&config, Default::default()));
        let trial = PipelineTrial {
            pdf: pdf.clone(),
            index: 0,
            config: config.clone(),
            pipeline: OcrConfig::default().effective_pipeline().unwrap(),
            page: TrialPage {
                rotation: rotations[0],
                hints: super::super::PageOcrHints {
                    number_source: Some(number_recovery::DetachedSource {
                        document: pdf.clone(),
                        run: number_run.clone(),
                        raster: None,
                        layers: Arc::new(ahash::AHashMap::from_iter([(1, "Page 12".to_owned())])),
                        config,
                    }),
                    probe_reads: Some(probes.clone()),
                    detached_recognition_recovery: Some(orientation_retry::RecognitionRecovery::TableOrProse),
                    ..Default::default()
                },
                orientation_retry: Some(orientation.clone()),
            },
            words: MarkedWords {
                media,
                words: Vec::new(),
            },
        };
        let (image, dpi, warnings) = trial.render().await.unwrap();
        assert!(warnings.is_empty());
        let (hints, detached) = trial.read_inputs(image.width(), dpi);
        assert_eq!(hints.source_dpi, Some(120.0));
        assert!(hints.sparse_glyph_trial.is_some());
        assert!(Arc::ptr_eq(hints.probe_reads.as_ref().unwrap(), &probes));
        let source = hints.number_source.as_ref().unwrap();
        assert!(Arc::ptr_eq(&source.run, &number_run));
        assert_eq!(source.layer(1), Some("Page 12"));
        assert!(source.raster.is_some());
        assert!(Arc::ptr_eq(detached.orientation_retry.unwrap(), &orientation));
        assert!(matches!(
            hints.detached_recognition_recovery,
            Some(orientation_retry::RecognitionRecovery::TableOrProse)
        ));
    });
}
