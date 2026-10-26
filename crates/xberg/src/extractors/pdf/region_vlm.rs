//! Per-region VLM extraction for PDF layout regions.
//!
//! When `vlm_fallback != Disabled` and layout detection has identified
//! `Picture` (figure), `Table` (dense table), or `Other` (unclassified complex
//! layout) regions, this module crops those regions from the page raster, calls
//! the VLM via [`crate::llm::region_extractor`], and splices the resulting
//! markdown back into the assembled document at the region's original position.
//! A table or other region's answer replaces the native tables and body paragraphs inside
//! it, matched by box, when it carries at least 90% of their words; the rest stay and the
//! answer is added beside them, as is a figure's description.
//! Native paragraphs have a box only when the PDF is built as a structured document
//! (`needs_structured_extraction` in `extraction.rs`), so on the flat path (Plain, Json
//! and other formats not built from Markdown) they stay, OCR paragraphs merged in with
//! boxes aside, and the answer goes after the page's last element.
//! The region and the text are compared as if in one page frame, so on a rotated
//! page, or one whose MediaBox does not start at the origin, a region can miss the
//! text it covers.
//!
//! Only compiled when `liter-llm` + `layout-detection` are both enabled
//! (not on Windows).

#![cfg(all(feature = "liter-llm", feature = "layout-detection"))]

use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder};

use crate::RegionKind;
use crate::core::config::{ExtractionConfig, LlmConfig, VlmFallbackPolicy};
use crate::layout::{DetectionResult, LayoutDetection};
use crate::llm::region_extractor::extract_region_with_vlm;
use crate::pdf::structure::types::{LayoutHint, LayoutHintClass};
use crate::types::BoundingBox;
use crate::types::document_structure::ContentLayer;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, RelationshipKind};

/// Minimum confidence threshold for a layout hint to trigger VLM region extraction.
const MIN_REGION_CONFIDENCE: f32 = 0.6;

/// Minimum pixel area for a cropped region to be sent to VLM.
///
/// Regions smaller than this are skipped — they are unlikely to contain meaningful
/// content and would waste VLM API tokens.
const MIN_REGION_PIXEL_AREA: u32 = 1_000;

/// Share of a native element's area that must lie inside a table or other region for
/// the region's answer to replace it. Above one half, an element belongs to at most one
/// of two disjoint regions, and the margin absorbs jitter between the layout box and the
/// native geometry. An element straddling the edge is kept, as is one the answer does not
/// carry ([`take_if_carried`]): duplicated text over lost text.
const REPLACE_CONTAINMENT: f64 = 0.6;

/// A VLM-extracted markdown string and the page index it belongs to.
pub(crate) struct RegionVlmResult {
    /// 0-based page index.
    pub page_index: usize,
    /// Markdown text from the VLM.
    pub markdown: String,
    /// The layout hint that triggered this extraction. Used by [`inject_region_results`]
    /// to splice the result at the region's original position rather than at the end
    /// of the page.
    pub hint: LayoutHint,
}

/// Maps a layout hint class to the [`RegionKind`] that should handle it via VLM,
/// or `None` if the class should stay on the classical extraction path.
///
/// - [`LayoutHintClass::Picture`] → [`RegionKind::Figure`]: figures/diagrams never
///   have usable classical text.
/// - [`LayoutHintClass::Table`] → [`RegionKind::DenseTable`]: classical table
///   reconstruction is prone to garbling dense/borderless tables.
/// - [`LayoutHintClass::Other`] → [`RegionKind::ComplexLayout`]: by definition, a
///   region the classical layout classifier could not categorise.
///
/// Wrapper classes with dedicated classical handling (`Form`, `KeyValueRegion`,
/// `DocumentIndex`) are intentionally excluded to avoid double-processing regions
/// that already extract cleanly.
const fn region_kind_for_hint(class_name: LayoutHintClass) -> Option<RegionKind> {
    match class_name {
        LayoutHintClass::Picture => Some(RegionKind::Figure),
        LayoutHintClass::Table => Some(RegionKind::DenseTable),
        LayoutHintClass::Other => Some(RegionKind::ComplexLayout),
        _ => None,
    }
}

/// Whether a region's answer stands in for the native content inside it. A figure's
/// description does not transcribe the labels or text drawn in the figure.
const fn answer_replaces_native(class_name: LayoutHintClass) -> bool {
    matches!(class_name, LayoutHintClass::Table | LayoutHintClass::Other)
}

/// The VLM that layout regions are sent to, when the fallback policy is not `Disabled`.
pub(crate) fn region_vlm_config(config: &ExtractionConfig) -> Option<&LlmConfig> {
    config
        .ocr
        .as_ref()
        .filter(|ocr| ocr.vlm_fallback != VlmFallbackPolicy::Disabled)
        .and_then(|ocr| ocr.vlm_config.as_ref())
}

/// Extract markdown from VLM for all figure / complex-layout regions across all pages.
///
/// Iterates the layout hints for each page. For hints whose class warrants VLM
/// extraction ([`LayoutHintClass::Picture`] and, when requested, dense tables),
/// crops the region from the page raster at the hint's pixel-space detection in
/// `layout_detections`, encodes it as PNG, and calls
/// [`extract_region_with_vlm`]. Results from all pages are collected and returned.
///
/// # Arguments
///
/// * `layout_images` — One rasterised `RgbImage` per page (same order as `layout_hints`).
/// * `layout_hints` — Per-page layout detection results, each a `Vec<LayoutHint>`.
/// * `layout_detections` — The pixel-space detections each page's hints were converted from, one to one.
/// * `llm_config` — LLM provider configuration for the VLM calls.
///
/// # Returns
///
/// A `Vec<RegionVlmResult>` with one entry per successfully extracted region.
/// Errors from individual regions are logged as warnings but do not propagate —
/// a VLM failure on one region must not abort the whole extraction.
pub(crate) async fn extract_vlm_regions(
    layout_images: &[image::RgbImage],
    layout_hints: &[Vec<LayoutHint>],
    layout_detections: &[DetectionResult],
    llm_config: &LlmConfig,
) -> Vec<RegionVlmResult> {
    let mut results: Vec<RegionVlmResult> = Vec::new();

    for (page_index, ((page_image, hints), detection)) in layout_images
        .iter()
        .zip(layout_hints.iter())
        .zip(layout_detections.iter())
        .enumerate()
    {
        let Some(detections) = paired_detections(page_index, hints, detection) else {
            continue;
        };
        for (hint, det) in hints.iter().zip(detections) {
            if let Some(result) = extract_vlm_region(page_index, page_image, hint, det, llm_config).await {
                results.push(result);
            }
        }
    }

    results
}

/// Returns a page's layout detections only when they pair one to one with its hints.
fn paired_detections<'a>(
    page_index: usize,
    hints: &[LayoutHint],
    detection: &'a DetectionResult,
) -> Option<&'a [LayoutDetection]> {
    if hints.len() != detection.detections.len() {
        tracing::warn!(
            page = page_index,
            hints = hints.len(),
            detections = detection.detections.len(),
            "layout hints and detections disagree; skipping VLM regions on this page"
        );
        return None;
    }
    Some(&detection.detections)
}

/// PNG-encodes a cropped region, logging and returning `None` on encode failure so a
/// single bad crop suppresses only that region rather than aborting extraction. Split
/// out of [`extract_vlm_region`] purely to shorten that function.
fn encode_region_png(page_index: usize, crop: &image::RgbImage) -> Option<Vec<u8>> {
    let mut png_buf = Cursor::new(Vec::<u8>::new());
    let encode_result = image::codecs::png::PngEncoder::new(&mut png_buf).write_image(
        crop.as_raw(),
        crop.width(),
        crop.height(),
        ExtendedColorType::Rgb8,
    );
    if let Err(e) = encode_result {
        tracing::warn!(
            page = page_index,
            error = %e,
            "failed to PNG-encode region crop; skipping VLM call"
        );
        return None;
    }
    Some(png_buf.into_inner())
}

/// Returns the region kind and the pixel crop `(x, y, w, h)` for `hint`, cut from its detection `det`,
/// or `None` when the hint is below confidence or ineligible for VLM handling, or the crop is empty
/// or under `MIN_REGION_PIXEL_AREA`.
fn region_crop(
    page_index: usize,
    page_image: &image::RgbImage,
    hint: &LayoutHint,
    det: &LayoutDetection,
) -> Option<(RegionKind, (u32, u32, u32, u32))> {
    if hint.confidence < MIN_REGION_CONFIDENCE {
        return None;
    }

    let region_kind = region_kind_for_hint(hint.class_name)?;

    let (x_left, y_top, crop_w, crop_h) = det.bbox.clamp_to_image(page_image.width(), page_image.height())?;

    if crop_w * crop_h < MIN_REGION_PIXEL_AREA {
        tracing::trace!(
            page = page_index,
            crop_w,
            crop_h,
            "region too small for VLM extraction; skipping"
        );
        return None;
    }

    Some((region_kind, (x_left, y_top, crop_w, crop_h)))
}

/// Extracts a single layout hint's region via VLM.
async fn extract_vlm_region(
    page_index: usize,
    page_image: &image::RgbImage,
    hint: &LayoutHint,
    det: &LayoutDetection,
    llm_config: &LlmConfig,
) -> Option<RegionVlmResult> {
    let (region_kind, (x_left, y_top, crop_w, crop_h)) = region_crop(page_index, page_image, hint, det)?;

    let crop = image::imageops::crop_imm(page_image, x_left, y_top, crop_w, crop_h).to_image();
    let crop_bytes = encode_region_png(page_index, &crop)?;

    tracing::debug!(
        page = page_index,
        region_kind = ?region_kind,
        confidence = hint.confidence,
        crop_w,
        crop_h,
        "sending region to VLM"
    );

    match extract_region_with_vlm(&crop_bytes, "image/png", region_kind, llm_config, None).await {
        Ok(markdown) => {
            let trimmed = markdown.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(RegionVlmResult {
                    page_index,
                    markdown: trimmed,
                    hint: hint.clone(),
                })
            }
        }
        Err(e) => {
            tracing::warn!(
                page = page_index,
                region_kind = ?region_kind,
                error = %e,
                "VLM region extraction failed; region suppressed"
            );
            None
        }
    }
}

/// Finds the index in `elements` at which a region with top edge `hint_top` (PDF
/// coordinate space, y=0 at page bottom) belonging to `page_num` should be spliced.
///
/// Scans elements belonging to `page_num` and returns the index of the first one
/// whose bounding box starts below `hint_top` (i.e. the region sits above it in
/// reading order) — the new element is inserted right before that element. If no
/// element on the page has a bbox positioned below the region, the region is
/// inserted immediately after the last element of that page. If the page has no
/// elements at all, it is inserted right before the first element of the next
/// page, or at the end of the document if there is none.
fn find_insertion_index(elements: &[crate::types::internal::InternalElement], page_num: u32, hint_top: f32) -> usize {
    let mut insert_at = elements.len();
    for (index, element) in elements.iter().enumerate() {
        match element.page {
            Some(page) if page == page_num => {
                if let Some(bbox) = element.bbox
                    && (bbox.y1 as f32) < hint_top
                {
                    return index;
                }
                insert_at = index + 1;
            }
            Some(page) if page > page_num => return insert_at.min(index),
            _ => {}
        }
    }
    insert_at
}

/// Shifts every `Relationship` index at or past `from_index` by one position.
///
/// `InternalDocument::elements` is documented as append-only during extraction
/// precisely because `Relationship::source` and `RelationshipTarget::Index` store
/// raw positions into that vector (e.g. caption-to-figure links set up in
/// `pdf::structure::assembly`). Splicing a new element into the middle of
/// `elements` — as [`inject_region_results`] does to land VLM output at its
/// anchor — must shift those already-recorded indices in lockstep, or the
/// relationship silently starts pointing at the wrong element.
fn shift_relationship_indices(document: &mut crate::types::internal::InternalDocument, from_index: usize) {
    use crate::types::internal::RelationshipTarget;

    let Ok(from_index) = u32::try_from(from_index) else {
        return;
    };
    for relationship in &mut document.relationships {
        if relationship.source >= from_index {
            relationship.source += 1;
        }
        if let RelationshipTarget::Index(target_index) = &mut relationship.target
            && *target_index >= from_index
        {
            *target_index += 1;
        }
    }
}

/// Inserts `element` at `at` and returns its final index. The assembly emits page breaks
/// only between pages with content, so the first element on an empty page (a scan) gets
/// its own breaks; without them it is attributed to the previous page. Container starts
/// carry no page, so it goes before any that open the next page, not inside its list.
fn splice_element(
    document: &mut crate::types::internal::InternalDocument,
    mut at: usize,
    element: crate::types::internal::InternalElement,
) -> usize {
    let is_content = |element: &InternalElement| !matches!(element.kind, ElementKind::PageBreak);
    let first_on_page = element.page.is_some()
        && !document
            .elements
            .iter()
            .any(|existing| existing.page == element.page && is_content(existing));
    if first_on_page {
        while at > 0 && document.elements[at - 1].kind.is_container_start() && document.elements[at - 1].page.is_none()
        {
            at -= 1;
        }
    }
    if first_on_page && at > 0 && is_content(&document.elements[at - 1]) {
        insert_element(document, at, InternalElement::text(ElementKind::PageBreak, "", 0));
        at += 1;
    }
    insert_element(document, at, element);
    if first_on_page && document.elements.get(at + 1).is_some_and(is_content) {
        insert_element(document, at + 1, InternalElement::text(ElementKind::PageBreak, "", 0));
    }
    at
}

fn insert_element(
    document: &mut crate::types::internal::InternalDocument,
    at: usize,
    element: crate::types::internal::InternalElement,
) {
    shift_relationship_indices(document, at);
    document.elements.insert(at, element);
}

/// Share of `inner`'s area inside the hint's box, both in PDF points. The hint needs no corner swap:
/// `region_crop` refuses an inverted detection, and an inverted hint overlaps no element anyway.
fn contained_share(inner: &BoundingBox, hint: &LayoutHint) -> f64 {
    let (left, right) = (f64::from(hint.left), f64::from(hint.right));
    let (bottom, top) = (f64::from(hint.bottom), f64::from(hint.top));
    let (x0, x1) = (inner.x0.min(inner.x1), inner.x0.max(inner.x1));
    let (y0, y1) = (inner.y0.min(inner.y1), inner.y0.max(inner.y1));
    let area = (x1 - x0) * (y1 - y0);
    let width = (x1.min(right) - x0.max(left)).max(0.0);
    let height = (y1.min(top) - y0.max(bottom)).max(0.0);
    if area > 0.0 { width * height / area } else { 0.0 }
}

/// Page and box of an element; a table element without its own falls back to its table's.
fn element_page_box(document: &InternalDocument, element: &InternalElement) -> Option<(u32, BoundingBox)> {
    let table = match element.kind {
        ElementKind::Table { table_index } => document.tables.get(table_index as usize),
        _ => None,
    };
    let page = element.page.or(table.map(|table| table.page_number))?;
    let bbox = element.bbox.or(table.and_then(|table| table.bounding_box))?;
    Some((page, bbox))
}

/// Whether `element` is mostly inside the region's box on its page.
fn inside_region(document: &InternalDocument, element: &InternalElement, page_num: u32, hint: &LayoutHint) -> bool {
    element_page_box(document, element)
        .is_some_and(|(page, bbox)| page == page_num && contained_share(&bbox, hint) >= REPLACE_CONTAINMENT)
}

/// Whether an answer may stand in for `element`: a table, or a body paragraph that is not linked as
/// a caption. Either goes only if the answer carries its words ([`take_if_carried`]). Everything else stays.
fn replaceable(element: &InternalElement, is_caption: bool) -> bool {
    match element.kind {
        ElementKind::Table { .. } => true,
        ElementKind::Paragraph => !is_caption && element.layer == ContentLayer::Body,
        _ => false,
    }
}

/// Whether `answer`'s remaining words carry at least `MIN_TABLE_TOKEN_REPRESENTATION` of `element`'s
/// (a table's are its cells'); if so the matched words are taken out, so two elements cannot claim one,
/// and otherwise nothing is. An element of fewer than ten words needs every word; one with none passes.
/// A region answer can be degenerate (a repeated symbol, an invented table, a cut-off transcription),
/// and replacing native text with it loses that text, while a missed match only prints it twice.
fn take_if_carried(
    document: &InternalDocument,
    element: &InternalElement,
    answer: &mut ahash::AHashMap<String, usize>,
) -> bool {
    let words = match element.kind {
        ElementKind::Table { table_index } => {
            let cells = document.tables.get(table_index as usize).into_iter();
            super::token_multiset(cells.flat_map(|table| table.cells.iter().flatten()).map(String::as_str))
        }
        _ => super::token_multiset([element.text.as_str()]),
    };
    super::take_if_represented(words, answer)
}

/// `answer` with every cell separator turned into a space, so the word count sees the cells of a compact
/// pipe table (`|a|b|`), of `<br>` line breaks and of an HTML table as the native table's words.
/// A tag is `<` followed by a letter, `/` or `!` up to the next `>`; any other `<` stays text. The scan
/// moves forward only, so the cost is linear in the answer's length.
fn answer_text_for_words(answer: &str) -> String {
    let mut text = String::with_capacity(answer.len());
    let mut rest = answer;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let tail = &rest[open + 1..];
        let starts_tag = tail.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!');
        if !starts_tag {
            text.push('<');
            rest = tail;
            continue;
        }
        let Some(close) = tail.find('>') else {
            // No `>` remains, so no later tag can close: keep the rest as text, in one forward scan.
            text.push('<');
            rest = tail;
            break;
        };
        text.push(' ');
        rest = &tail[close + 1..];
    }
    text.push_str(rest);
    text.replace('|', " ")
}

/// Also drops each container start and end pair the removal left with nothing inside, so the
/// answer is not spliced into an empty group and the page reads as empty to [`splice_element`].
fn drop_emptied_containers(elements: &[InternalElement], drop: &mut [bool]) {
    // (start index, holds a kept element, holds a dropped one)
    let mut open: Vec<(usize, bool, bool)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if element.kind.is_container_start() {
            open.push((index, false, false));
            continue;
        }
        let (kept, dropped) = if element.kind.is_container_end()
            && let Some((start, kept, dropped)) = open.pop()
        {
            let emptied = dropped && !kept;
            if emptied {
                drop[start] = true;
                drop[index] = true;
            }
            (!emptied, emptied)
        } else {
            (!drop[index], drop[index])
        };
        if let Some(parent) = open.last_mut() {
            parent.1 |= kept;
            parent.2 |= dropped;
        }
    }
}

/// Also drops a page break left leading or doubled by the removal, so a page emptied by it
/// reads as empty to [`splice_element`]. A trailing one needs no drop: the answer follows it.
fn drop_emptied_page_breaks(elements: &[InternalElement], drop: &mut [bool]) {
    let mut last_kept: Option<usize> = None;
    let mut removed_since = false;
    for (index, element) in elements.iter().enumerate() {
        if drop[index] {
            removed_since = true;
            continue;
        }
        let is_break = matches!(element.kind, ElementKind::PageBreak);
        let after_break = last_kept.is_none_or(|kept| matches!(elements[kept].kind, ElementKind::PageBreak));
        if is_break && removed_since && after_break {
            drop[index] = true;
            continue;
        }
        last_kept = Some(index);
        removed_since = false;
    }
}

/// Removes the elements flagged in `drop` and every relationship whose source or `Index` target is
/// one of them; the remaining indices are remapped. Not the OCR path's `remap_relationships`: it also drops
/// `Key` links that never resolved, which `resolve_relationships` reports in a warning later.
fn remove_elements(document: &mut InternalDocument, drop: &[bool]) {
    use crate::types::internal::RelationshipTarget;

    let mut next = 0u32;
    let new_index: Vec<Option<u32>> = drop
        .iter()
        .map(|dropped| {
            (!dropped).then(|| {
                next += 1;
                next - 1
            })
        })
        .collect();
    let mut flags = drop.iter();
    document.elements.retain(|_| !flags.next().copied().unwrap_or(false));
    document.relationships.retain_mut(|relationship| {
        let Some(source) = new_index.get(relationship.source as usize).copied().flatten() else {
            return false;
        };
        relationship.source = source;
        if let RelationshipTarget::Index(target) = &mut relationship.target {
            let Some(remapped) = new_index.get(*target as usize).copied().flatten() else {
                return false;
            };
            *target = remapped;
        }
        true
    });
}

/// Removes the native content a table or other region's `answer` replaces from the document elements.
/// `document.tables` and any prebuilt pages are not touched, so the `tables` list keeps the native
/// table, as do prebuilt `pages[n].tables` and Plain or Json page content; page content rendered
/// from the elements, and pages built from them, show the answer. Returns where the removed content
/// began in the remaining elements, and the depth of the first element replaced, so the answer takes
/// its place; `None` when that element has no page of its own, as the flat path's table element
/// pushed at the document's end, whose slot is not where its page is.
fn remove_region_content(
    document: &mut InternalDocument,
    page_num: u32,
    hint: &LayoutHint,
    answer: &str,
) -> Option<(usize, u16)> {
    let captions: ahash::AHashSet<u32> = document
        .relationships
        .iter()
        .filter(|relationship| matches!(relationship.kind, RelationshipKind::Caption))
        .map(|relationship| relationship.source)
        .collect();
    let mut answer_words = super::token_multiset([answer_text_for_words(answer).as_str()]);
    let candidates: Vec<usize> = (0..document.elements.len())
        .filter(|&index| {
            let element = &document.elements[index];
            replaceable(element, captions.contains(&(index as u32))) && inside_region(document, element, page_num, hint)
        })
        .collect();
    let (tables, paragraphs): (Vec<usize>, Vec<usize>) = candidates
        .into_iter()
        .partition(|&index| matches!(document.elements[index].kind, ElementKind::Table { .. }));
    let mut drop = vec![false; document.elements.len()];
    // A passing table's cells are what the answer transcribes, so they spend their words first; otherwise a
    // title or note repeating a cell's words would be removed on the strength of the table's transcription. ~keep
    for index in tables.into_iter().chain(paragraphs) {
        drop[index] = take_if_carried(document, &document.elements[index], &mut answer_words);
    }
    let first = &document.elements[drop.iter().position(|dropped| *dropped)?];
    let (depth, paged) = (first.depth, first.page == Some(page_num));
    drop_emptied_containers(&document.elements, &mut drop);
    drop_emptied_page_breaks(&document.elements, &mut drop);
    let at = drop.iter().take_while(|dropped| !**dropped).count();
    remove_elements(document, &drop);
    paged.then_some((at, depth))
}

/// Inject VLM-extracted region markdown into an `InternalDocument`.
///
/// A table or other region's answer first replaces the native tables and body paragraphs mostly
/// inside it ([`REPLACE_CONTAINMENT`]) whose words it carries ([`take_if_carried`]), and takes their
/// place: the position the removed content began at, and the depth of the first element replaced.
/// Any other answer, a figure's, one that replaced nothing or one whose replaced content had no page
/// of its own, is placed by [`find_insertion_index`] at depth 0. Placing a replacing answer by
/// geometry instead would land it inside a list or group that follows the table, since container
/// starts carry no box. Existing `Relationship` indices are remapped so caption/link relationships
/// recorded before the splice keep pointing at the correct element.
pub(crate) fn inject_region_results(document: &mut InternalDocument, results: Vec<RegionVlmResult>) {
    for result in results {
        let page_num = (result.page_index + 1) as u32;
        let replaced = answer_replaces_native(result.hint.class_name)
            .then(|| remove_region_content(document, page_num, &result.hint, &result.markdown))
            .flatten();
        let (insert_at, depth) =
            replaced.unwrap_or_else(|| (find_insertion_index(&document.elements, page_num, result.hint.top), 0));
        let inserted_at = splice_element(
            document,
            insert_at,
            InternalElement::text(ElementKind::Paragraph, result.markdown, depth).with_page(page_num),
        );
        tracing::debug!(page = page_num, inserted_at, "injected VLM region result into document");
    }
}

#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod tests;
