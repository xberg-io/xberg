//! Flat alphabetic clauses use paragraphs only inside their own decimal source list.

use super::{comrak_options, render_markdown};
use crate::rendering::render_plain;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use comrak::nodes::{ListType, NodeValue};
use comrak::{Arena, parse_document};

fn source_list(doc: &mut InternalDocument, items: &[(&str, &str)]) {
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: true }, "", 0));
    for (label, body) in items {
        let mut item = InternalElement::text(ElementKind::ListItem { ordered: true }, *body, 1);
        item.set_list_item_source_label(*label);
        doc.push_element(item);
    }
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
}

fn qualify(doc: &InternalDocument, source_lists: usize) -> String {
    assert_eq!(
        doc.elements
            .iter()
            .filter(|element| matches!(element.kind, ElementKind::ListStart { .. }))
            .count(),
        source_lists
    );
    let plain = render_plain(doc);
    let markdown = render_markdown(doc);
    for item in &doc.elements {
        if matches!(item.kind, ElementKind::ListItem { .. }) {
            assert_eq!(item.depth, 1);
            let label = item.list_item_source_label().unwrap();
            assert!(plain.contains(&format!("{label} {}", item.text)));
            assert_eq!(markdown.matches(&item.text).count(), 1);
        }
    }
    markdown
}

fn lists(markdown: &str) -> Vec<(ListType, usize, usize)> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.children()
        .filter_map(|node| match node.data.borrow().value {
            NodeValue::List(list) => Some((list.list_type, list.start, node.children().count())),
            _ => None,
        })
        .collect()
}

#[test]
fn leading_alphabetic_clauses_use_paragraphs_before_a_decimal_sibling() {
    let mut doc = InternalDocument::new("pdf");
    source_list(
        &mut doc,
        &[
            ("A.", "Leading alpha clause"),
            ("B.", "Leading beta clause"),
            ("7.", "Seventh clause"),
        ],
    );
    let markdown = qualify(&doc, 1);
    assert_eq!(lists(&markdown), [(ListType::Ordered, 7, 1)], "{markdown:?}");
    let arena = Arena::new();
    let root = parse_document(&arena, &markdown, &comrak_options());
    let paragraphs: Vec<String> = root
        .children()
        .filter_map(|node| {
            if !matches!(node.data.borrow().value, NodeValue::Paragraph) {
                return None;
            }
            Some(
                node.descendants()
                    .filter_map(|child| match &child.data.borrow().value {
                        NodeValue::Text(text) => Some(text.to_string()),
                        _ => None,
                    })
                    .collect(),
            )
        })
        .collect();
    assert_eq!(paragraphs, ["A. Leading alpha clause", "B. Leading beta clause"]);
    let positions: Vec<_> = ["Leading alpha clause", "Leading beta clause", "Seventh clause"]
        .into_iter()
        .map(|body| markdown.find(body).unwrap())
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn literal_only_sibling_source_lists_keep_bullets_beside_decimal_lists() {
    for decimal_first in [false, true] {
        let mut doc = InternalDocument::new("pdf");
        let literal = [("B.", "Independent beta clause"), ("D.", "Independent delta clause")];
        let decimal = [
            ("7.", "Independent seventh clause"),
            ("8.", "Independent eighth clause"),
        ];
        for items in if decimal_first {
            [&decimal, &literal]
        } else {
            [&literal, &decimal]
        } {
            source_list(&mut doc, items);
        }
        let markdown = qualify(&doc, 2);
        let mut expected = vec![(ListType::Bullet, 1, 2), (ListType::Ordered, 7, 2)];
        if decimal_first {
            expected.reverse();
        }
        assert_eq!(lists(&markdown), expected, "{markdown:?}");
        assert!(markdown.contains("- B. Independent beta clause"));
        assert!(markdown.contains("- D. Independent delta clause"));
    }
}
