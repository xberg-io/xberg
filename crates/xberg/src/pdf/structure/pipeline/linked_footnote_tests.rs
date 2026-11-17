use super::super::tests::{linked_footnote_image_pdf, linked_footnote_reference, linked_footnote_text, seg_at};
use super::*;

#[test]
fn compact_note_refuses_semantic_roles_already_owned_elsewhere() {
    let cases: [(&str, fn(&mut PdfParagraph)); 7] = [
        ("page furniture", |note| note.is_page_furniture = true),
        ("heading", |note| note.heading_level = Some(1)),
        ("list", |note| note.is_list_item = true),
        ("code", |note| note.is_code_block = true),
        ("formula", |note| note.is_formula = true),
        ("caption ownership", |note| note.caption_for = Some(0)),
        ("other layout class", |note| {
            note.layout_class = Some(LayoutHintClass::Picture)
        }),
    ];
    assert!(compact_note(&linked_footnote_text()).is_some());
    for (label, change) in cases {
        let mut note = linked_footnote_text();
        change(&mut note);
        assert!(compact_note(&note).is_none(), "{label}");
    }
}

#[test]
fn compact_note_requires_numeric_upright_marker_and_single_note_row() {
    type Segments = Vec<crate::pdf::hierarchy::SegmentData>;
    let cases: [(&str, fn(&mut Segments)); 13] = [
        ("symbol accepted by the shared marker grammar", |segments| {
            segments[0].text = "*".into()
        }),
        ("rotated marker", |segments| segments[0].rotation_degrees = 90.0),
        ("rotated first note word", |segments| {
            segments[1].rotation_degrees = 90.0
        }),
        ("empty first note word", |segments| segments[1].text = " ".into()),
        ("negative marker gap", |segments| segments[1].x = 72.9),
        ("wide marker gap", |segments| segments[1].x = 77.1),
        ("flat marker", |segments| segments[0].baseline_y = 74.0),
        ("rise under one tenth em", |segments| segments[0].baseline_y = 74.7),
        ("rise above three quarters em", |segments| segments[0].baseline_y = 80.1),
        ("full-size marker", |segments| segments[0].font_size = 8.0),
        ("marker above 0.85 em", |segments| segments[0].font_size = 6.81),
        ("rotated continuation", |segments| segments[2].rotation_degrees = 90.0),
        ("continuation on a different row", |segments| {
            segments[2].baseline_y = 76.0
        }),
    ];
    let base = || {
        let mut note = linked_footnote_text();
        let mut continuation = seg_at("online", 165.0, 74.0, 8.0, false);
        continuation.width = 20.0;
        note.lines[0].segments.push(continuation);
        note
    };
    assert!(super::super::is_compact_footnote_marker("*"));
    assert!(compact_note(&base()).is_some());
    for (label, change) in cases {
        let mut note = base();
        change(&mut note.lines[0].segments);
        assert!(compact_note(&note).is_none(), "{label}");
    }
    for ratio in [0.8, 0.8125, 0.85] {
        let mut note = base();
        note.lines[0].segments[0].font_size = 8.0 * ratio;
        assert!(compact_note(&note).is_some(), "{ratio}");
    }
}

#[test]
fn note_must_fit_horizontally_and_vertically_below_rule() {
    let rule = Rect::new(70.0, 90.0, 144.0, 0.5);
    for (label, bounds, accepted) in [
        ("exact bounds", (70.0, 58.0, 214.0, 74.0), true),
        ("right edge", (70.0, 58.0, 214.25, 74.0), false),
        ("note height", (70.0, 57.75, 214.0, 74.0), false),
        ("inverted note height", (70.0, 74.25, 214.0, 74.0), false),
        ("rule gap", (70.0, 58.0, 214.0, 73.75), false),
        ("overlapping rule", (70.0, 80.0, 214.0, 90.25), false),
    ] {
        let note = Note {
            marker: "2",
            bounds,
            font_size: 8.0,
        };
        assert_eq!(note.below_rule(rule), accepted, "{label}");
    }
}

#[test]
fn references_require_prose_prefix_and_unambiguous_upright_source() {
    let hyphens = HyphenWitnesses::default();
    let cases: [(&str, fn(&mut PdfParagraph), bool); 4] = [
        ("ordinary prose", |_| {}, true),
        ("formula", |paragraph| paragraph.is_formula = true, false),
        (
            "punctuation prefix",
            |paragraph| paragraph.lines[0].segments[0].text.push('!'),
            false,
        ),
        (
            "same marker in a second reading frame",
            |paragraph| {
                let mut line = paragraph.lines[0].clone();
                let mut marker = line.segments.remove(1);
                marker.rotation_degrees = 90.0;
                line.segments = vec![marker];
                paragraph.lines.push(line);
            },
            false,
        ),
    ];
    for (label, change, accepted) in cases {
        let mut paragraph = linked_footnote_reference("2", true);
        change(&mut paragraph);
        assert!(finite_paragraph_bbox(&paragraph).unwrap().1 > 90.5, "{label}");
        let (text, annotations) = super::super::super::assembly::extract_text_and_annotations(&paragraph, &hyphens);
        assert_eq!(
            annotations
                .iter()
                .filter(|annotation| {
                    annotation.kind == AnnotationKind::Superscript
                        && text.get(annotation.start as usize..annotation.end as usize) == Some("2")
                })
                .count(),
            1,
            "{label}"
        );
        assert_eq!(
            exactly_one_reference_above_rule(&[paragraph], "2", 90.5, &hyphens),
            accepted,
            "{label}"
        );
    }
}

#[test]
fn linked_footnote_preserves_captions_and_detected_tables_on_the_current_page() {
    let overlap = BoundingBox {
        x0: 60.0,
        y0: 65.0,
        x1: 180.0,
        y1: 90.0,
    };
    let caption = LayoutHint {
        class_name: LayoutHintClass::Caption,
        confidence: 1.0,
        left: 60.0,
        bottom: 65.0,
        right: 180.0,
        top: 90.0,
    };
    for (table_page, has_caption, marked) in [(1, false, false), (2, false, true), (2, true, false)] {
        let tables = [Table {
            page_number: table_page,
            bounding_box: Some(overlap),
            ..Default::default()
        }];
        let hints = if has_caption {
            std::slice::from_ref(&caption)
        } else {
            &[]
        };
        let mut paragraphs = [linked_footnote_reference("2", true), linked_footnote_text()];
        classify(
            &mut paragraphs,
            false,
            || Some(Rect::new(70.0, 90.0, 144.0, 0.5)),
            || Some(vec![]),
            PageRegions {
                page_number: 1,
                tables: &tables,
                hints,
            },
            &HyphenWitnesses::default(),
        );
        assert_eq!(paragraphs[1].is_page_furniture, marked, "{table_page} {has_caption}");
    }
}

#[test]
fn separator_requires_one_rule_in_the_upright_bottom_margin() {
    use lopdf::Object;

    for (height, rotation, second_rule, expected_count, accepted) in [
        (90, 0, false, 1, true),
        (90, 90, false, 1, false),
        (119, 0, false, 1, false),
        (90, 0, true, 2, false),
    ] {
        let extra = if second_rule {
            "/Artifact << /Type /Pagination /Attached [/Bottom] >> BDC 70 100 144 0.5 re f EMC"
        } else {
            ""
        };
        let mut pdf = lopdf::Document::load_mem(&linked_footnote_image_pdf(extra)).unwrap();
        let page = pdf.get_pages()[&1];
        if rotation != 0 {
            pdf.get_object_mut(page)
                .unwrap()
                .as_dict_mut()
                .unwrap()
                .set("Rotate", rotation);
        }
        let content_id = pdf
            .get_dictionary(page)
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_reference()
            .unwrap();
        let content = pdf.get_object_mut(content_id).and_then(Object::as_stream_mut).unwrap();
        let paint = String::from_utf8(content.content.clone()).unwrap();
        assert_eq!(paint.matches("70 90 144").count(), 1);
        content.set_plain_content(paint.replace("70 90 144", &format!("70 {height} 144")).into_bytes());
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        assert_eq!(
            document.bottom_pagination_separator_bounds(0).unwrap().len(),
            expected_count
        );
        assert_eq!(
            page_separator(&document, 0).is_some(),
            accepted,
            "{height} {rotation} {second_rule}"
        );
    }
}
