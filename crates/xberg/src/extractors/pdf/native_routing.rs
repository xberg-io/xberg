//! OCR routing of the native PDF path: which pages are read by OCR, and what each route produced.

use super::OcrLayoutGateDecisions;
#[cfg(feature = "layout-detection")]
use super::markdown_layout_reusable_for_ocr;
#[cfg(all(
    feature = "layout-detection",
    any(feature = "formula-recognition", feature = "liter-llm")
))]
use super::markdown_layout_usable_for_native;
use super::native_auto_route;
#[cfg(all(
    feature = "layout-detection",
    any(feature = "formula-recognition", feature = "liter-llm")
))]
use super::native_layout_readers_configured;
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
use super::picture_layer;
use super::{
    PageOcrLayoutContext, automatic_ocr_backend_is_registered, extraction_method_after_mixed_ocr,
    failed_ocr_fallback_is_total_loss, native_page_texts_for_restore, native_text_stopped_by_time_limit,
    near_empty_ocr_fallback_applies, ocr, restore_discarded_page_reads, run_ocr_with_layout_pages,
    scanned_pages_to_ocr, unmapped_glyphs,
};
use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::types::ExtractionMethod;
use crate::types::internal::InternalDocument;

/// What the OCR routes of `extract_core_native_with_layout_pages` produced, kept apart from the
/// native extraction so each route fills only the parts it owns.
#[derive(Default)]
pub(super) struct OcrRoutingState {
    pub(super) tables: Vec<crate::types::Table>,
    pub(super) elements: Vec<crate::types::OcrElement>,
    pub(super) coordinate_frames: Vec<crate::types::internal::OcrPageCoordinateFrame>,
    pub(super) internal_doc: Option<InternalDocument>,
    pub(super) llm_usage: Vec<crate::types::LlmUsage>,
    pub(super) page_texts: Option<Vec<String>>,
    pub(super) document_page_texts: Option<Vec<String>>,
    pub(super) results_map: Option<ahash::AHashMap<u32, String>>,
    pub(super) structured_pages: Option<ahash::AHashMap<u32, InternalDocument>>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    pub(super) picture_pages: ahash::AHashMap<u32, picture_layer::RetainedPicturePage>,
    pub(super) page_rasters: Option<Vec<crate::types::ExtractedImage>>,
    pub(super) formulas: Vec<crate::types::Formula>,
    pub(super) preprocessing_by_page: ahash::AHashMap<u32, crate::types::ImagePreprocessingMetadata>,
    pub(super) confidence_by_page: ahash::AHashMap<u32, crate::types::page::PageOcrConfidence>,
    pub(super) fallback_warnings: Vec<crate::types::ProcessingWarning>,
    pub(super) page_failures: Vec<crate::types::OcrPageFailure>,
    pub(super) layout_gate_audit: OcrLayoutGateDecisions,
    #[cfg(feature = "formula-recognition")]
    pub(super) mixed_formula_layout_inputs: Option<(Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>)>,
}

#[cfg(feature = "layout-detection")]
pub(super) struct MarkdownLayoutInputs<'a> {
    pub(super) images: &'a mut Option<Vec<image::RgbImage>>,
    pub(super) detections: &'a mut Option<Vec<crate::layout::DetectionResult>>,
    pub(super) acceleration_override: &'a mut Option<crate::core::config::acceleration::AccelerationConfig>,
    pub(super) rendered_pages: &'a mut Vec<bool>,
    #[cfg(any(feature = "formula-recognition", feature = "liter-llm"))]
    pub(super) native_only: &'a mut Option<(
        Option<Vec<image::RgbImage>>,
        Option<Vec<crate::layout::DetectionResult>>,
    )>,
}

/// The native extraction's view the OCR routes decide from.
pub(super) struct OcrRouteInputs<'a> {
    pub(super) content: &'a [u8],
    pub(super) config: &'a ExtractionConfig,
    pub(super) path: Option<&'a std::path::Path>,
    pub(super) layout_pages: Option<&'a crate::core::layout_pages::LayoutPageSelection>,
    pub(super) pdf_metadata: &'a crate::pdf::metadata::PdfExtractionMetadata,
    pub(super) boundaries: Option<&'a [crate::types::PageBoundary]>,
    pub(super) page_contents: Option<&'a [crate::types::PageContent]>,
    pub(super) has_pre_rendered_doc: bool,
    pub(super) outlined_ocr_pages: &'a mut Vec<u32>,
    #[cfg(feature = "layout-detection")]
    pub(super) layout: MarkdownLayoutInputs<'a>,
}

pub(super) async fn route_native_text(
    route: &mut OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
) -> Result<(String, ExtractionMethod)> {
    let config = route.config;
    if config.effective_disable_ocr() || native_text_stopped_by_time_limit(config) {
        Ok((native_text, ExtractionMethod::Native))
    } else if config.force_ocr {
        route_force_ocr(route, routing, &native_text).await
    } else if let Some(ref ocr_pages) = config.force_ocr_pages {
        route_force_ocr_pages(route, routing, native_text, ocr_pages).await
    } else if let Some(scanned_pages) = scanned_pages_to_ocr(config, route.pdf_metadata, &native_text, route.boundaries)
        .filter(|_| config.ocr.is_some() || automatic_ocr_backend_is_registered())
    {
        route_scanned_pages(route, routing, native_text, scanned_pages).await
    } else if near_empty_ocr_fallback_applies(config, &native_text) {
        native_auto_route::route_auto_fallback(route, routing, native_text).await
    } else {
        Ok((native_text, ExtractionMethod::Native))
    }
}

async fn route_force_ocr(
    route: &mut OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: &str,
) -> Result<(String, ExtractionMethod)> {
    let (content, config, path, layout_pages, boundaries) = (
        route.content,
        route.config,
        route.path,
        route.layout_pages,
        route.boundaries,
    );
    let (
        mut ocr_text,
        ocr_tbls,
        ocr_elems,
        mut ocr_doc,
        llm_usage,
        ocr_pts,
        ocr_rstrs,
        formulas,
        preprocessing,
        page_ocr_confidence,
        gate_audit,
        layout_warning,
        layout_glyph_drop_warnings,
        _,
    ) = run_ocr_with_layout_pages(
        content,
        config,
        path,
        PageOcrLayoutContext {
            pages: layout_pages,
            hints: None,
            #[cfg(all(feature = "pdf", feature = "layout-detection"))]
            rendered_layout_pages: std::mem::take(route.layout.rendered_pages),
        },
        #[cfg(feature = "layout-detection")]
        route.layout.images.take(),
        #[cfg(feature = "layout-detection")]
        route.layout.detections.take(),
        #[cfg(feature = "layout-detection")]
        route.layout.acceleration_override.take(),
    )
    .await?;
    if let Some(warning) = layout_warning {
        crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
    }
    for warning in layout_glyph_drop_warnings {
        crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
    }
    if ocr_pts.iter().any(|page| page.trim().is_empty()) {
        let native_pages = native_page_texts_for_restore(route.page_contents, native_text, boundaries);
        if let Some((pages, lost_glyphs)) =
            restore_discarded_page_reads(&mut ocr_text, &mut ocr_doc, &ocr_pts, &native_pages, config)
        {
            routing.document_page_texts = Some(pages);
            let total_lost = lost_glyphs.iter().map(|&(_, lost)| lost).sum();
            if let Some(warning) = unmapped_glyphs::lost_glyph_warning(total_lost, config) {
                crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
            }
            for (page, lost) in lost_glyphs {
                if let Some(warning) = unmapped_glyphs::unrecovered_page_glyph_warning(page, lost) {
                    crate::core::diagnostics::push_warning_deduped(&mut routing.fallback_warnings, warning);
                }
            }
        }
    }
    routing.layout_gate_audit = gate_audit;
    routing.tables = ocr_tbls;
    routing.elements = ocr_elems;
    routing.internal_doc = ocr_doc;
    routing.llm_usage = llm_usage;
    routing.page_texts = Some(ocr_pts);
    routing.page_rasters = ocr_rstrs;
    routing.formulas = formulas;
    routing.preprocessing_by_page = preprocessing;
    routing.confidence_by_page = page_ocr_confidence;
    Ok((ocr_text, ExtractionMethod::Ocr))
}

async fn route_force_ocr_pages(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    ocr_pages: &[u32],
) -> Result<(String, ExtractionMethod)> {
    let (content, config, layout_pages, boundaries) =
        (route.content, route.config, route.layout_pages, route.boundaries);
    Ok(if !ocr_pages.is_empty() {
        if let Some(bounds) = boundaries {
            if !bounds.is_empty() {
                let mixed_run = ocr::extract_mixed_ocr_native_with_layout_pages(
                    &native_text,
                    bounds,
                    (ocr_pages, &[]),
                    content,
                    config,
                    layout_pages,
                    (&[], &[]),
                )
                .await?;
                #[cfg(feature = "formula-recognition")]
                {
                    routing.mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
                }
                let (
                    mixed,
                    results_map,
                    mixed_structured_pages,
                    mixed_llm_usage,
                    mixed_rstrs,
                    mixed_formulas,
                    mixed_preprocessing,
                    mixed_ocr_confidence,
                    mixed_warnings,
                    mixed_page_failures,
                ) = mixed_run.output;
                let extraction_method = extraction_method_after_mixed_ocr(&results_map);
                routing.llm_usage = mixed_llm_usage;
                routing.results_map = Some(results_map);
                routing.structured_pages = Some(mixed_structured_pages);
                routing.page_rasters = mixed_rstrs;
                if !mixed_formulas.is_empty() {
                    routing.formulas = mixed_formulas;
                }
                routing.preprocessing_by_page.extend(mixed_preprocessing);
                routing.confidence_by_page.extend(mixed_ocr_confidence);
                routing.page_failures.extend(mixed_page_failures);
                routing.fallback_warnings.extend(mixed_warnings);
                (mixed, extraction_method)
            } else {
                tracing::warn!(
                    target: "xberg::extractors::pdf",
                    "force_ocr_pages set but no page boundaries available; using native text"
                );
                (native_text, ExtractionMethod::Native)
            }
        } else {
            tracing::warn!(
                target: "xberg::extractors::pdf",
                "force_ocr_pages set but no page boundaries available; using native text"
            );
            (native_text, ExtractionMethod::Native)
        }
    } else {
        (native_text, ExtractionMethod::Native)
    })
}

async fn route_scanned_pages(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    scanned_pages: Vec<u32>,
) -> Result<(String, ExtractionMethod)> {
    Ok(
        // A scanner's invisible sidecar passes the gate below, so detected
        // pages are selected before it runs. ~keep
        if let Some(bounds) = route.boundaries
            && !bounds.is_empty()
        {
            read_scanned_pages(route, routing, native_text, bounds, &scanned_pages).await?
        } else {
            tracing::warn!(
                target: "xberg::extractors::pdf",
                "scanned pages detected but no page boundaries available; using native text"
            );
            (native_text, ExtractionMethod::Native)
        },
    )
}

async fn read_scanned_pages(
    route: &OcrRouteInputs<'_>,
    routing: &mut OcrRoutingState,
    native_text: String,
    bounds: &[crate::types::PageBoundary],
    scanned_pages: &[u32],
) -> Result<(String, ExtractionMethod)> {
    let (content, config, layout_pages, pdf_metadata) =
        (route.content, route.config, route.layout_pages, route.pdf_metadata);
    // This OCR run is AUTOMATIC -- scanned-page detection asked for it, the
    // caller did not -- so a failure here must degrade to the native text rather
    // than abort, exactly as the quality-gate fallback below already does. With
    // `ocr-pipeline` enabled and no backend registered (a legitimate feature
    // selection: `ocr` implies `ocr-pipeline`, not the reverse) the `?` that used
    // to be here failed an ordinary PDF extraction that never requested OCR. The
    // EXPLICIT sites (`force_ocr_pages` above, `ocr_inline_images`) keep their
    // hard error, because there the caller asked for something the build cannot
    // do. See GH#1610. ~keep
    let selected_run = ocr::extract_mixed_ocr_native_with_layout_pages(
        &native_text,
        bounds,
        (
            scanned_pages,
            pdf_metadata
                .pdf_specific
                .fabricated_text_pages
                .as_deref()
                .unwrap_or(&[]),
        ),
        content,
        config,
        layout_pages,
        (&[], &[]),
    )
    .await;
    Ok(match selected_run {
        Ok(mixed_run) => adopt_scanned_run(routing, mixed_run, scanned_pages),
        Err(e @ crate::XbergError::Cancelled) => return Err(e),
        Err(e) => scanned_pages_failed(routing, native_text, bounds, scanned_pages, e)?,
    })
}

fn adopt_scanned_run(
    routing: &mut OcrRoutingState,
    mixed_run: ocr::MixedOcrRun,
    scanned_pages: &[u32],
) -> (String, ExtractionMethod) {
    #[cfg(feature = "formula-recognition")]
    {
        routing.mixed_formula_layout_inputs = mixed_run.formula_layout_inputs;
    }
    let (
        mixed,
        results_map,
        mixed_structured_pages,
        mixed_llm_usage,
        mixed_rstrs,
        mixed_formulas,
        mixed_preprocessing,
        mixed_ocr_confidence,
        mixed_warnings,
        mixed_page_failures,
    ) = mixed_run.output;
    // `Mixed` must mean "OCR contributed text", not "OCR was attempted". When
    // every candidate page was rejected (blank render, failed decode, empty
    // backend output) nothing was replaced and the result IS the native text --
    // reporting `Mixed` there tells a caller the document was OCR'd when it was
    // not, which is how a silent whole-document OCR failure reads as success. ~keep
    let mixed_method = extraction_method_after_mixed_ocr(&results_map);
    let ocr_contributed = mixed_method == ExtractionMethod::Mixed;
    if !ocr_contributed {
        tracing::warn!(
            target: "xberg::extractors::pdf",
            candidate_pages = scanned_pages.len(),
            "OCR was attempted on every detected scanned page but no page produced usable \
            text; reporting the native extraction method rather than `mixed`"
        );
    }
    routing.llm_usage = mixed_llm_usage;
    routing.results_map = Some(results_map);
    routing.structured_pages = Some(mixed_structured_pages);
    routing.page_rasters = mixed_rstrs;
    if !mixed_formulas.is_empty() {
        routing.formulas = mixed_formulas;
    }
    routing.preprocessing_by_page.extend(mixed_preprocessing);
    routing.confidence_by_page.extend(mixed_ocr_confidence);
    routing.page_failures.extend(mixed_page_failures);
    routing.fallback_warnings.extend(mixed_warnings);
    if ocr_contributed {
        (mixed, mixed_method)
    } else {
        (mixed, ExtractionMethod::Native)
    }
}

fn scanned_pages_failed(
    routing: &mut OcrRoutingState,
    native_text: String,
    bounds: &[crate::types::PageBoundary],
    scanned_pages: &[u32],
    e: crate::XbergError,
) -> Result<(String, ExtractionMethod)> {
    tracing::warn!(
        target: "xberg::extractors::pdf",
        error = %e.pdf_ocr_trace_category(),
        candidate_pages = ?scanned_pages,
        "Automatic OCR of detected scanned pages failed; using native text extraction result"
    );
    if failed_ocr_fallback_is_total_loss(&native_text) {
        return Err(e);
    }
    routing.page_failures.extend(ocr::route_level_ocr_page_failures(
        scanned_pages,
        &e,
        &native_text,
        bounds,
    ));
    routing.fallback_warnings.push(crate::types::ProcessingWarning {
        source: std::borrow::Cow::Borrowed("ocr"),
        message: std::borrow::Cow::Owned(format!(
            "Automatic OCR of detected scanned pages {scanned_pages:?} failed ({e}); those \
                                 pages retain their native text, which may be empty or incomplete."
        )),
    });
    Ok((native_text, ExtractionMethod::Native))
}

// These values exist only with layout detection; keep the reuse gate in the same cfg block. ~keep
#[cfg(feature = "layout-detection")]
#[cfg_attr(
    not(any(feature = "formula-recognition", feature = "liter-llm")),
    allow(unused_variables)
)]
pub(super) fn withhold_unreusable_layout(
    config: &ExtractionConfig,
    content: &[u8],
    markdown_layout_gate_decisions: Option<&[crate::pdf::layout_gate::PageGateDecision]>,
    layout: MarkdownLayoutInputs<'_>,
) {
    let markdown_page_rotations = layout
        .images
        .as_ref()
        .map(|images| crate::pdf::render::get_page_rotations_from_bytes(content, images.len()))
        .unwrap_or_default();
    if !markdown_layout_reusable_for_ocr(markdown_layout_gate_decisions, &markdown_page_rotations) {
        let native_inputs = (layout.images.take(), layout.detections.take());
        *layout.acceleration_override = None;
        #[cfg(any(feature = "formula-recognition", feature = "liter-llm"))]
        {
            // Keep the rasters only when this run configures a native reader of them.
            if native_layout_readers_configured(config) && markdown_layout_usable_for_native(&markdown_page_rotations) {
                *layout.native_only = Some(native_inputs);
            }
        }
        #[cfg(not(any(feature = "formula-recognition", feature = "liter-llm")))]
        drop(native_inputs);
    }
}
