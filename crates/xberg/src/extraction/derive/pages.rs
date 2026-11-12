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
    let raw_pages = match doc.prebuilt_pages.take() {
        Some(prebuilt) if content_is_plain => Some(with_plain_page_text(doc, prebuilt)),
        Some(prebuilt) => Some(prebuilt),
        None => build_pages(doc, content_is_plain),
    };
    let mut pages = apply_page_content_format(raw_pages, doc, output_format, observer);
    let mut reconciled = !content_is_plain || reconcile_page_boundaries(&mut doc.metadata, content, pages.as_deref());
    // ~keep Prebuilt page text (OCR) renders tables and block separators its own way, so it
    // cannot be found in the plain content. Render the pages that are not found from their
    // elements as `render_plain` does and locate that instead (#2825).
    if !reconciled
        && let Some((rerendered, diverging)) = rerender_pages_plain(pages.as_deref(), doc, content)
        && reconcile_located_page_boundaries(&mut doc.metadata, content, &rerendered, &diverging)
    {
        pages = Some(rerendered);
        reconciled = true;
        if !diverging.is_empty() {
            crate::core::diagnostics::push_warning(
                &mut doc.processing_warnings,
                "page_boundaries",
                format!(
                    "Page byte ranges of pages {diverging:?} were measured on a different rendering of the text \
                     and could not be mapped onto the returned content, so they were omitted"
                ),
            );
        }
    }
    if !reconciled && let Some(omitted) = place_page_boundaries(doc, content, pages.as_deref()) {
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "page_boundaries",
            format!(
                "The page text of pages {omitted:?} is not found in the returned content, so their page byte \
                 ranges were omitted"
            ),
        );
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

/// Give each prebuilt page that is not OCR text (see `is_ocr_page_text`), and that has elements,
/// its part of the plain content, as `render_plain_pages` writes it. A DOCX page keeps its notes:
/// the definitions are part of the last page, as in the text of the parser. Every other field of
/// the page stays.
fn with_plain_page_text(doc: &InternalDocument, mut pages: Vec<PageContent>) -> Vec<PageContent> {
    if pages.iter().all(|page| is_ocr_page_text(doc, page)) {
        return pages;
    }
    let mut texts = crate::rendering::render_plain_pages(doc, &compute_page_membership(doc));
    for page in &mut pages {
        if !is_ocr_page_text(doc, page)
            && let Some(text) = texts.remove(&page.page_number)
        {
            page.content = text;
        }
    }
    pages
}

/// Point the range of each page that is not OCR text (see `is_ocr_page_text`) at that page's text
/// in `content`, and omit the range of a page whose text is not there. A range that already selects
/// its page text stays. Any other range moves to the first place of its page text after the ranges
/// before it, in the order of the ranges. A PDF or OCR range keeps its measured place (#2128).
/// The range of a page number that has no extracted page is omitted. Returns the omitted page
/// numbers, sorted and without repeats, or `None` when nothing was omitted. When no range is
/// left, the boundaries become `None`.
fn place_page_boundaries(doc: &mut InternalDocument, content: &str, pages: Option<&[PageContent]>) -> Option<Vec<u32>> {
    // `None` marks an OCR page, whose range keeps its measured place.
    let text_of: ahash::AHashMap<u32, Option<&str>> = pages?
        .iter()
        .map(|page| {
            let text = (!is_ocr_page_text(doc, page)).then_some(page.content.as_str());
            (page.page_number, text)
        })
        .collect();
    let structure = doc.metadata.pages.as_mut()?;
    let boundaries = structure.boundaries.as_mut()?;
    let mut omitted = Vec::new();
    let mut search_offset = 0;
    boundaries.retain_mut(|boundary| {
        let Some(&page_text) = text_of.get(&boundary.page_number) else {
            omitted.push(boundary.page_number);
            return false;
        };
        let Some(text) = page_text else {
            return true;
        };
        let start = if content.get(boundary.byte_start..boundary.byte_end) == Some(text) {
            Some(boundary.byte_start)
        } else {
            content
                .get(search_offset..)
                .and_then(|rest| rest.find(text))
                .map(|index| search_offset + index)
        };
        let Some(start) = start else {
            omitted.push(boundary.page_number);
            return false;
        };
        boundary.byte_start = start;
        boundary.byte_end = start + text.len();
        search_offset = search_offset.max(boundary.byte_end);
        true
    });
    if omitted.is_empty() {
        return None;
    }
    if boundaries.is_empty() {
        structure.boundaries = None;
    }
    omitted.sort_unstable();
    omitted.dedup();
    Some(omitted)
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
/// a page has the text that `render_plain` writes for the page's elements in the whole
/// document, so `pages[n].content` is found in `result.content`. For every other output a page
/// has the text of its elements only. ~keep
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
    let mut plain_texts =
        content_is_plain.then(|| crate::rendering::render_plain_pages(doc, &compute_page_membership(doc)));

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
                match elem.kind {
                    ElementKind::Table { table_index } => {
                        if let Some(arc_table) = arc_tables.get(table_index as usize) {
                            tables.push(Arc::clone(arc_table));
                        }
                    }
                    ElementKind::Image { image_index } if (image_index as usize) < doc.images.len() => {
                        image_indices.push(image_index);
                    }
                    _ => {}
                }
                if plain_texts.is_none() && !elem.text.is_empty() {
                    if !content.is_empty() {
                        content.push_str("\n\n");
                    }
                    content.push_str(&elem.text);
                }
            }
            if let Some(texts) = plain_texts.as_mut() {
                content = texts.remove(&page_num).unwrap_or_default();
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

#[cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
pub(crate) fn render_plain_selected_pages(
    doc: &InternalDocument,
    requested: &std::collections::BTreeSet<u32>,
) -> std::collections::BTreeMap<u32, String> {
    let membership = compute_page_membership(doc);
    let mut observer = ();
    build_pages(doc, true)
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter(|(_, page)| requested.contains(&page.page_number))
        .filter_map(|(index, page)| {
            let indices = membership.get(&page.page_number)?;
            let rendered = render_sub_page(
                page,
                indices,
                doc,
                Some(crate::rendering::render_plain),
                (index, &crate::core::config::OutputFormat::Plain),
                &mut observer,
            );
            Some((rendered.page_number, rendered.content))
        })
        .collect()
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

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(crate) fn render_plain_page_elements(doc: &InternalDocument, page: u32) -> Option<String> {
    let membership = compute_page_membership(doc);
    membership.get(&page).map(|indices| render_plain_elements(indices, doc))
}

fn render_plain_elements(indices: &[usize], doc: &InternalDocument) -> String {
    crate::rendering::render_plain(&build_sub_doc(indices, doc))
}

/// Whether every page of a document of `source_format` is OCR text whether or not the page took
/// OCR text: on a `pdf`, `image` or `ocr` source a native page's text layer differs from the
/// plain content by spacing alone, so the OCR text page list is never read there (#3014). The
/// extractor's own documents are always `pdf`, so it attaches the list only to an adopted
/// document of another source format, and both sides share this one list of source formats.
/// Whether every page of a document of `source_format` is OCR text whether or not the page took
/// OCR text: on a `pdf`, `image` or `ocr` source a native page's text layer differs from the
/// plain content by spacing alone, so the OCR text page list is never read there (#3014). The
/// extractor's own documents are always `pdf`, so it attaches the list only to an adopted
/// document of another source format, and both sides share this one list of source formats.
pub(crate) fn source_short_circuits_ocr_page_text(source_format: &str) -> bool {
    matches!(source_format, "pdf" | "image" | "ocr")
}

/// Whether `page`'s prebuilt text may be replaced by the plain rendering of its elements: every
/// page of a PDF, image or OCR source (a native page's text layer differs from the plain content
/// by spacing alone), and a page of another format that took OCR text. A DOCX page is not: in
/// plain output it already has its part of the plain content (`with_plain_page_text`).
fn is_ocr_page_text(doc: &InternalDocument, page: &PageContent) -> bool {
    source_short_circuits_ocr_page_text(&doc.source_format) || doc.ocr_text_pages.contains(&page.page_number)
}

/// OCR pages whose prebuilt text is not found in `content` (in page order) rendered from their
/// own elements by `render_plain`; a page whose text is found keeps it. Returns the pages and
/// the numbers of those that still diverge from `content`: a page that is not OCR text (see
/// `is_ocr_page_text`) keeps its text, and so does an OCR page whose re-rendering is empty or
/// not found. `None` when no page has elements.
///
/// Pages are located with the leftmost `find` after the previous page, so short or repeated
/// page text can match early, as in `reconcile_page_boundaries`.
fn rerender_pages_plain(
    pages: Option<&[PageContent]>,
    doc: &InternalDocument,
    content: &str,
) -> Option<(Vec<PageContent>, Vec<u32>)> {
    let pages = pages?;
    let elements_by_page = compute_page_membership(doc);
    if elements_by_page.is_empty() {
        return None;
    }
    let mut search_offset = 0;
    let mut out = Vec::with_capacity(pages.len());
    let mut diverging = Vec::new();
    for page in pages {
        let mut page = page.clone();
        let found = |text: &str, from: usize| content.get(from..).and_then(|rest| rest.find(text)).map(|i| from + i);
        if !page.content.is_empty() {
            if let Some(start) = found(&page.content, search_offset) {
                search_offset = start + page.content.len();
            } else {
                let rendered = is_ocr_page_text(doc, &page)
                    .then(|| elements_by_page.get(&page.page_number))
                    .flatten()
                    .map(|indices| render_plain_elements(indices, doc))
                    .filter(|rendered| !rendered.is_empty());
                match rendered.and_then(|rendered| found(&rendered, search_offset).map(|start| (rendered, start))) {
                    Some((rendered, start)) => {
                        tracing::debug!(
                            page = page.page_number,
                            prebuilt_len = page.content.len(),
                            rendered_len = rendered.len(),
                            "page text replaced by the plain rendering of its elements"
                        );
                        search_offset = start + rendered.len();
                        page.content = rendered;
                    }
                    None => diverging.push(page.page_number),
                }
            }
        }
        out.push(page);
    }
    Some((out, diverging))
}

/// Re-point the boundaries of the pages not in `diverging` at `content`, omitting the boundaries
/// of the diverging pages. Leaves the boundaries as they were and returns `false` when the
/// remaining pages cannot be located.
fn reconcile_located_page_boundaries(
    metadata: &mut crate::types::Metadata,
    content: &str,
    pages: &[PageContent],
    diverging: &[u32],
) -> bool {
    let Some(structure) = metadata.pages.as_mut() else {
        return false;
    };
    let Some(original) = structure.boundaries.take() else {
        return false;
    };
    structure.boundaries = Some(
        original
            .iter()
            .filter(|boundary| !diverging.contains(&boundary.page_number))
            .cloned()
            .collect(),
    );
    let located: Vec<PageContent> = pages
        .iter()
        .filter(|page| !diverging.contains(&page.page_number))
        .cloned()
        .collect();
    let reconciled = reconcile_page_boundaries(metadata, content, Some(&located));
    if !reconciled && let Some(structure) = metadata.pages.as_mut() {
        structure.boundaries = Some(original);
    }
    reconciled
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
        doc.ocr_text_pages = pages.iter().map(|p| p.page_number).collect();
        doc.prebuilt_pages = Some(pages);
        doc
    }

    fn boundary_pages(doc: &InternalDocument) -> Vec<u32> {
        doc.metadata
            .pages
            .as_ref()
            .and_then(|s| s.boundaries.as_ref())
            .map(|b| b.iter().map(|b| b.page_number).collect())
            .unwrap_or_default()
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
        assert_eq!(boundaries.len(), pages.len());
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
    fn an_untagged_page_that_is_not_found_loses_only_its_own_boundary_with_a_warning() {
        let content = "one\n\ntwo";
        let mut doc = doc_with(vec![page(1, "| one |"), page(2, "| two |")], &[(1, "one")]);
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "one");
        assert_eq!(pages[1].content, "| two |");
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1]);
    }

    #[test]
    fn a_page_found_in_the_content_keeps_its_text_even_when_its_elements_render_differently() {
        let content = "kept page one\n\nx";
        let mut doc = doc_with(
            vec![page(1, "kept page one"), page(2, "| x |")],
            &[(1, "other"), (2, "x")],
        );
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "kept page one");
        assert_eq!(pages[1].content, "x");
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn an_empty_rerender_of_a_non_empty_page_loses_only_its_own_boundary() {
        let content = "two";
        let mut doc = doc_with(vec![page(1, "| a |"), page(2, "two")], &[(1, ""), (2, "two")]);
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "| a |");
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![2]);
    }

    #[test]
    fn a_docx_page_with_its_own_table_markdown_gets_its_part_of_the_plain_content() {
        let content = "Body\n\nplain two";
        let native_text = "| a | b |\n| --- | --- |\n| 1 | 2 |[^1]\n\n1: THE NOTE";
        let mut native = page(1, native_text);
        native.tables.push(Arc::new(Table {
            markdown: "| a | b |\n| --- | --- |\n| 1 | 2 |".to_string(),
            ..Default::default()
        }));
        let mut doc = doc_with(vec![native], &[(1, "Body")]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "Body");
        assert_eq!(pages[0].tables.len(), 1, "the page keeps its tables");
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn a_docx_page_gets_its_plain_text_while_the_ocr_pages_of_a_mixed_document_are_rerendered() {
        let content = "one\n\ntwo\n\nthree";
        let mut doc = doc_with(
            vec![page(1, "| one |"), page(2, "native | two"), page(3, "| three |")],
            &[(1, "one"), (2, "two"), (3, "three")],
        );
        doc.ocr_text_pages = vec![1, 3];
        doc.source_format = "docx".to_string();
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "one");
        assert_eq!(pages[1].content, "two");
        assert_eq!(pages[2].content, "three");
        assert!(!warned(&doc));
        assert_ranges(&doc, content, &pages);
    }

    fn set_boundaries(doc: &mut InternalDocument, ranges: &[(u32, usize, usize)]) {
        let structure = doc.metadata.pages.as_mut().unwrap();
        structure.boundaries = Some(
            ranges
                .iter()
                .map(|&(page_number, byte_start, byte_end)| PageBoundary {
                    byte_start,
                    byte_end,
                    page_number,
                })
                .collect(),
        );
    }

    #[test]
    fn a_docx_page_whose_range_selects_other_content_text_loses_the_range_with_a_warning() {
        let content = "Memo (style_name: heading 1)\n\nBody";
        let mut doc = doc_with(vec![page(1, "Memo\n\nBody")], &[(1, "Memo"), (1, "Body")]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 0, content.len())]);
        let pages = derive(&mut doc, content);
        assert_eq!(pages[0].content, "Memo\n\nBody");
        assert!(warned(&doc));
        assert!(doc.metadata.pages.as_ref().unwrap().boundaries.is_none());
    }

    #[test]
    fn a_range_that_selects_its_page_stays_when_another_page_has_no_elements_to_rerender() {
        let content = "one\n\ntwo (note)";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "two\nnote")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 5, content.len())]);
        let pages = derive(&mut doc, content);
        assert_eq!(pages[1].content, "two\nnote");
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1]);
    }

    #[test]
    fn exact_ranges_of_pages_that_are_not_located_in_order_stay_without_a_warning() {
        let content = "two\n\none";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "two")], &[]);
        set_boundaries(&mut doc, &[(1, 5, 8), (2, 0, 3)]);
        derive(&mut doc, content);
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 2]);
    }

    /// A DOCX document with a note reference on each of its two pages, and page ranges measured on
    /// the text of the parser.
    fn docx_with_a_note_on_each_page() -> InternalDocument {
        let mut builder = crate::types::internal_builder::InternalDocumentBuilder::new("docx");
        builder.push_paragraph("First page", Vec::new(), Some(1), None);
        builder.push_footnote_ref("1", "fn1", Some(1));
        builder.push_paragraph("Second page", Vec::new(), Some(2), None);
        builder.push_footnote_ref("2", "fn2", Some(2));
        builder.push_footnote_definition("Note one", "fn1", None);
        builder.push_footnote_definition("Note two", "fn2", None);
        let mut doc = builder.build();
        doc.metadata.pages = Some(PageStructure {
            total_count: 2,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: None,
        });
        set_boundaries(&mut doc, &[(1, 0, 13), (2, 13, 26)]);
        doc
    }

    #[test]
    fn a_plain_page_has_the_plain_text_of_its_elements_with_the_note_numbers_of_the_document() {
        let mut doc = docx_with_a_note_on_each_page();
        let content = crate::rendering::render_plain(&doc);
        let pages = derive(&mut doc, &content);
        let texts: Vec<&str> = pages.iter().map(|page| page.content.as_str()).collect();
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], "First page[^1]");
        let (body, notes) = texts[1]
            .split_once("\n\n")
            .expect("the last page has the note definitions");
        assert_eq!(body, "Second page[^2]");
        assert!(notes.contains("Note one") && notes.contains("Note two"), "{notes}");
        assert_eq!(
            content.find(texts[1]).map(|start| start + texts[1].len()),
            Some(content.len())
        );
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    #[test]
    fn a_prebuilt_docx_page_has_its_part_of_the_plain_content_and_keeps_its_notes() {
        let mut doc = docx_with_a_note_on_each_page();
        doc.prebuilt_pages = Some(vec![
            page(1, "First page"),
            page(2, "Second page\n\nNote one\nNote two"),
        ]);
        let content = crate::rendering::render_plain(&doc);
        let pages = derive(&mut doc, &content);
        assert_eq!(pages[0].content, "First page[^1]");
        assert!(pages[1].content.starts_with("Second page[^2]"));
        assert!(pages[1].content.contains("Note one") && pages[1].content.contains("Note two"));
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    #[test]
    fn a_prebuilt_pdf_page_keeps_its_text() {
        let mut doc = docx_with_a_note_on_each_page();
        doc.source_format = "pdf".to_string();
        doc.prebuilt_pages = Some(vec![page(1, "First page"), page(2, "Second page")]);
        let content = crate::rendering::render_plain(&doc);
        let pages = derive(&mut doc, &content);
        assert_eq!(pages[0].content, "First page");
    }

    #[test]
    fn a_page_range_that_does_not_select_its_page_text_moves_to_the_page_text() {
        let content = "alpha (note)\n\nbeta";
        let mut doc = doc_with(vec![page(1, "alpha\nnote"), page(2, "beta")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 0, 12), (2, 0, 4)]);
        derive(&mut doc, content);
        assert!(warned(&doc));
        let boundaries = doc.metadata.pages.as_ref().and_then(|s| s.boundaries.clone());
        let kept: Vec<(u32, Option<&str>)> = boundaries
            .iter()
            .flatten()
            .map(|b| (b.page_number, content.get(b.byte_start..b.byte_end)))
            .collect();
        assert_eq!(kept, [(2, Some("beta"))]);
    }

    #[test]
    fn a_docx_range_that_selects_its_page_text_stays_where_it_is() {
        let content = "two\n\none";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "two"), page(3, "gone")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 5, 8), (2, 0, 3), (3, 0, 1)]);
        derive(&mut doc, content);
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 2]);
    }

    #[test]
    fn a_moved_page_range_starts_after_the_range_of_the_page_before_it() {
        let content = "beta\n\nalpha\n\nbeta";
        let mut doc = doc_with(vec![page(1, "alpha"), page(2, "beta"), page(3, "gamma")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "pptx".to_string();
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 0, 3), (3, 0, 3)]);
        derive(&mut doc, content);
        let boundaries = doc.metadata.pages.as_ref().and_then(|s| s.boundaries.clone());
        let ranges: Vec<(u32, usize, usize)> = boundaries
            .iter()
            .flatten()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect();
        assert_eq!(ranges, [(1, 6, 11), (2, 13, 17)]);
        assert!(warned(&doc));
    }

    #[test]
    fn the_range_of_a_page_number_with_no_extracted_page_is_omitted_with_a_warning() {
        let content = "one\n\ntwo";
        let mut doc = doc_with(vec![page(1, "one")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 5, 8)]);
        derive(&mut doc, content);
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert!(warned(&doc));
    }

    #[test]
    fn the_warning_names_a_page_with_two_omitted_ranges_one_time() {
        let content = "one";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "gone")], &[]);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 0, 1), (2, 1, 2)]);
        derive(&mut doc, content);
        assert_eq!(boundary_pages(&doc), vec![1]);
        let messages: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|w| w.source == "page_boundaries")
            .map(|w| w.message.as_ref())
            .collect();
        assert_eq!(messages.len(), 1, "one page_boundaries warning: {messages:?}");
        assert!(
            messages[0].contains("pages [2] "),
            "page 2 is named one time: {messages:?}"
        );
    }
}
