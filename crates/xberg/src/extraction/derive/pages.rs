//! Per-page `PageContent` / `OcrElement` derivation, split out of `derive` by area.

use std::sync::Arc;

use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrConfidence, OcrElement};
use crate::types::page::PageContent;
use crate::types::tables::Table;

use super::surfaces::MarkupPageBody;

/// The place of each page's text in the plain content, keyed by page number: a page with
/// elements has its byte range, or `None` when its elements write nothing.
type PlainPageRanges = std::collections::BTreeMap<u32, Option<(usize, usize)>>;

/// `spans` say where each element is in `content`, given when `content` is the plain rendering
/// of the elements. They are used only when the caller gets that content (`content_is_plain`).
pub(super) fn derive_pages(
    doc: &mut InternalDocument,
    content: &str,
    output_format: &crate::core::config::OutputFormat,
    content_is_plain: bool,
    spans: Option<&crate::rendering::ElementSpans>,
    observer: &mut impl super::surfaces::DerivationObserver,
) -> Option<Vec<PageContent>> {
    // ~keep Renderer spans own native page text and ranges. Adopted OCR documents can
    // declare any source format, so their transcripts follow the OCR page marks below.
    let adopted_ocr = doc
        .metadata
        .additional
        .remove(super::ADOPTED_OCR_DOCUMENT_KEY)
        .is_some_and(|value| value.as_bool() == Some(true));
    let native_ranges = spans
        .filter(|_| content_is_plain && !adopted_ocr && !source_short_circuits_ocr_page_text(&doc.source_format))
        .map(|spans| plain_page_ranges(doc, content, spans))
        .filter(|ranges| ranges.values().any(Option::is_some));
    let native_ranges = native_ranges.as_ref();
    let raw_pages = doc.prebuilt_pages.take().or_else(|| build_pages(doc, content_is_plain));
    let raw_pages = match native_ranges {
        Some(ranges) => raw_pages.map(|pages| with_plain_page_text(pages, content, ranges)),
        None => raw_pages,
    };
    let mut pages = apply_page_content_format(raw_pages, doc, output_format, observer);
    if let Some(ranges) = native_ranges {
        if let Some(omitted) = set_plain_page_boundaries(&mut doc.metadata, pages.as_deref(), ranges) {
            crate::core::diagnostics::push_warning(
                &mut doc.processing_warnings,
                "page_boundaries",
                format!(
                    "Pages {omitted:?} have no text of their own in the returned content, so their page byte \
                     ranges were omitted"
                ),
            );
        }
        return pages;
    }
    let had_boundaries = has_page_boundaries(&doc.metadata);
    let mut reconciled = !content_is_plain || reconcile_page_boundaries(&mut doc.metadata, content, pages.as_deref());
    // ~keep Prebuilt page text (OCR) renders tables and block separators its own way, so it
    // cannot be found in the plain content. Render the pages that are not found from their
    // elements as `render_plain` does and locate that instead (#2825).
    if !reconciled
        && has_page_boundaries(&doc.metadata)
        && let Some((rerendered, unlocated)) = rerender_pages_plain(pages.as_deref(), doc, content)
        && let Some(omitted) = reconcile_located_page_boundaries(&mut doc.metadata, content, &rerendered, &unlocated)
    {
        pages = Some(rerendered);
        reconciled = true;
        if !omitted.is_empty() {
            crate::core::diagnostics::push_warning(
                &mut doc.processing_warnings,
                "page_boundaries",
                format!(
                    "Page byte ranges of pages {omitted:?} were measured on a different rendering of the text \
                     and could not be mapped onto the returned content, so they were omitted"
                ),
            );
        }
    }
    if !reconciled
        && ((had_boundaries && !has_page_boundaries(&doc.metadata))
            || drop_unusable_page_boundaries(&mut doc.metadata, content))
    {
        crate::core::diagnostics::push_warning(
            &mut doc.processing_warnings,
            "page_boundaries",
            "Page byte ranges were measured on a different rendering of the text and could not be \
             mapped onto the returned content, so they were omitted",
        );
    }
    pages
}

/// Where the text of each page is in the plain `content`: from the first byte that the page's
/// elements write to the last, without the whitespace around it. The page whose elements end
/// the body also has the note and comment definitions and the annotations that follow the body.
fn plain_page_ranges(doc: &InternalDocument, content: &str, spans: &crate::rendering::ElementSpans) -> PlainPageRanges {
    let body_text_end = content[..spans.body_end].trim_end().len();
    let mut ranges: PlainPageRanges = compute_page_membership(doc)
        .into_iter()
        .map(|(page_number, indices)| {
            let page_spans = || {
                indices
                    .iter()
                    .filter_map(|&i| spans.by_element.get(i).copied().flatten())
            };
            let range = page_spans()
                .map(|(start, _)| start)
                .min()
                .zip(page_spans().map(|(_, end)| end).max())
                .and_then(|(start, end)| {
                    if start >= body_text_end {
                        return None;
                    }
                    let end = if end >= body_text_end { content.len() } else { end };
                    let text = content.get(start..end)?;
                    let text_start = start + (text.len() - text.trim_start().len());
                    let text_end = start + text.trim_end().len();
                    (text_start < text_end).then_some((text_start, text_end))
                });
            (page_number, range)
        })
        .collect();
    if doc.source_format == "docx" {
        for element in &doc.elements {
            if let (ElementKind::PageBreak, Some(page)) = (&element.kind, element.page) {
                // ~keep A final explicit break opens a page even if the parser only puts note definitions there.
                ranges.entry(page + 1).or_insert(None);
            }
        }
    }
    ranges
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
pub(crate) fn project_final_ocr_occurrence(
    doc: &InternalDocument,
    (text, spans): (&str, &crate::rendering::ElementSpans),
    page: &PageContent,
    owner: &crate::types::internal::InternalElementId,
    local: std::ops::Range<usize>,
) -> Option<[std::ops::Range<usize>; 2]> {
    let membership = compute_page_membership(doc);
    let mut owners = membership
        .get(&page.page_number)?
        .iter()
        .copied()
        .filter(|&index| doc.elements[index].id == *owner);
    let index = owners.next()?;
    if owners.next().is_some() || local.is_empty() {
        return None;
    }
    let (start, end) = spans.by_element.get(index).copied().flatten()?;
    let body_end = end.min(spans.body_end);
    let element = &doc.elements[index].text;
    if text.get(start..body_end)?.trim_end() != element {
        return None;
    }
    let document_range = start.checked_add(local.start)?..start.checked_add(local.end)?;
    let token = element.get(local)?;
    let ranges = plain_page_ranges(doc, text, spans);
    let (page_start, page_end) = ranges.get(&page.page_number).copied().flatten()?;
    if text.get(page_start..page_end)? != page.content
        || document_range.start < page_start
        || document_range.end > page_end
    {
        return None;
    }
    let page_range = document_range.start - page_start..document_range.end - page_start;
    (text.get(document_range.clone())? == token && page.content.get(page_range.clone())? == token)
        .then_some([document_range, page_range])
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
pub(super) fn adopt_canonical_ocr_headings(doc: &mut InternalDocument) {
    tracing::trace!(
        source_format = %doc.source_format,
        preformatted = doc.pre_rendered_content.is_some(),
        prebuilt_ocr = doc.prebuilt_ocr_elements.as_ref().map_or(0, Vec::len),
        pending = doc.elements.iter().flat_map(|element| &element.ocr_source_spans)
            .filter(|span| span.replacement.is_some()).count(),
        "canonical OCR heading admission"
    );
    if doc.source_format != "pdf" || doc.pre_rendered_content.is_some() || doc.prebuilt_ocr_elements.is_some() {
        return;
    }
    for index in 0..doc.elements.len() {
        let pending = doc.elements[index]
            .ocr_source_spans
            .iter()
            .filter(|span| span.replacement.is_some())
            .count();
        if pending > 0 {
            tracing::trace!(pending, spans = doc.elements[index].ocr_source_spans.len(),
                kind = ?doc.elements[index].kind, anchor = doc.elements[index].anchor.is_some(),
                inline_annotations = doc.elements[index].annotations.len(), "canonical OCR heading owner");
        }
        let Some(replacement) = canonical_heading_text(doc, index) else {
            continue;
        };
        let element = &mut doc.elements[index];
        element.text = replacement;
        element.ocr_source_spans.clear();
    }
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn canonical_heading_text(doc: &InternalDocument, index: usize) -> Option<String> {
    let element = &doc.elements[index];
    if !matches!(element.kind, ElementKind::Paragraph | ElementKind::Heading { .. })
        || !element.annotations.is_empty()
        || element.anchor.is_some()
        || element.page.is_none()
    {
        return None;
    }
    let pending = element.ocr_source_spans.as_slice();
    if let [span] = pending {
        let (expected, replacement, 1) = span.replacement.as_ref()? else {
            return None;
        };
        let end = span.destination.checked_add(span.source.len())?;
        let page_spans = || {
            doc.elements
                .iter()
                .filter(|other| other.page == element.page)
                .flat_map(|other| &other.ocr_source_spans)
        };
        if expected.is_empty()
            || !expected.bytes().all(|byte| byte.is_ascii_punctuation())
            || replacement.split_whitespace().count() < 2
            || element.text.get(span.destination..end)? != expected
            || page_spans().filter(|other| other.replacement.is_some()).count() != 1
            || page_spans()
                .filter(|other| other.occurrence == span.occurrence && other.owner == span.owner)
                .count()
                != 1
        {
            return None;
        }
        let mut text = element.text.clone();
        text.replace_range(span.destination..end, replacement);
        return Some(text);
    }
    let [first, last] = pending else {
        return None;
    };
    let (first_expected, first_new, 2) = first.replacement.as_ref()? else {
        return None;
    };
    let (last_expected, last_new, 2) = last.replacement.as_ref()? else {
        return None;
    };
    if first.owner != last.owner
        || first.destination.checked_add(first_expected.len())?.checked_add(1)? != last.destination
        || element.text.get(first.destination..last.destination - 1)? != first_expected
        || element.text.get(last.destination - 1..last.destination)? != " "
        || element
            .text
            .get(last.destination..last.destination.checked_add(last_expected.len())?)?
            != last_expected
        || doc
            .elements
            .iter()
            .filter(|other| other.page == element.page)
            .flat_map(|other| &other.ocr_source_spans)
            .filter(|span| span.replacement.is_some())
            .count()
            != 2
        || [first, last].into_iter().any(|target| {
            doc.elements
                .iter()
                .filter(|other| other.page == element.page)
                .flat_map(|other| &other.ocr_source_spans)
                .filter(|span| span.occurrence == target.occurrence && span.owner == target.owner)
                .count()
                != 1
        })
    {
        return None;
    }
    let mut text = element.text.clone();
    text.replace_range(
        first.destination..last.destination + last_expected.len(),
        &format!("{first_new} {last_new}"),
    );
    Some(text)
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
type FinalOcrEdit<'a> = (std::ops::Range<usize>, std::ops::Range<usize>, usize, &'a str);

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn plan_page_ocr_replacement<'a>(
    doc: &'a InternalDocument,
    text: &str,
    spans: &crate::rendering::ElementSpans,
    page: &PageContent,
) -> Option<Vec<FinalOcrEdit<'a>>> {
    let pending: Vec<_> = doc
        .elements
        .iter()
        .filter(|element| element.page == Some(page.page_number))
        .flat_map(|element| {
            element
                .ocr_source_spans
                .iter()
                .filter(|span| span.replacement.is_some())
                .map(move |span| (element, span))
        })
        .collect();
    if !(1..=64).contains(&pending.len()) {
        return None;
    }
    let mut edits = Vec::new();
    for (element, span) in &pending {
        let (expected, replacement, count) = span.replacement.as_ref()?;
        if *count != pending.len()
            || doc
                .elements
                .iter()
                .filter(|other| other.page == element.page)
                .flat_map(|other| &other.ocr_source_spans)
                .filter(|other| other.occurrence == span.occurrence && other.owner == span.owner)
                .count()
                != 1
        {
            return None;
        }
        let end = span.destination.checked_add(span.source.len())?;
        let [document_bytes, page_bytes] =
            project_final_ocr_occurrence(doc, (text, spans), page, &element.id, span.destination..end)?;
        if text.get(document_bytes.clone())? != expected || page.content.get(page_bytes.clone())? != expected {
            return None;
        }
        edits.push((
            &span.owner,
            (document_bytes, page_bytes, expected.len(), replacement.as_str()),
        ));
    }
    edits.sort_by_key(|(_, (bytes, _, _, _))| bytes.start);
    if edits
        .windows(2)
        .any(|pair| pair[0].1.0.end > pair[1].1.0.start || pair[0].1.1.end > pair[1].1.1.start)
    {
        return None;
    }
    if edits.len() <= 2 {
        return Some(edits.into_iter().map(|(_, edit)| edit).collect());
    }
    let mut groups = Vec::new();
    let mut seen = Vec::new();
    for group in edits.chunk_by(|a, b| a.0 == b.0) {
        let owner = group.first()?.0;
        if seen.contains(&owner) {
            return None;
        }
        seen.push(owner);
        groups.push(join_final_line(group, text, &page.content)?);
    }
    tracing::trace!(
        page = page.page_number,
        groups = groups.len(),
        owners = edits.len(),
        "full line OCR projected final owners"
    );
    Some(groups)
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn join_final_line<'a>(
    group: &[(&crate::types::internal::InternalElementId, FinalOcrEdit<'a>)],
    document: &str,
    page: &str,
) -> Option<FinalOcrEdit<'a>> {
    let (_, (first_document, first_page, _, replacement)) = group.first()?;
    let (_, (last_document, last_page, ..)) = group.last()?;
    if group.len() < 3
        || replacement.is_empty()
        || group[1..].iter().any(|(_, edit)| !edit.3.is_empty())
        || group.windows(2).any(|pair| {
            document.get(pair[0].1.0.end..pair[1].1.0.start) != Some(" ")
                || page.get(pair[0].1.1.end..pair[1].1.1.start) != Some(" ")
        })
    {
        return None;
    }
    let document = first_document.start..last_document.end;
    let page = first_page.start..last_page.end;
    let length = document.len();
    Some((document, page, length, *replacement))
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn plan_page_ocr_insertion<'a>(
    doc: &'a InternalDocument,
    text: &str,
    spans: &crate::rendering::ElementSpans,
    page: &PageContent,
) -> Option<FinalOcrEdit<'a>> {
    let owners = || {
        doc.elements
            .iter()
            .filter(|element| element.page == Some(page.page_number))
            .flat_map(|element| element.ocr_source_spans.iter().map(move |span| (element, span)))
    };
    let mut pending = owners().filter_map(|(_, span)| span.insertion.as_ref());
    let insertion = pending.next()?;
    if pending.next().is_some() || owners().any(|(_, span)| span.replacement.is_some()) {
        return None;
    }
    let anchor = |[first, last]: [usize; 2]| {
        let lookup = |id| {
            let mut matching = owners().filter(|(_, span)| span.occurrence == id);
            let owner = matching.next()?;
            matching.next().is_none().then_some(owner)
        };
        let (element, first) = lookup(first)?;
        let (last_element, last) = lookup(last)?;
        if first.owner != last.owner || element.id != last_element.id || first.destination != 0 {
            return None;
        }
        let first_end = first.source.len();
        let end = last.destination.checked_add(last.source.len())?;
        if end != element.text.len() || element.text.get(first_end..last.destination)? != " " {
            return None;
        }
        project_final_ocr_occurrence(doc, (text, spans), page, &element.id, 0..end)
    };
    let [before, after] = insertion.anchors;
    let [left_doc, left_page] = anchor(before)?;
    let [right_doc, right_page] = anchor(after)?;
    if text.get(left_doc.end..right_doc.start)? != "\n\n"
        || page.content.get(left_page.end..right_page.start)? != "\n\n"
    {
        return None;
    }
    Some((
        right_doc.start..right_doc.start,
        right_page.start..right_page.start,
        0,
        &insertion.text,
    ))
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
type PanelOrder = (std::ops::Range<usize>, usize, std::ops::Range<usize>, usize, String);

#[cfg(all(feature = "pdf", feature = "ocr"))]
fn plan_page_panel_order(
    doc: &InternalDocument,
    text: &str,
    spans: &crate::rendering::ElementSpans,
    page: &PageContent,
    page_index: usize,
    verified_edits: &[FinalOcrEdit<'_>],
) -> Option<PanelOrder> {
    let owners = || {
        doc.elements
            .iter()
            .filter(|element| element.page == Some(page.page_number))
            .flat_map(|element| element.ocr_source_spans.iter().map(move |span| (element, span)))
    };
    let mut orders = owners().filter_map(|(_, span)| span.panel_order.as_ref());
    let order = orders.next()?;
    if orders.next().is_some() || !page.tables.is_empty() || verified_edits.len() != 2 {
        return None;
    }
    let project = |id, complete| {
        let mut matches = owners().filter(|(_, span)| span.occurrence == id);
        let (element, span) = matches.next()?;
        if matches.next().is_some()
            || !element.annotations.is_empty()
            || (complete && (span.destination != 0 || span.source.len() != element.text.len()))
        {
            return None;
        }
        let end = span.destination.checked_add(span.source.len())?;
        project_final_ocr_occurrence(doc, (text, spans), page, &element.id, span.destination..end)
    };
    let mapped = order
        .panel
        .iter()
        .map(|&id| project(id, true))
        .collect::<Option<Vec<_>>>()?;
    for id in order.heading {
        let [doc_range, page_range] = project(id, false)?;
        if doc_range.end > mapped[0][0].start
            || page_range.end > mapped[0][1].start
            || !verified_edits
                .iter()
                .any(|edit| edit.0 == doc_range && edit.1 == page_range)
        {
            return None;
        }
    }
    if mapped.windows(2).any(|pair| {
        text.get(pair[0][0].end..pair[1][0].start) != Some("\n\n")
            || page.content.get(pair[0][1].end..pair[1][1].start) != Some("\n\n")
    }) {
        return None;
    }
    let [_, _, left, right, _] = mapped.as_slice() else {
        return None;
    };
    let document = left[0].start..right[0].end;
    let page_bytes = left[1].start..right[1].end;
    let replacement = format!("{}\n\n{}", text.get(right[0].clone())?, text.get(left[0].clone())?);
    Some((document.clone(), page_index, page_bytes, document.len(), replacement))
}

#[cfg(all(feature = "pdf", feature = "ocr"))]
pub(super) fn adopt_final_ocr_replacement(
    doc: &mut InternalDocument,
    text: &mut String,
    spans: &crate::rendering::ElementSpans,
    pages: &mut [PageContent],
) -> Option<()> {
    if doc.source_format != "pdf"
        || !doc.elements.iter().any(|element| {
            element
                .ocr_source_spans
                .iter()
                .any(|span| span.replacement.is_some() || span.insertion.is_some() || span.panel_order.is_some())
        })
    {
        return None;
    }
    let mut all = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        let edits = plan_page_ocr_replacement(doc, text, spans, page).unwrap_or_default();
        let insertion = plan_page_ocr_insertion(doc, text, spans, page);
        let order = plan_page_panel_order(doc, text, spans, page, page_index, &edits);
        all.extend(
            edits
                .into_iter()
                .chain(insertion)
                .map(|(document, page, expected, replacement)| {
                    (document, page_index, page, expected, replacement.to_owned())
                }),
        );
        all.extend(order);
    }
    if all.is_empty() {
        return None;
    }
    all.sort_by_key(|(bytes, ..)| bytes.start);
    if all
        .windows(2)
        .any(|pair| pair[0].0.end > pair[1].0.start || (pair[0].1 == pair[1].1 && pair[0].2.end > pair[1].2.start))
    {
        return None;
    }
    if let Some(boundaries) = doc.metadata.pages.as_mut().and_then(|pages| pages.boundaries.as_mut()) {
        let mut adjusted = boundaries.clone();
        for (original, new) in boundaries.iter().zip(&mut adjusted) {
            if original.byte_start >= original.byte_end || original.byte_end > text.len() {
                return None;
            }
            for (range, page_index, _, expected, replacement) in &all {
                let shift = |offset: usize| offset.checked_sub(*expected)?.checked_add(replacement.len());
                if range.end <= original.byte_start {
                    new.byte_start = shift(new.byte_start)?;
                    new.byte_end = shift(new.byte_end)?;
                } else if range.start < original.byte_end {
                    if range.start < original.byte_start
                        || range.end > original.byte_end
                        || pages[*page_index].page_number != original.page_number
                    {
                        return None;
                    }
                    new.byte_end = shift(new.byte_end)?;
                }
            }
        }
        *boundaries = adjusted;
    }
    for (document_bytes, page_index, page_bytes, _, replacement) in all.into_iter().rev() {
        text.replace_range(document_bytes, &replacement);
        pages[page_index].content.replace_range(page_bytes, &replacement);
    }
    Some(())
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
type FinalOcrOccurrence = (u32, usize, std::ops::Range<usize>, std::ops::Range<usize>);

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
pub(crate) static FINAL_OCR_REPLAY: parking_lot::Mutex<Option<Vec<FinalOcrOccurrence>>> = parking_lot::Mutex::new(None);

#[cfg(all(feature = "pdf", feature = "ocr"))]
pub(super) fn trace_final_ocr_occurrences(
    doc: &InternalDocument,
    rendered: (&str, &crate::rendering::ElementSpans),
    pages: &[PageContent],
) {
    #[cfg(test)]
    let replay_enabled = FINAL_OCR_REPLAY.lock().is_some();
    #[cfg(not(test))]
    let replay_enabled = false;
    if !replay_enabled && !tracing::enabled!(tracing::Level::TRACE) {
        return;
    }
    let mut projected = Vec::new();
    for element in &doc.elements {
        let Some(page) = pages.iter().find(|page| element.page == Some(page.page_number)) else {
            continue;
        };
        for span in &element.ocr_source_spans {
            let Some(end) = span.destination.checked_add(span.source.len()) else {
                continue;
            };
            let owners = doc
                .elements
                .iter()
                .filter(|other| other.page == element.page)
                .flat_map(|other| &other.ocr_source_spans)
                .filter(|other| other.occurrence == span.occurrence && other.owner == span.owner)
                .count();
            if owners == 1
                && let Some([document, page_bytes]) =
                    project_final_ocr_occurrence(doc, rendered, page, &element.id, span.destination..end)
            {
                projected.push((page.page_number, span.occurrence, document, page_bytes));
            }
        }
    }
    #[cfg(test)]
    if let Some(replay) = FINAL_OCR_REPLAY.lock().as_mut() {
        replay.extend(projected.iter().cloned());
    }
    if !projected.is_empty() {
        tracing::trace!(?projected, "final PDF OCR occurrence ranges");
    }
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[test]
fn final_occurrence_projection_rejects_missing_and_ambiguous_owners() {
    let mut doc = InternalDocument::new("pdf");
    let paragraph = |text: &str, index| {
        let mut element = InternalElement::text(ElementKind::Paragraph, text, 0).with_index(index);
        element.page = Some(1);
        element
    };
    let first = paragraph("wi", 0);
    let target = paragraph("wi", 1);
    let owner = target.id.clone();
    doc.elements.extend([first, target]);
    let project = |doc: &InternalDocument| {
        let rendered = crate::rendering::render_plain_with_spans(doc);
        let page = serde_json::from_value(serde_json::json!({
            "page_number": 1, "content": rendered.text
        }))
        .unwrap();
        project_final_ocr_occurrence(doc, (&rendered.text, &rendered.spans), &page, &owner, 0..2)
    };
    assert_eq!(project(&doc), Some([4..6, 4..6]));
    doc.elements[1].id = doc.elements[0].id.clone();
    assert!(project(&doc).is_none());
    doc.elements[0].id = owner.clone();
    doc.elements[1].id = owner.clone();
    assert!(project(&doc).is_none());
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[test]
fn ocr_edit_on_nonfinal_page_shifts_later_document_boundaries() {
    use crate::types::internal::OcrSourceSpan;
    let mut doc = InternalDocument::new("pdf");
    let mut first = InternalElement::text(ElementKind::Paragraph, "wi", 0).with_index(0);
    first.page = Some(1);
    first.ocr_source_spans.push(OcrSourceSpan {
        owner: first.id.clone(),
        occurrence: 7,
        source: 0..2,
        destination: 0,
        insertion: None,
        panel_order: None,
        replacement: Some(("wi".to_owned(), "View".to_owned(), 1)),
    });
    let mut second = InternalElement::text(ElementKind::Paragraph, "keep", 0).with_index(1);
    second.page = Some(2);
    doc.elements.extend([first, second]);
    doc.metadata.pages = Some(
        serde_json::from_value(serde_json::json!({
            "total_count": 2, "unit_type": "page",
            "boundaries": [
                {"page_number": 1,"byte_start": 0,"byte_end": 2},
                {"page_number": 2,"byte_start": 4,"byte_end": 8}
            ]
        }))
        .unwrap(),
    );
    let mut pages: Vec<PageContent> = serde_json::from_value(serde_json::json!([
        {"page_number": 1, "content": "wi"},
        {"page_number": 2, "content": "keep"}
    ]))
    .unwrap();
    let mut rendered = crate::rendering::render_plain_with_spans(&doc);
    assert_eq!(rendered.text, "wi\n\nkeep");
    adopt_final_ocr_replacement(&mut doc, &mut rendered.text, &rendered.spans, &mut pages).unwrap();
    assert_eq!(rendered.text, "View\n\nkeep");
    assert_eq!((&*pages[0].content, &*pages[1].content), ("View", "keep"));
    let boundaries = doc.metadata.pages.unwrap().boundaries.unwrap();
    assert_eq!((boundaries[0].byte_start, boundaries[0].byte_end), (0, 4));
    assert_eq!((boundaries[1].byte_start, boundaries[1].byte_end), (6, 10));
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[test]
fn full_ocr_line_groups_publish_together_or_keep_both_originals() {
    use crate::types::internal::OcrSourceSpan;
    let mut doc = InternalDocument::new("pdf");
    for (index, line) in ["clean phrase", "bad bad line", "bad bad line"].into_iter().enumerate() {
        let mut element = InternalElement::text(ElementKind::Paragraph, line, 0).with_index(index as u32);
        element.page = Some(1);
        if index > 0 {
            for (word, range) in [0..3, 4..7, 8..12].into_iter().enumerate() {
                element.ocr_source_spans.push(OcrSourceSpan {
                    owner: element.id.clone(),
                    occurrence: index * 3 + word,
                    destination: range.start,
                    insertion: None,
                    panel_order: None,
                    replacement: Some((
                        line[range.clone()].to_owned(),
                        if word == 0 {
                            "clean phrase".to_owned()
                        } else {
                            String::new()
                        },
                        6,
                    )),
                    source: range,
                });
            }
        }
        doc.elements.push(element);
    }
    let assert_publication = |mut doc: InternalDocument, accepted| {
        let mut rendered = crate::rendering::render_plain_with_spans(&doc);
        let before = rendered.text.clone();
        let mut pages: Vec<PageContent> = serde_json::from_value(serde_json::json!([
            {"page_number": 1, "content": before}
        ]))
        .unwrap();
        let adopted = adopt_final_ocr_replacement(&mut doc, &mut rendered.text, &rendered.spans, &mut pages);
        assert_eq!(adopted.is_some(), accepted);
        let expected = if accepted {
            "clean phrase\n\nclean phrase\n\nclean phrase"
        } else {
            &before
        };
        assert_eq!(rendered.text, expected);
        assert_eq!(pages[0].content, expected);
    };
    assert_publication(doc.clone(), true);

    let mut overlap = doc.clone();
    overlap.elements[2].ocr_source_spans[1].destination = 0;
    assert_publication(overlap, false);

    let mut mismatch = doc.clone();
    mismatch.elements[2].ocr_source_spans[1].owner = doc.elements[1].id.clone();
    assert_publication(mismatch, false);

    doc.elements[2].ocr_source_spans.clear();
    assert_publication(doc, false);
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[test]
fn ocr_gap_insertion_requires_complete_unique_anchors_in_both_views() {
    use crate::types::internal::{OcrGapInsertion, OcrSourceSpan};
    let mut doc = InternalDocument::new("pdf");
    for (index, line) in ["@ Hero", "@ Intro"].into_iter().enumerate() {
        let mut element = InternalElement::text(ElementKind::Paragraph, line, 0).with_index(index as u32);
        element.page = Some(1);
        for (word, bytes) in [0..1, 2..line.len()].into_iter().enumerate() {
            element.ocr_source_spans.push(OcrSourceSpan {
                owner: element.id.clone(),
                occurrence: index * 2 + word,
                destination: bytes.start,
                source: bytes,
                insertion: None,
                panel_order: None,
                replacement: None,
            });
        }
        doc.elements.push(element);
    }
    doc.elements[1].ocr_source_spans[0].insertion = Some(OcrGapInsertion {
        anchors: [[0, 1], [2, 3]],
        text: "1920x840\n\n".to_owned(),
    });
    let verify = |mut doc: InternalDocument, page_override: Option<&str>, expected: bool| {
        let mut rendered = crate::rendering::render_plain_with_spans(&doc);
        let original = rendered.text.clone();
        let original_page = page_override.unwrap_or(&original);
        let mut pages: Vec<PageContent> = serde_json::from_value(serde_json::json!([
            {"page_number": 1, "content": original_page}
        ]))
        .unwrap();
        let applied = adopt_final_ocr_replacement(&mut doc, &mut rendered.text, &rendered.spans, &mut pages);
        assert_eq!(applied.is_some(), expected);
        if expected {
            assert_eq!(rendered.text, "@ Hero\n\n1920x840\n\n@ Intro");
            assert_eq!(pages[0].content, rendered.text);
        } else {
            assert_eq!(rendered.text, original);
            assert_eq!(pages[0].content, original_page);
        }
    };
    verify(doc.clone(), None, true);
    verify(doc.clone(), Some("@ Hero\n\ncompeting\n\n@ Intro"), false);
    let mut competing = doc.clone();
    let mut element = InternalElement::text(ElementKind::Paragraph, "competing", 0).with_index(2);
    element.page = Some(1);
    competing.elements.insert(1, element);
    verify(competing, None, false);
    let mut duplicate = doc.clone();
    let span = duplicate.elements[0].ocr_source_spans[1].clone();
    duplicate.elements[0].ocr_source_spans.push(span);
    verify(duplicate, None, false);
    doc.elements[0].ocr_source_spans.pop();
    verify(doc, None, false);
}

#[cfg(all(test, feature = "pdf", feature = "ocr"))]
#[test]
fn panel_reorder_requires_recovery_of_its_own_heading() {
    use crate::types::internal::{OcrPanelOrder, OcrSourceSpan};
    let mut doc = InternalDocument::new("pdf");
    for (index, text) in ["Near Head", "Far Head", "A", "B", "Left", "C", "Size"]
        .into_iter()
        .enumerate()
    {
        let mut element = InternalElement::text(ElementKind::Paragraph, text, 0).with_index(index as u32);
        element.page = Some(1);
        let ranges: Vec<_> = if let Some(space) = text.find(' ') {
            vec![0..space, space + 1..text.len()]
        } else {
            std::iter::once(0..text.len()).collect()
        };
        for (word, range) in ranges.into_iter().enumerate() {
            element.ocr_source_spans.push(OcrSourceSpan {
                owner: element.id.clone(),
                occurrence: index * 2 + word,
                destination: range.start,
                source: range,
                replacement: None,
                insertion: None,
                panel_order: (index == 4).then_some(OcrPanelOrder {
                    heading: [0, 1],
                    panel: [4, 6, 8, 10, 12],
                }),
            });
        }
        doc.elements.push(element);
    }
    let verify = |mut doc: InternalDocument, heading: usize, should_reorder| {
        let element = &mut doc.elements[heading];
        for span in &mut element.ocr_source_spans {
            span.replacement = Some((element.text[span.source.clone()].to_owned(), "New".to_owned(), 2));
        }
        let mut rendered = crate::rendering::render_plain_with_spans(&doc);
        let mut pages: Vec<PageContent> = serde_json::from_value(serde_json::json!([
            {"page_number": 1, "content": rendered.text}
        ]))
        .unwrap();
        adopt_final_ocr_replacement(&mut doc, &mut rendered.text, &rendered.spans, &mut pages).unwrap();
        assert!(rendered.text.contains("New New"));
        assert_eq!(rendered.text.contains("B\n\nC\n\nLeft"), should_reorder);
        assert_eq!(rendered.text, pages[0].content);
    };
    verify(doc.clone(), 0, true);
    verify(doc, 1, false);
}

/// Give each page that has elements its part of the plain `content`. A page with no
/// elements keeps its text, and every other field of a page stays.
fn with_plain_page_text(mut pages: Vec<PageContent>, content: &str, ranges: &PlainPageRanges) -> Vec<PageContent> {
    for page in &mut pages {
        if let Some(range) = ranges.get(&page.page_number) {
            page.content = range_text(content, *range).to_string();
        }
    }
    pages
}

fn range_text(content: &str, range: Option<(usize, usize)>) -> &str {
    range
        .and_then(|(start, end)| content.get(start..end))
        .unwrap_or_default()
}

/// Set the range of each page to the place of its text in the plain content. A page with no
/// text loses its range, and nothing is lost with it. Any other range is omitted: the range of a
/// page with text that is not in the content, as the text of a page with no elements, and the
/// range of a page number that has no extracted page. Returns the omitted page numbers, sorted and
/// without repeats, or `None` when nothing was omitted. When no range is left, the boundaries
/// become `None`.
fn set_plain_page_boundaries(
    metadata: &mut crate::types::Metadata,
    pages: Option<&[PageContent]>,
    ranges: &PlainPageRanges,
) -> Option<Vec<u32>> {
    let structure = metadata.pages.as_mut()?;
    let boundaries = structure.boundaries.as_mut()?;
    let mut omitted = Vec::new();
    boundaries.retain_mut(|boundary| {
        let page = pages
            .unwrap_or_default()
            .iter()
            .find(|page| page.page_number == boundary.page_number);
        match (page, ranges.get(&boundary.page_number)) {
            (Some(_), Some(Some((start, end)))) => {
                boundary.byte_start = *start;
                boundary.byte_end = *end;
                return true;
            }
            (Some(page), _) if page.content.is_empty() => {}
            _ => omitted.push(boundary.page_number),
        }
        false
    });
    if boundaries.is_empty() {
        structure.boundaries = None;
    }
    if omitted.is_empty() {
        return None;
    }
    omitted.sort_unstable();
    omitted.dedup();
    Some(omitted)
}

fn has_page_boundaries(metadata: &crate::types::Metadata) -> bool {
    metadata.pages.as_ref().is_some_and(|pages| pages.boundaries.is_some())
}

/// Locate page bodies in order. Keep the old offsets on a missing match so the caller
/// can try another representation of the page. Drop them when the mapping is ambiguous.
pub(crate) fn reconcile_page_boundaries(
    metadata: &mut crate::types::Metadata,
    content: &str,
    pages: Option<&[PageContent]>,
) -> bool {
    let Some(structure) = metadata.pages.as_mut() else {
        return false;
    };
    let Some(boundaries) = structure.boundaries.as_mut() else {
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
    // Both scans must agree: a marker or preamble can contain a complete page body. ~keep
    let mut end = content.len();
    for (page, boundary) in pages.iter().zip(&reconciled).rev() {
        if page.content.is_empty() {
            continue;
        }
        if content[..end].rfind(page.content.as_str()) != Some(boundary.byte_start) {
            structure.boundaries = None;
            return false;
        }
        end = boundary.byte_start;
    }
    *boundaries = reconciled;
    true
}

/// Remove `metadata.pages.boundaries` when they cannot index `content`.
///
/// The offsets were measured on another rendering of the document, so a range that falls
/// outside `content`, inside a UTF-8 sequence, or overlapping another page is unusable.
/// Returns `true` when the boundaries were dropped.
pub(crate) fn drop_unusable_page_boundaries(metadata: &mut crate::types::Metadata, content: &str) -> bool {
    let Some(structure) = metadata.pages.as_mut() else {
        return false;
    };
    let usable = structure.boundaries.as_ref().is_none_or(|boundaries| {
        boundaries
            .iter()
            .all(|b| content.get(b.byte_start..b.byte_end).is_some())
            && overlapping_page_ranges(boundaries, 0).is_empty()
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
pub(crate) fn render_plain_page_elements(
    doc: &InternalDocument,
    page: u32,
) -> Option<(String, crate::rendering::ElementSpans, Vec<usize>)> {
    let indices = compute_page_membership(doc).remove(&page)?;
    let rendered = crate::rendering::render_plain_with_spans(&build_sub_doc(&indices, doc));
    Some((rendered.text, rendered.spans, indices))
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
/// by spacing alone), and a page of another format that took OCR text. In plain output a page of
/// another format with elements already has its part of the plain content (`plain_page_ranges`).
fn is_ocr_page_text(doc: &InternalDocument, page: &PageContent) -> bool {
    source_short_circuits_ocr_page_text(&doc.source_format) || doc.ocr_text_pages.contains(&page.page_number)
}

/// OCR pages whose prebuilt text is not found in `content` (in page order) rendered from their
/// own elements by `render_plain`; a page whose text is found keeps it. Returns the pages and
/// the numbers of those still unlocated in page order: a page that is not OCR text (see
/// `is_ocr_page_text`) keeps its text, and so does an OCR page whose re-rendering is empty or
/// not found.
///
/// The search here selects page representations. Boundary reconciliation separately checks
/// whether their positions are unique.
fn rerender_pages_plain(
    pages: Option<&[PageContent]>,
    doc: &InternalDocument,
    content: &str,
) -> Option<(Vec<PageContent>, Vec<u32>)> {
    let pages = pages?;
    let elements_by_page = compute_page_membership(doc);
    let mut search_offset = 0;
    let mut out = Vec::with_capacity(pages.len());
    let mut unlocated = Vec::new();
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
                    None => unlocated.push(page.page_number),
                }
            }
        }
        out.push(page);
    }
    Some((out, unlocated))
}

/// Reconcile the ordered matches first, then check the original ranges of the remaining pages.
fn reconcile_located_page_boundaries(
    metadata: &mut crate::types::Metadata,
    content: &str,
    pages: &[PageContent],
    unlocated: &[u32],
) -> Option<Vec<u32>> {
    let structure = metadata.pages.as_mut()?;
    let original = structure.boundaries.take()?;
    let page_positions: std::collections::BTreeMap<_, _> = pages
        .iter()
        .enumerate()
        .map(|(index, page)| (page.page_number, index))
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    let duplicates: std::collections::BTreeSet<_> = original
        .iter()
        .map(|boundary| boundary.page_number)
        .filter(|&number| !seen.insert(number))
        .collect();
    let mut ordered = Vec::new();
    let mut remaining = Vec::new();
    let mut omitted = Vec::new();
    for boundary in original {
        if duplicates.contains(&boundary.page_number) {
            omitted.push(boundary.page_number);
            continue;
        }
        match page_positions.get(&boundary.page_number) {
            Some(&index) if !unlocated.contains(&boundary.page_number) => ordered.push((index, boundary)),
            Some(&index) => remaining.push((index, boundary)),
            None => omitted.push(boundary.page_number),
        }
    }
    ordered.sort_by_key(|&(index, _)| index);
    let (located, ordered): (Vec<_>, Vec<_>) = ordered
        .into_iter()
        .map(|(index, boundary)| (pages[index].clone(), boundary))
        .unzip();
    structure.boundaries = Some(ordered);
    let reconciled = reconcile_page_boundaries(metadata, content, Some(&located));
    let structure = metadata.pages.as_mut()?;
    let mut boundaries = structure.boundaries.take().unwrap_or_else(|| {
        omitted.extend(located.iter().map(|page| page.page_number));
        Vec::new()
    });
    boundaries.retain(|boundary| {
        let usable = content.get(boundary.byte_start..boundary.byte_end).is_some();
        if !usable {
            omitted.push(boundary.page_number);
        }
        usable
    });
    let reconciled_count = if reconciled { boundaries.len() } else { 0 };
    for (index, boundary) in remaining {
        if usable_unlocated_boundary(content, &boundary, &pages[index].content) {
            boundaries.push(boundary);
        } else {
            omitted.push(boundary.page_number);
        }
    }
    let overlapping = overlapping_page_ranges(&boundaries, reconciled_count);
    let mut index = 0;
    boundaries.retain(|boundary| {
        let keep = !overlapping.contains(&index);
        index += 1;
        if !keep {
            omitted.push(boundary.page_number);
        }
        keep
    });
    boundaries.sort_by_key(|boundary| boundary.page_number);
    structure.boundaries = (!boundaries.is_empty()).then_some(boundaries);
    omitted.sort_unstable();
    omitted.dedup();
    Some(omitted)
}

/// Find conflicting retained ranges by byte position. Reconciled ranges are already disjoint.
fn overlapping_page_ranges(
    boundaries: &[crate::types::PageBoundary],
    reconciled_count: usize,
) -> std::collections::BTreeSet<usize> {
    let mut by_start: Vec<_> = boundaries
        .iter()
        .enumerate()
        .filter(|(_, boundary)| boundary.byte_start < boundary.byte_end)
        .collect();
    by_start.sort_unstable_by_key(|(_, boundary)| boundary.byte_start);
    let mut conflicts = std::collections::BTreeSet::new();
    let mut furthest: Option<(usize, usize)> = None;
    for (index, boundary) in by_start {
        if let Some((prior, end)) = furthest.filter(|&(_, end)| boundary.byte_start < end) {
            if prior >= reconciled_count {
                conflicts.insert(prior);
            }
            if index >= reconciled_count {
                conflicts.insert(index);
            }
            if boundary.byte_end <= end {
                continue;
            }
        }
        furthest = Some((index, boundary.byte_end));
    }
    conflicts
}

fn usable_unlocated_boundary(content: &str, boundary: &crate::types::PageBoundary, page_text: &str) -> bool {
    let Some(slice) = content.get(boundary.byte_start..boundary.byte_end) else {
        return false;
    };
    content.find(page_text).is_none_or(|start| {
        start == boundary.byte_start && slice == page_text && content.rfind(page_text) == Some(start)
    })
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

    fn boundary_ranges(doc: &InternalDocument) -> Vec<(u32, usize, usize)> {
        doc.metadata
            .pages
            .as_ref()
            .and_then(|s| s.boundaries.as_ref())
            .map(|b| b.iter().map(|b| (b.page_number, b.byte_start, b.byte_end)).collect())
            .unwrap_or_default()
    }

    fn derive(doc: &mut InternalDocument, content: &str) -> Vec<PageContent> {
        derive_pages(doc, content, &OutputFormat::Plain, true, None, &mut ()).expect("pages")
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
    fn recovered_khmer_digits_have_exact_plain_byte_ranges() {
        for count in 1..=2 {
            let mut pages = vec![page(1, "VOU ០")];
            let mut tagged = vec![(1, "VOU០")];
            if count == 2 {
                pages.push(page(2, "Total ១២"));
                tagged.push((2, "Total១២"));
            }
            let mut doc = doc_with(pages, &tagged);
            let (content, pages) = derive_plain(&mut doc);
            let expected = if count == 1 { "VOU០" } else { "VOU០\n\nTotal១២" };
            assert_eq!(content, expected);
            assert_eq!(pages.len(), count);
            assert!(!warned(&doc), "{:?}", doc.processing_warnings);
            assert_ranges(&doc, &content, &pages);
            let boundaries = doc.metadata.pages.unwrap().boundaries.unwrap();
            let actual: Vec<_> = boundaries
                .iter()
                .map(|b| (b.page_number, b.byte_start, b.byte_end))
                .collect();
            assert_eq!(actual, [(1, 0, 6), (2, 8, 19)][..count]);
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
        let native_text = "| a | b |\n| --- | --- |\n| 1 | 2 |[^1]\n\n1: THE NOTE";
        let mut native = page(1, native_text);
        native.tables.push(Arc::new(Table {
            markdown: "| a | b |\n| --- | --- |\n| 1 | 2 |".to_string(),
            ..Default::default()
        }));
        let mut doc = office_doc_with(vec![native, page(2, "Next")], &[(1, "Body"), (2, "Next")]);
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(content, "Body\n\nNext");
        assert_eq!(pages[0].content, "Body");
        assert_eq!(pages[0].tables.len(), 1, "the page keeps its tables");
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    #[test]
    fn every_page_of_a_docx_document_with_ocr_pages_gets_its_part_of_the_plain_content() {
        let mut doc = office_doc_with(
            vec![page(1, "| one |"), page(2, "native | two"), page(3, "| three |")],
            &[(1, "one"), (2, "two"), (3, "three")],
        );
        doc.ocr_text_pages = vec![1, 3];
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(pages[0].content, "one");
        assert_eq!(pages[1].content, "two");
        assert_eq!(pages[2].content, "three");
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    /// The Plain spans say nothing about Markdown content, so a Markdown page keeps the range
    /// that its extractor measured, as on main.
    #[test]
    fn a_markdown_docx_page_keeps_the_range_its_extractor_measured() {
        let doc = office_doc_with(vec![page(1, "one"), page(2, "two")], &[(1, "one"), (2, "two")]);
        let result = super::super::derive_extraction_result(doc, false, OutputFormat::Markdown);
        let boundaries = result.metadata.pages.and_then(|s| s.boundaries).expect("boundaries");
        let ranges: Vec<_> = boundaries
            .iter()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect();
        assert_eq!(ranges, [(1, 0, 999), (2, 0, 999)]);
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

    /// A DOCX document with `pages` built by its parser and the page-tagged paragraphs `tagged`.
    fn office_doc_with(pages: Vec<PageContent>, tagged: &[(u32, &str)]) -> InternalDocument {
        let mut doc = doc_with(pages, tagged);
        doc.ocr_text_pages.clear();
        doc.source_format = "docx".to_string();
        doc
    }

    /// Derive the pages of `doc` for plain output, as `derive` does: from the one plain rendering
    /// that makes the content and the place of each element in it. Returns the content and pages.
    fn derive_plain(doc: &mut InternalDocument) -> (String, Vec<PageContent>) {
        let rendering = crate::rendering::render_plain_with_spans(doc);
        let pages = derive_pages(
            doc,
            &rendering.text,
            &OutputFormat::Plain,
            true,
            Some(&rendering.spans),
            &mut (),
        )
        .expect("pages");
        (rendering.text, pages)
    }

    #[test]
    fn a_docx_range_measured_on_the_parser_text_selects_its_page_text_in_the_plain_content() {
        let mut doc = office_doc_with(
            vec![page(1, "Memo\n\nBody"), page(2, "| a |\n| --- |")],
            &[(1, "Memo"), (1, "Body"), (2, "a")],
        );
        set_boundaries(&mut doc, &[(1, 0, 10), (2, 10, 23)]);
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(pages[0].content, "Memo\n\nBody");
        assert_eq!(pages[1].content, "a");
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    #[test]
    fn pages_with_the_same_text_each_get_the_range_of_their_own_text() {
        let mut doc = office_doc_with(
            vec![page(1, "beta"), page(2, "alpha"), page(3, "beta")],
            &[(1, "beta"), (2, "alpha"), (3, "beta")],
        );
        doc.source_format = "pptx".to_string();
        derive_plain(&mut doc);
        let boundaries = doc.metadata.pages.as_ref().and_then(|s| s.boundaries.clone());
        let ranges: Vec<(u32, usize, usize)> = boundaries
            .iter()
            .flatten()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect();
        assert_eq!(ranges, [(1, 0, 4), (2, 6, 11), (3, 13, 17)]);
        assert!(!warned(&doc));
    }

    #[test]
    fn a_page_with_text_and_no_elements_keeps_its_text_and_loses_its_range_with_a_warning() {
        let mut doc = office_doc_with(vec![page(1, "one"), page(2, "two\nnote")], &[(1, "one")]);
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(content, "one");
        assert_eq!(pages[1].content, "two\nnote");
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1]);
    }

    #[test]
    fn a_page_with_no_text_loses_its_range_without_a_warning() {
        let mut doc = office_doc_with(vec![page(1, "one"), page(2, "")], &[(1, "one")]);
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(pages[1].content, "");
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, &content, &pages[..1]);
    }

    #[test]
    fn a_page_keeps_its_text_when_no_element_of_the_document_writes_text() {
        let mut doc = office_doc_with(vec![page(1, "one")], &[(1, "")]);
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(content, "");
        assert_eq!(pages[0].content, "one");
    }

    #[test]
    fn the_last_page_with_text_keeps_its_range_when_a_later_page_writes_only_whitespace() {
        let mut doc = office_doc_with(
            vec![page(1, "one"), page(2, "two"), page(3, " ")],
            &[(1, "one"), (2, "two"), (3, " ")],
        );
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(content, "one\n\ntwo");
        assert_eq!(pages[1].content, "two");
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 2]);
        assert_ranges(&doc, &content, &pages[..2]);
    }

    #[test]
    fn a_page_whose_elements_write_only_whitespace_loses_its_range_without_a_warning() {
        let mut doc = office_doc_with(
            vec![page(1, "one"), page(2, " "), page(3, "three")],
            &[(1, "one"), (2, " "), (3, "three")],
        );
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(pages[1].content, "");
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 3]);
        assert_ranges(&doc, &content, &[pages[0].clone(), pages[2].clone()]);
    }

    #[test]
    fn exact_ranges_of_pages_that_are_not_located_in_order_stay_without_a_warning() {
        let content = "two\n\none";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "two")], &[]);
        set_boundaries(&mut doc, &[(1, 5, 8), (2, 0, 3)]);
        let pages = derive(&mut doc, content);
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 2]);
        assert_ranges(&doc, content, &pages);
    }

    #[test]
    fn a_located_page_is_reconciled_alongside_an_unmatched_page_with_a_usable_range() {
        let content = "one\n\ntwo";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "| two |")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 999), (2, 5, 8)]);
        derive(&mut doc, content);
        assert!(!warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1, 2]);
        let boundaries = doc.metadata.pages.unwrap().boundaries.unwrap();
        assert_eq!(
            content.get(boundaries[0].byte_start..boundaries[0].byte_end),
            Some("one")
        );
        assert_eq!(
            content.get(boundaries[1].byte_start..boundaries[1].byte_end),
            Some("two")
        );
    }

    #[test]
    fn a_page_range_is_repaired_when_another_page_has_no_boundary() {
        let mut doc = doc_with(vec![page(1, "one"), page(2, "missing")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 999)]);
        let pages = derive(&mut doc, "one");
        assert_eq!(pages[1].content, "missing");
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, "one", &pages[..1]);
        assert!(!warned(&doc));
    }

    #[test]
    fn an_exact_out_of_order_range_survives_an_ambiguous_ordered_match() {
        let content = "two\n\none\n\none";
        for ranges in [[(1, 5, 8), (2, 0, 3)], [(2, 0, 3), (1, 5, 8)]] {
            let mut doc = doc_with(vec![page(1, "one"), page(2, "two")], &[]);
            set_boundaries(&mut doc, &ranges);
            let pages = derive(&mut doc, content);
            assert_eq!(boundary_pages(&doc), vec![2], "input ranges: {ranges:?}");
            assert_ranges(&doc, content, &pages[1..]);
            let warnings: Vec<&str> = doc
                .processing_warnings
                .iter()
                .filter(|warning| warning.source == "page_boundaries")
                .map(|warning| warning.message.as_ref())
                .collect();
            assert_eq!(
                warnings,
                [
                    "Page byte ranges of pages [1] were measured on a different rendering of the text \
                  and could not be mapped onto the returned content, so they were omitted"
                ]
            );
        }
    }

    #[test]
    fn duplicate_boundaries_cannot_make_repeated_page_text_unambiguous() {
        let mut doc = doc_with(vec![page(1, "one")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 3), (1, 5, 8)]);
        let pages = derive(&mut doc, "one\n\none");
        assert_eq!(pages[0].content, "one");
        assert!(!has_page_boundaries(&doc.metadata));
        assert!(warned(&doc));
    }

    #[test]
    fn an_adopted_ocr_document_takes_its_page_ranges_from_its_own_pages_not_from_renderer_spans() {
        let mut doc = office_doc_with(vec![page(1, "one"), page(2, "two\nnote")], &[(1, "one")]);
        doc.metadata.additional.insert(
            super::super::ADOPTED_OCR_DOCUMENT_KEY.into(),
            serde_json::Value::Bool(true),
        );
        let (content, pages) = derive_plain(&mut doc);
        assert_eq!(content, "one");
        assert_eq!(pages[1].content, "two\nnote");
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_eq!(boundary_ranges(&doc), [(1, 0, 3)]);
        let warnings: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "page_boundaries")
            .map(|warning| warning.message.as_ref())
            .collect();
        assert_eq!(
            warnings,
            [
                "Page byte ranges of pages [2] were measured on a different rendering of the text \
                 and could not be mapped onto the returned content, so they were omitted"
            ]
        );
        assert!(
            !doc.metadata
                .additional
                .contains_key(super::super::ADOPTED_OCR_DOCUMENT_KEY),
            "the internal marker must not reach the result metadata"
        );
    }

    #[test]
    fn a_native_document_takes_its_page_ranges_from_renderer_spans() {
        let mut doc = office_doc_with(vec![page(1, "one"), page(2, "two\nnote")], &[(1, "one")]);
        let (content, _) = derive_plain(&mut doc);
        assert_eq!(content, "one");
        assert_eq!(boundary_ranges(&doc), [(1, 0, 3)]);
        let warnings: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "page_boundaries")
            .map(|warning| warning.message.as_ref())
            .collect();
        assert_eq!(
            warnings,
            ["Pages [2] have no text of their own in the returned content, so their page byte ranges were omitted"]
        );
    }

    #[test]
    fn a_range_that_cannot_be_verified_beside_a_diverging_page_is_omitted_with_a_warning() {
        let content = "one\n\none";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "| x |")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 5, 8)]);
        derive(&mut doc, content);
        let boundaries = doc.metadata.pages.as_ref().and_then(|s| s.boundaries.as_ref()).unwrap();
        let ranges: Vec<_> = boundaries
            .iter()
            .map(|b| (b.page_number, b.byte_start, b.byte_end))
            .collect();
        assert_eq!(
            ranges,
            [(2, 5, 8)],
            "page 1 repeats in the content, so its range is unverified"
        );
        let warnings: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "page_boundaries")
            .map(|warning| warning.message.as_ref())
            .collect();
        assert_eq!(
            warnings,
            [
                "Page byte ranges of pages [1] were measured on a different rendering of the text \
                 and could not be mapped onto the returned content, so they were omitted"
            ]
        );
    }

    #[test]
    fn a_retained_range_cannot_reuse_the_span_of_a_reconciled_page() {
        let mut doc = doc_with(vec![page(1, "one"), page(2, "one")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 999), (2, 0, 3)]);
        let pages = derive(&mut doc, "one");
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, "one", &pages[..1]);
        assert!(warned(&doc));
    }

    #[test]
    fn overlapping_stale_ranges_are_omitted_even_when_their_endpoints_are_valid() {
        let content = "one two three";
        let mut doc = doc_with(
            vec![page(1, "one"), page(2, "| two three |"), page(3, "| three |")],
            &[],
        );
        set_boundaries(&mut doc, &[(1, 0, 999), (2, 4, 13), (3, 8, 13)]);
        let pages = derive(&mut doc, content);
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, content, &pages[..1]);
        assert!(warned(&doc));
    }

    #[test]
    fn retained_ranges_with_endpoints_inside_multibyte_characters_are_omitted() {
        let content = "one é";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "| é |"), page(3, "| e |")], &[]);
        set_boundaries(&mut doc, &[(1, 0, 999), (2, 5, 6), (3, 4, 5)]);
        let pages = derive(&mut doc, content);
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, content, &pages[..1]);
        assert!(warned(&doc));
    }

    #[test]
    fn an_out_of_order_page_with_multiple_placements_loses_its_range() {
        let content = "two\n\ntwo\n\none";
        let mut doc = doc_with(vec![page(1, "one"), page(2, "two")], &[]);
        set_boundaries(&mut doc, &[(1, 10, 13), (2, 0, 3)]);
        let pages = derive(&mut doc, content);
        assert!(warned(&doc));
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert_ranges(&doc, content, &pages[..1]);
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
        let (content, pages) = derive_plain(&mut doc);
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
        let (content, pages) = derive_plain(&mut doc);
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
        let (_, pages) = derive_plain(&mut doc);
        assert_eq!(pages[0].content, "First page");
    }

    #[test]
    fn a_note_marker_after_the_last_text_of_a_page_stays_on_that_page() {
        let mut builder = crate::types::internal_builder::InternalDocumentBuilder::new("docx");
        builder.push_paragraph("First page", Vec::new(), Some(1), None);
        builder.push_footnote_ref("1", "fn1", Some(2));
        builder.push_paragraph("Second page", Vec::new(), Some(2), None);
        builder.push_footnote_definition("Note one", "fn1", None);
        let mut doc = builder.build();
        doc.metadata.pages = Some(PageStructure {
            total_count: 2,
            unit_type: PageUnitType::Page,
            boundaries: None,
            pages: None,
        });
        set_boundaries(&mut doc, &[(1, 0, 10), (2, 10, 21)]);
        let (content, pages) = derive_plain(&mut doc);
        assert!(content.starts_with("First page[^1]\n\nSecond page"), "{content}");
        assert_eq!(pages[0].content, "First page[^1]");
        assert!(pages[1].content.starts_with("Second page"));
        assert!(!warned(&doc));
        assert_ranges(&doc, &content, &pages);
    }

    #[test]
    fn the_range_of_a_page_number_with_no_extracted_page_is_omitted_with_a_warning() {
        let mut doc = office_doc_with(vec![page(1, "one")], &[(1, "one"), (2, "two")]);
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 5, 8), (3, 8, 9)]);
        derive_plain(&mut doc);
        assert_eq!(boundary_pages(&doc), vec![1]);
        assert!(warned(&doc));
    }

    #[test]
    fn the_warning_names_a_page_with_two_omitted_ranges_one_time() {
        let mut doc = office_doc_with(vec![page(1, "one"), page(2, "gone")], &[(1, "one")]);
        set_boundaries(&mut doc, &[(1, 0, 3), (2, 0, 1), (2, 1, 2)]);
        derive_plain(&mut doc);
        assert_eq!(boundary_pages(&doc), vec![1]);
        let messages: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|w| w.source == "page_boundaries")
            .map(|w| w.message.as_ref())
            .collect();
        assert_eq!(messages.len(), 1, "one page_boundaries warning: {messages:?}");
        assert!(
            messages[0].contains("Pages [2] "),
            "page 2 is named one time: {messages:?}"
        );
    }

    #[test]
    fn the_warning_names_omitted_pages_in_page_order_and_no_range_is_left() {
        let mut doc = office_doc_with(vec![page(1, "one")], &[(1, "one")]);
        set_boundaries(&mut doc, &[(3, 5, 8), (2, 0, 3)]);
        derive_plain(&mut doc);
        let structure = doc.metadata.pages.as_ref().expect("page structure");
        assert!(
            structure.boundaries.is_none(),
            "no range is left: {:?}",
            structure.boundaries
        );
        let messages: Vec<&str> = doc
            .processing_warnings
            .iter()
            .filter(|w| w.source == "page_boundaries")
            .map(|w| w.message.as_ref())
            .collect();
        assert_eq!(messages.len(), 1, "one page_boundaries warning: {messages:?}");
        assert!(messages[0].contains("Pages [2, 3] "), "pages in order: {messages:?}");
    }
}
