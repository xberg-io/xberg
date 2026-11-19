//! Stages that finish the `InternalDocument` built by `PdfExtractor::extract_core_native_with_layout_pages`.

use super::OcrLayoutGateDecisions;
use super::extraction;
#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
use super::insert_image_ocr_text_anchor;
#[cfg(feature = "layout-detection")]
use super::layout_gate_metadata;
use crate::core::config::ExtractionConfig;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use crate::types::ExtractionMethod;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, PdfPageCoordinateFrame};
use crate::types::{ExtractedImage, PageContent};

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
