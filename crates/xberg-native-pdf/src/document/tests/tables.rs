/// Multi-row-spanning label cell (test item name vertically centered
/// across N data rows) must be placed at the top of its row block in
/// reading-order output, not interleaved mid-group by Y.
///
/// Simulates a simplified 2-column table:
/// - Column A (sparse, "labels"): 2 labels, each centered in its
///   block of 6 data rows.
/// - Column B (dense, "data"): 12 data rows.
///
/// Expected sort: Label1, d1..d6, Label2, d7..d12.
#[test]
fn test_rowspan_label_promoted_to_top_of_block() {
    use crate::layout::TextSpan;

    fn mk(text: &str, x: f32, y: f32, w: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: crate::geometry::Rect::new(x, y, w, 10.0),
            font_size: 12.0,
            font_name: "Arial".into(),
            font_weight: crate::layout::FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: crate::layout::Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            primary_detected: false,
            char_widths: vec![],
            char_x_offsets: Vec::new(),
            heading_level: None,
            rotation_degrees: 0.0,
            wmode: 0,
            rtl_draw_logical: false,
            ligature_glyph: false,
            mirrored: false,
            page_rotation_applied: 0,
        }
    }

    // Data rows at x=200, y=100..30 step -10 (12 rows).
    // Label1 at x=50, y=75 (middle of rows 100..60).
    // Label2 at x=50, y=45 (middle of rows 50..30... but actually 50..30 is 3 values,
    //   and label2 should be centered in rows 50..30 → y=40 but we choose 45 to be clearly in 2nd block).
    // Target split: Label1 owns rows 100,90,80,70,60,50; Label2 owns 40,30,20,10.
    // Both labels' Y (75 and 45) sit between their block rows. ~keep
    let mut spans = vec![mk("L1", 50.0, 75.0, 40.0), mk("L2", 50.0, 45.0, 40.0)];
    for i in 0..12 {
        let y = 100.0 - (i as f32) * 10.0;
        spans.push(mk(&format!("d{:02}", i), 200.0, y, 20.0));
    }

    super::super::PdfDocument::reorder_rowspan_labels(&mut spans);

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let pos_l1 = texts.iter().position(|t| *t == "L1").expect("L1 present");
    let pos_l2 = texts.iter().position(|t| *t == "L2").expect("L2 present");
    assert!(
        pos_l1 < pos_l2,
        "L1 should precede L2 in reading order, got {:?}",
        texts
    );
    // L1 must come before ALL data rows that belong to L1's block.
    // With distance-based partitioning, L1 owns rows closer to y=75 than y=45:
    //   100,90,80,70,60 are closer to 75. 50 is equidistant (tie → L1).
    //   Expect L1 at index 0 and L2 somewhere after L1's block. ~keep
    assert_eq!(texts[0], "L1", "L1 must be first, got: {:?}", &texts[..5]);
    assert!(
        pos_l2 > pos_l1 + 3,
        "L2 must come after several data rows of L1's block, got {:?}",
        texts
    );
}

fn decimal_anchor_from_table_word(rotate: bool) -> (Vec<f32>, f32) {
    use super::super::PdfDocument;

    let axis_rule = "10 10 m 160 10 l S 10 30 m 160 30 l S 10 50 m 160 50 l S 10 70 m 160 70 l S\n\
                     10 10 m 10 70 l S 60 10 m 60 70 l S 110 10 m 110 70 l S 160 10 m 160 70 l S\n";
    let matrix = if rotate { "1 .02 -.02 1" } else { "1 0 0 1" };
    let stream = format!(
        "{axis_rule}BT /F1 8 Tf 1 0 0 1 15 55 Tm (Metric) Tj 1 0 0 1 65 55 Tm (Value) Tj \
         1 0 0 1 115 55 Tm (Unit) Tj 1 0 0 1 15 35 Tm (Latency) Tj {matrix} 65 35 Tm (12.5) Tj \
         1 0 0 1 115 35 Tm (ms) Tj 1 0 0 1 15 15 Tm (Duration) Tj 1 0 0 1 65 15 Tm (10) Tj \
         1 0 0 1 115 15 Tm (ms) Tj ET\n"
    );
    let content = format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len());
    let pdf = super::pdf_fixtures::build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
            (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
            (5, content.as_bytes()),
        ],
    );
    let document = PdfDocument::from_bytes(pdf).expect("parse the ruled table fixture");
    let glyphs = document.extract_chars(0).expect("read raw glyph orientation");
    let dot = glyphs
        .iter()
        .find(|glyph| glyph.char == '.')
        .expect("source decimal point");
    if rotate {
        assert!((1.0..1.2).contains(&dot.rotation_degrees), "{dot:?}");
    } else {
        assert_eq!(dot.rotation_degrees, 0.0);
    }
    let tables = document.extract_tables(0).expect("extract native tables");
    let [table] = tables.as_slice() else {
        panic!("expected a single ruled table");
    };
    let word = table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| cell.text == "12.5")
        .flat_map(|cell| &cell.spans)
        .find(|span| span.text.trim() == "12.5")
        .expect("decimal cell retains the table word");
    (word.char_x_offsets.clone(), dot.origin_x)
}

#[test]
fn shallow_rotated_table_word_does_not_supply_horizontal_decimal_anchor() {
    let (positions, _) = decimal_anchor_from_table_word(true);
    assert!(positions.is_empty(), "tilted raw glyphs supplied {positions:?}");
}

#[test]
fn horizontal_table_word_retains_raw_decimal_anchor() {
    let (positions, source_dot) = decimal_anchor_from_table_word(false);
    assert_eq!(positions.len(), 4);
    assert_eq!(positions[2], source_dot);
}
