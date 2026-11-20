//! Vector geometry in the extraction result (`pdf_options.extract_vector_geometry`).
//!
//! The fixture pages are built here, not read from the corpus, so every coordinate and
//! paint attribute below is a literal of this test's own construction
//! (<https://github.com/xberg-io/xberg/issues/2966>).

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, PageConfig, PdfConfig};
use xberg::types::{PageContent, VectorLineCap, VectorLineJoin, VectorPoint, VectorSegment};

const PDF_MIME_TYPE: &str = "application/pdf";

/// A three-page Letter PDF whose pages hold known vector paths.
///
/// Page 1 draws, in this order: a dashed red rule, a filled blue rectangle, a black curve,
/// and the text `VALVE-12` on the same y as the rule. Page 2 draws one vertical green line.
/// Page 3 holds text only, so it has no geometry to report.
fn geometry_pdf() -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let first_page = vec![
        // Dashed red rule, round caps and bevel joins.
        Operation::new("w", vec![2.into()]),
        Operation::new("RG", vec![1.into(), 0.into(), 0.into()]),
        Operation::new("d", vec![Object::Array(vec![3.into(), 2.into()]), 1.into()]),
        Operation::new("J", vec![1.into()]),
        Operation::new("j", vec![2.into()]),
        Operation::new("m", vec![72.into(), 700.into()]),
        Operation::new("l", vec![300.into(), 700.into()]),
        Operation::new("S", vec![]),
        // Filled blue rectangle, no stroke.
        Operation::new("rg", vec![0.2.into(), 0.4.into(), 0.6.into()]),
        Operation::new("re", vec![100.into(), 200.into(), 150.into(), 50.into()]),
        Operation::new("f", vec![]),
        // Black cubic curve, stroked, back to a solid line.
        Operation::new("RG", vec![0.into(), 0.into(), 0.into()]),
        Operation::new("w", vec![0.5.into()]),
        Operation::new("d", vec![Object::Array(vec![]), 0.into()]),
        Operation::new("m", vec![400.into(), 500.into()]),
        Operation::new(
            "c",
            vec![420.into(), 520.into(), 460.into(), 520.into(), 480.into(), 500.into()],
        ),
        Operation::new("S", vec![]),
        // Text on the rule's own baseline.
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![72.into(), 700.into()]),
        Operation::new("Tj", vec![Object::string_literal("VALVE-12")]),
        Operation::new("ET", vec![]),
    ];
    let second_page = vec![
        Operation::new("RG", vec![0.into(), 1.into(), 0.into()]),
        Operation::new("w", vec![1.into()]),
        Operation::new("m", vec![50.into(), 100.into()]),
        Operation::new("l", vec![50.into(), 200.into()]),
        Operation::new("S", vec![]),
        // A unit rule drawn under a scale-2, translate-100 `cm`: reported at its final
        // on-page size, coordinates and stroke width both.
        Operation::new("q", vec![]),
        Operation::new(
            "cm",
            vec![2.into(), 0.into(), 0.into(), 2.into(), 100.into(), 100.into()],
        ),
        Operation::new("RG", vec![0.5.into(), 0.into(), 0.5.into()]),
        Operation::new("w", vec![1.into()]),
        Operation::new("m", vec![0.into(), 0.into()]),
        Operation::new("l", vec![10.into(), 0.into()]),
        Operation::new("S", vec![]),
        Operation::new("Q", vec![]),
    ];
    let third_page = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new("Td", vec![72.into(), 700.into()]),
        Operation::new("Tj", vec![Object::string_literal("REF-7")]),
        Operation::new("ET", vec![]),
    ];

    let mut kids: Vec<Object> = Vec::new();
    for operations in [first_page, second_page, third_page] {
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().expect("the page content encodes"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        kids.push(page_id.into());
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => 3,
            "Kids" => kids,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("the fixture serializes");
    bytes
}

fn config(extract_vector_geometry: bool) -> ExtractionConfig {
    ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_vector_geometry,
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    }
}

fn extract(config: &ExtractionConfig) -> ExtractedDocument {
    extract_bytes_document_blocking(&geometry_pdf(), PDF_MIME_TYPE, config).expect("the fixture extracts")
}

fn pages(config: &ExtractionConfig) -> Vec<PageContent> {
    extract(config).pages.expect("the fixture keeps pages")
}

fn point(x: f32, y: f32) -> VectorPoint {
    VectorPoint { x, y }
}

#[test]
fn geometry_reports_the_paths_it_drew_with_exact_page_coordinates() {
    let pages = pages(&config(true));
    let page = &pages[0];
    let geometry = page.geometry.as_ref().expect("page 1 carries geometry");
    assert_eq!(geometry.paths.len(), 3, "one path per painting operator");

    let rule = &geometry.paths[0];
    let [subpath] = rule.subpaths.as_slice() else {
        panic!("the rule is one subpath, got {:?}", rule.subpaths);
    };
    assert_eq!(subpath.start, point(72.0, 700.0));
    assert_eq!(
        subpath.segments,
        vec![VectorSegment::Line {
            to: point(300.0, 700.0)
        }]
    );
    assert!(!subpath.closed);
    assert_eq!((rule.bounding_box.x0, rule.bounding_box.y0), (72.0, 700.0));
    assert_eq!((rule.bounding_box.x1, rule.bounding_box.y1), (300.0, 700.0));
    assert!(rule.fill_color.is_none(), "the rule is stroked only");
    let stroke = rule.stroke.as_ref().expect("the rule is stroked");
    assert_eq!((stroke.color.r, stroke.color.g, stroke.color.b), (1.0, 0.0, 0.0));
    assert_eq!(stroke.width, 2.0);
    assert_eq!(stroke.line_cap, VectorLineCap::Round);
    assert_eq!(stroke.line_join, VectorLineJoin::Bevel);
    let dash = stroke.dash_pattern.as_ref().expect("the rule is dashed");
    assert_eq!(dash.dashes, vec![3.0, 2.0]);
    assert_eq!(dash.phase, 1.0);

    let rectangle = &geometry.paths[1];
    let [subpath] = rectangle.subpaths.as_slice() else {
        panic!("the rectangle is one subpath, got {:?}", rectangle.subpaths);
    };
    assert_eq!(subpath.start, point(100.0, 200.0));
    assert!(subpath.closed);
    assert_eq!(
        subpath.segments,
        vec![
            VectorSegment::Line {
                to: point(250.0, 200.0)
            },
            VectorSegment::Line {
                to: point(250.0, 250.0)
            },
            VectorSegment::Line {
                to: point(100.0, 250.0)
            },
        ],
        "the close paints the fourth side"
    );
    assert_eq!((rectangle.bounding_box.x0, rectangle.bounding_box.y0), (100.0, 200.0));
    assert_eq!((rectangle.bounding_box.x1, rectangle.bounding_box.y1), (250.0, 250.0));
    let fill = rectangle.fill_color.expect("the rectangle is filled");
    assert_eq!((fill.r, fill.g, fill.b), (0.2, 0.4, 0.6));
    assert!(rectangle.stroke.is_none(), "the rectangle is not stroked");

    let curve = &geometry.paths[2];
    let [subpath] = curve.subpaths.as_slice() else {
        panic!("the curve is one subpath, got {:?}", curve.subpaths);
    };
    assert_eq!(
        subpath.segments,
        vec![VectorSegment::Cubic {
            control1: point(420.0, 520.0),
            control2: point(460.0, 520.0),
            to: point(480.0, 500.0),
        }]
    );
    assert_eq!(
        (curve.bounding_box.x0, curve.bounding_box.y0),
        (400.0, 500.0),
        "the bounds include the control points, as the constructed geometry does"
    );
    assert_eq!((curve.bounding_box.x1, curve.bounding_box.y1), (480.0, 520.0));
    let stroke = curve.stroke.as_ref().expect("the curve is stroked");
    assert_eq!(stroke.width, 0.5);
    assert!(stroke.dash_pattern.is_none(), "the curve is solid");
}

#[test]
fn each_page_reports_its_own_paths() {
    let pages = pages(&config(true));

    let second = pages[1]
        .geometry
        .as_ref()
        .expect("page 2 carries geometry")
        .paths
        .as_slice();
    assert_eq!(second.len(), 2, "page 2 drew two paths");
    assert_eq!(second[0].subpaths[0].start, point(50.0, 100.0));
    assert_eq!(
        second[0].subpaths[0].segments,
        vec![VectorSegment::Line { to: point(50.0, 200.0) }],
        "page 2 must not report page 1's paths"
    );
}

#[test]
fn a_drawing_transform_is_applied_to_the_reported_geometry() {
    let pages = pages(&config(true));
    let transformed = &pages[1].geometry.as_ref().expect("page 2 carries geometry").paths[1];

    assert_eq!(
        transformed.subpaths[0].start,
        point(100.0, 100.0),
        "the translate moves the rule origin onto the page"
    );
    assert_eq!(
        transformed.subpaths[0].segments,
        vec![VectorSegment::Line {
            to: point(120.0, 100.0)
        }],
        "the scale-2 transform doubles the unit rule"
    );
    assert_eq!(
        transformed.stroke.as_ref().expect("the rule is stroked").width,
        2.0,
        "the 1 pt width is scaled like the geometry"
    );
}

#[test]
fn geometry_is_absent_unless_requested() {
    let default_pages = pages(&ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    });

    assert!(
        default_pages.iter().all(|page| page.geometry.is_none()),
        "the default result must carry no geometry"
    );
    let json = serde_json::to_value(&default_pages[0]).expect("the page serializes");
    assert!(
        !json.as_object().expect("a page is an object").contains_key("geometry"),
        "the default page must not serialize a geometry key: {json}"
    );
}

#[test]
fn asking_for_geometry_turns_on_the_page_tracking_it_is_returned_on() {
    // No `pages` block at all: the option alone must still produce the pages that carry it.
    let document = extract(&ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_vector_geometry: true,
            ..Default::default()
        }),
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    });

    let pages = document.pages.expect("asking for geometry asks for pages");
    assert_eq!(pages.len(), 3);
    assert!(pages[0].geometry.is_some() && pages[1].geometry.is_some());
    assert!(
        pages[2].geometry.is_none(),
        "a page that drew no path reports no geometry, not an empty one"
    );
}

#[test]
fn text_bounds_and_geometry_share_one_coordinate_origin() {
    let document = extract(&ExtractionConfig {
        include_document_structure: true,
        ..config(true)
    });

    let node = document
        .document
        .as_ref()
        .expect("the structure passes text bounds through")
        .nodes
        .iter()
        .find(|node| node.page == Some(1) && node.content.text().is_some_and(|text| text.contains("VALVE-12")))
        .expect("the fixture text is a structure node");
    let text_box = node.bbox.expect("the native PDF node carries text bounds");

    let page = document.pages.as_ref().expect("pages are tracked")[0]
        .geometry
        .as_ref()
        .expect("page 1 carries geometry");
    let rule = &page.paths[0];

    assert!(
        (rule.bounding_box.y0 - 700.0).abs() < 1e-3,
        "the rule is drawn on y = 700 in page space, got {}",
        rule.bounding_box.y0
    );
    assert!(
        (text_box.x0 - 72.0).abs() < 1.0,
        "the label starts where the rule starts in x, got {}",
        text_box.x0
    );
    // A top-left-origin frame would place this y = 700 rule at 792 - 700 = 92; both
    // coordinates landing near 700 is what proves the two outputs share one y-up origin.
    assert!(
        (text_box.y0 - 700.0).abs() < 1.0,
        "the label's baseline sits on the rule, got {}",
        text_box.y0
    );
    assert!(
        text_box.y0 < text_box.y1,
        "text bounds keep y increasing upwards, got {text_box:?}"
    );
}
