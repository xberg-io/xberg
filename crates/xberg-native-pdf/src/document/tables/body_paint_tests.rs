use super::*;
use crate::structure::spatial_table_detector::TableDetectionConfig;

fn fixture(extra: &str, split_row: bool) -> crate::PdfDocument {
    let mut content = String::from("0 80 m 200 80 l S\n");
    for (index, (low, high)) in [(60, 80), (40, 60), (20, 40)].into_iter().enumerate() {
        content.push_str(&format!(
            "BT /F1 8 Tf 10 {low} Td (Stock {}) Tj 50 0 Td (20.2) Tj ",
            index + 1
        ));
        if split_row && index == 0 {
            content.push_str("ET BT /F1 8 Tf 150 60 Td (30.3) Tj");
        } else {
            content.push_str("90 0 Td (30.3) Tj");
        }
        content.push_str(" ET\n");
        for x in [50, 120] {
            content.push_str(&format!("{x} {low} m {x} {high} l S\n"));
        }
    }
    content.push_str("0 16 m 200 16 l S 50 16 m 50 20 l S 120 16 m 120 20 l S\n");
    content.push_str(extra);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
    ];
    let mut pdf = "%PDF-1.4\n".to_string();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
    }
    let xref = pdf.len();
    pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    crate::PdfDocument::from_bytes(pdf.into_bytes()).unwrap()
}

#[test]
#[ignore = "requires the local archived Redfish PDF"]
fn archived_body_paint_witness() {
    let path = std::env::var_os("XBERG_BODY_PAINT_WITNESS").expect("path to the archived Redfish PDF");
    let document = crate::PdfDocument::from_bytes(std::fs::read(path).unwrap()).unwrap();
    let config = TableDetectionConfig::strict();
    let table = recover(&document, 0, &config).expect("seven source-owned records");
    assert_eq!(table.rows.len(), 7);
    assert_eq!(table.col_count, 6);
    assert!(table.rows[2].cells[0].text.contains("Redfish Bay/Espiritu Santo Bay"));
    assert_eq!(table.rows[2].cells[5].text, "14.3 3 (13.6)");

    let [ordinary] = document.extract_tables_with_configs(0, [&config]).unwrap();
    assert!(
        !ordinary.is_empty(),
        "the archived page also has an ordinary ruled candidate"
    );
    let [(_, _), (public, _)] = document
        .extract_tables_with_wide_source_recovery(0, [&config, &config])
        .unwrap();
    assert_eq!(ordinary.len(), 1);
    assert_eq!(ordinary[0].col_count, 4);
    assert_eq!(ordinary[0].rows.len(), 2, "one label row and one combined body band");
    assert_eq!(ordinary[0].rows[1].cells[0].text.lines().count(), table.rows.len());
    assert!(associate_body_rows(&ordinary[0], &table).is_none());
    assert_eq!(public.len(), 1);
    assert_eq!(public[0].col_count, 6);
    assert_eq!(public[0].rows.len(), 8);
    assert_eq!(public[0].bbox, ordinary[0].bbox);
    assert_eq!(public[0].rows[0].cells[0].text, "Stock");
    assert_eq!(public[0].rows[0].cells[1].text, "Annual M/SI (specified activity)");
    assert_eq!(
        public[0].rows[0].cells[3].text,
        "Estimated annual M/SI (other activities)"
    );
    assert_eq!(public[0].rows[0].cells[5].text, "Take/r-PBR (%)");
    assert_eq!(public[0].rows[3].cells[5].text, "14.3 3 (13.6)");
}

fn control_text(runs: &[(&str, f32, f32)]) -> Paint {
    Paint::Text(paint_glyphs(
        runs.iter()
            .flat_map(|&(text, x, y)| {
                text.chars().enumerate().map(move |(i, ch)| {
                    TextChar::simple(ch, Rect::new(x + i as f32 * 3., y, 3., 8.), "Helvetica".into(), 8.)
                })
            })
            .collect(),
        &[],
    ))
}

fn ruled_control() -> (Vec<Paint>, Table) {
    let doc = fixture("", false);
    let mut events = page_paint(&doc, 0).unwrap();
    let index = events.iter().position(|event| matches!(event, Paint::Text(_))).unwrap();
    for event in &mut events {
        match event {
            Paint::Rule(rule) if rule.start.x == 120. && rule.end.x == 120. => {
                rule.start.x = 100.;
                rule.end.x = 100.;
            }
            _ => {}
        }
    }
    for event in &mut events {
        if let Paint::Text(chars) = event {
            let y = chars[0].origin_y;
            let Paint::Text(addition) = control_text(&[("40.4", 110., y)]) else {
                unreachable!()
            };
            chars.extend(addition);
        }
    }
    let rule = |x, y, end_x, end_y| {
        Paint::Rule(Rule {
            start: Point::new(x, y),
            end: Point::new(end_x, end_y),
        })
    };
    events.insert(
        index - 1,
        control_text(&[
            ("Stock", 10., 91.),
            ("Activity", 55., 91.),
            ("(annual)", 55., 84.),
            ("Other", 108., 91.),
            ("Take", 150., 91.),
            ("(%)", 150., 84.),
        ]),
    );
    events.insert(index, rule(0., 100., 200., 100.));
    for (offset, x) in [50., 100., 130.].into_iter().enumerate() {
        events.insert(index + 2 + offset, rule(x, 80., x, 100.));
    }
    // Each body record has one divider after each of its first three cells.
    for y in [60., 40., 20., 16.] {
        let index = events
            .iter()
            .position(|event| {
                matches!(event, Paint::Rule(r)
            if r.start.x == 100. && r.start.y == y)
            })
            .unwrap();
        let top = if y == 16. { 20. } else { y + 20. };
        events.insert(index + 1, rule(130., y, 130., top));
    }
    let mut header = TableRow::new(false);
    let mut body = TableRow::new(false);
    for text in ["Activity (annual)", "Other"] {
        header.cells.push(TableCell::new(text.into(), false));
    }
    for text in ["20.2\n20.2\n20.2", "40.4\n40.4\n40.4"] {
        body.cells.push(TableCell::new(text.into(), false));
    }
    (
        events,
        Table {
            rows: vec![header, body],
            has_header: false,
            col_count: 2,
            bbox: Some(Rect::new(0., 16., 200., 84.)),
        },
    )
}

#[test]
fn collapsed_replacement_rejects_same_object_body_and_header_callouts() {
    let config = TableDetectionConfig::default();
    for (y, text) in [(60., "(2)"), (91., "(1)")] {
        let (mut events, ordinary) = ruled_control();
        assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
        let chars = events
            .iter_mut()
            .find_map(|event| match event {
                Paint::Text(chars) if chars[0].origin_y == y => Some(chars),
                _ => None,
            })
            .unwrap();
        let Paint::Text(note) = control_text(&[(text, 180., y)]) else {
            unreachable!()
        };
        chars.extend(note);
        assert!(
            replacement_from_paint(&events, &config, &ordinary).is_none(),
            "same-object {text}"
        );
    }
}

#[test]
fn raised_marker_does_not_hide_a_disconnected_same_baseline_callout() {
    let (mut events, ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    let chars = events
        .iter_mut()
        .find_map(|event| match event {
            Paint::Text(chars) if chars[0].origin_y == 60. => Some(chars),
            _ => None,
        })
        .unwrap();
    let Paint::Text(marker) = control_text(&[("3", 168., 62.)]) else {
        unreachable!()
    };
    chars.extend(marker);
    assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
    let chars = events
        .iter_mut()
        .find_map(|event| match event {
            Paint::Text(chars) if chars[0].origin_y == 60. => Some(chars),
            _ => None,
        })
        .unwrap();
    let Paint::Text(note) = control_text(&[("(2)", 190., 60.)]) else {
        unreachable!()
    };
    chars.extend(note);
    assert!(replacement_from_paint(&events, &config, &ordinary).is_none());
}

#[test]
fn distant_shifted_callout_in_the_same_text_object_is_not_owned() {
    let (mut events, ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
    let chars = events
        .iter_mut()
        .find_map(|event| match event {
            Paint::Text(chars) if chars[0].origin_y == 60. => Some(chars),
            _ => None,
        })
        .unwrap();
    let Paint::Text(note) = control_text(&[("(2)", 186., 62.)]) else {
        unreachable!()
    };
    chars.extend(note);
    assert!(replacement_from_paint(&events, &config, &ordinary).is_none());
}

#[test]
fn cross_baseline_callout_cannot_attach_through_a_raised_marker() {
    assert_callout_cannot_attach_through_marker(64.);
}

#[test]
fn same_baseline_callout_cannot_attach_through_a_raised_marker() {
    assert_callout_cannot_attach_through_marker(62.);
}

fn assert_callout_cannot_attach_through_marker(note_y: f32) {
    let (mut events, ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
    let chars = events
        .iter_mut()
        .find_map(|event| match event {
            Paint::Text(chars) if chars[0].origin_y == 60. => Some(chars),
            _ => None,
        })
        .unwrap();
    let Paint::Text(marker) = control_text(&[("3", 168., 62.)]) else {
        unreachable!()
    };
    chars.extend(marker);
    assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
    let chars = events
        .iter_mut()
        .find_map(|event| match event {
            Paint::Text(chars) if chars[0].origin_y == 60. => Some(chars),
            _ => None,
        })
        .unwrap();
    let Paint::Text(note) = control_text(&[("(2)", 180., note_y)]) else {
        unreachable!()
    };
    chars.extend(note);
    assert!(replacement_from_paint(&events, &config, &ordinary).is_none());
}

#[test]
fn empty_ordinary_routes_do_not_publish_a_headerless_body() {
    use crate::structure::spatial_table_detector::TableStrategy;
    let document = fixture("", false);
    let config = TableDetectionConfig {
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        text_fallback: false,
        ..Default::default()
    };
    assert!(recover(&document, 0, &config).is_some());
    let ordinary = document.extract_tables_with_configs(0, [&config, &config]).unwrap();
    assert!(ordinary.iter().all(Vec::is_empty));
    let recovered = document
        .extract_tables_with_wide_source_recovery(0, [&config, &config])
        .unwrap();
    assert!(
        recovered
            .iter()
            .all(|(tables, ownership)| tables.is_empty() && ownership.is_none())
    );
}

#[test]
fn collapsed_replacement_rejects_vertical_displacement_and_permuted_values() {
    let (events, mut ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    ordinary.rows[1].cells[0].text = "20.2\n21.2\n20.2".into();
    let mut paints = events;
    let Paint::Text(chars) = paints
        .iter_mut()
        .find(|event| matches!(event, Paint::Text(chars) if chars[0].origin_y == 40.))
        .unwrap()
    else {
        unreachable!()
    };
    let digit = chars
        .iter_mut()
        .find(|ch| ch.char == '0' && (60. ..100.).contains(&ch.origin_x))
        .unwrap();
    digit.glyph.char = '1';
    assert!(replacement_from_paint(&paints, &config, &ordinary).is_some());
    let original = ordinary.bbox.unwrap();
    ordinary.bbox = Some(Rect::new(
        original.x,
        original.y + 100.,
        original.width,
        original.height,
    ));
    let displaced = replacement_from_paint(&paints, &config, &ordinary);
    ordinary.bbox = Some(original);
    ordinary.rows[1].cells[0].text = "21.2\n20.2\n20.2".into();
    assert!(
        replacement_from_paint(&paints, &config, &ordinary).is_none(),
        "permuted values"
    );
    assert!(displaced.is_none(), "displaced bounds");
}

#[test]
fn collapsed_replacement_abstains_on_independently_painted_numeric_callouts() {
    let (mut events, ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    let replacement = replacement_from_paint(&events, &config, &ordinary).expect("control must recover");
    assert_eq!(replacement.col_count, 4);
    assert_eq!(replacement.rows[0].cells[1].text, "Activity (annual)");
    assert_eq!(replacement.rows[0].cells[3].text, "Take (%)");
    for (label, y) in [("(1)", 60.), ("(2)", 40.)] {
        events.push(control_text(&[(label, 170., y)]));
    }
    assert!(replacement_from_paint(&events, &config, &ordinary).is_none());
}

#[test]
fn collapsed_replacement_abstains_when_qualified_labels_are_painted_separately() {
    let (mut events, ordinary) = ruled_control();
    let config = TableDetectionConfig::default();
    assert!(replacement_from_paint(&events, &config, &ordinary).is_some());
    let Paint::Text(chars) = &mut events[0] else {
        panic!("header text")
    };
    let mut qualifiers = Vec::new();
    chars.retain(|ch| {
        if ch.origin_y == 84. {
            qualifiers.push(ch.clone());
            false
        } else {
            true
        }
    });
    assert_eq!(qualifiers.iter().map(|ch| ch.char).collect::<String>(), "(annual)(%)");
    events.insert(1, Paint::Text(qualifiers));
    assert!(replacement_from_paint(&events, &config, &ordinary).is_none());
}

#[test]
fn body_paint_owns_only_the_text_object_followed_by_its_dividers() {
    let document = fixture(r"BT /F1 8 Tf 10 60 Td (\(1\)) Tj 140 0 Td (\(2\)) Tj ET", false);
    let body = recover(&document, 0, &TableDetectionConfig::default()).unwrap();
    assert_eq!(body.col_count, 3);
    assert_eq!(body.rows.len(), 3);
    assert!(!body.has_header);
    assert_eq!(
        body.rows[0]
            .cells
            .iter()
            .map(|cell| cell.text.as_str())
            .collect::<Vec<_>>(),
        ["Stock 1", "20.2", "30.3"]
    );
    assert!(document.extract_text(0).unwrap().contains("(2)"));
}

#[test]
fn one_to_one_body_association_keeps_header_and_excludes_aligned_callouts() {
    let document = fixture(r"BT /F1 8 Tf 10 60 Td (\(1\)) Tj 140 0 Td (\(2\)) Tj ET", false);
    let body = recover(&document, 0, &TableDetectionConfig::default()).unwrap();
    let mut ruled = body.clone();
    let mut header = TableRow::new(true);
    for (index, text) in ["Stock", "Activity\n(annual)", "Take\n(%)"].into_iter().enumerate() {
        let mut cell = TableCell::new(text.into(), true);
        cell.bbox = Some(Rect::new(5. + 60. * index as f32, 84., 40., 12.));
        header.cells.push(cell);
    }
    ruled.rows.insert(0, header);
    ruled.has_header = true;
    let original_bounds = ruled.bbox;
    let original_cell_bounds = ruled.rows[1].cells[2].bbox;
    ruled.rows[1].cells[2].text.push_str(" (2)");

    let associated = associate_body_rows(&ruled, &body).unwrap();
    assert!(associated.has_header);
    assert_eq!(associated.bbox, original_bounds);
    assert_eq!(associated.rows[0].cells[1].text, "Activity\n(annual)");
    assert_eq!(associated.rows[0].cells[2].text, "Take\n(%)");
    assert_eq!(associated.rows[1].cells[2].bbox, original_cell_bounds);
    assert_eq!(associated.rows[1].cells[2].text, "30.3");
    assert!(document.extract_text(0).unwrap().contains("(2)"));

    ruled.rows[1].cells[0].bbox = ruled.rows[2].cells[0].bbox;
    assert!(associate_body_rows(&ruled, &body).is_none());
}

#[test]
fn body_paint_abstains_when_an_independent_callout_interrupts_the_record() {
    let document = fixture("", false);
    let data = document.get_page_content_data(0).unwrap();
    let content = String::from_utf8(data).unwrap();
    let content = content.replacen("ET\n50 60", r"ET BT /F1 8 Tf 150 60 Td (\(1\)) Tj ET 50 60", 1);
    let operators = parse_content_stream(content.as_bytes()).unwrap();
    let mut extractor = TextExtractor::new();
    let resources = document.get_page_resources(0).unwrap();
    document.load_fonts(&resources, &mut extractor).unwrap();
    let paints = paint_events(operators, &mut extractor).unwrap();
    assert!(body_from_paint(&paints).is_none());
}

#[test]
fn body_paint_abstains_when_a_row_is_split_across_text_objects() {
    let document = fixture("", true);
    assert!(recover(&document, 0, &TableDetectionConfig::default()).is_none());
}

#[test]
fn shifted_text_restart_requires_direct_base_attachment() {
    let document = fixture("", false);
    let resources = document.get_page_resources(0).unwrap();
    let body = "BT /F1 8 Tf 1 0 0 1 150 60 Tm (30.3) Tj 1 0 0 1 172 62 Tm (3) Tj";
    for (suffix, expected) in [
        ("", Some("30.3 3")),
        (" (4) Tj", Some("30.3 34")),
        (" [(4)] TJ", Some("30.3 34")),
        (" [0 (4)] TJ", Some("30.3 34")),
        (" [(4) 0 (5)] TJ", Some("30.3 345")),
        (" 1 0 0 1 179 62 Tm (4) Tj", None),
        (" 7 0 Td (4) Tj", None),
    ] {
        let operators = parse_content_stream(format!("{body}{suffix} ET").as_bytes()).unwrap();
        let mut extractor = TextExtractor::new();
        document.load_fonts(&resources, &mut extractor).unwrap();
        let paints = paint_events(operators, &mut extractor).unwrap();
        let [Paint::Text(chars)] = paints.as_slice() else {
            panic!("expected one text object");
        };
        let row = row_from_chars(chars, &[], 60., 80.);
        assert_eq!(row.as_ref().map(|row| row.cells[0].text.as_str()), expected, "{suffix}");
    }
}

#[test]
fn tj_displacements_restart_shifted_text_ownership() {
    let document = fixture("", false);
    let resources = document.get_page_resources(0).unwrap();
    let base = "BT /F1 8 Tf 1 0 0 1 150 60 Tm (30.3) Tj 1 0 0 1 172 62 Tm";
    let mut accepted_restarts = Vec::new();
    for suffix in [
        "(3) Tj [-300 (4)] TJ",
        "[(3) -300 (4)] TJ",
        "[(3) -300] TJ (4) Tj",
        "(3) Tj [-100 -200 (4)] TJ",
        "(3) Tj [-300] TJ (4) Tj",
    ] {
        let operators = parse_content_stream(format!("{base} {suffix} ET").as_bytes()).unwrap();
        let mut extractor = TextExtractor::new();
        document.load_fonts(&resources, &mut extractor).unwrap();
        let paints = paint_events(operators, &mut extractor).unwrap();
        let [Paint::Text(chars)] = paints.as_slice() else {
            panic!("expected one text object");
        };
        if row_from_chars(chars, &[], 60., 80.).is_some() {
            accepted_restarts.push(suffix);
        }
    }
    assert!(
        accepted_restarts.is_empty(),
        "accepted TJ restarts: {accepted_restarts:?}"
    );
}

#[test]
fn text_position_operators_start_new_paint_runs() {
    let document = fixture("", false);
    let resources = document.get_page_resources(0).unwrap();
    for between in [
        "1 0 0 1 153 60 Tm (4) Tj",
        "3 0 Td (4) Tj",
        "3 0 TD (4) Tj",
        "T* (4) Tj",
        "(4) '",
        "0 0 (4) \"",
        "2 Ts (4) Tj",
        "q 1 0 0 1 3 0 cm (4) Tj Q",
        "q Q (4) Tj",
    ] {
        let content = format!("BT /F1 8 Tf 1 0 0 1 150 60 Tm (3) Tj {between} ET");
        let mut extractor = TextExtractor::new();
        document.load_fonts(&resources, &mut extractor).unwrap();
        let paints = paint_events(parse_content_stream(content.as_bytes()).unwrap(), &mut extractor).unwrap();
        let [Paint::Text(chars)] = paints.as_slice() else {
            panic!("expected one text object for {between}");
        };
        assert_eq!(
            chars.iter().map(|glyph| glyph.char).collect::<String>(),
            "34",
            "{between}"
        );
        assert_ne!(chars[0].run, chars[1].run, "{between}");
    }
}
