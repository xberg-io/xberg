//! A tagged-PDF table cell whose marked-content sequence carries several text
//! runs (a wrapped line, or a gap-separated pair) must keep every run — the
//! cell owns all content sharing its MCID (ISO 32000-1 §14.8.4.3.4), not just
//! the first block encountered.

use xberg_native_pdf::layout::{Color, FontWeight, TextSpan};
use xberg_native_pdf::structure::{McidScope, StructChild, StructElem, StructType, extract_table_from_spans};

fn span(text: &str, x: f32, y: f32, mcid: u32, seq: usize) -> TextSpan {
    TextSpan {
        provenance: None,
        artifact_type: None,
        text: text.to_string(),
        bbox: xberg_native_pdf::geometry::Rect::new(x, y, 30.0, 12.0),
        font_name: "Helvetica".to_string(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: Some(mcid),
        mcid_scope: None,
        sequence: seq,
        offset_semantic: false,
        split_boundary_before: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        text_rise: 0.0,
        rtl_draw_logical: false,
        ligature_glyph: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

fn td(mcid: u32) -> StructElem {
    let mut e = StructElem::new(StructType::TD);
    e.add_child(StructChild::MarkedContentRef {
        mcid,
        page: 0,
        scope: McidScope::Page(0),
    });
    e
}

fn tr(cells: Vec<StructElem>) -> StructElem {
    let mut r = StructElem::new(StructType::TR);
    for c in cells {
        r.add_child(StructChild::StructElem(Box::new(c)));
    }
    r
}

#[test]
fn cell_with_two_wrapped_lines_keeps_both() {
    let spans = vec![
        span("Hello", 72.0, 700.0, 0, 0),
        span("World", 72.0, 686.0, 0, 1),
        span("Alpha", 200.0, 700.0, 1, 2),
    ];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "Hello World",
        "second span sharing the cell's MCID was dropped"
    );
}

#[test]
fn cell_with_gap_separated_spans_keeps_both() {
    let spans = vec![
        span("Total", 72.0, 700.0, 0, 0),
        span("100", 300.0, 700.0, 0, 1),
        span("x", 400.0, 700.0, 1, 2),
    ];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "Total 100",
        "second span sharing the cell's MCID was dropped"
    );
}

/// A 90° rotated column header (reads bottom-to-top) must join along its own
/// writing axis. Its runs share an x and step in y, so a page-frame y-then-x
/// sort would emit them in reverse.
#[test]
fn rotated_cell_runs_join_along_their_writing_axis() {
    let mut first = span("Reve", 100.0, 200.0, 0, 0);
    first.rotation_degrees = 90.0;
    let mut second = span("nue", 100.0, 230.0, 0, 1);
    second.rotation_degrees = 90.0;
    let spans = vec![second, first, span("x", 400.0, 700.0, 1, 2)];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "Revenue",
        "abutting runs of one 90° rotated line must rejoin without a space"
    );
}

/// Two separate lines of a rotated cell — a genuinely wrapped rotated header —
/// still get their space. The line break is decided in the writing frame, so
/// it must survive there too.
#[test]
fn rotated_cell_separate_lines_keep_their_space() {
    let mut first = span("Total", 100.0, 200.0, 0, 0);
    first.rotation_degrees = 90.0;
    let mut second = span("Cost", 140.0, 200.0, 0, 1);
    second.rotation_degrees = 90.0;
    let spans = vec![first, second, span("x", 400.0, 700.0, 1, 2)];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "Total Cost",
        "runs on different rotated lines are separate words"
    );
}

/// Right-to-left script is stored in logical order but drawn toward
/// decreasing x, so ordering by ascending x would reverse the words.
#[test]
fn rtl_cell_runs_keep_logical_order() {
    let spans = vec![
        span("مرحبا", 300.0, 700.0, 0, 0),
        span("بالعالم", 200.0, 700.0, 0, 1),
        span("x", 400.0, 660.0, 1, 2),
    ];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "مرحبا بالعالم",
        "RTL runs must join in logical order, not left-to-right visual order"
    );
}

/// Wrapped lines arriving in reverse content order must join in visual
/// order, even when a tall sibling block sits in the same cell — line
/// grouping is per-pair, so one tall block cannot swallow line spacing
/// tighter than its own height.
#[test]
fn cell_with_tight_wrap_and_tall_block_keeps_visual_order() {
    let mut tall = span("tall", 72.0, 660.0, 0, 0);
    tall.bbox = xberg_native_pdf::geometry::Rect::new(72.0, 660.0, 30.0, 30.0);
    let mut upper = span("upper", 100.0, 700.0, 0, 1);
    upper.bbox = xberg_native_pdf::geometry::Rect::new(100.0, 700.0, 30.0, 8.0);
    let mut lower = span("lower", 72.0, 691.5, 0, 2);
    lower.bbox = xberg_native_pdf::geometry::Rect::new(72.0, 691.5, 30.0, 8.0);
    let spans = vec![tall, lower, upper, span("x", 400.0, 700.0, 1, 3)];
    let mut table = StructElem::new(StructType::Table);
    table.add_child(StructChild::StructElem(Box::new(tr(vec![td(0), td(1)]))));
    let t = extract_table_from_spans(&table, &spans).unwrap();
    assert_eq!(
        t.rows[0].cells[0].text, "upper lower tall",
        "tight 8.5pt line spacing beside a 30pt block must still order top-to-bottom"
    );
}
