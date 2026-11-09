use super::ocr;
use crate::core::config::ExtractionConfig;

pub(super) fn automatic_page_acceptance_supported(config: &ExtractionConfig) -> bool {
    // Document-scoped OCR must keep its original input; only native-preserving single-backend reads may detach. ~keep
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    {
        let Some(ocr_config) = config.ocr.as_ref() else {
            return false;
        };
        if config.output_format != crate::core::config::OutputFormat::Plain
            || config.include_document_structure
            || config.result_format == crate::types::ResultFormat::ElementBased
        {
            return false;
        }
        if config.force_ocr_pages.as_ref().is_some_and(|pages| !pages.is_empty()) {
            return false;
        }
        // No page override remains, so the shared page-specific predicate is uniform across the document. ~keep
        if !ocr::native_preservation_is_automatic(config, 1)
            || ocr_config.vlm_fallback != crate::core::config::VlmFallbackPolicy::Disabled
        {
            return false;
        }
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        registry
            .get(&ocr_config.backend)
            .is_ok_and(|backend| registry.is_builtin_tesseract(&backend))
    }
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    {
        let _ = config;
        false
    }
}

fn complete_page_selection(
    bounds: &[crate::types::PageBoundary],
    failing_pages: &[u32],
    native: &str,
    page_count: Option<u32>,
) -> bool {
    page_count.and_then(|count| usize::try_from(count).ok()) == Some(bounds.len())
        && failing_pages.len() == bounds.len()
        && bounds
            .iter()
            .zip(failing_pages)
            .enumerate()
            .all(|(index, (bound, page))| {
                usize::try_from(bound.page_number).ok() == Some(index + 1)
                    && bound.page_number == *page
                    && native.get(bound.byte_start..bound.byte_end).is_some()
            })
}

fn boundaries_cover_native_text(bounds: &[crate::types::PageBoundary], native: &str) -> bool {
    let (Some(first), Some(last)) = (bounds.first(), bounds.last()) else {
        return false;
    };
    native
        .get(..first.byte_start)
        .is_some_and(|text| text.trim().is_empty())
        && native.get(last.byte_end..).is_some_and(|text| text.trim().is_empty())
        && bounds.windows(2).all(|pair| {
            native
                .get(pair[0].byte_end..pair[1].byte_start)
                .is_some_and(|text| text.trim().is_empty())
        })
}

pub(super) fn with_native_page_acceptance(
    outcome: ocr::OcrGateOutcome,
    failing_pages: &[u32],
    boundaries: Option<&[crate::types::PageBoundary]>,
    native: &str,
    page_count: Option<u32>,
) -> ocr::OcrGateOutcome {
    let Some(bounds) = boundaries.filter(|bounds| !bounds.is_empty()) else {
        return outcome;
    };
    if !matches!(outcome, ocr::OcrGateOutcome::RunFallback)
        || native.trim().is_empty()
        || !complete_page_selection(bounds, failing_pages, native, page_count)
        || !boundaries_cover_native_text(bounds, native)
    {
        return outcome;
    }
    // Even one native Khmer digit in otherwise English text keeps the original whole-document fallback. ~keep
    // This preserves #2408 glyph/raster proofs, missing-model warnings and model-cache behavior; the tradeoff ~keep
    // is withholding native-page acceptance for the entire document when the whole-document gate requests OCR. ~keep
    if native
        .chars()
        .any(|character| ('\u{17e0}'..='\u{17e9}').contains(&character))
    {
        return outcome;
    }
    // Every page still needs OCR; using page acceptance also retains native text when its read duplicates it. ~keep
    ocr::OcrGateOutcome::RunFallbackOnPages(failing_pages.to_vec())
}
