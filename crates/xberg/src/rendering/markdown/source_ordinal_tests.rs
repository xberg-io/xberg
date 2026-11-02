//! Source labels survive CommonMark list grouping and a real reader round trip.

use super::{comrak_options, render_markdown};
use crate::rendering::render_plain;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{Arena, parse_document};

fn start(doc: &mut InternalDocument, depth: u16) {
    doc.push_element(InternalElement::text(
        ElementKind::ListStart { ordered: true },
        "",
        depth,
    ));
}

fn end(doc: &mut InternalDocument, depth: u16) {
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", depth));
}

fn item(doc: &mut InternalDocument, label: &str, body: &str, depth: u16) {
    let mut element = InternalElement::text(ElementKind::ListItem { ordered: true }, body, depth);
    element.set_list_item_source_label(label);
    doc.push_element(element);
}

fn document(items: &[(&str, &str)]) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    start(&mut doc, 0);
    for (label, body) in items {
        item(&mut doc, label, body, 1);
    }
    end(&mut doc, 0);
    doc
}

fn render_qualified(doc: &InternalDocument) -> String {
    let plain = render_plain(doc);
    let markdown = render_markdown(doc);
    for element in &doc.elements {
        if let ElementKind::ListItem { .. } = element.kind {
            let label = element.list_item_source_label().expect("fixture has a source label");
            assert!(plain.contains(&format!("{label} {}", element.text)), "{plain:?}");
            assert_eq!(markdown.matches(&element.text).count(), 1, "{markdown:?}");
        }
    }
    markdown
}

fn list_shape<'a>(node: &'a AstNode<'a>) -> (ListType, usize, usize) {
    let data = node.data.borrow();
    let NodeValue::List(list) = data.value else {
        panic!("expected an actual parsed CommonMark list");
    };
    let count = node
        .children()
        .filter(|child| matches!(child.data.borrow().value, NodeValue::Item(_)))
        .count();
    (list.list_type, list.start, count)
}

fn top_lists(markdown: &str) -> Vec<(ListType, usize, usize)> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.children()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::List(_)))
        .map(list_shape)
        .collect()
}

#[test]
fn source_gap_remains_two_ordered_runs_after_commonmark_reparse() {
    assert_eq!(
        top_lists("1. First obligation\n\n3. Third obligation\n"),
        vec![(ListType::Ordered, 1, 2)],
        "blank lines alone do not prevent a reader from renumbering the third clause"
    );
    let doc = document(&[("1.", "First obligation"), ("3.", "Third obligation")]);
    let markdown = render_qualified(&doc);
    assert_eq!(
        top_lists(&markdown),
        vec![(ListType::Ordered, 1, 1), (ListType::Ordered, 3, 1)],
        "{markdown:?}"
    );
}

#[test]
fn source_restart_does_not_continue_the_preceding_ordered_run() {
    let doc = document(&[
        ("1.", "Initial first clause"),
        ("2.", "Initial second clause"),
        ("1.", "Restarted first clause"),
        ("2.", "Restarted second clause"),
    ]);
    let markdown = render_qualified(&doc);
    assert_eq!(
        top_lists(&markdown),
        vec![(ListType::Ordered, 1, 2), (ListType::Ordered, 1, 2)],
        "{markdown:?}"
    );
}

#[test]
fn same_level_literal_family_does_not_demote_either_decimal_run() {
    let doc = document(&[
        ("1.", "Principal clause"),
        ("A.", "Alpha qualification"),
        ("B.", "Beta qualification"),
        ("7.", "Seventh clause"),
    ]);
    let markdown = render_qualified(&doc);
    assert!(markdown.contains("- A. Alpha qualification"));
    assert!(markdown.contains("- B. Beta qualification"));
    assert_eq!(
        top_lists(&markdown),
        vec![
            (ListType::Ordered, 1, 1),
            (ListType::Bullet, 1, 2),
            (ListType::Ordered, 7, 1),
        ],
        "{markdown:?}"
    );
}

#[test]
fn nested_literal_children_do_not_split_or_renumber_the_outer_run() {
    let mut doc = InternalDocument::new("pdf");
    start(&mut doc, 0);
    item(&mut doc, "7.", "Outer seventh clause", 1);
    start(&mut doc, 1);
    item(&mut doc, "B.", "Nested beta clause", 2);
    item(&mut doc, "D.", "Nested delta clause", 2);
    end(&mut doc, 1);
    item(&mut doc, "8.", "Outer eighth clause", 1);
    end(&mut doc, 0);
    assert_eq!(doc.elements[2].kind, ElementKind::ListStart { ordered: true });
    assert_eq!(doc.elements[2].depth, 1);
    assert_eq!(doc.elements[3].depth, 2);
    let markdown = render_qualified(&doc);
    let arena = Arena::new();
    let root = parse_document(&arena, &markdown, &comrak_options());
    let lists: Vec<_> = root
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::List(_)))
        .collect();
    assert_eq!(lists.len(), 2, "{markdown:?}");
    let nested_parent = lists[1].parent().expect("nested list must have an item parent");
    assert!(matches!(nested_parent.data.borrow().value, NodeValue::Item(_)));
    assert!(std::ptr::eq(nested_parent.parent().unwrap(), lists[0]));
    assert_eq!(list_shape(lists[0]), (ListType::Ordered, 7, 2), "{markdown:?}");
    assert_eq!(list_shape(lists[1]), (ListType::Bullet, 1, 2), "{markdown:?}");
}

#[test]
fn explicit_sibling_source_lists_remain_distinct_after_commonmark_reparse() {
    let mut doc = document(&[("1.", "First separate clause")]);
    start(&mut doc, 0);
    item(&mut doc, "3.", "Third separate clause", 1);
    end(&mut doc, 0);
    let markdown = render_qualified(&doc);
    assert_eq!(
        top_lists(&markdown),
        vec![(ListType::Ordered, 1, 1), (ListType::Ordered, 3, 1)],
        "{markdown:?}"
    );
}

#[test]
fn noncanonical_decimal_labels_keep_the_existing_literal_fallback() {
    for label in ["01.", "(2)", "3)", "[4]", "١.", "1000000000."] {
        let doc = document(&[(label, "Literal source clause")]);
        let markdown = render_qualified(&doc);
        assert_eq!(top_lists(&markdown), vec![(ListType::Bullet, 1, 1)], "{markdown:?}");
        let arena = Arena::new();
        let root = parse_document(&arena, &markdown, &comrak_options());
        let visible: String = root
            .descendants()
            .filter_map(|node| match &node.data.borrow().value {
                NodeValue::Text(text) => Some(text.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(visible, format!("{label} Literal source clause"));
    }
}

#[test]
fn canonical_zero_and_nine_digit_limits_keep_actual_source_starts() {
    for (labels, expected_start) in [(["0.", "1."], 0), (["999999998.", "999999999."], 999_999_998)] {
        let doc = document(&[
            (labels[0], "Boundary first clause"),
            (labels[1], "Boundary second clause"),
        ]);
        let markdown = render_qualified(&doc);
        assert_eq!(
            top_lists(&markdown),
            vec![(ListType::Ordered, expected_start, 2)],
            "{markdown:?}"
        );
    }
}

#[test]
fn actual_comrak_boundary_comments_preserve_sibling_list_reader_semantics() {
    let mut doc = document(&[("1.", "First explicit list body")]);
    start(&mut doc, 0);
    item(&mut doc, "1.", "Second explicit list body", 1);
    end(&mut doc, 0);
    for element in &mut doc.elements {
        if matches!(element.kind, ElementKind::ListItem { .. }) {
            element.attributes = None;
        }
    }
    let arena = Arena::new();
    let root = super::build_comrak_ast(&doc, &arena);
    let mut raw = String::new();
    comrak::format_commonmark(root, &comrak_options(), &mut raw).expect("format actual producer AST");
    assert_eq!(raw.matches("<!-- end list -->").count(), 1, "{raw:?}");
    assert_eq!(
        top_lists(&raw),
        vec![(ListType::Ordered, 1, 1), (ListType::Ordered, 1, 1)]
    );
    let markdown = render_markdown(&doc);
    for body in ["First explicit list body", "Second explicit list body"] {
        assert_eq!(markdown.matches(body).count(), 1);
    }
    assert_eq!(
        top_lists(&markdown),
        vec![(ListType::Ordered, 1, 1), (ListType::Ordered, 1, 1)],
        "{markdown:?}"
    );
}

#[test]
fn noncanonical_comments_stay_filtered_and_configured_page_markers_stay_exact() {
    let source = [
        "Before",
        "<!-- end list-->",
        "<!-- end  list -->",
        "<!-- END LIST -->",
        "<!-- ordinary comment -->",
        "<!-- PAGE 12 -->",
        "<!-- PAGE twelve -->",
        "After",
    ]
    .join("\n");
    assert_eq!(
        super::filter_page_marker_comments(source.clone(), None),
        "Before\nAfter"
    );
    assert_eq!(
        super::filter_page_marker_comments(source, Some("\n\n<!-- PAGE {page_num} -->\n\n")),
        "Before\n<!-- PAGE 12 -->\nAfter"
    );
}

#[test]
fn identical_source_authored_list_comment_never_grants_generated_tag_ownership() {
    let mut doc = InternalDocument::new("test");
    doc.push_element(InternalElement::text(ElementKind::RawBlock, "<!-- end list -->", 0));
    assert!(!super::markdown_tag_exemptions_are_owned(&doc));
    let _ = render_markdown(&doc);
    assert!(!super::markdown_tag_exemptions_are_owned(&doc));
    assert_eq!(doc.elements[0].kind, ElementKind::RawBlock);
    assert_eq!(doc.elements[0].text, "<!-- end list -->");
}
