use crate::core::config::{ExtractionConfig, OcrConfig, OcrStrategy, OutputFormat, VlmFallbackPolicy};
use crate::extractors::security::SecurityLimits;
use crate::plugins::OcrBackend;
use crate::types::{ExtractedImage, ImageDpiConfig, ImagePreprocessingConfig, OcrElementConfig, OcrElementLevel};
use std::cell::RefCell;
use std::future::Future;
use std::sync::Arc;

#[path = "pdf_gap/occurrences.rs"]
mod occurrences;

tokio::task_local! {
    static PLACEMENT: RefCell<Capture>;
}

enum Capture {
    Empty,
    One(Placement),
    Disabled,
}

const MAX_NATIVE_TEXT_BYTES: usize = 64 * 1024;
const MAX_RESAMPLED_PIXELS: u64 = 8 * 1024 * 1024;

pub(crate) struct Placement {
    bytes: bytes::Bytes,
    dimensions: (u32, u32),
    scaled: (u32, u32),
    dpi: f64,
}

impl Placement {
    pub(crate) fn new(
        image: &ExtractedImage,
        dimensions: (u32, u32),
        dpi: f64,
        limits: &SecurityLimits,
    ) -> Option<Self> {
        let scaled = crate::image::preprocessing::normalized_image_dimensions(
            dimensions.0,
            dimensions.1,
            &ImageDpiConfig::default(),
            Some(dpi),
        );
        let pixels = u64::from(scaled.0) * u64::from(scaled.1);
        if scaled.0 <= dimensions.0 || scaled.1 <= dimensions.1 || pixels > MAX_RESAMPLED_PIXELS {
            return None;
        }
        let stored = u64::try_from(image.data.len()).ok()?.checked_mul(2)?;
        let source = u64::from(dimensions.0)
            .checked_mul(u64::from(dimensions.1))?
            .checked_mul(3)?;
        let retained = stored.checked_add(source)?;
        // Charge scaled RGB, resize scratch and a 32-bit Pix. The processor applies its
        // own phase admission to the actual decode and resize. ~keep
        const RETRY_WORKSPACE_BYTES_PER_PIXEL: u64 = 3 + 3 + 4;
        let workspace = pixels.checked_mul(RETRY_WORKSPACE_BYTES_PER_PIXEL)?;
        crate::extraction::image_decode::validate_image_live_bytes(scaled.0, scaled.1, retained, workspace, limits)
            .ok()?;
        Some(Self {
            bytes: image.data.clone(),
            dimensions,
            scaled,
            dpi,
        })
    }

    pub(crate) fn matches(&self, image: &ExtractedImage) -> bool {
        image.page_number == Some(1)
            && image.image_index == 0
            && image.data.len() == self.bytes.len()
            && image.data.as_ptr() == self.bytes.as_ptr()
    }
}

pub(crate) async fn capture<F: Future>(enabled: bool, future: F) -> (F::Output, Option<Placement>) {
    if !enabled && PLACEMENT.try_with(|_| ()).is_err() {
        return (future.await, None);
    }
    let initial = if enabled { Capture::Empty } else { Capture::Disabled };
    Box::pin(PLACEMENT.scope(RefCell::new(initial), async {
        let result = future.await;
        let captured = PLACEMENT.with(|slot| slot.replace(Capture::Disabled));
        let placement = match captured {
            Capture::One(placement) => Some(placement),
            _ => None,
        };
        (result, placement)
    }))
    .await
}

pub(crate) fn publish(placement: Placement) {
    let _ = PLACEMENT.try_with(|slot| {
        let previous = slot.replace(Capture::Disabled);
        if matches!(previous, Capture::Empty) {
            slot.replace(Capture::One(placement));
        }
    });
}

pub(crate) fn eligible(config: &ExtractionConfig) -> bool {
    #[cfg(feature = "layout-types")]
    if config.layout.is_some() {
        return false;
    }
    #[cfg(feature = "redaction")]
    if crate::text::redaction::external::external_redaction_is_scoped() {
        return false;
    }
    !config.use_cache
        && config.redaction.is_none()
        && config.output_format == OutputFormat::Plain
        && config.ocr_strategy == OcrStrategy::Auto
        && !config.force_ocr
        && config.force_ocr_pages.is_none()
        && !config.disable_ocr
        && !config.include_document_structure
        && config.content_filter.is_none()
        && config.runs_ocr_on_embedded_images()
        && config.images.as_ref().is_none_or(|images| {
            !images.include_page_rasters && !images.inject_placeholders && images.max_images_per_page.is_none()
        })
        && config.pdf_options.as_ref().is_none_or(|pdf| {
            pdf.backend == crate::core::config::PdfBackend::Native
                && !pdf.ocr_inline_images
                && !pdf.reading_order
                && pdf.hierarchy.is_none()
                && pdf.top_margin_fraction.is_none()
                && pdf.bottom_margin_fraction.is_none()
        })
        && config.ocr.as_ref().is_some_and(eligible_tesseract)
}

fn eligible_tesseract(ocr: &OcrConfig) -> bool {
    ocr.backend == "tesseract"
        && ocr.effective_tesseract_language() == ["eng"]
        && ocr.element_config.is_none()
        && ocr
            .tesseract_config
            .as_ref()
            .is_some_and(|config| config.psm == Some(6) && !config.use_cache && config.preprocessing.is_none())
        && ocr.backend_options.is_none()
        && ocr.pipeline.is_none()
        && ocr.quality_thresholds.is_none()
        && ocr.tessdata_bytes.is_none()
        && ocr.tessdata_path.is_none()
        && ocr.output_format.is_none()
        && !ocr.auto_rotate
        && !ocr.numeric_repair
        && ocr.vlm_fallback == VlmFallbackPolicy::Disabled
}

pub(crate) fn stock_backend(config: &ExtractionConfig) -> Option<Arc<dyn OcrBackend>> {
    let ocr = config.ocr.as_ref()?;
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let registry = registry.read();
    let backend = registry.get(&ocr.backend).ok()?;
    registry.is_builtin_tesseract(&backend).then_some(backend)
}

pub(crate) async fn process(
    image: ExtractedImage,
    placement: Placement,
    backend: Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    native: &str,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> crate::Result<(ExtractedImage, Option<String>)> {
    check_cancelled(config)?;
    let ocr = config.ocr.as_ref().expect("eligible explicit OCR configuration");
    let mut images = [image];
    let Some((index, bytes, mut primary_config)) = super::build_pending_ocr_tasks(&images, ocr, config).pop_front()
    else {
        let [image] = images;
        return Ok((image, None));
    };
    primary_config.element_config = Some(OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        ..Default::default()
    });
    let primary = backend.process_image(&bytes, &primary_config).await;
    check_cancelled(config)?;
    let addition = if let Ok(primary) = &primary {
        Box::pin(retry(
            &placement,
            backend.as_ref(),
            primary_config,
            primary,
            native,
            config,
        ))
        .await?
    } else {
        None
    };
    super::apply_ocr_result(&mut images, index, primary, ocr, warnings);
    let [image] = images;
    Ok((image, addition))
}

async fn retry(
    placement: &Placement,
    backend: &dyn OcrBackend,
    mut config: OcrConfig,
    primary: &crate::types::ExtractedDocument,
    native: &str,
    extraction: &ExtractionConfig,
) -> crate::Result<Option<String>> {
    if !occurrences::has_primary(primary, placement.dimensions) {
        return Ok(None);
    }
    config.backend_options = Some(serde_json::json!({"source_dpi": placement.dpi, "use_cache": false}));
    config
        .tesseract_config
        .as_mut()
        .expect("eligible explicit PSM")
        .preprocessing = Some(ImagePreprocessingConfig {
        deskew: false,
        binarization_method: "none".into(),
        ..Default::default()
    });
    let result = backend.process_image(&placement.bytes, &config).await;
    check_cancelled(extraction)?;
    match result {
        Ok(result) => Ok(occurrences::insert(primary, &result, placement, native)),
        Err(error) => {
            tracing::debug!(%error, "optional embedded image line OCR failed");
            Ok(None)
        }
    }
}

fn check_cancelled(config: &ExtractionConfig) -> crate::Result<()> {
    if config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled()) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

pub(crate) fn native_anchor_text(
    doc: &crate::types::internal::InternalDocument,
    placement: &Placement,
) -> Option<String> {
    let [image] = doc.images.as_slice() else { return None };
    if !placement.matches(image) || image.ocr_result.is_some() {
        return None;
    }
    let mut anchors = doc
        .elements
        .iter()
        .filter(|element| element.image_ocr_text_anchor_index() == Some(0));
    if anchors.next()?.page != Some(1) || anchors.next().is_some() {
        return None;
    }
    doc.elements.iter().try_fold(String::new(), |mut native, element| {
        if native.len().checked_add(element.text.len())?.checked_add(1)? > MAX_NATIVE_TEXT_BYTES {
            return None;
        }
        native.push_str(&element.text);
        native.push('\n');
        Some(native)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(bytes: &'static [u8]) -> Placement {
        Placement {
            bytes: bytes::Bytes::from_static(bytes),
            dimensions: (1, 1),
            scaled: (2, 2),
            dpi: 72.0,
        }
    }

    #[tokio::test]
    async fn placement_is_operation_local_and_ambiguous_writes_are_discarded() {
        let first = capture(true, async {
            publish(evidence(b"first"));
            let (_, disabled) = capture(false, async {
                publish(evidence(b"nested"));
            })
            .await;
            assert!(disabled.is_none());
            tokio::task::yield_now().await;
        });
        let second = capture(true, async {
            publish(evidence(b"other"));
            tokio::task::yield_now().await;
            publish(evidence(b"duplicate"));
        });
        let (((), first), ((), second)) = tokio::join!(first, second);
        assert_eq!(&first.expect("first scoped proof").bytes[..], b"first");
        assert!(second.is_none());
        let (_, absent) = capture(false, async {
            publish(evidence(b"outside"));
        })
        .await;
        assert!(absent.is_none());
    }

    #[test]
    fn forced_page_and_document_ocr_are_ineligible() {
        let mut config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                tesseract_config: Some(crate::types::TesseractConfig {
                    psm: Some(6),
                    use_cache: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(eligible(&config));
        config.force_ocr = true;
        assert!(!eligible(&config));
        config.force_ocr = false;
        config.force_ocr_pages = Some(vec![1]);
        assert!(!eligible(&config));
    }
}
