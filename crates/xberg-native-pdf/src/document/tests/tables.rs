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
