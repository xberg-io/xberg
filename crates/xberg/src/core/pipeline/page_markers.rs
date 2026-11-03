//! Page marker element injection for rendered output formats.
//!
//! Extractors insert page markers into the plain-text content string, but the
//! Markdown/Djot renderers work from `InternalDocument` elements, where markers
//! either arrive as escapable `Paragraph` text (flat documents) or not at all
//! (structured documents, which carry `PageBreak` elements instead). This pass
//! normalizes both shapes to verbatim `RawBlock` marker elements.

use crate::core::config::page::marker_line_regex;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

/// Represent page markers as `RawBlock` elements so renderers emit them verbatim.
///
/// Flat documents (built by splitting marker-bearing text into paragraphs) get
/// their marker paragraphs converted in place. Structured documents (no marker
/// text) get `RawBlock` markers synthesized. An extractor that lists the pages
/// it iterated in `InternalDocument::page_numbers` gets one marker per listed
/// page, an empty page included. Any other structured document gets one marker
/// before the first element and one after each page break.
pub(crate) fn inject_page_marker_elements(doc: &mut InternalDocument, marker_format: &str) {
    if doc.elements.is_empty() && doc.page_numbers.is_empty() {
        return;
    }

    let marker_re = marker_line_regex(marker_format);
    let mut converted = false;
    for elem in doc.elements.iter_mut() {
        if matches!(elem.kind, ElementKind::Paragraph) && marker_re.is_match(elem.text.trim()) {
            elem.kind = ElementKind::RawBlock;
            elem.text = elem.text.trim().to_string();
            converted = true;
        }
    }
    if converted {
        return;
    }

    let render_marker = |page: u32| {
        marker_format
            .replace("{page_num}", &page.to_string())
            .trim()
            .to_string()
    };
    let marker_elem = |page: u32| InternalElement::text(ElementKind::RawBlock, render_marker(page), 0);

    if !doc.page_numbers.is_empty() {
        insert_markers_for_listed_pages(doc, &marker_elem);
        return;
    }

    let elements = std::mem::take(&mut doc.elements);
    let total = elements.len();
    let mut out = Vec::with_capacity(total + 4);
    let mut current_page = elements.iter().find_map(|e| e.page).unwrap_or(1);
    out.push(marker_elem(current_page));

    let mut iter = elements.into_iter().enumerate().peekable();
    while let Some((idx, elem)) = iter.next() {
        let is_break = matches!(elem.kind, ElementKind::PageBreak);
        out.push(elem);
        if is_break && idx + 1 < total {
            current_page = iter.peek().and_then(|(_, next)| next.page).unwrap_or(current_page + 1);
            out.push(marker_elem(current_page));
        }
    }
    doc.elements = out;
}

/// Put one marker per page of `doc.page_numbers`, in list order, before the first element of a later page.
///
/// The first marker leads the document. Each other marker waits for the first element
/// whose page is at least its own, so a page with no element still gets its marker and
/// elements without a page number never move one. Markers left over after the last
/// element belong to trailing pages with no content.
///
/// A container start (a list, a quote, a group) carries no page number, so the first
/// element of a page can sit right behind one. The marker goes before that container
/// start: the page begins where its first block begins, and a marker inside the block
/// would render as a bullet or a child of the block.
fn insert_markers_for_listed_pages(doc: &mut InternalDocument, marker_elem: &dyn Fn(u32) -> InternalElement) {
    let mut pages = doc.page_numbers.iter().copied();
    let mut pending = pages.next();
    let mut out = Vec::with_capacity(doc.elements.len() + doc.page_numbers.len());
    if let Some(first) = pending.take() {
        out.push(marker_elem(first));
        pending = pages.next();
    }
    for elem in std::mem::take(&mut doc.elements) {
        if let Some(element_page) = elem.page {
            while let Some(page) = pending.filter(|&page| page <= element_page) {
                let open_blocks = out.iter().rev().take_while(|e| e.kind.is_container_start()).count();
                out.insert(out.len() - open_blocks, marker_elem(page));
                pending = pages.next();
            }
        }
        out.push(elem);
    }
    out.extend(pending.into_iter().chain(pages).map(marker_elem));
    doc.elements = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_FORMAT: &str = "\n\n<!-- PAGE {page_num} -->\n\n";

    fn paragraph(text: &str) -> InternalElement {
        InternalElement::text(ElementKind::Paragraph, text, 0)
    }

    #[test]
    fn converts_flat_marker_paragraphs_to_raw_blocks() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("<!-- PAGE 1 -->"));
        doc.push_element(paragraph("First page text."));
        doc.push_element(paragraph("<!-- PAGE 2 -->"));
        doc.push_element(paragraph("Second page text."));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        assert!(matches!(doc.elements[0].kind, ElementKind::RawBlock));
        assert_eq!(doc.elements[0].text, "<!-- PAGE 1 -->");
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
        assert!(matches!(doc.elements[2].kind, ElementKind::RawBlock));
        assert_eq!(doc.elements.len(), 4);
    }

    #[test]
    fn synthesizes_markers_around_page_breaks() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("First page text."));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.push_element(paragraph("Second page text."));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let texts: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::RawBlock))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(texts, vec!["<!-- PAGE 1 -->", "<!-- PAGE 2 -->"]);
        assert!(matches!(doc.elements[0].kind, ElementKind::RawBlock));
    }

    #[test]
    fn uses_element_page_numbers_when_present() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("Page three text.").with_page(3));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.push_element(paragraph("Page seven text.").with_page(7));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let texts: Vec<&str> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::RawBlock))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(texts, vec!["<!-- PAGE 3 -->", "<!-- PAGE 7 -->"]);
    }

    fn marker_texts(doc: &InternalDocument) -> Vec<&str> {
        doc.elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::RawBlock))
            .map(|e| e.text.as_str())
            .collect()
    }

    #[test]
    fn listed_pages_get_a_marker_each_when_the_extractor_lists_them() {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2, 3, 4, 5];
        doc.push_element(paragraph("Page two text.").with_page(2));
        doc.push_element(paragraph("Page four text.").with_page(4));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let order: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "<!-- PAGE 1 -->",
                "<!-- PAGE 2 -->",
                "Page two text.",
                "<!-- PAGE 3 -->",
                "<!-- PAGE 4 -->",
                "Page four text.",
                "<!-- PAGE 5 -->",
            ]
        );
    }

    #[test]
    fn listed_pages_ignore_page_breaks_and_unnumbered_elements() {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2];
        doc.push_element(paragraph("Unnumbered lead text."));
        doc.push_element(paragraph("Page one text.").with_page(1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));
        doc.push_element(paragraph("Page two text.").with_page(2));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        assert_eq!(marker_texts(&doc), vec!["<!-- PAGE 1 -->", "<!-- PAGE 2 -->"]);
        assert_eq!(doc.elements[0].text, "<!-- PAGE 1 -->");
    }

    #[test]
    fn listed_page_marker_goes_before_a_list_that_opens_the_page() {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2, 3];
        doc.push_element(paragraph("Page one text.").with_page(1));
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
        let item = InternalElement::text(ElementKind::ListItem { ordered: false }, "First item", 0);
        doc.push_element(item.with_page(2));
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
        doc.push_element(InternalElement::text(ElementKind::QuoteStart, "", 0));
        doc.push_element(InternalElement::text(ElementKind::GroupStart, "", 0));
        doc.push_element(paragraph("Quoted page three text.").with_page(3));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let order: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "<!-- PAGE 1 -->",
                "Page one text.",
                "<!-- PAGE 2 -->",
                "",
                "First item",
                "",
                "<!-- PAGE 3 -->",
                "",
                "",
                "Quoted page three text.",
            ]
        );
        assert_eq!(doc.elements[3].kind, ElementKind::ListStart { ordered: false });
    }

    #[test]
    fn listed_empty_page_before_a_list_keeps_its_markers_ahead_of_the_list() {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2, 3];
        doc.push_element(paragraph("Page one text.").with_page(1));
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: true }, "", 0));
        doc.push_element(InternalElement::text(ElementKind::ListItem { ordered: true }, "Item", 0).with_page(3));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let order: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(
            order,
            vec![
                "<!-- PAGE 1 -->",
                "Page one text.",
                "<!-- PAGE 2 -->",
                "<!-- PAGE 3 -->",
                "",
                "Item"
            ]
        );
    }

    fn three_pages_with_text_only_on_page_two() -> InternalDocument {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2, 3];
        doc.push_element(paragraph("Page two text.").with_page(2));
        doc
    }

    #[test]
    fn listed_pages_survive_a_json_round_trip() {
        let json = serde_json::to_string(&three_pages_with_text_only_on_page_two()).unwrap();
        let mut doc: InternalDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc.page_numbers, vec![1, 2, 3]);

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        assert_eq!(
            marker_texts(&doc),
            vec!["<!-- PAGE 1 -->", "<!-- PAGE 2 -->", "<!-- PAGE 3 -->"]
        );
    }

    #[test]
    fn json_without_the_page_list_reads_as_no_listed_pages() {
        let mut value = serde_json::to_value(three_pages_with_text_only_on_page_two()).unwrap();
        assert!(value.as_object_mut().unwrap().remove("page_numbers").is_some());
        let mut doc: InternalDocument = serde_json::from_value(value).unwrap();
        assert!(doc.page_numbers.is_empty());

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        assert_eq!(marker_texts(&doc), vec!["<!-- PAGE 2 -->"]);
        assert!(
            !serde_json::to_string(&InternalDocument::new("pdf"))
                .unwrap()
                .contains("page_numbers")
        );
    }

    #[test]
    fn listed_pages_with_no_elements_still_get_every_marker() {
        let mut doc = InternalDocument::new("pdf");
        doc.page_numbers = vec![1, 2];

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        assert_eq!(marker_texts(&doc), vec!["<!-- PAGE 1 -->", "<!-- PAGE 2 -->"]);
    }

    #[test]
    fn trailing_page_break_gets_no_dangling_marker() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("Only page text."));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 0));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        let raw_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::RawBlock))
            .count();
        assert_eq!(raw_count, 1);
    }

    #[test]
    fn custom_format_with_repeated_placeholder() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("Page 2 of document (page 2)"));
        doc.push_element(paragraph("Body text."));

        inject_page_marker_elements(&mut doc, "Page {page_num} of document (page {page_num})");

        assert!(matches!(doc.elements[0].kind, ElementKind::RawBlock));
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
    }

    #[test]
    fn empty_document_is_untouched() {
        let mut doc = InternalDocument::new("pdf");
        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);
        assert!(doc.elements.is_empty());
    }

    #[test]
    fn non_marker_paragraphs_are_not_converted() {
        let mut doc = InternalDocument::new("pdf");
        doc.push_element(paragraph("This mentions <!-- PAGE 1 --> inline but is prose."));

        inject_page_marker_elements(&mut doc, DEFAULT_FORMAT);

        // No marker paragraph and no page breaks: page-1 marker is synthesized,
        // prose paragraph stays a paragraph.
        assert!(matches!(doc.elements[0].kind, ElementKind::RawBlock));
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
    }
}
