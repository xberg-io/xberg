use super::ocr;
use crate::core::config::ExtractionConfig;

pub(super) fn automatic_page_acceptance_supported(config: &ExtractionConfig) -> bool {
    // Document-scoped OCR must keep its original input; only native-preserving single-backend reads may detach. ~keep
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    {
        let Some(ocr_config) = config.ocr.as_ref() else {
            return false;
        };
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
        || page_count.and_then(|count| usize::try_from(count).ok()) != Some(bounds.len())
        || failing_pages.len() != bounds.len()
        || !bounds
            .iter()
            .zip(failing_pages)
            .enumerate()
            .all(|(index, (bound, page))| {
                usize::try_from(bound.page_number).ok() == Some(index + 1)
                    && bound.page_number == *page
                    && native.get(bound.byte_start..bound.byte_end).is_some()
            })
        || !bounds.windows(2).all(|pair| {
            native
                .get(pair[0].byte_end..pair[1].byte_start)
                .is_some_and(|gap| gap.trim().is_empty())
        })
        || !native
            .get(..bounds[0].byte_start)
            .is_some_and(|prefix| prefix.trim().is_empty())
        || !native
            .get(bounds[bounds.len() - 1].byte_end..)
            .is_some_and(|suffix| suffix.trim().is_empty())
    {
        return outcome;
    }
    // Every page still needs OCR; using page acceptance also retains native text when its read duplicates it. ~keep
    ocr::OcrGateOutcome::RunFallbackOnPages(failing_pages.to_vec())
}
