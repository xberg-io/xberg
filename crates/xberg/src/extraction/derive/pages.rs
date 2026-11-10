//! Per-page `PageContent` / `OcrElement` derivation, split out of `derive` by area.

use std::sync::Arc;

use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrConfidence, OcrElement};
use crate::types::page::PageContent;
use crate::types::tables::Table;

use super::surfaces::MarkupPageBody;

pub(super) fn derive_pages(
    doc: &mut InternalDocument,
    content: &str,
    output_format: &crate::core::config::OutputFormat,
    content_is_plain: bool,
    observer: &mut impl super::surfaces::DerivationObserver,
) -> Option<Vec<PageContent>> {
    let raw_pages = doc.prebuilt_pages.take().or_else(|| build_pages(doc, content_is_plain));
    let mut pages = apply_page_content_format(raw_pages, doc, output_format, observer);
    let mut reconciled = !content_is_plain || reconcile_page_boundaries(&mut doc.metadata, content, pages.as_deref());
    // ~keep Prebuilt page text (OCR) renders tables and block separators its own way, so it
    // cannot be found in the plain content. Render the pages that are not found from their
    // elements as `render_plain` does and locate that instead (#2825).
    if !reconciled
        && let Some(rerendered) = rerender_pages_plain(pages.as_deref(), doc, content)
        && reconcile_page_boundaries(&mut doc.metadata, content, Some(&rerendered))
    {
        pages = Some(rerendered);
        reconciled = true;
    }
    if !reconciled && drop_unusable_page_boundaries(&mut doc.metadata, content) {
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "page_boundaries",
            "Page byte ranges were measured on a different rendering of the text and could not be \
             mapped onto the returned content, so they were omitted",
        );
    }
    pages
}

/// Re-point `metadata.pages.boundaries` at `content` by locating each page's text in it, in
/// order. Returns `true` only when every page was located exactly and the boundaries were
/// replaced; otherwise the boundaries are left as they were. A metadata with no boundaries
/// returns `false`.
pub(crate) fn reconcile_page_boundaries(
    metadata: &mut crate::types::Metadata,
    content: &str,
    pages: Option<&[PageContent]>,
) -> bool {
    let Some(boundaries) = metadata
        .pages
        .as_mut()
        .and_then(|structure| structure.boundaries.as_mut())
    else {
        return false;
    };
    let Some(pages) = pages.filter(|pages| pages.len() == boundaries.len()) else {
        return false;
    };
    let mut reconciled = Vec::with_capacity(boundaries.len());
    let mut search_offset = 0;
    let mut located_any = false;
    for (boundary, page) in boundaries.iter().zip(pages) {
        if boundary.page_number != page.page_number {
            return false;
        }
        if page.content.is_empty() {
            reconciled.push(crate::types::PageBoundary {
                byte_start: search_offset,
                byte_end: search_offset,
                page_number: page.page_number,
            });
            continue;
        }
        located_any = true;
        let Some(relative_start) = content[search_offset..].find(&page.content) else {
            return false;
        };
        let byte_start = search_offset + relative_start;
        let byte_end = byte_start + page.content.len();
        reconciled.push(crate::types::PageBoundary {
            byte_start,
            byte_end,
            page_number: page.page_number,
        });
        search_offset = byte_end;
    }
    // ~keep a blank page is an empty range, but only text actually located proves the mapping; all-blank pages prove nothing.
    if !located_any {
        return false;
    }
    // ~keep OCR boundaries can describe an intermediate rendering with different separators (#2128).
    // Replace only a complete exact match; unmatched pages retain their original extraction metadata.
    *boundaries = reconciled;
    true
}

/// Remove `metadata.pages.boundaries` when they cannot index `content`.
///
/// The offsets were measured on another rendering of the document, so a range that falls
/// outside `content` or inside a UTF-8 sequence is provably stale. Returns `true` when
/// the boundaries were dropped.
pub(crate) fn drop_unusable_page_boundaries(metadata: &mut crate::types::Metadata, content: &str) -> bool {
    let Some(structure) = metadata.pages.as_mut() else {
        return false;
    };
    let usable = structure.boundaries.as_ref().is_none_or(|boundaries| {
        boundaries
            .iter()
            .all(|b| b.byte_start <= b.byte_end && content.get(b.byte_start..b.byte_end).is_some())
    });
    if usable {
        return false;
    }
    structure.boundaries = None;
    true
}

/// Build per-page `PageContent` from page-grouped elements.
///
/// `content_is_plain` says that `result.content` is the plain rendering of the document:
/// plain output, or a custom format name with no renderer, which falls back to plain. Then
/// a page has the OCR text of a picture where `render_plain` writes it, so `pages[n].content`
/// and `result.content` agree. For every other output a page has the text of its elements
/// only. ~keep
pub(super) fn build_pages(doc: &InternalDocument, content_is_plain: bool) -> Option<Vec<PageContent>> {
    let mut page_map: std::collections::BTreeMap<u32, Vec<&InternalElement>> = std::collections::BTreeMap::new();

    for elem in &doc.elements {
        if let Some(page) = elem.page {
            page_map.entry(page).or_default().push(elem);
        }
    }

    if page_map.is_empty() {
        return None;
    }

    let arc_tables: Vec<Arc<Table>> = doc.tables.iter().map(|t| Arc::new(t.clone())).collect();

    let pages: Vec<PageContent> = page_map
        .into_iter()
        .map(|(page_num, elems)| {
            let mut content = String::new();
            let mut tables = Vec::new();
            let mut image_indices = Vec::new();
            for elem in &elems {
                // `render_plain` drops everything outside the body layer, so page content
                // must drop it too — otherwise running headers and footers appear in
                // `pages[n].content` but not in `result.content` for `OutputFormat::Plain`. ~keep
                if !crate::rendering::common::is_body_element(elem) {
                    continue;
                }
                if elem.kind.is_container_start() || elem.kind.is_container_end() {
                    continue;
                }
                let mut picture_ocr_text = None;
                match elem.kind {
                    ElementKind::Table { table_index } => {
                        if let Some(arc_table) = arc_tables.get(table_index as usize) {
                            tables.push(Arc::clone(arc_table));
                        }
                    }
                    ElementKind::Image { image_index } if (image_index as usize) < doc.images.len() => {
                        image_indices.push(image_index);
                        // The OCR text of a picture goes after the picture, as `render_plain`
                        // writes it. An image element of a PDF document can hold that text as
                        // its own text already. ~keep
                        if content_is_plain {
                            picture_ocr_text = crate::rendering::common::image_ocr_text(doc, elem, image_index)
                                .filter(|ocr_text| *ocr_text != elem.text);
                        }
                    }
                    _ => {}
                }
                let own_text = Some(elem.text.as_str()).filter(|text| !text.is_empty());
                for block in [own_text, picture_ocr_text].into_iter().flatten() {
                    if !content.is_empty() {
                        content.push_str("\n\n");
                    }
                    content.push_str(block);
                }
            }

            PageContent {
                page_number: page_num,
                content,
                tables,
                image_indices,
                image_preprocessing: None,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
                ocr_confidence: None,
                native_content: None,
            }
        })
        .collect();

    Some(pages)
}

/// Re-render each page's content using the requested output format.
///
/// Called after pages are built but before `derive_document_structure_inner` moves
/// element text out of the document. For Plain/Json/Custom formats this
/// is a no-op. For Markdown/Djot/Html/DocTags, each page's element subset is
/// rendered with the same renderer used for the full document, so `pages[n].content`
/// matches the format of `result.content` after `apply_output_format`.
///
/// Pages whose `page_number` has no matching page-tagged elements (e.g., natively
/// extracted PDF pages where individual elements are not page-tracked) are returned
/// unchanged — their original content is preserved.
pub(super) fn apply_page_content_format(
    pages: Option<Vec<PageContent>>,
    doc: &InternalDocument,
    output_format: &crate::core::config::OutputFormat,
    observer: &mut impl super::surfaces::DerivationObserver,
) -> Option<Vec<PageContent>> {
    use crate::core::config::OutputFormat;

    let observe_markup = observer.captures_verified_markup()
        && (matches!(output_format, OutputFormat::Html | OutputFormat::Djot)
            || matches!(output_format, OutputFormat::Custom(name) if name == "html"));
    let renderer: Option<fn(&InternalDocument) -> String> = match output_format {
        OutputFormat::Markdown => Some(crate::rendering::render_markdown),
        OutputFormat::Djot => Some(crate::rendering::render_djot),
        OutputFormat::Html => Some(crate::rendering::render_html),
        OutputFormat::DocTags => Some(crate::rendering::render_doctags),
        OutputFormat::Custom(_) if observe_markup => None,
        OutputFormat::Plain | OutputFormat::Json | OutputFormat::Custom(_) => return pages,
    };

    let pages = pages?;
    let elements_by_page = compute_page_membership(doc);
    if elements_by_page.is_empty() && !observe_markup {
        return Some(pages);
    }

    let pages = pages
        .into_iter()
        .enumerate()
        .map(|(index, page)| match elements_by_page.get(&page.page_number) {
            Some(elem_indices) => render_sub_page(page, elem_indices, doc, renderer, (index, output_format), observer),
            None => {
                if observe_markup {
                    // No page body was rendered. Only an empty original has no unproved payload. ~keep
                    observe_literal_page(&page, index, "", observer);
                }
                page
            }
        })
        .collect();

    Some(pages)
}

/// Which element indices belong to each page, keyed by page number.
///
/// Container markers (`ListStart`/`ListEnd`, quotes, groups) are never page-tagged:
/// `InternalDocumentBuilder::push_list`/`end_list` pass `page: None` even when every element
/// they wrap is tagged. Filtering strictly on `elem.page.is_some()` therefore dropped them
/// from a page's subset, and `build_comrak_ast` then saw each `ListItem` with no open list
/// parent and wrapped it in a fresh single-item list -- rendering a nested list as flat,
/// blank-line-separated bullets in `pages[N].content` (GH#1503 made this reachable by tagging
/// every element with a page). A start inherits the page of the next tagged element it opens
/// before; an end inherits the page of the last tagged element it closes after. ~keep
fn compute_page_membership(doc: &InternalDocument) -> std::collections::BTreeMap<u32, Vec<usize>> {
    let mut next_tagged_page: Vec<Option<u32>> = vec![None; doc.elements.len()];
    let mut seen: Option<u32> = None;
    for (idx, elem) in doc.elements.iter().enumerate().rev() {
        if elem.page.is_some() {
            seen = elem.page;
        }
        next_tagged_page[idx] = seen;
    }
    let mut prev_tagged_page: Vec<Option<u32>> = vec![None; doc.elements.len()];
    seen = None;
    for (idx, elem) in doc.elements.iter().enumerate() {
        prev_tagged_page[idx] = seen;
        if elem.page.is_some() {
            seen = elem.page;
        }
    }

    let mut elements_by_page: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    for (idx, elem) in doc.elements.iter().enumerate() {
        let page_num = elem.page.or_else(|| {
            if elem.kind.is_container_start() {
                next_tagged_page[idx]
            } else if elem.kind.is_container_end() {
                prev_tagged_page[idx]
            } else {
                None
            }
        });
        if let Some(page_num) = page_num {
            elements_by_page.entry(page_num).or_default().push(idx);
        }
    }
    elements_by_page
}

/// A document of just the elements at `elem_indices`, with the tables and images they
/// reference renumbered.
fn build_sub_doc(elem_indices: &[usize], doc: &InternalDocument) -> InternalDocument {
    let mut table_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
    let mut sub_tables: Vec<Table> = Vec::new();
    let mut image_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
    let mut sub_images: Vec<crate::types::ExtractedImage> = Vec::new();
    for &i in elem_indices {
        match doc.elements[i].kind {
            ElementKind::Table { table_index } if !table_remap.contains_key(&table_index) => {
                let new_idx = sub_tables.len() as u32;
                table_remap.insert(table_index, new_idx);
                if let Some(t) = doc.tables.get(table_index as usize) {
                    sub_tables.push(t.clone());
                }
            }
            ElementKind::Image { image_index } if !image_remap.contains_key(&image_index) => {
                let new_idx = sub_images.len() as u32;
                image_remap.insert(image_index, new_idx);
                if let Some(img) = doc.images.get(image_index as usize) {
                    sub_images.push(img.clone());
                }
            }
            _ => {}
        }
    }

    let elements: Vec<InternalElement> = elem_indices
        .iter()
        .map(|&i| {
            let mut elem = doc.elements[i].clone();
            match elem.kind {
                ElementKind::Table { ref mut table_index } => {
                    if let Some(&new_idx) = table_remap.get(table_index) {
                        *table_index = new_idx;
                    }
                }
                ElementKind::Image { ref mut image_index } => {
                    if let Some(&new_idx) = image_remap.get(image_index) {
                        *image_index = new_idx;
                    }
                }
                _ => {}
            }
            elem
        })
        .collect();

    let mut sub_doc = InternalDocument::new(&doc.source_format);
    sub_doc.elements = elements;
    sub_doc.tables = sub_tables;
    sub_doc.images = sub_images;
    // A page shows the OCR text of a picture the same way the document does. ~keep
    sub_doc.ocr_text_only = doc.ocr_text_only;
    sub_doc.append_ocr_text = doc.append_ocr_text;
    sub_doc
}

/// Pages whose prebuilt text is not found in `content` (in page order) rendered from their
/// own elements by `render_plain`; a page whose text is found keeps it. `None` when no page has
/// elements or a re-rendered page is still not found.
fn rerender_pages_plain(
    pages: Option<&[PageContent]>,
    doc: &InternalDocument,
    content: &str,
) -> Option<Vec<PageContent>> {
    let pages = pages?;
    let elements_by_page = compute_page_membership(doc);
    if elements_by_page.is_empty() {
        return None;
    }
    let mut search_offset = 0;
    let mut out = Vec::with_capacity(pages.len());
    for page in pages {
        let mut page = page.clone();
        let found = |text: &str, from: usize| content.get(from..).and_then(|rest| rest.find(text)).map(|i| from + i);
        if !page.content.is_empty() {
            if let Some(start) = found(&page.content, search_offset) {
                search_offset = start + page.content.len();
                out.push(page);
                continue;
            }
            let indices = elements_by_page.get(&page.page_number)?;
            let rendered = crate::rendering::render_plain(&build_sub_doc(indices, doc));
            let start = found(&rendered, search_offset)?;
            tracing::debug!(
                page = page.page_number,
                prebuilt_len = page.content.len(),
                rendered_len = rendered.len(),
                "page text replaced by the plain rendering of its elements"
            );
            search_offset = start + rendered.len();
            page.content = rendered;
        }
        out.push(page);
    }
    Some(out)
}

/// Re-render one page's `content` from just the elements (and the tables/images they
/// reference, renumbered) that belong to it.
fn render_sub_page(
    page: PageContent,
    elem_indices: &[usize],
    doc: &InternalDocument,
    renderer: Option<fn(&InternalDocument) -> String>,
    observation: (usize, &crate::core::config::OutputFormat),
    observer: &mut impl super::surfaces::DerivationObserver,
) -> PageContent {
    let sub_doc = build_sub_doc(elem_indices, doc);

    render_observed_page(page, &sub_doc, renderer, observation, observer)
}

fn render_observed_page(
    mut page: PageContent,
    doc: &InternalDocument,
    renderer: Option<fn(&InternalDocument) -> String>,
    (index, format): (usize, &crate::core::config::OutputFormat),
    observer: &mut impl super::surfaces::DerivationObserver,
) -> PageContent {
    use crate::core::config::OutputFormat;
    if matches!(format, OutputFormat::Markdown) && observer.captures_pages() {
        let all_owned = crate::rendering::markdown_tag_exemptions_are_owned(doc);
        let budget = if all_owned {
            None
        } else {
            observer.markdown_tag_budget()
        };
        let (rendered, ranges) = crate::rendering::render_markdown_observed(doc, budget);
        if !rendered.is_empty() {
            observer.page_from_source(
                index,
                page.page_number,
                &rendered,
                crate::rendering::render_plain(doc),
                all_owned,
            );
            observer.page_tag_ranges(ranges);
            page.content = rendered;
        }
        return page;
    }
    let rendered = match renderer {
        Some(renderer)
            if observer.captures_verified_markup() && matches!(format, OutputFormat::Html | OutputFormat::Djot) =>
        {
            let plain = crate::rendering::render_plain(doc);
            let (rendered, verified) = if matches!(format, OutputFormat::Html) {
                crate::rendering::render_html_checked(doc, Some(&plain))
            } else {
                let rendered = renderer(doc);
                let verified = crate::rendering::has_djot_plain_body(doc);
                (rendered, verified)
            };
            if rendered.is_empty() {
                observe_literal_page(&page, index, &plain, observer);
            } else {
                let body = if verified {
                    MarkupPageBody::Rendered(
                        #[cfg(feature = "redaction")]
                        plain,
                    )
                } else {
                    MarkupPageBody::Unverified
                };
                observer.page_markup(index, &rendered, body);
            }
            rendered
        }
        Some(renderer) => renderer(doc),
        None => {
            // Custom HTML deliberately keeps the pre-render page representation. ~keep
            observe_literal_page(&page, index, &crate::rendering::render_plain(doc), observer);
            return page;
        }
    };
    if !rendered.is_empty() {
        page.content = rendered;
    }
    page
}

fn observe_literal_page(
    page: &PageContent,
    index: usize,
    plain: &str,
    observer: &mut impl super::surfaces::DerivationObserver,
) {
    let body = if page.content == plain {
        MarkupPageBody::Literal
    } else {
        MarkupPageBody::Unverified
    };
    observer.page_markup(index, &page.content, body);
}

/// Extract `OcrElement` entries from OCR-typed internal elements.
///
/// An element without geometry is kept with a zero bounding box rather than
/// discarded (#75): backends that report text without word boxes (VLM OCR, hOCR
/// without `bbox` properties) would otherwise lose their recognised text entirely.
pub(crate) fn build_ocr_elements(doc: &InternalDocument) -> Option<Vec<OcrElement>> {
    let ocr_elems: Vec<OcrElement> = doc
        .elements
        .iter()
        .filter_map(|elem| {
            if let ElementKind::OcrText { level } = elem.kind {
                let geometry = elem.ocr_geometry.clone().unwrap_or_default();
                let confidence = elem.ocr_confidence.clone().unwrap_or(OcrConfidence {
                    detection: None,
                    recognition: 0.0,
                });
                Some(OcrElement {
                    text: elem.text.clone(),
                    geometry,
                    confidence,
                    level,
                    rotation: elem.ocr_rotation.clone(),
                    page_number: elem.page.unwrap_or(1),
                    parent_id: None,
                    backend_metadata: std::collections::HashMap::new(),
                })
            } else {
                None
            }
        })
        .collect();

    if ocr_elems.is_empty() { None } else { Some(ocr_elems) }
}

#[cfg(test)]
mod plain_page_rerender_tests {
    use super::*;
    use crate::core::config::OutputFormat;
    use crate::types::{PageBoundary, PageStructure, PageUnitType};

    fn page(number: u32, content: &str) -> PageContent {
        PageContent {
            page_number: number,
            content: content.to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        }
    }

    fn doc_with(pages: Vec<PageContent>, tagged: &[(u32, &str)]) -> InternalDocument {
        let mut doc = InternalDocument::new("pdf");
        for (number, text) in tagged {
            doc.push_element(InternalElement::text(ElementKind::Paragraph, *text, 0).with_page(*number));
        }
        doc.metadata.pages = Some(PageStructure {
            total_count: pages.len() as u32,
            unit_type: PageUnitType::Page,
            boundaries: Some(
                pages
                    .iter()
                    .map(|p| PageBoundary {
                        byte_start: 0,
                        byte_end: 999,
                        page_number: p.page_number,
                    })
                    .collect(),
            ),
            pages: None,
        });
        doc.prebuilt_pages = Some(pages);
        doc
    }

    fn derive(doc: &mut InternalDocument, content: &str) -> Vec<PageContent> {
        derive_pages(doc, content, &OutputFormat::Plain, true, &mut ()).expect("pages")
    }

    fn warned(doc: &InternalDocument) -> bool {
        doc.processing_warnings.iter().any(|w| w.source == "page_boundaries")
    }

    fn assert_ranges(doc: &InternalDocument, content: &str, pages: &[PageContent]) {
        let boundaries = doc
            .metadata
            .pages
            .as_ref()
            .and_then(|s| s.boundaries.as_ref())
            .expect("boundaries kept");
        for (page, b) in pages.iter().zip(boundaries) {
            assert_eq!(content.get(b.byte_start..b.byte_end), Some(page.content.as_str()));
        }
    }

    #[test]
    fn markdown_table_page_text_is_replaced_by_the_plain_rendering_of_its_elements() {
        let content = "Intro\n\na b\n1 2";
        let mut doc = doc_with(
            vec![page(1, "Intro\n\n| a | b |\n| --- | --- |\n| 1 | 2 |")],
            &[(1, "Intro")],
        );
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "a b\n1 2", 0).with_page(1));
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, content);
        assert!(!pages[0].content.contains("---"));
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn only_the_pages_that_are_not_found_in_the_content_are_rerendered() {
        let content = "kept page one\n\nplain two\n\nkept page three";
        let mut doc = doc_with(
            vec![
                page(1, "kept page one"),
                page(2, "| plain | two |\n| --- | --- |"),
                page(3, "kept page three"),
            ],
            &[(1, "kept page one"), (2, "plain two"), (3, "kept page three")],
        );
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "kept page one");
        assert_eq!(pages[1].content, "plain two");
        assert_eq!(pages[2].content, "kept page three");
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn an_untagged_page_found_in_the_content_stays_between_rerendered_pages() {
        let content = "one\n\nuntagged text\n\nthree";
        let mut doc = doc_with(
            vec![page(1, "| one |"), page(2, "untagged text"), page(3, "| three |")],
            &[(1, "one"), (3, "three")],
        );
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "one");
        assert_eq!(pages[1].content, "untagged text");
        assert_eq!(pages[2].content, "three");
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn an_untagged_page_that_is_not_found_still_drops_the_boundaries_with_a_warning() {
        let content = "one\n\ntwo";
        let mut doc = doc_with(vec![page(1, "| one |"), page(2, "| two |")], &[(1, "one")]);
        let pages = derive(&mut doc, content);
        assert_eq!(pages[1].content, "| two |");
        assert!(warned(&doc));
        assert!(doc.metadata.pages.as_ref().unwrap().boundaries.is_none());
    }
}
