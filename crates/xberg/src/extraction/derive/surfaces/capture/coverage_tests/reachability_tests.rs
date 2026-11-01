use super::*;
use crate::types::document_structure::{AnnotationKind, ContentLayer};

fn canonical_options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.math_dollars = true;
    options.extension.math_code = true;
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options
}

fn rule() -> RedactionConfig {
    RedactionConfig {
        categories: [PiiCategory::Custom("person".into())].into_iter().collect(),
        ..config("Jane Doe", RedactionStrategy::Mask)
    }
}

async fn process_literal(mut result: DerivedResult, count: usize, plain: &str, rendered: &str) {
    let config = rule();
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(findings(markdown, &config).await.len(), count);
    assert_eq!(findings(&visible(markdown), &config).await.len(), count);
    assert_eq!(findings(&result.content, &config).await.len(), count);
    apply(&mut result, config)
        .await
        .expect("real canonical nodes must remain processable");
    let result = finish(result);
    let expected_count = match result.metadata.output_format.as_deref() {
        Some("markdown") => {
            assert_eq!(visible(&result.content), rendered);
            count * 2
        }
        Some("plain") => {
            assert_plain(&result, plain);
            count
        }
        other => panic!("unexpected output format: {other:?}"),
    };
    assert_eq!(
        result.redaction_report.as_ref().unwrap().total_redacted as usize,
        expected_count
    );
    assert!(!result.content.contains("Jane Doe"));
}

fn inline_math(literal: &str) -> DerivedResult {
    let source = format!("Let ${literal}$ hold.");
    let annotation = TextAnnotation {
        start: 4,
        end: u32::try_from(6 + literal.len()).unwrap(),
        kind: AnnotationKind::math(false),
    };
    let result = paragraph(&source, vec![annotation]);
    let markdown = result.formatted_content.as_ref().unwrap();
    assert_eq!(markdown.matches(&format!("$`{literal}`$")).count(), 1);
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &canonical_options());
    let formulas = root
        .descendants()
        .filter_map(|node| {
            if let comrak::nodes::NodeValue::Math(math) = &node.data.borrow().value {
                Some((math.dollar_math, math.display_math, math.literal.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(formulas, [(false, false, literal.to_owned())]);
    assert_eq!(visible(markdown), format!("Let {literal} hold."));
    result
}

#[tokio::test]
async fn canonical_safe_inline_math_remains_processable_with_its_visible_literal() {
    process_literal(inline_math("x+y"), 0, "Let $x+y$ hold.", "Let x+y hold.").await;
}

#[tokio::test]
async fn canonical_sensitive_inline_math_is_completely_redacted() {
    process_literal(
        inline_math("Jane Doe"),
        1,
        "Let $[REDACTED]$ hold.",
        "Let [REDACTED] hold.",
    )
    .await;
}

fn footnote(text: &str) -> DerivedResult {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_paragraph("See note", vec![], None, None);
    builder.push_footnote_ref("1", "note", None);
    builder.push_footnote_definition(text, "note", None);
    let document = builder.build();
    assert_eq!(document.elements.len(), 3);
    assert_eq!(
        document.elements[1].kind,
        crate::types::internal::ElementKind::FootnoteRef
    );
    assert_eq!(document.elements[2].layer, ContentLayer::Footnote);
    let result = prepare(document);
    assert_eq!(result.content, format!("See note[^1]\n\n\n[^1]: {text}"));
    let markdown = result.formatted_content.as_ref().unwrap();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &canonical_options());
    let references = root
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, comrak::nodes::NodeValue::FootnoteReference(_)))
        .count();
    let definitions = root
        .descendants()
        .filter(|node| {
            matches!(
                node.data.borrow().value,
                comrak::nodes::NodeValue::FootnoteDefinition(_)
            )
        })
        .count();
    assert_eq!((references, definitions), (1, 1));
    assert_eq!(visible(markdown), format!("See note11 {text}"));
    result
}

#[tokio::test]
async fn canonical_safe_footnote_keeps_reference_definition_and_visible_text() {
    process_literal(
        footnote("safe note"),
        0,
        "See note[^1]\n\n\n[^1]: safe note",
        "See note11 safe note",
    )
    .await;
}

#[tokio::test]
async fn canonical_sensitive_footnote_definition_is_completely_redacted() {
    process_literal(
        footnote("Jane Doe"),
        1,
        "See note[^1]\n\n\n[^1]: [REDACTED]",
        "See note11 [REDACTED]",
    )
    .await;
}

fn alert(title: Option<&str>) -> DerivedResult {
    let body = "safe body";
    let mut builder = InternalDocumentBuilder::new("test");
    let index = builder.push_admonition("note", title, None);
    builder.set_text(index, body);
    let document = builder.build();
    assert_eq!(document.elements.len(), 1);
    assert_eq!(
        document.elements[0].kind,
        crate::types::internal::ElementKind::Admonition
    );
    assert_eq!(document.elements[0].text, body);
    let result = prepare(document);
    assert_eq!(result.content, format!("{}\n\n{body}", title.unwrap_or("note")));
    let markdown = result.formatted_content.as_ref().unwrap();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &canonical_options());
    let alerts = root
        .descendants()
        .filter_map(|node| {
            if let comrak::nodes::NodeValue::Alert(alert) = &node.data.borrow().value {
                Some((
                    alert.alert_type.default_title().to_owned(),
                    alert.title.clone(),
                    alert.multiline,
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(alerts, [("Note".into(), title.map(str::to_owned), false)]);
    assert_eq!(visible(markdown), format!("{}{body}", title.unwrap_or("Note")));
    result
}

#[tokio::test]
async fn canonical_safe_default_alert_keeps_its_title_and_body() {
    process_literal(alert(None), 0, "note\n\nsafe body", "Notesafe body").await;
}

#[tokio::test]
async fn canonical_sensitive_alert_title_is_completely_redacted() {
    process_literal(
        alert(Some("Jane Doe")),
        1,
        "[REDACTED]\n\nsafe body",
        "[REDACTED]safe body",
    )
    .await;
}
