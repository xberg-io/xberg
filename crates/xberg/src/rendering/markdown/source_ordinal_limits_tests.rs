//! Unlabelled ordered continuations cannot exceed CommonMark's nine-digit domain.

use super::{build_comrak_ast, comrak_options, render_markdown};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use comrak::nodes::{ListType, NodeValue};
use comrak::{Arena, parse_document};

fn document(items: &[(Option<&str>, &str)]) -> InternalDocument {
    let mut doc = InternalDocument::new("test");
    doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: true }, "", 0));
    for (label, body) in items {
        let mut item = InternalElement::text(ElementKind::ListItem { ordered: true }, *body, 1);
        if let Some(label) = label {
            item.set_list_item_source_label(*label);
        }
        doc.push_element(item);
    }
    doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));
    doc
}

fn qualify(doc: &InternalDocument, items: &[(Option<&str>, &str)]) -> String {
    let source_items: Vec<_> = doc
        .elements
        .iter()
        .filter(|element| matches!(element.kind, ElementKind::ListItem { ordered: true }))
        .collect();
    assert_eq!(source_items.len(), items.len());
    for (element, (label, body)) in source_items.iter().zip(items) {
        assert_eq!(element.text, *body);
        assert_eq!(element.list_item_source_label(), *label);
        if label.is_none() {
            assert!(element.attributes.is_none(), "no printed source number may be invented");
        }
    }
    let arena = Arena::new();
    let root = build_comrak_ast(doc, &arena);
    assert_eq!(
        root.descendants()
            .filter(|node| matches!(node.data.borrow().value, NodeValue::Item(_)))
            .count(),
        items.len()
    );
    let mut raw = String::new();
    comrak::format_commonmark(root, &comrak_options(), &mut raw).expect("format genuine structured source");
    let first_label = items[0]
        .0
        .expect("fixture begins with an actual printed source ordinal");
    assert!(raw.lines().any(|line| line.starts_with(&format!("{first_label} "))));
    let markdown = render_markdown(doc);
    for (_, body) in items {
        assert_eq!(raw.matches(body).count(), 1);
        assert_eq!(markdown.matches(body).count(), 1);
    }
    markdown
}

fn read_runs(markdown: &str) -> Vec<(usize, Vec<String>)> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &comrak_options());
    root.children()
        .filter_map(|node| {
            let data = node.data.borrow();
            let NodeValue::List(list) = data.value else {
                return None;
            };
            assert_eq!(list.list_type, ListType::Ordered);
            let bodies = node
                .children()
                .map(|item| {
                    assert!(matches!(item.data.borrow().value, NodeValue::Item(_)));
                    item.descendants()
                        .filter_map(|child| match &child.data.borrow().value {
                            NodeValue::Text(text) => Some(text.to_string()),
                            _ => None,
                        })
                        .collect()
                })
                .collect();
            Some((list.start, bodies))
        })
        .collect()
}

#[test]
fn maximum_source_ordinal_restarts_unlabelled_continuations_in_a_representable_run() {
    let items = [
        (Some("999999999."), "Maximum source clause"),
        (None, "First unlabelled continuation"),
        (None, "Second unlabelled continuation"),
    ];
    let doc = document(&items);
    let markdown = qualify(&doc, &items);
    assert_eq!(
        read_runs(&markdown),
        vec![
            (999_999_999, vec!["Maximum source clause".to_string()]),
            (
                1,
                vec![
                    "First unlabelled continuation".to_string(),
                    "Second unlabelled continuation".to_string(),
                ],
            ),
        ],
        "{markdown:?}"
    );
}

#[test]
fn near_maximum_source_ordinal_keeps_a_valid_unlabelled_continuation_in_the_same_run() {
    let items = [
        (Some("999999998."), "Near maximum source clause"),
        (None, "Last representable continuation"),
    ];
    let doc = document(&items);
    let markdown = qualify(&doc, &items);
    assert_eq!(
        read_runs(&markdown),
        vec![(
            999_999_998,
            vec![
                "Near maximum source clause".to_string(),
                "Last representable continuation".to_string(),
            ],
        )],
        "{markdown:?}"
    );
}

#[test]
fn explicit_valid_source_reanchor_takes_precedence_over_the_default_restart() {
    let items = [
        (Some("999999999."), "Maximum anchored source clause"),
        (Some("7."), "Explicit seventh clause"),
        (None, "Unlabelled eighth continuation"),
    ];
    let doc = document(&items);
    let markdown = qualify(&doc, &items);
    assert_eq!(
        read_runs(&markdown),
        vec![
            (999_999_999, vec!["Maximum anchored source clause".to_string()]),
            (
                7,
                vec![
                    "Explicit seventh clause".to_string(),
                    "Unlabelled eighth continuation".to_string(),
                ],
            ),
        ],
        "{markdown:?}"
    );
}
