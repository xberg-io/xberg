//! Stages that finish the `InternalDocument` built by `PdfExtractor::extract_core_native_with_layout_pages`.

use super::OcrLayoutGateDecisions;
use super::apply_extracted_pdf_metadata;
use super::extraction;
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
use super::insert_image_ocr_text_anchor;
#[cfg(feature = "layout-detection")]
use super::layout_gate_metadata;
#[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
use super::region_vlm;
use super::unmapped_glyphs;
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
use super::{recognize_pdf_formula_page_inputs, select_formula_page_inputs};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use super::{replace_page_content_with_ocr_text, replace_pages_with_ocr_page_texts};
use crate::core::config::ExtractionConfig;
use crate::types::ExtractionMethod;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, PdfPageCoordinateFrame};
use crate::types::{ExtractedImage, Metadata, PageContent};

pub(super) fn push_link_annotation_uris(doc: &mut InternalDocument) {
    use crate::types::annotations::PdfAnnotationType;
    use crate::types::uri::{ExtractedUri, UriKind};

    let uris: Vec<ExtractedUri> = doc
        .annotations
        .as_ref()
        .map(|annotations| {
            annotations
                .iter()
                .filter(|a| a.annotation_type == PdfAnnotationType::Link)
                .filter_map(|a| {
                    a.content.as_ref().map(|url| {
                        let kind = if url.starts_with('#') {
                            UriKind::Anchor
                        } else if url.starts_with("mailto:") {
                            UriKind::Email
                        } else {
                            UriKind::Hyperlink
                        };
                        ExtractedUri {
                            url: url.clone(),
                            label: Some(url.clone()),
                            page: Some(a.page_number),
                            kind,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    for uri in uris {
        doc.push_uri(uri);
    }
}

// GH#1653 + GH#1654: one raw-MediaBox coordinate frame per page that ended up with
// hierarchy blocks, once `assign_hierarchy_to_pages` has populated `PageContent::hierarchy`.
// One shared record covers both issues: a consumer needs the MediaBox origin (which can be
// non-zero and negative) and the page rotation together to place a page's raw-space geometry.
// Rides in `additional` rather than a new public binding type, same as the `page_labels` key
// (issue #66) and `ocr_page_coordinate_frames` (GH#1645). ~keep
pub(super) fn attach_page_coordinate_frames(
    doc: &mut InternalDocument,
    pages: &[PageContent],
    pdf_page_coordinate_frames: Vec<PdfPageCoordinateFrame>,
) {
    let pages_with_hierarchy: std::collections::HashSet<u32> = pages
        .iter()
        .filter(|page| {
            page.hierarchy
                .as_ref()
                .is_some_and(|hierarchy| !hierarchy.blocks.is_empty())
        })
        .map(|page| page.page_number)
        .collect();
    let pdf_page_coordinate_frames: Vec<_> = pdf_page_coordinate_frames
        .into_iter()
        .filter(|frame| pages_with_hierarchy.contains(&frame.page_number))
        .collect();
    if !pdf_page_coordinate_frames.is_empty() {
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed("pdf_page_coordinate_frames"),
            serde_json::json!(pdf_page_coordinate_frames),
        );
    }
}

pub(super) fn inject_native_image_elements(
    doc: &mut InternalDocument,
    imgs: Vec<ExtractedImage>,
    config: &ExtractionConfig,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))] extraction_method: ExtractionMethod,
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))] ocr_results_map: Option<&ahash::AHashMap<u32, String>>,
) {
    // The OCR path has its own guarded injection block below (see the `#[cfg(feature = "ocr")]`
    let image_anchor = extraction::image_anchor(config);
    let document_has_image_elements = doc.elements.iter().any(|element| {
        #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
        if element.image_ocr_text_anchor_index().is_some() {
            return true;
        }
        matches!(element.kind, ElementKind::Image { .. })
    });
    if !document_has_image_elements && image_anchor != extraction::ImageAnchor::None {
        for (idx, img) in imgs.iter().enumerate() {
            // Page OCR already read the picture as part of the page, so the picture's
            // own OCR text is not rendered there: no anchor, and a placeholder without
            // the text. The OCR merge of a structured document does the same for the
            // pages it replaces. ~keep
            #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
            let page_text_is_from_ocr = extraction_method == ExtractionMethod::Ocr
                || img
                    .page_number
                    .is_some_and(|page| ocr_results_map.is_some_and(|results| results.contains_key(&page)));
            #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
            if image_anchor == extraction::ImageAnchor::OcrTextOnly {
                if !page_text_is_from_ocr {
                    insert_image_ocr_text_anchor(
                        doc,
                        InternalElement::image_ocr_text_anchor(idx as u32, img.page_number),
                    );
                }
                continue;
            }
            let mut elem = InternalElement::text(
                ElementKind::Image {
                    image_index: idx as u32,
                },
                "",
                0,
            );
            elem.page = img.page_number;
            #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
            if page_text_is_from_ocr {
                elem.suppress_image_ocr_rendering();
            }
            doc.push_element(elem);
        }
    }
    doc.images = imgs;
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn append_ocr_page_rasters(
    doc: &mut InternalDocument,
    ocr_page_rasters: Option<Vec<ExtractedImage>>,
    config: &ExtractionConfig,
    used_ocr: bool,
    extraction_method: ExtractionMethod,
) {
    let ocr_rasters_bypass = ocr_page_rasters.is_none();
    if let Some(rasters) = ocr_page_rasters {
        let base_idx = doc.images.len() as u32;
        for (offset, mut raster) in rasters.into_iter().enumerate() {
            raster.image_index = base_idx + offset as u32;
            doc.images.push(raster);
        }
    }

    if used_ocr
        && ocr_rasters_bypass
        && extraction_method == ExtractionMethod::Ocr
        && config.images.as_ref().is_some_and(|c| c.include_page_rasters)
    {
        doc.processing_warnings.push(crate::types::ProcessingWarning {
            source: std::borrow::Cow::Borrowed("page_rasters"),
            message: std::borrow::Cow::Borrowed(
                "include_page_rasters is set but no page rasters were produced; \
                 the active OCR backend used document-level processing without per-page rendering",
            ),
        });
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn inject_ocr_image_placeholders(doc: &mut InternalDocument, config: &ExtractionConfig, used_ocr: bool) {
    if used_ocr && !doc.images.is_empty() {
        let images_enabled = config.images.as_ref().map(|c| c.extract_images).unwrap_or(false)
            || config.pdf_options.as_ref().map(|p| p.extract_images).unwrap_or(false);
        if images_enabled && config.images.as_ref().map(|c| c.inject_placeholders).unwrap_or(false) {
            let referenced_images: std::collections::HashSet<u32> = doc
                .elements
                .iter()
                .filter_map(|element| match element.kind {
                    ElementKind::Image { image_index } => Some(image_index),
                    _ => None,
                })
                .collect();
            let elems: Vec<InternalElement> = doc
                .images
                .iter()
                .filter(|image| !referenced_images.contains(&image.image_index))
                .map(|img| {
                    let elem = InternalElement::text(
                        ElementKind::Image {
                            image_index: img.image_index,
                        },
                        "",
                        0,
                    );
                    if let Some(page) = img.page_number {
                        elem.with_page(page)
                    } else {
                        elem
                    }
                })
                .collect();
            for elem in elems {
                doc.push_element(elem);
            }
        }
    }
}

// Record the layout gate's audit trail from whichever pass ran
// it. Compiled whenever `pdf` is on so `ocr_layout_gate_audit` keeps
// a reader in profiles without layout-detection (it is always
// `(None, None)` there and the fields stay `None`). ~keep
pub(super) fn record_layout_gate_audit(
    pdf_metadata: &mut crate::pdf::metadata::PdfExtractionMetadata,
    #[cfg(feature = "layout-detection")] markdown_layout_gate_decisions: Option<
        &[crate::pdf::layout_gate::PageGateDecision],
    >,
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))] ocr_layout_gate_audit: OcrLayoutGateDecisions,
) {
    #[cfg(feature = "layout-detection")]
    #[allow(unused_mut)]
    let (mut gated_pages, mut gate_reasons) = layout_gate_metadata(markdown_layout_gate_decisions);
    #[cfg(not(feature = "layout-detection"))]
    #[allow(unused_mut)]
    let (mut gated_pages, mut gate_reasons): OcrLayoutGateDecisions = (None, None);
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    if gated_pages.is_none() {
        let (ocr_gated_pages, ocr_gate_reasons) = ocr_layout_gate_audit;
        gated_pages = ocr_gated_pages;
        gate_reasons = ocr_gate_reasons;
    }
    pdf_metadata.pdf_specific.layout_gated_pages = gated_pages;
    pdf_metadata.pdf_specific.layout_gate_reasons = gate_reasons;
}

pub(super) struct DocumentWarningSources<'a> {
    pub(super) lost_glyphs: usize,
    pub(super) pdf_extraction_warnings: &'a mut Vec<crate::types::ProcessingWarning>,
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    pub(super) ocr_fallback_warnings: &'a mut Vec<crate::types::ProcessingWarning>,
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    pub(super) ocr_page_failures: &'a mut Vec<crate::types::OcrPageFailure>,
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub(super) markdown_layout_warning: Option<crate::types::ProcessingWarning>,
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    pub(super) markdown_layout_glyph_drop_warnings: Vec<crate::types::ProcessingWarning>,
}

pub(super) fn merge_document_warnings(
    doc: &mut InternalDocument,
    config: &ExtractionConfig,
    sources: DocumentWarningSources<'_>,
) {
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    crate::core::diagnostics::dedup_extend_warnings(
        &mut doc.processing_warnings,
        std::mem::take(sources.ocr_fallback_warnings),
    );
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    doc.ocr_page_failures.append(sources.ocr_page_failures);
    if let Some(warning) = unmapped_glyphs::lost_glyph_warning(sources.lost_glyphs, config) {
        crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
    }

    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    crate::core::diagnostics::dedup_extend_warnings(
        &mut doc.processing_warnings,
        std::mem::take(sources.pdf_extraction_warnings),
    );
    #[cfg(not(any(feature = "ocr", feature = "ocr-pipeline")))]
    doc.processing_warnings.append(sources.pdf_extraction_warnings);

    // #340: drain any xberg_native_pdf glyph-drop warnings captured while this
    // document's pages were rendered by *this* task (OCR rasterization
    // and any other call routed through `render_page_capturing_glyph_drops`
    // in `crate::pdf::render` that runs inline on it;
    // `with_task_render_warnings` keeps them with the task). The drain is unconditionally cheap and correct even when
    // nothing was ever captured: `install_pdf_render_diagnostics` is
    // opt-in and only an embedding application calls it (a library must
    // not seize the process-global `log` backend on its own — see
    // `pdf::render` for why), so this buffer stays empty for every caller
    // that has not opted in and this loop is a no-op `Vec::is_empty`
    // check. Deduped because a multi-page document can hit the identical
    // xberg_native_pdf cause on many pages.
    //
    // Layout-detection rasterization runs inside `tokio::task::spawn_blocking`
    // on a different OS thread, so this drain never sees those warnings —
    // `layout_runner::run_layout_for_pdf_pages_async` drains them itself
    // from inside that closure and threads them back through its return
    // value instead (#353); they are merged below via
    // `markdown_layout_glyph_drop_warnings` (markdown path) and were
    // already folded into `ocr_fallback_warnings` above (OCR path). ~keep
    for pdf_render_warning in crate::pdf::render::take_xberg_native_pdf_render_warnings() {
        crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, pdf_render_warning);
    }

    // Surface a hard layout-inference failure (e.g. CoreML kernel error) so
    // degraded no-layout output is never silent to the caller (#1344). Runs
    // whenever layout-detection is on, independent of OCR.
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    if let Some(warning) = sources.markdown_layout_warning {
        crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
    }

    // #353: merge `xberg_native_pdf` glyph-drop warnings captured while the
    // markdown layout pass rendered its pages off-thread inside
    // `spawn_blocking` (see the drain comment above).
    #[cfg(all(feature = "pdf", feature = "layout-detection"))]
    for warning in sources.markdown_layout_glyph_drop_warnings {
        crate::core::diagnostics::push_warning_deduped(&mut doc.processing_warnings, warning);
    }
}

pub(super) struct DocumentMetadataInputs {
    pub(super) pdf_metadata: crate::pdf::metadata::PdfExtractionMetadata,
    pub(super) output_format: Option<String>,
    pub(super) image_preprocessing: Option<crate::types::ImagePreprocessingMetadata>,
    pub(super) used_ocr: bool,
    pub(super) extraction_method: ExtractionMethod,
    pub(super) pdf_page_labels: Option<Vec<String>>,
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    pub(super) ocr_backend_additional_metadata: ahash::AHashMap<std::borrow::Cow<'static, str>, serde_json::Value>,
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    pub(super) ocr_coordinate_frames: Vec<crate::types::internal::OcrPageCoordinateFrame>,
}

pub(super) fn apply_document_metadata(
    doc: &mut InternalDocument,
    config: &ExtractionConfig,
    inputs: DocumentMetadataInputs,
) {
    let extract_pdf_metadata = config
        .pdf_options
        .as_ref()
        .is_none_or(|options| options.extract_metadata);
    doc.metadata = Metadata {
        output_format: inputs.output_format,
        image_preprocessing: inputs.image_preprocessing,
        ocr_used: inputs.used_ocr,
        ..Default::default()
    };
    apply_extracted_pdf_metadata(&mut doc.metadata, inputs.pdf_metadata, extract_pdf_metadata);
    doc.metadata.additional.insert(
        std::borrow::Cow::Borrowed("extraction_method"),
        serde_json::Value::String(inputs.extraction_method.as_str().to_string()),
    );
    // #1575: restore the OCR-pipeline keys taken above, now that the fresh `Metadata`
    // literal they would otherwise have been lost to already exists.
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    doc.metadata.additional.extend(inputs.ocr_backend_additional_metadata);

    // GH#1645: one authoritative processed-raster coordinate frame per OCR page with
    // public elements, so a multi-page consumer can normalize `OcrElement` geometry
    // without relying on the single document-wide `ocr_processed_image_width/height`
    // pair, which cannot describe differently sized, preprocessed, or rotated pages.
    // Rides in `additional` rather than a new public binding type, same as the
    // `page_labels` key below (issue #66).
    #[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
    if !inputs.ocr_coordinate_frames.is_empty() {
        doc.metadata.additional.insert(
            std::borrow::Cow::Borrowed(crate::ocr_metadata_keys::OCR_PAGE_COORDINATE_FRAMES_METADATA_KEY),
            serde_json::json!(inputs.ocr_coordinate_frames),
        );
    }

    // Issue #66: `/PageLabels` — one display label per page, index-aligned
    // with `pdf_metadata.page_structure`/`PageBoundary::page_number`.
    // `PdfMetadata` is an alef-listed type (no new public fields), so this
    // rides in `additional` instead. ~keep
    if extract_pdf_metadata && let Some(labels) = inputs.pdf_page_labels {
        doc.metadata
            .additional
            .insert(std::borrow::Cow::Borrowed("page_labels"), serde_json::json!(labels));
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) struct OcrPageSources<'a> {
    pub(super) page_texts: Option<&'a [String]>,
    pub(super) document_page_texts: Option<&'a [String]>,
    pub(super) results_map: Option<&'a ahash::AHashMap<u32, String>>,
}

/// Returns the pages whose content came from an accepted OCR read.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn apply_ocr_to_page_contents(
    config: &ExtractionConfig,
    sources: OcrPageSources<'_>,
    page_contents: &mut Option<Vec<PageContent>>,
    pdf_metadata: &mut crate::pdf::metadata::PdfExtractionMetadata,
) -> Vec<u32> {
    let mut ocr_text_pages: Vec<u32> = Vec::new();
    let keep_native_content = config.pages.as_ref().is_some_and(|pages| pages.keep_native_content);

    if let Some(pts) = sources.page_texts {
        if let Some(pages) = page_contents.as_mut() {
            ocr_text_pages.extend(replace_pages_with_ocr_page_texts(
                pages,
                pts,
                sources.document_page_texts,
                keep_native_content,
            ));
        } else {
            // Only the pages whose read the output kept: an empty per-page text was
            // discarded, not accepted (#2939).
            ocr_text_pages.extend(
                pts.iter()
                    .enumerate()
                    .filter(|(_, text)| !text.trim().is_empty())
                    .map(|(index, _)| (index + 1) as u32),
            );
            // The pages carry the restored native text of a discarded read, like the
            // document does (GH#3013).
            let page_texts = sources.document_page_texts.unwrap_or(pts);
            *page_contents = Some(
                page_texts
                    .iter()
                    .enumerate()
                    .map(|(i, text)| {
                        let content = crate::pdf::text::fix_pdf_control_chars(text).into_owned();
                        let is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&content));
                        crate::types::PageContent {
                            page_number: (i + 1) as u32,
                            content,
                            tables: Vec::new(),
                            image_indices: vec![],
                            image_preprocessing: None,
                            hierarchy: None,
                            is_blank,
                            layout_regions: None,
                            speaker_notes: None,
                            section_name: None,
                            sheet_name: None,
                            ocr_confidence: None,
                            native_content: None,
                        }
                    })
                    .collect(),
            );
        }
    }

    if let Some(results_map) = sources.results_map
        && let Some(pages) = page_contents.as_mut()
    {
        // The map holds only the pages `accepted_ocr_page_replacements` kept -- a
        // blank or discarded read is filtered and warned about there (unit-tested in
        // `ocr::tests`) -- so a page whose read the noise gate dropped is never
        // marked here (#2939).
        for page in pages.iter_mut() {
            if let Some(ocr_text) = results_map.get(&page.page_number) {
                replace_page_content_with_ocr_text(page, ocr_text, keep_native_content);
                ocr_text_pages.push(page.page_number);
            }
        }
    }

    if let Some(content_pages) = page_contents.as_ref()
        && let Some(ref mut page_structure) = pdf_metadata.page_structure
        && let Some(ref mut info_pages) = page_structure.pages
    {
        for info in info_pages.iter_mut() {
            if let Some(content_page) = content_pages.iter().find(|p| p.page_number == info.number) {
                info.is_blank = content_page.is_blank;
            }
        }
    }
    ocr_text_pages
}

#[cfg(all(feature = "pdf", feature = "tokio-runtime"))]
pub(super) async fn attach_embedded_files(doc: &mut InternalDocument, content: &[u8], config: &ExtractionConfig) {
    let (embedded_children, embedded_warnings) =
        crate::pdf::embedded_files::extract_and_process_embedded_files(content, config).await;
    if !embedded_children.is_empty() {
        match doc.children {
            Some(ref mut existing) => existing.extend(embedded_children),
            None => doc.children = Some(embedded_children),
        }
    }
    for warning in embedded_warnings {
        doc.processing_warnings.push(warning);
    }
}

// Native pages with layout hints carry formula elements holding the
// region's glyph text; a configured model replaces it with LaTeX.
// Accepted mixed OCR pages use their own layout inputs; untouched pages
// retain their Markdown inputs. Each page has one formula owner. ~keep
#[cfg(all(feature = "formula-recognition", feature = "layout-detection"))]
pub(super) async fn recognize_native_formulas(
    doc: &mut InternalDocument,
    config: &ExtractionConfig,
    content: &[u8],
    markdown_layout_inputs: Option<(&[image::RgbImage], &[crate::layout::DetectionResult])>,
    mixed_formula_layout_inputs: &mut Option<(Vec<image::DynamicImage>, Vec<crate::layout::DetectionResult>)>,
) {
    if let Some(layout) = config.layout.as_ref() {
        let mut side_channel = std::mem::take(&mut doc.formulas);
        let mixed_inputs = mixed_formula_layout_inputs.take();
        let inputs = select_formula_page_inputs(
            markdown_layout_inputs,
            mixed_inputs
                .as_ref()
                .map(|(images, detections)| (images.as_slice(), detections.as_slice())),
        );
        recognize_pdf_formula_page_inputs(Some(&mut *doc), &mut side_channel, &inputs, layout, content).await;
        doc.formulas = side_channel;
    }
}

#[cfg(all(feature = "liter-llm", feature = "layout-detection"))]
pub(super) async fn inject_vlm_region_results(
    doc: &mut InternalDocument,
    config: &ExtractionConfig,
    markdown_layout_images: Option<&[image::RgbImage]>,
    markdown_layout_hints: Option<&[Vec<crate::pdf::structure::types::LayoutHint>]>,
    markdown_layout_detections: Option<&[crate::layout::DetectionResult]>,
) {
    if let Some(vlm_cfg) = region_vlm::region_vlm_config(config)
        && let (Some(layout_images), Some(hints), Some(detections)) = (
            markdown_layout_images,
            markdown_layout_hints,
            markdown_layout_detections,
        )
    {
        let region_results = region_vlm::extract_vlm_regions(layout_images, hints, detections, vlm_cfg).await;
        if !region_results.is_empty() {
            tracing::debug!(
                target: "xberg::extractors::pdf",
                count = region_results.len(),
                "injecting VLM region results into document"
            );
            region_vlm::inject_region_results(doc, region_results);
        }
    }
}

pub(super) fn account_document_text(doc: &InternalDocument, config: &ExtractionConfig) -> crate::Result<()> {
    let mut budget = crate::extractors::security::SecurityBudget::from_config(config);
    for elem in &doc.elements {
        budget.account_text(elem.text.len())?;
    }
    Ok(())
}

// The routing gates have read the lost-glyph markers. Pages that stay native must not
// show them, so remove them from every native structure before any later step reads it.
// The text itself, which carries byte offsets, follows once its page boundaries are known. ~keep
pub(super) fn strip_native_lost_glyphs(
    page_contents: &mut Option<Vec<PageContent>>,
    tables: &mut [crate::types::Table],
    pre_rendered_doc: &mut Option<InternalDocument>,
) {
    for page in page_contents.iter_mut().flatten() {
        unmapped_glyphs::strip_page(page);
    }
    for table in tables {
        unmapped_glyphs::strip_table(table);
    }
    if let Some(native_doc) = pre_rendered_doc.as_mut() {
        unmapped_glyphs::strip_document(native_doc);
    }
}
