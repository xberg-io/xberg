use super::*;
use crate::pdf::structure::{SegmentStructureConfig, extract_document_structure_from_segments};
use crate::types::document_structure::AnnotationKind;
use crate::types::internal::{ElementKind, InternalDocument};
use sha2::{Digest, Sha256};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

fn source_spans() -> Vec<TextSpan> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents/pdf/regression_pages/nist-fips-197-upd1-p16.pdf");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory inline-dot source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "ddbb5a63987345c809b99487f583bc7eca2e9883131b38cc8e5fea59763ef33d"
    );
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    doc.extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
        .unwrap()
        .spans
}

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        ..Default::default()
    }
}

fn position(spans: &[TextSpan], text: &str) -> usize {
    let matches = spans
        .iter()
        .enumerate()
        .filter(|(_, span)| span.text == text)
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "unique source span {text:?}");
    matches[0].0
}

fn document(mut spans: Vec<TextSpan>) -> InternalDocument {
    reorder_page_reading_order(&mut spans, 612.0, 792.0, 0);
    let segments = rejoin_inline_scripts_inner(spans, ScriptGeometry::Preserve)
        .into_iter()
        .map(|span| SegmentData {
            text: span.text,
            x: span.bbox.x,
            y: span.bbox.y,
            width: span.bbox.width,
            height: span.bbox.height,
            font_size: span.font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: span.bbox.y,
            rotation_degrees: 0.0,
            assigned_role: None,
        })
        .collect();
    extract_document_structure_from_segments(
        vec![segments],
        SegmentStructureConfig {
            k_clusters: 6,
            tables: &[],
            outline_entries: &[],
            strip_repeating_text: false,
            include_headers: true,
            include_footers: true,
            include_footnotes: true,
            include_watermarks: false,
            used_structure_tree: false,
            image_positions: &[],
            images: None,
            inject_placeholders: false,
            layout_hints: None,
            allow_single_column: true,
            cancel_token: None,
            #[cfg(feature = "layout-detection")]
            layout_images: None,
            #[cfg(feature = "layout-detection")]
            layout_results: None,
            #[cfg(feature = "layout-detection")]
            table_model: crate::core::config::layout::TableModel::Disabled,
            #[cfg(feature = "layout-detection")]
            table_overlap_preference: Default::default(),
            #[cfg(feature = "layout-detection")]
            acceleration: None,
            #[cfg(feature = "layout-detection")]
            session_thread_budget: 1,
        },
    )
    .unwrap()
}

fn assert_no_scripted_dot(document: &InternalDocument) {
    let mut dot_count = 0;
    for element in &document.elements {
        for (offset, _) in element.text.match_indices(['•', '·']) {
            dot_count += 1;
            for annotation in &element.annotations {
                assert!(
                    !matches!(annotation.kind, AnnotationKind::Superscript | AnnotationKind::Subscript)
                        || annotation.end as usize <= offset
                        || annotation.start as usize > offset,
                    "ordinary dot at byte {offset} received a script annotation: {element:?}"
                );
            }
        }
    }
    assert!(
        dot_count > 0,
        "the ordinary-dot annotation check must examine an actual glyph"
    );
}

#[test]
fn source_inline_dot_keeps_its_native_neighbors_through_hierarchy_ordering() {
    let mut spans = source_spans();
    assert_eq!(spans.len(), 425);
    assert_eq!(position(&spans, "The symbol "), 382);
    assert_eq!(position(&spans, "•"), 383);
    assert_eq!(position(&spans, "denotes multiplication in GF 2"), 385);
    assert_eq!((spans[382].bbox.x, spans[382].bbox.y), (77.029, 184.49));
    assert_eq!((spans[383].bbox.x, spans[383].bbox.y), (139.619, 185.52));
    assert_eq!(spans[383].font_size, 8.9664);
    assert_eq!(spans[385].bbox.x, 147.96);
    reorder_page_reading_order(&mut spans, 612.0, 792.0, 0);
    let left = position(&spans, "The symbol ");
    let dot = position(&spans, "•");
    let right = position(&spans, "denotes multiplication in GF 2");
    assert!(
        left < dot && dot < right,
        "inline source order became left={left}, dot={dot}, right={right}"
    );
}

#[test]
fn measured_inline_dot_stays_ordinary_inside_its_prose() {
    let doc = document(vec![
        span("The symbol ", 77.029, 184.49, 61.77632, 11.9552),
        span("•", 139.619, 185.52, 4.4832, 8.9664),
        span("denotes multiplication", 147.96, 184.49, 115.0, 11.9552),
    ]);
    assert!(
        doc.elements.iter().any(|element| element
            .text
            .replace('•', "·")
            .contains("The symbol · denotes multiplication")
            && !matches!(element.kind, ElementKind::ListItem { .. })),
        "actual measured inline elements: {:?}",
        doc.elements
    );
    assert_no_scripted_dot(&doc);
}

#[test]
fn a_leading_bullet_on_its_body_baseline_remains_a_real_list_item() {
    // A true split list marker shares the body baseline, unlike the raised math dot. ~keep
    let doc = document(vec![
        span("•", 139.619, 184.49, 4.4832, 8.9664),
        span("Genuine list content.", 147.96, 184.49, 115.0, 11.9552),
    ]);
    assert!(
        doc.elements.iter().any(
            |element| matches!(element.kind, ElementKind::ListItem { ordered: false })
                && element.text.contains("Genuine list content.")
        ),
        "actual leading bullet elements: {:?}",
        doc.elements
    );
}

#[test]
fn a_marker_on_a_separate_row_is_not_folded_into_the_prose() {
    let doc = document(vec![
        span("The symbol ", 77.029, 214.49, 61.77632, 11.9552),
        span("•", 139.619, 185.52, 4.4832, 8.9664),
        span("denotes multiplication", 147.96, 154.49, 115.0, 11.9552),
    ]);
    assert!(
        !doc.elements
            .iter()
            .any(|element| element.text.contains("The symbol • denotes multiplication"))
    );
    assert!(doc.elements.iter().any(|element| element.text.contains("The symbol")));
    assert!(
        doc.elements
            .iter()
            .any(|element| element.text.contains("denotes multiplication"))
    );
}

#[test]
fn a_bullet_in_another_column_does_not_gain_a_left_prose_owner() {
    let doc = document(vec![
        span("Independent left paragraph.", 77.029, 184.49, 160.0, 11.9552),
        span("•", 400.0, 184.49, 4.4832, 8.9664),
        span("Right column list content.", 408.341, 184.49, 125.0, 11.9552),
    ]);
    assert!(
        doc.elements.iter().any(
            |element| matches!(element.kind, ElementKind::ListItem { ordered: false })
                && element.text.contains("Right column list content.")
        ),
        "actual separated-column elements: {:?}",
        doc.elements
    );
    assert!(
        !doc.elements
            .iter()
            .any(|element| element.text.contains("Independent left paragraph.")
                && element.text.contains("Right column list content."))
    );
}

#[test]
fn measured_real_footnote_marker_and_body_remain_together_without_a_prose_owner() {
    let doc = document(vec![
        span("Independent body paragraph.", 77.029, 184.49, 170.0, 11.9552),
        span("1", 77.4, 76.048, 3.68615, 7.3723),
        span(
            "Information about finite fields is in Algebra [4].",
            81.584,
            72.432,
            350.0,
            9.9626,
        ),
    ]);
    assert!(doc.elements.iter().any(|element| element.text.contains("Algebra [4].")));
    assert!(
        !doc.elements.iter().any(
            |element| element.text.contains("Independent body paragraph.") && element.text.contains("Algebra [4].")
        )
    );
    let text = doc
        .elements
        .iter()
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(text.chars().filter(|character| *character == '1').count(), 1);
}
