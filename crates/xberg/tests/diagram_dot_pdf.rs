//! End-to-end diagram recovery from vector PDF (#579).
//!
//! The diagram is built here rather than read from the corpus. A vector PDF
//! fixture would have to be a binary, and building the drawing in the test
//! makes the input readable: the operators below are the whole diagram, so a
//! failure points at a coordinate someone can see rather than at bytes.
//!
//! Everything runs through the public extraction API with `output_format="dot"`,
//! so this covers the hook into the PDF extractor as well as the recogniser.

#![cfg(all(feature = "pdf", feature = "xml"))]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ExtractionConfig, OutputFormat};

/// Two stroked boxes joined by a line that ends in a filled arrowhead, with a
/// label centred in each box.
///
/// Coordinates are PDF user space, so the origin is the bottom-left corner and
/// `Alpha` at y=150 is the *upper* box. Recovery has to flip that; the
/// assertions below are written in reading order and would fail if it did not.
fn two_boxes_and_an_arrow() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    // Boxes are stroked with `S` and the arrowhead filled with `f`. `B`
    // (fill-and-stroke) is deliberately avoided: xberg_native_pdf 0.3.77 does not
    // finalize a path on `B`, so its geometry leaks into the next painted path
    // and the fixture would be testing that bug rather than this recogniser.
    let content = b"1 w 0 0 0 RG
10 150 80 30 re S
10 20 80 30 re S
50 150 m 50 56 l S
46 56 m 54 56 l 50 50 l h f
BT /F1 12 Tf 30 160 Td (Alpha) Tj ET
BT /F1 12 Tf 30 30 Td (Beta) Tj ET
"
    .to_vec();

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let content_id = document.add_object(Stream::new(dictionary! {}, content));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must save");
    bytes
}

/// Build a one-page PDF whose content stream is exactly `content`, parameterised
/// by canvas size so each fixture below states only the geometry that matters.
fn single_page_pdf(content: &[u8], width: i64, height: i64) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let content_id = document.add_object(Stream::new(dictionary! {}, content.to_vec()));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must save");
    bytes
}

fn extract_dot(bytes: &[u8]) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Custom("dot".to_string()),
        ..Default::default()
    };
    extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("extraction must succeed")
        .content
}

#[test]
fn recovers_a_graph_from_vector_pdf() {
    let dot = extract_dot(&two_boxes_and_an_arrow());

    assert!(dot.contains("digraph"), "expected a graph, got:\n{dot}");
    assert!(dot.contains(r#"label="Alpha""#), "missing Alpha:\n{dot}");
    assert!(dot.contains(r#"label="Beta""#), "missing Beta:\n{dot}");
    assert!(dot.contains("shape=box"), "boxes must be named as boxes:\n{dot}");

    // Reading order decides the ids, so the upper box is n0. An unflipped
    // y-axis would number them the other way round and reverse this edge.
    assert!(dot.contains("n0 -> n1"), "expected one edge Alpha to Beta:\n{dot}");
    assert_eq!(dot.matches("->").count(), 1, "expected exactly one edge:\n{dot}");
}

/// A PDF of prose draws no graph, and reporting one would be a false positive
/// on the most common input the extractor sees.
#[test]
fn a_text_only_pdf_recovers_nothing() {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 20 100 Td (Just a paragraph of ordinary text.) Tj ET\n".to_vec(),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must save");

    assert!(extract_dot(&bytes).trim().is_empty(), "prose is not a diagram");
}

/// A UML class box or a BPMN task carries its own label alongside the shapes
/// it draws inside itself (compartments, markers). The container test used to
/// run before labels were resolved, so it saw only the enclosure and dropped
/// the box outright, taking its edge down with it. The box must survive
/// because it is labelled, whatever it encloses.
#[test]
fn a_labelled_box_enclosing_two_shapes_is_not_dropped_as_a_container() {
    let content = b"1 w 0 0 0 RG
10 10 180 180 re S
20 130 40 40 re S
120 30 40 40 re S
190 100 m 220 100 l S
220 80 60 40 re S
BT /F1 12 Tf 90 100 Td (Container) Tj ET
BT /F1 12 Tf 230 95 Td (Target) Tj ET
"
    .to_vec();
    let dot = extract_dot(&single_page_pdf(&content, 300, 300));

    assert!(
        dot.contains(r#"label="Container""#),
        "the labelled enclosure must survive as a node:\n{dot}"
    );
    assert!(dot.contains(r#"label="Target""#), "missing Target:\n{dot}");
    assert!(
        dot.contains("n0 -> n1"),
        "the container's edge must not be lost with it:\n{dot}"
    );
    assert_eq!(dot.matches("->").count(), 1, "expected exactly one edge:\n{dot}");
}

/// A node's own incoming arrowheads are drawn just inside its border and
/// satisfy `Rect::encloses` exactly as a real member would. The container
/// test used to count them anyway, so a node with two incoming edges was
/// indistinguishable from a two-member container and both edges vanished
/// when it was dropped. Arrowheads must be excluded from the count.
#[test]
fn a_node_with_two_arrowheads_inside_its_border_is_not_dropped_as_a_container() {
    let content = b"1 w 0 0 0 RG
10 150 40 30 re S
10 20 40 30 re S
100 100 80 80 re S
50 165 m 100 165 l S
50 50 m 100 110 l S
100 161 m 110 165 l 100 169 l h f
100 106 m 110 110 l 100 114 l h f
BT /F1 12 Tf 20 160 Td (A) Tj ET
BT /F1 12 Tf 20 30 Td (B) Tj ET
BT /F1 12 Tf 130 135 Td (Hub) Tj ET
"
    .to_vec();
    let dot = extract_dot(&single_page_pdf(&content, 220, 220));

    assert!(dot.contains(r#"label="A""#), "missing A:\n{dot}");
    assert!(dot.contains(r#"label="B""#), "missing B:\n{dot}");
    assert!(
        dot.contains(r#"label="Hub""#),
        "the node enclosing its own arrowheads must survive:\n{dot}"
    );
    assert!(dot.contains("n0 -> n1"), "expected A -> Hub:\n{dot}");
    assert!(dot.contains("n2 -> n1"), "expected B -> Hub:\n{dot}");
    assert_eq!(dot.matches("->").count(), 2, "both incoming edges must survive:\n{dot}");
}

/// Four evenly spaced horizontal gridlines of one colour span the plot between two bars. Two are
/// dashed and two solid, so a family keyed on dash style splits into pairs below the minimum of
/// three and the gridlines surface as edges between the bars.
#[test]
fn mixed_dashed_and_solid_gridlines_between_two_bars_are_not_edges() {
    let content = b"1 w 0 0 0 RG
10 20 40 160 re S
150 20 40 160 re S
[4 2] 0 d 50 40 m 150 40 l S
[] 0 d 50 80 m 150 80 l S
[4 2] 0 d 50 120 m 150 120 l S
[] 0 d 50 160 m 150 160 l S
BT /F1 12 Tf 20 100 Td (A) Tj ET
BT /F1 12 Tf 160 100 Td (B) Tj ET
"
    .to_vec();
    let dot = extract_dot(&single_page_pdf(&content, 200, 200));

    assert_eq!(dot.matches("->").count(), 0, "gridlines must not become edges:\n{dot}");
}

/// A single dashed connector between two boxes is a real edge and keeps its dashed style.
#[test]
fn a_dashed_connector_between_two_boxes_is_a_dashed_edge() {
    let content = b"1 w 0 0 0 RG
10 150 80 30 re S
10 20 80 30 re S
[4 2] 0 d 50 150 m 50 50 l S
BT /F1 12 Tf 30 160 Td (Alpha) Tj ET
BT /F1 12 Tf 30 30 Td (Beta) Tj ET
"
    .to_vec();
    let dot = extract_dot(&single_page_pdf(&content, 200, 200));

    assert_eq!(dot.matches("->").count(), 1, "expected one edge:\n{dot}");
    assert!(dot.contains("style=dashed"), "the edge must stay dashed:\n{dot}");
}

/// Every renderer but `dot` discards the recovered graph, so recovery must be
/// requested through the output format rather than run unconditionally. This
/// checks the contract from outside the crate: the same diagram that produces
/// a graph under `dot` must not leak one into another renderer's output.
/// (Proving the underlying recovery pass itself does not *run* needs
/// `InternalDocument::diagrams`, which is private to the crate; see
/// `recovery_is_skipped_unless_dot_output_is_requested` in
/// `extractors/pdf/mod.rs`.)
#[test]
fn recovered_graph_never_surfaces_outside_dot_output() {
    let bytes = two_boxes_and_an_arrow();
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        ..Default::default()
    };
    let plain = extract_bytes_document_blocking(&bytes, "application/pdf", &config)
        .expect("extraction must succeed")
        .content;

    assert!(
        !plain.contains("digraph"),
        "plain output must not contain the recovered graph:\n{plain}"
    );
    assert!(
        plain.contains("Alpha"),
        "the page's own text must still extract:\n{plain}"
    );
}
