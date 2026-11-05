//! Regression tests for https://github.com/xberg-io/xberg/issues/2339
//!
//! A JPEG file whose full-size raster does not fit `security_limits.max_content_size` failed OCR,
//! and with an explicit `ocr.pipeline` a JPEG also failed when its raster fit but the image encode
//! of each stage did not. OCR now decodes such a JPEG at the finest IDCT scale (1/2, 1/4, 1/8) that
//! fits, and divides the source DPI by that scale. Each test sends JPEG bytes through the public
//! extraction entry point; the limits are scaled-down copies of the issue's 512 MiB and 100 MiB.

#![cfg(feature = "ocr")]

use xberg::core::config::{ExtractInput, ExtractionConfig};

const SCAN_WIDTH: u32 = 2400;
const SCAN_HEIGHT: u32 = 1600;

/// A baseline RGB JPEG of dark bars on white, so the encoded file is small and the raster is not blank.
fn scan_jpeg(width: u32, height: u32) -> Vec<u8> {
    let raster = image::RgbImage::from_fn(width, height, |x, y| {
        if (x / 40 + y / 25) % 3 == 0 {
            image::Rgb([20, 20, 20])
        } else {
            image::Rgb([250, 250, 250])
        }
    });
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode(raster.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .expect("encode the JPEG fixture");
    assert_eq!(image::guess_format(&bytes).expect("fixture format"), image::ImageFormat::Jpeg);
    bytes
}

fn config(value: serde_json::Value) -> ExtractionConfig {
    serde_json::from_value(value).expect("test configuration")
}

async fn extract_jpeg(bytes: Vec<u8>, config: &ExtractionConfig) -> xberg::Result<xberg::ExtractedDocument> {
    let mut batch = xberg::extract(ExtractInput::from_bytes(bytes, "image/jpeg", None), config).await?;
    assert!(batch.errors.is_empty(), "extraction errors: {:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    Ok(batch.results.remove(0))
}

/// The default OCR route: the built-in Tesseract backend decodes the file itself.
mod default_route {
    use super::*;

    fn tesseract_config(max_content_size: usize, source_dpi: f64) -> ExtractionConfig {
        config(serde_json::json!({
            "use_cache": false,
            "security_limits": { "max_content_size": max_content_size },
            "ocr": {
                "backend": "tesseract",
                "backend_options": { "source_dpi": source_dpi },
                "tesseract_config": { "preprocessing": { "target_dpi": 300 } }
            }
        }))
    }

    /// The decoded size and source DPI that OCR reports for the raster it read.
    fn raster_read(document: &xberg::ExtractedDocument) -> ((usize, usize), f64) {
        assert_eq!(
            document.extraction_method,
            Some(xberg::types::ExtractionMethod::Ocr),
            "OCR must run on the image"
        );
        let preprocessing = document
            .metadata
            .image_preprocessing
            .as_ref()
            .expect("explicit preprocessing reports the raster it read");
        (
            (
                preprocessing.original_dimensions.width,
                preprocessing.original_dimensions.height,
            ),
            preprocessing.original_dpi.horizontal,
        )
    }

    /// Control: a JPEG whose raster fits keeps its size and DPI.
    #[tokio::test]
    async fn should_read_a_jpeg_that_fits_at_full_size() {
        let document = extract_jpeg(scan_jpeg(1200, 800), &tesseract_config(8 * 1024 * 1024, 300.0))
            .await
            .expect("a JPEG that fits must extract");
        assert_eq!(raster_read(&document), ((1200, 800), 300.0));
    }

    /// The 100 MiB case of the issue: the 11,520,000-byte raster is over an 8 MiB limit; the half
    /// scale (2,880,000 bytes) fits, and the 600 DPI source becomes a 300 DPI raster.
    #[tokio::test]
    async fn should_read_a_jpeg_over_the_limit_at_the_finest_scale_that_fits() {
        let document = extract_jpeg(
            scan_jpeg(SCAN_WIDTH, SCAN_HEIGHT),
            &tesseract_config(8 * 1024 * 1024, 600.0),
        )
        .await
        .expect("a JPEG over the limit must extract at a reduced scale");
        assert_eq!(raster_read(&document), ((1200, 800), 300.0));
    }

    /// A JPEG that does not fit even at 1/8 scale keeps the error that names the limit.
    #[tokio::test]
    async fn should_keep_the_limit_error_when_no_scale_fits() {
        const LIMIT: usize = 100_000;
        let message = match extract_jpeg(scan_jpeg(SCAN_WIDTH, SCAN_HEIGHT), &tesseract_config(LIMIT, 600.0)).await {
            Ok(document) => panic!("1/8 scale needs 180,000 bytes, over the limit: {:?}", document.content),
            Err(error) => error.to_string(),
        };
        assert!(
            message.contains("max_content_size") && message.contains(&LIMIT.to_string()),
            "the error must name the limit: {message}"
        );
    }
}

/// The explicit pipeline: the image extractor decodes the file and encodes it once for each stage.
#[cfg(feature = "pdf")]
mod explicit_pipeline {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use xberg::ExtractedDocument;
    use xberg::core::config::OcrConfig;
    use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

    const BACKEND: &str = "issue-2339-recorder";

    /// The image size a stage received and the source DPI it was given.
    type StageRead = ((u32, u32), Option<f64>);
    type Reads = Arc<Mutex<Vec<StageRead>>>;

    /// Records the size of the image each stage receives and the source DPI it is given.
    struct Recorder(Reads);

    impl Plugin for Recorder {
        fn name(&self) -> &str {
            BACKEND
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> xberg::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> xberg::Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for Recorder {
        async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
            let size = image::ImageReader::new(std::io::Cursor::new(image_bytes))
                .with_guessed_format()
                .expect("stage image format")
                .into_dimensions()
                .expect("the stage receives a readable image");
            let source_dpi = config
                .backend_options
                .as_ref()
                .and_then(|options| options.get("source_dpi"))
                .and_then(serde_json::Value::as_f64);
            self.0.lock().expect("reads").push((size, source_dpi));
            let mut document = ExtractedDocument::default();
            document.content.push_str("HELLO WORLD");
            Ok(document)
        }
        fn supports_language(&self, _language: &str) -> bool {
            true
        }
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Custom
        }
    }

    struct Registration;

    impl Drop for Registration {
        fn drop(&mut self) {
            unregister_ocr_backend(BACKEND).expect("remove the test backend");
        }
    }

    async fn stage_reads(width: u32, height: u32, max_content_size: usize, source_dpi: f64) -> Vec<StageRead> {
        let reads = Reads::default();
        register_ocr_backend(Arc::new(Recorder(reads.clone()))).expect("register the test backend");
        let _registration = Registration;
        let config = config(serde_json::json!({
            "use_cache": false,
            "security_limits": { "max_content_size": max_content_size },
            "ocr": {
                "backend": BACKEND,
                "backend_options": { "source_dpi": source_dpi },
                "pipeline": {
                    "stages": [{ "backend": BACKEND, "priority": 100 }],
                    "quality_thresholds": { "pipeline_min_quality": 0.0 }
                }
            }
        }));
        let document = extract_jpeg(scan_jpeg(width, height), &config)
            .await
            .expect("the pipeline must extract the JPEG");
        assert!(document.content.contains("HELLO WORLD"), "{:?}", document.content);
        reads.lock().expect("reads").clone()
    }

    /// Control: a JPEG whose encode fits keeps its size and DPI.
    #[tokio::test]
    #[serial_test::serial]
    async fn should_encode_a_jpeg_that_fits_at_full_size() {
        assert_eq!(
            stage_reads(1200, 800, 64 * 1024 * 1024, 300.0).await,
            vec![((1200, 800), Some(300.0))]
        );
    }

    /// The 512 MiB case of the issue: the raster fits a 16 MiB limit, but its stage encode
    /// (10 bytes a pixel, 38,662,144 bytes) does not; at half scale the encode fits.
    #[tokio::test]
    #[serial_test::serial]
    async fn should_encode_a_jpeg_whose_encode_is_over_the_limit_at_half_scale() {
        assert_eq!(
            stage_reads(SCAN_WIDTH, SCAN_HEIGHT, 16 * 1024 * 1024, 600.0).await,
            vec![((1200, 800), Some(300.0))]
        );
    }

    /// The 100 MiB case of the issue: the raster is over a 4 MiB limit and the half-scale encode
    /// (9,862,144 bytes) is too; the quarter scale fits, so the 600 DPI source becomes 150 DPI.
    #[tokio::test]
    #[serial_test::serial]
    async fn should_encode_a_jpeg_over_the_limit_at_the_finest_scale_that_fits() {
        assert_eq!(
            stage_reads(SCAN_WIDTH, SCAN_HEIGHT, 4 * 1024 * 1024, 600.0).await,
            vec![((600, 400), Some(150.0))]
        );
    }
}
