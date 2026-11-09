use super::super::*;
use super::common::*;

/// Repeated `search_page_index()` calls (what `search()`/`search_page()`
/// use internally) must reuse the cached index instead of re-extracting
/// and rebuilding it every time — the whole point of the search index
/// (issue: `search()` re-extracted the full document on every call).
/// Redaction (`erase_region`) changes a page's spans, so it must
/// invalidate the cached index the same way it already invalidates
/// `page_spans_cache`.
#[test]
fn search_index_reused_across_calls_and_invalidated_by_redaction() {
    let content = b"BT /F1 12 Tf 1 0 0 1 72 700 Tm (Hello World) Tj ET";
    let pdf = build_minimal_pdf_with_font(content);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    let first = doc.search_page_index(0).expect("build search index");
    let second = doc.search_page_index(0).expect("reuse search index");
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "a second search_page_index() call should hit the cache, not rebuild"
    );

    doc.erase_region(0, crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0))
        .expect("erase_region");
    let third = doc.search_page_index(0).expect("rebuild after redaction");
    assert!(
        !std::sync::Arc::ptr_eq(&first, &third),
        "erase_region must invalidate the cached search index"
    );
}

/// `clear_search_index()` is the caller-facing escape hatch for dropping
/// the (otherwise unbounded) search index to reclaim memory.
#[test]
fn clear_search_index_forces_rebuild() {
    let content = b"BT /F1 12 Tf 1 0 0 1 72 700 Tm (Hello World) Tj ET";
    let pdf = build_minimal_pdf_with_font(content);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    let first = doc.search_page_index(0).expect("build search index");
    doc.clear_search_index();
    let second = doc.search_page_index(0).expect("rebuild after clear");
    assert!(
        !std::sync::Arc::ptr_eq(&first, &second),
        "clear_search_index() should force the next call to rebuild"
    );
}

/// `prepare_search()` must populate the index for every page so a
/// subsequent full-document `search()` sweep hits cache on every page,
/// not just the last few (the failure mode `search()`'s own bounded
/// `page_spans_cache` has for documents with more than 8 pages).
#[test]
fn prepare_search_populates_every_page() {
    let pdf = build_multi_page_pdf(10);
    let doc = PdfDocument::from_bytes(pdf).expect("open pdf");

    doc.prepare_search().expect("prepare_search");
    for page in 0..10 {
        let a = doc.search_page_index(page).expect("indexed page");
        let b = doc.search_page_index(page).expect("still cached");
        assert!(
            std::sync::Arc::ptr_eq(&a, &b),
            "page {page} should already be cached by prepare_search()"
        );
    }
}

#[test]
fn test_may_contain_text_with_bt() {
    let data = b"q BT /F1 12 Tf (Hello) Tj ET Q";
    assert!(PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_with_do() {
    let data = b"q /Im0 Do Q";
    assert!(PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_no_text_operators() {
    let data = b"100 200 300 400 re S";
    assert!(!PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_empty() {
    let data = b"";
    assert!(!PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_bt_at_start() {
    let data = b"BT /F1 12 Tf ET";
    assert!(PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_bt_at_end() {
    let data = b"q Q BT";
    assert!(PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_false_positive_btype() {
    // "BTerror" should not match BT (BT must be delimited) ~keep
    let data = b"BTerror";
    assert!(!PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_false_positive_document() {
    // "Document" contains "Do" but not as a standalone operator ~keep
    let data = b"Document";
    assert!(!PdfDocument::may_contain_text(data));
}

#[test]
fn test_may_contain_text_do_with_name() {
    let data = b"/Im0 Do\n";
    assert!(PdfDocument::may_contain_text(data));
}

// The tagged struct-tree path collapses a page into one MCID
// whose pure-RTL word-spans are laid out left-to-right (visual, X
// ascending). `order_mcid_spans` must emit them right-to-left (logical)
// using geometry, since the tagged path never reaches the untagged
// `reverse_rtl_visual_order_runs`. (Per-span glyph order is handled
// separately by `push_span_text_bidi`; this test asserts span ORDER.) ~keep
#[test]
fn test_order_mcid_spans_pure_rtl_emitted_right_to_left() {
    // One Hebrew row, three words placed left-to-right by X. ~keep
    let spans = vec![
        make_rtl_test_span("שלוש", 100.0, 700.0), // leftmost  → logically last ~keep
        make_rtl_test_span("שתיים", 200.0, 700.0),
        make_rtl_test_span("אחת", 300.0, 700.0), // rightmost → logically first ~keep
    ];
    let ordered = PdfDocument::order_mcid_spans(&spans);
    let texts: Vec<&str> = ordered.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["אחת", "שתיים", "שלוש"],
        "pure-RTL MCID spans must emit rightmost-first (logical RTL order), got {texts:?}"
    );
}

/// SEG-AR cross-span glyph interleave: a word drawn as a body span plus a
/// zero-width consonant span whose x falls INSIDE the body must be repaired
/// (merged + reversed) into correct logical order, not atom-scrambled.
#[test]
fn test_merge_interleaved_rtl_word_reconstructs_logical() {
    use crate::geometry::Rect;
    // الثدييات visual L→R is "ت ا ي ي د ث ل ا"; the producer draws the body
    // "تاييدلا" (7 glyphs @ x=100,110,…,160) and the consonant ث as a
    // zero-width span at x=145, strictly inside the body's [100,170] extent. ~keep
    let body = TextSpan {
        text: "تاييدلا".to_string(),
        bbox: Rect::new(100.0, 700.0, 70.0, 12.0),
        char_widths: vec![10.0; 7],
        char_x_offsets: Vec::new(),
        font_size: 12.0,
        ..TextSpan::default()
    };
    let theh = TextSpan {
        text: "ث".to_string(),
        bbox: Rect::new(145.0, 700.0, 0.0, 12.0),
        font_size: 12.0,
        ..TextSpan::default()
    };
    let spans = [body, theh];
    let line: Vec<&TextSpan> = spans.iter().collect();
    assert!(
        PdfDocument::rtl_line_needs_glyph_reorder(&line),
        "interleaved zero-width consonant must trigger the glyph-reorder gate"
    );
    // The merged span is VISUAL order; push_span_text_bidi reverses it. ~keep
    let merged = PdfDocument::merge_rtl_line_to_visual_span(&line);
    let mut out = String::new();
    PdfDocument::push_span_text_bidi(&mut out, &merged, true);
    assert_eq!(out, "الثدييات", "interleaved word not reconstructed, got {out:?}");
}

/// P3: a producer-segmented word boundary between two Arabic words must
/// survive the `merge_rtl_line_to_visual_span` → `push_span_text_bidi`
/// pipeline even when it falls after a DUAL-joining letter (ع in `أنواع`).
/// The merge records the boundary from the producer's STANDALONE space span;
/// without the word-boundary sentinel, `strip_interior_arabic_spaces`'s
/// sparse branch deletes it (a space after a dual-joining letter looks like a
/// cursive-shatter artefact), gluing `أنواع شائعة` → `أنواعشائعة`.
#[test]
fn test_merge_rtl_preserves_producer_word_boundary_after_dual_joining() {
    use crate::geometry::Rect;
    // Two words laid out left-to-right in VISUAL order (logical order is the
    // reverse): word two `شائعة` (visual `ةعئاش`, x 100‒140), a standalone
    // producer space span at x=150, then word one `أنواع` (visual `عاونأ`,
    // x 160‒200). The logical boundary sits after ع (dual-joining). ~keep
    let word_two = TextSpan {
        text: "ةعئاش".to_string(),
        bbox: Rect::new(100.0, 700.0, 50.0, 12.0),
        char_widths: vec![10.0; 5],
        char_x_offsets: Vec::new(),
        font_size: 12.0,
        ..TextSpan::default()
    };
    let space = TextSpan {
        text: " ".to_string(),
        bbox: Rect::new(150.0, 700.0, 8.0, 12.0),
        font_size: 12.0,
        ..TextSpan::default()
    };
    let word_one = TextSpan {
        text: "عاونأ".to_string(),
        bbox: Rect::new(160.0, 700.0, 50.0, 12.0),
        char_widths: vec![10.0; 5],
        char_x_offsets: Vec::new(),
        font_size: 12.0,
        ..TextSpan::default()
    };
    let spans = [word_two, space, word_one];
    let line: Vec<&TextSpan> = spans.iter().collect();
    let merged = PdfDocument::merge_rtl_line_to_visual_span(&line);
    let mut out = String::new();
    PdfDocument::push_span_text_bidi(&mut out, &merged, true);
    assert_eq!(
        out, "أنواع شائعة",
        "producer word boundary after a dual-joining letter must be kept, got {out:?}"
    );
    assert!(
        !out.contains(PdfDocument::RTL_WORD_BOUNDARY),
        "word-boundary sentinel must be restored to a space, not leaked: {out:?}"
    );
}

/// P2: a line-final zero-width glyph seated a few points below the baseline
/// (`ي` in `في`, drawn at dy≈3pt, width 0) must stay on its own line's band
/// rather than splitting off and reversing to land after the sentence
/// terminator (`في العالم.` → `ف العالم.ي`). The gated RTL line must collapse
/// to a single merged span with `ي` reattached before the full stop.
#[test]
fn test_merge_keeps_subbaseline_zero_width_glyph_on_line() {
    use crate::geometry::Rect;
    let span = |text: &str, x: f32, y: f32, w: f32| TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, y, w, 12.0),
        font_size: 13.0,
        ..TextSpan::default()
    };
    // Visual (ascending-x) fragments of "… استهلاكا في العالم.":
    // "." (terminator, leftmost), العالم body, ي (zero-width, dy≈3 low),
    // ف (zero-width), a standalone space, then a body word carrying a
    // zero-width mark INSIDE it so the line trips the glyph-reorder gate. ~keep
    let spans = vec![
        span(".", 95.94, 664.5, 3.48),
        span("ملاعلا", 99.42, 664.5, 27.71),
        span(" ", 127.13, 664.5, 3.38),
        span("ي", 132.92, 661.48, 0.0), // line-final, below baseline, width 0 ~keep
        span("ف", 141.99, 664.99, 0.0),
        span(" ", 145.89, 664.5, 3.38),
        span("االهسا", 149.26, 664.5, 41.64),
        span("كً", 153.57, 666.57, 0.0),
        // ~keep
    ];
    let merged = PdfDocument::merge_interleaved_rtl_lines(&spans).expect("interleaved gate must fire on this RTL line");
    assert_eq!(
        merged.len(),
        1,
        "the sub-baseline ي must stay in the line's band, not split off (got {} spans)",
        merged.len()
    );
    let mut out = String::new();
    for s in &merged {
        PdfDocument::push_span_text_bidi(&mut out, s, true);
    }
    assert!(out.contains("في"), "ي must reattach to ف as the word في; got {out:?}");
    assert!(
        !out.trim_end().ends_with('ي'),
        "ي must not be stranded after the sentence terminator; got {out:?}"
    );
}

/// Negative: a pure-RTL line with NO zero-width interleaved span (logical-
/// order word spans, the BidiSample shape) must NOT trigger the reorder gate,
/// so already-correct RTL pages stay on the unchanged path.
#[test]
fn test_rtl_line_no_interleave_skips_glyph_reorder() {
    let spans = vec![
        make_rtl_test_span("אחת", 300.0, 700.0),
        make_rtl_test_span("שתיים", 200.0, 700.0),
        make_rtl_test_span("שלוש", 100.0, 700.0),
    ];
    let line: Vec<&TextSpan> = spans.iter().collect();
    assert!(
        !PdfDocument::rtl_line_needs_glyph_reorder(&line),
        "no zero-width interleave → gate must stay off (byte-identical path)"
    );
    assert!(
        PdfDocument::merge_interleaved_rtl_lines(&spans).is_none(),
        "no interleaved line → no merge (caller uses original spans)"
    );
}

/// A pure-RTL line whose zero-advance glyphs (hamza seats, marks,
/// producer-positioned consonants) are drawn a couple of points off the
/// baseline must NOT be scattered into separate rows. The fixed quantized
/// row band split them out and emitted them first (the leading stray-alef
/// cluster); font-relative line grouping keeps the whole line together and
/// in rightmost-first order.
#[test]
fn test_order_pure_rtl_spans_keeps_jittery_baseline_in_one_line() {
    // Five Arabic letters on ONE visual line at X = 300..100 (rightmost
    // first is logical), but with ±2pt baseline jitter — two of them drawn
    // above the baseline as a zero-width producer would. Font size 12 →
    // tolerance 6pt, so the 4pt spread stays a single line. ~keep
    let spans = vec![
        make_rtl_test_span("\u{0627}", 300.0, 701.0), // ا  rightmost, +1 ~keep
        make_rtl_test_span("\u{0644}", 250.0, 703.0), // ل  above baseline ~keep
        make_rtl_test_span("\u{0642}", 200.0, 700.0), // ق  baseline ~keep
        make_rtl_test_span("\u{0637}", 150.0, 702.0),
        make_rtl_test_span("\u{0645}", 100.0, 699.0), // م  leftmost, -1 ~keep
    ];
    let ordered = PdfDocument::order_pure_rtl_spans(&spans);
    let texts: Vec<&str> = ordered.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["\u{0627}", "\u{0644}", "\u{0642}", "\u{0637}", "\u{0645}"],
        "jittery-baseline RTL line must stay in one rightmost-first run, got {texts:?}"
    );
}

/// Genuinely separate RTL lines (leading ~1.2x font size) must still break:
/// the font-relative tolerance groups jitter, not whole lines.
#[test]
fn test_order_pure_rtl_spans_breaks_distinct_lines() {
    // Two lines, 14pt apart (font size 12 → tol 6pt, so they split). Each
    // line emits rightmost-first; the top line precedes the bottom line. ~keep
    let spans = vec![
        make_rtl_test_span("\u{0628}", 200.0, 714.0),
        make_rtl_test_span("\u{0627}", 300.0, 714.0),
        make_rtl_test_span("\u{062F}", 200.0, 700.0),
        make_rtl_test_span("\u{062C}", 300.0, 700.0),
    ];
    let ordered = PdfDocument::order_pure_rtl_spans(&spans);
    let texts: Vec<&str> = ordered.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["\u{0627}", "\u{0628}", "\u{062C}", "\u{062F}"],
        "distinct RTL lines must break (top first, each rightmost-first), got {texts:?}"
    );
}

// Mixed RTL+Latin MCIDs are left in raw order (full UAX #9 deferred) —
// guards against the pure-RTL reorder accidentally firing on mixed runs. ~keep
#[test]
fn test_order_mcid_spans_mixed_rtl_latin_kept_raw() {
    let spans = vec![
        make_rtl_test_span("שלום", 100.0, 700.0),
        make_rtl_test_span("World", 200.0, 700.0),
    ];
    let ordered = PdfDocument::order_mcid_spans(&spans);
    let texts: Vec<&str> = ordered.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["שלום", "World"], "mixed RTL+Latin must stay in raw order");
}

#[test]
fn test_may_contain_text_bt_with_newline() {
    assert!(PdfDocument::may_contain_text(b"\nBT\n"));
}

#[test]
fn test_may_contain_text_do_with_bracket() {
    assert!(PdfDocument::may_contain_text(b"]Do["));
}

#[test]
fn test_may_contain_text_single_b() {
    assert!(!PdfDocument::may_contain_text(b"B"));
}

#[test]
fn test_may_contain_text_single_d() {
    assert!(!PdfDocument::may_contain_text(b"D"));
}

#[test]
fn test_reading_order_enum_default() {
    let order = ReadingOrder::default();
    assert_eq!(order, ReadingOrder::TopToBottom);
}

#[test]
fn test_reading_order_enum_variants() {
    assert_ne!(ReadingOrder::TopToBottom, ReadingOrder::ColumnAware);
    let a = ReadingOrder::ColumnAware;
    let b = a;
    assert_eq!(a, b);
}

/// Verify that ColumnAware reading order reads column 1 fully before column 2.
///
/// Layout:
/// ```text
///   Left col (x=10) Right col (x=200)
///   +-----------+ +-----------+
///   | L1 (y=700)| | R1 (y=700)|
///   | L2 (y=680)| | R2 (y=680)|
///   | L3 (y=660)| | R3 (y=660)|
///   +-----------+ +-----------+
/// ```
/// Expected ColumnAware order: L1, L2, L3, R1, R2, R3
/// TopToBottom order would interleave: L1, R1, L2, R2, L3, R3
#[test]
fn test_column_aware_reads_column1_before_column2() {
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextSpan};
    use crate::pipeline::reading_order::{ReadingOrderContext as ROContext, ReadingOrderStrategy, XYCutStrategy};

    fn make_span(label: &str, x: f32, y: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: label.to_string(),
            bbox: Rect::new(x, y, 80.0, 12.0),
            font_size: 12.0,
            font_name: "Test".to_string(),
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
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

    let spans = vec![
        make_span("L1", 10.0, 700.0),
        make_span("R1", 200.0, 700.0),
        make_span("L2", 10.0, 680.0),
        make_span("R2", 200.0, 680.0),
        make_span("L3", 10.0, 660.0),
        make_span("R3", 200.0, 660.0),
    ];

    let strategy = XYCutStrategy::new();
    let context = ROContext::new();
    let ordered = strategy.apply(spans, &context).expect("XYCut should not fail");
    let labels: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

    assert_eq!(
        labels,
        vec!["L1", "L2", "L3", "R1", "R2", "R3"],
        "ColumnAware should read left column fully before right column"
    );
}

/// Regression test: a page with only 2 spans per column
/// (4 spans total) is below `min_spans_for_split` (5), so it never
/// reaches the geometric column-split logic the 6-span test above
/// exercises — every statistical prose/table classifier
/// (`classify_region_kind`, `detect_two_column_prose`,
/// `detect_narrow_gutter_prose`) also has its own internal minimum-span
/// floor (6/8/24) far above 4, so none of them can classify this page
/// either. Before the fix, the base case fell back to a flat
/// Y-then-X sort, interleaving the two columns (L1, R1, L2, R2)
/// instead of reading each column through.
///
/// A pure geometric gutter check can't distinguish this from a 2x2
/// table at this scale (see `test_column_aware_sparse_2x2_table_stays_row_major`
/// below), so the fix defers to content-stream emission order when a
/// clean gutter exists — PDFium parity per the issue's own cross-tool
/// probe. This fixture's `sequence` mirrors the exact reporter's
/// repro (`reportlab` draws the whole left column, then the whole
/// right column): L1, L2, R1, R2.
#[test]
fn test_column_aware_sparse_two_column_follows_stream_order() {
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextSpan};
    use crate::pipeline::reading_order::{ReadingOrderContext as ROContext, ReadingOrderStrategy, XYCutStrategy};

    fn make_span(label: &str, x: f32, y: f32, sequence: usize) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            mirrored: false,
            page_rotation_applied: 0,
            artifact_type: None,
            text: label.to_string(),
            bbox: Rect::new(x, y, 80.0, 12.0),
            font_size: 12.0,
            font_name: "Test".to_string(),
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
            mcid: None,
            mcid_scope: None,
            sequence,
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
        }
    }

    // Column-major stream order, matching a two-column-prose generator
    // that fills the left text box then the right one — exactly the
    // reporter's `reportlab` repro. ~keep
    let spans = vec![
        make_span("L1", 10.0, 700.0, 0),
        make_span("L2", 10.0, 680.0, 1),
        make_span("R1", 200.0, 700.0, 2),
        make_span("R2", 200.0, 680.0, 3),
    ];

    let strategy = XYCutStrategy::new();
    let context = ROContext::new();
    let ordered = strategy.apply(spans, &context).expect("XYCut should not fail");
    let labels: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

    assert_eq!(
        labels,
        vec!["L1", "L2", "R1", "R2"],
        "a sparse 2-column page below min_spans_for_split must follow \
             content-stream order (column-major here), not interleave the \
             columns via a flat Y-then-X sort"
    );
}

/// Companion to the test above: a genuine 2x2 table emitted **row-major**
/// in-stream (the common table-generator pattern — draw row 1's cells
/// left-to-right, then row 2's) must stay row-major. The same clean
/// gutter exists between the two columns as in the prose case above —
/// nothing in this codebase can geometrically tell the two apart at
/// 4-span scale — so the fix's content-stream-order fallback is
/// correct for *both* shapes precisely because it never has to decide
/// between them: it just preserves however the source authored it.
#[test]
fn test_column_aware_sparse_2x2_table_stays_row_major() {
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextSpan};
    use crate::pipeline::reading_order::{ReadingOrderContext as ROContext, ReadingOrderStrategy, XYCutStrategy};

    fn make_cell(label: &str, x: f32, y: f32, sequence: usize) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            mirrored: false,
            page_rotation_applied: 0,
            artifact_type: None,
            text: label.to_string(),
            bbox: Rect::new(x, y, 80.0, 12.0),
            font_size: 12.0,
            font_name: "Test".to_string(),
            font_weight: FontWeight::Normal,
            is_italic: false,
            is_monospace: false,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
            mcid: None,
            mcid_scope: None,
            sequence,
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
        }
    }

    let spans = vec![
        make_cell("R1C1", 10.0, 700.0, 0),
        make_cell("R1C2", 200.0, 700.0, 1),
        make_cell("R2C1", 10.0, 680.0, 2),
        make_cell("R2C2", 200.0, 680.0, 3),
    ];

    let strategy = XYCutStrategy::new();
    let context = ROContext::new();
    let ordered = strategy.apply(spans, &context).expect("XYCut should not fail");
    let labels: Vec<&str> = ordered.iter().map(|o| o.span.text.as_str()).collect();

    assert_eq!(
        labels,
        vec!["R1C1", "R1C2", "R2C1", "R2C2"],
        "a row-major-emitted 2x2 table must stay row-major, not be \
             reshuffled into a column-major read order"
    );
}

/// Regression: line-continuation spans that share a Y-band with the dense
/// column must NOT be promoted by `reorder_rowspan_labels`.
///
/// A resume-like PDF has two X groups: a dense main-text column (x=63)
/// and a sparse rightward column (x=430) whose spans are all on the SAME
/// lines as the dense column (same Y-bands). The sparse spans are
/// line-continuation text, not rowspan labels, so they must stay in their
/// natural sorted position rather than being hoisted to wrong Y values.
#[test]
fn test_rowspan_label_skips_spans_aligned_with_dense_column() {
    use crate::layout::TextSpan;

    fn mk(text: &str, x: f32, y: f32) -> TextSpan {
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: text.to_string(),
            bbox: crate::geometry::Rect::new(x, y, 80.0, 10.0),
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

    // Dense column (x=63): 10 spans at y=640,620,600,580,560,540,520,500,480,460
    // Sparse column (x=430): 4 spans at y=600,560,520,480 — same lines as dense
    // After reorder_rowspan_labels the sparse spans must NOT be promoted. ~keep
    let ys_dense = [640.0f32, 620.0, 600.0, 580.0, 560.0, 540.0, 520.0, 500.0, 480.0, 460.0];
    let ys_sparse = [600.0f32, 560.0, 520.0, 480.0];

    let mut spans: Vec<TextSpan> = Vec::new();
    for &y in &ys_dense {
        spans.push(mk(&format!("dense_y{}", y as i32), 63.0, y));
    }
    for &y in &ys_sparse {
        spans.push(mk(&format!("sparse_y{}", y as i32), 430.0, y));
    }

    // Sort descending Y, X ascending (as extract_spans does before calling this) ~keep
    spans.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
    let before: Vec<String> = spans.iter().map(|s| s.text.clone()).collect();

    super::super::PdfDocument::reorder_rowspan_labels(&mut spans);

    let after: Vec<String> = spans.iter().map(|s| s.text.clone()).collect();
    assert_eq!(
        before, after,
        "reorder_rowspan_labels must not change order when sparse spans \
             share Y-bands with the dense column; \
             before={before:?} after={after:?}"
    );
}

/// Regression: a numbered reference/bibliography list whose markers
/// ("1.", "2.", …) sit in a narrow left column between body rows must
/// NOT have those markers promoted as rowspan labels. The geometry is
/// identical to a genuine rowspan table — only the marker TEXT (a
/// vertical numbered list) distinguishes it — so the guard keys on the
/// numbered-marker signal and leaves reading order intact.
#[test]
fn test_rowspan_skips_numbered_reference_continuation() {
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

    // Dense body column (x=200): 12 rows y=100..-10 step -10.
    // Numbered markers (x=50): "1.".."4." sitting BETWEEN body rows —
    // the exact geometry that promotes a genuine rowspan label. ~keep
    let mut spans = vec![
        mk("1.", 50.0, 95.0, 40.0),
        mk("2.", 50.0, 65.0, 40.0),
        mk("3.", 50.0, 35.0, 40.0),
        mk("4.", 50.0, 5.0, 40.0),
    ];
    for i in 0..12 {
        let y = 100.0 - (i as f32) * 10.0;
        spans.push(mk(&format!("b{:02}", i), 200.0, y, 20.0));
    }

    // Sort as extract_spans does before calling reorder_rowspan_labels. ~keep
    spans.sort_by(|a, b| crate::utils::row_aware_span_cmp(a.bbox.y, a.bbox.x, b.bbox.y, b.bbox.x));
    let before: Vec<String> = spans.iter().map(|s| s.text.clone()).collect();

    super::super::PdfDocument::reorder_rowspan_labels(&mut spans);

    let after: Vec<String> = spans.iter().map(|s| s.text.clone()).collect();
    assert_eq!(
        before, after,
        "numbered reference markers must not be promoted as rowspan \
             labels; before={before:?} after={after:?}"
    );
}
