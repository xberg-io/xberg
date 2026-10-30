//! Artifact-state, adaptive-threshold, and span-deduplication tests split out of `extractors/text/tests.rs` for file size. ~keep

use super::super::*;
use super::extraction_and_rotation::create_test_font;

/// The O(1) accumulator path and the recompute-from-slice fallback must
/// produce identical results (same f64 formula, same sum order).
#[test]
fn test_tj_accumulator_matches_recompute() {
    let vals = vec![-50.0f32, -200.0, -75.0, -180.0, -60.0, -210.0, -90.0, -150.0];

    // O(1) path: accumulators kept consistent with the history (as `push` does). ~keep
    let mut a = TextExtractor::new();
    let mut sum = 0.0f64;
    let mut sq = 0.0f64;
    for &v in &vals {
        let x = v as f64;
        sum += x;
        sq += x * x;
        a.tj_offset_history.push(v);
    }
    a.tj_sum = sum;
    a.tj_sum_sq = sq;
    a.tj_stats_len = a.tj_offset_history.len();
    let (ja, cva) = a.analyze_tj_distribution();

    // Recompute path: only the history is set (stale accumulators). ~keep
    let mut b = TextExtractor::new();
    b.tj_offset_history = vals.clone();
    let (jb, cvb) = b.analyze_tj_distribution();

    assert_eq!(ja, jb, "is_justified must agree across paths");
    assert!((cva - cvb).abs() < 1e-6, "O(1) cv {cva} must equal recompute cv {cvb}");
}

#[test]
fn test_adaptive_threshold_disabled() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: false,
        space_insertion_threshold: -120.0,
        ..TextExtractionConfig::default()
    };
    let extractor = TextExtractor::with_config(config);
    let threshold = extractor.calculate_adaptive_tj_threshold();
    assert_eq!(threshold, -120.0);
}

#[test]
fn test_adaptive_threshold_enabled() {
    let config = TextExtractionConfig {
        use_adaptive_tj_threshold: true,
        word_margin_ratio: 0.1,
        ..TextExtractionConfig::default()
    };
    let mut extractor = TextExtractor::with_config(config);
    extractor.state_stack.current_mut().font_size = 12.0;
    let threshold = extractor.calculate_adaptive_tj_threshold();
    assert!(threshold < 0.0, "Adaptive threshold should be negative");
}

#[test]
fn test_update_artifact_state_empty_stack() {
    let mut extractor = TextExtractor::new();
    extractor.update_artifact_state();
    assert!(!extractor.inside_artifact);
}

#[test]
fn test_update_artifact_state_artifact_present() {
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Artifact".to_string(),
        is_artifact: true,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_artifact_state();
    assert!(extractor.inside_artifact);
}

#[test]
fn test_placed_pdf_suppresses_content() {
    // Text inside an InDesign /PlacedPDF figure region (the placed
    // artwork's own glyphs — e.g. a draft galley) must be suppressed,
    // matching pdftotext/PyMuPDF. Entering a /PlacedPDF BDC sets
    // inside_placed_pdf, which feeds is_content_suppressed(). ~keep
    let mut extractor = TextExtractor::new();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());

    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "PlacedPDF".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: true,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_layer_state();
    assert!(extractor.inside_placed_pdf);
    assert!(
        extractor.is_content_suppressed(),
        "text inside /PlacedPDF must be suppressed"
    );

    extractor.marked_content_stack.pop();
    extractor.update_layer_state();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());
}

#[test]
fn test_non_placed_pdf_tag_does_not_suppress() {
    // A regular (non-PlacedPDF) marked-content tag such as /Figure must
    // NOT suppress its text — only the placed-PDF wrapper does. ~keep
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Figure".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_layer_state();
    assert!(!extractor.inside_placed_pdf);
    assert!(!extractor.is_content_suppressed());
}

#[test]
fn test_placed_pdf_kept_when_it_is_the_whole_page_body() {
    // A publisher that places the ENTIRE article body inside one /PlacedPDF
    // region (e.g. MATEC Web of Conferences) leaves almost nothing outside.
    // There the placed text IS the page's logical content and must NOT be
    // suppressed (pymupdf/pdftotext extract it). The coverage pre-scan flags
    // this: placed text dominates, non-placed text is a tiny header. ~keep
    let body = "(This is the full article body typeset inside a placed PDF region) Tj\n".repeat(20);
    let stream = format!("/PlacedPDF BMC\nBT\n{body}ET\nEMC\nBT (Journal vol 1) Tj ET\n");
    assert!(
        TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "whole-body /PlacedPDF must be KEPT (not suppressed)"
    );
}

#[test]
fn test_placed_pdf_suppressed_when_minority_overlay() {
    // The decorative-figure case (PMC8100493): a small /PlacedPDF galley
    // duplicate sits amid a full page of real text OUTSIDE it. The placed
    // text is the minority, so it stays suppressed (the de-dup win). ~keep
    let outside = "(Real published paragraph of the article that lives outside the placed region) Tj\n".repeat(20);
    let stream = format!("BT\n{outside}ET\n/PlacedPDF BMC\nBT (draft galley) Tj ET\nEMC\n");
    assert!(
        !TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "minority-overlay /PlacedPDF must stay suppressed"
    );
}

#[test]
fn test_placed_pdf_coverage_noop_without_tag() {
    // No /PlacedPDF tag anywhere: the pre-scan must short-circuit to false
    // (keep the default suppression state; pay nothing for ordinary pages). ~keep
    let stream = b"BT (ordinary single column page of text) Tj ET\n";
    assert!(!TextExtractor::placed_pdf_text_dominates(stream));
}

#[test]
fn test_placed_pdf_kept_when_unique_body_amid_comparable_outside() {
    // Gate 3: an InDesign spread (e.g. a placed floor-plan / marketing page)
    // where the placed region carries a substantial body of UNIQUE text and
    // the non-placed text is comparable or larger but different (labels,
    // headers). The 3:1 dominance ratio fails, yet the placed words are not a
    // duplicate of the outside text, so it must be KEPT (pdftotext/pymupdf
    // extract it; suppressing it drops the whole spread's content). ~keep
    let placed = "(master bedroom terrace kitchen dimensions balcony) Tj\n".repeat(30);
    let outside = "(square footage residence penthouse skyline waterfront) Tj\n".repeat(35);
    let stream = format!("BT\n{outside}ET\n/PlacedPDF /MC0 BDC\nBT\n{placed}ET\nEMC\n");
    assert!(
        TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "unique placed body amid comparable outside text must be KEPT"
    );
}

#[test]
fn test_placed_pdf_suppressed_when_large_duplicate_overlay() {
    // Gate 3, the other side: a large placed region whose words DUPLICATE the
    // surrounding text is a draft galley / overlay copy and stays suppressed
    // even though it clears the size gate (the PMC8100493 de-dup intent, at
    // full body size rather than the minority-overlay size). ~keep
    let body = "(the published paragraph of the real article body content) Tj\n".repeat(30);
    let stream = format!("BT\n{body}ET\n/PlacedPDF /MC0 BDC\nBT\n{body}ET\nEMC\n");
    assert!(
        !TextExtractor::placed_pdf_text_dominates(stream.as_bytes()),
        "a full-size placed DUPLICATE of the outside text must stay suppressed"
    );
}

#[test]
fn test_update_artifact_state_nested_non_artifact() {
    let mut extractor = TextExtractor::new();
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Artifact".to_string(),
        is_artifact: true,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.marked_content_stack.push(MarkedContentContext {
        artifact_type: None,
        tag: "Span".to_string(),
        is_artifact: false,
        actual_text: None,
        expansion: None,
        is_excluded_layer: false,
        is_placed_pdf: false,
        actual_text_emitted: false,
        own_mcid: None,
    });
    extractor.update_artifact_state();
    // Should still be inside artifact because parent is artifact ~keep
    assert!(extractor.inside_artifact);
}

#[test]
fn test_parse_artifact_type_page() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Page".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Page));
}

#[test]
fn test_parse_artifact_type_pagination_page_number() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Pagination".to_string()));
    props.insert("Subtype".to_string(), Object::Name("PageNumber".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::PageNumber)));
}

#[test]
fn test_parse_artifact_type_pagination_other_subtype() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("Pagination".to_string()));
    props.insert("Subtype".to_string(), Object::Name("SomethingElse".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Other)));
}

#[test]
fn test_parse_artifact_type_unknown_type() {
    let mut props = HashMap::new();
    props.insert("Type".to_string(), Object::Name("UnknownType".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, None);
}

#[test]
fn test_parse_artifact_type_subtype_footer_only() {
    let mut props = HashMap::new();
    props.insert("Subtype".to_string(), Object::Name("Footer".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Footer)));
}

#[test]
fn test_parse_artifact_type_subtype_watermark_only() {
    let mut props = HashMap::new();
    props.insert("Subtype".to_string(), Object::Name("Watermark".to_string()));
    let result = TextExtractor::parse_artifact_type(&props);
    assert_eq!(result, Some(ArtifactType::Pagination(PaginationSubtype::Watermark)));
}

#[test]
fn test_decode_pdf_text_string_utf8() {
    let result = TextExtractor::decode_pdf_text_string(b"Hello World");
    assert_eq!(result, "Hello World");
}

#[test]
fn test_decode_pdf_text_string_utf16be_bom() {
    // UTF-16BE with BOM: FE FF, then "Hi" in UTF-16BE ~keep
    let bytes: Vec<u8> = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hi");
}

#[test]
fn test_decode_pdf_text_string_utf16le_bom() {
    // UTF-16LE with BOM: FF FE, then "Hi" in UTF-16LE ~keep
    let bytes: Vec<u8> = vec![0xFF, 0xFE, 0x48, 0x00, 0x69, 0x00];
    let result = TextExtractor::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hi");
}

#[test]
fn test_decode_pdf_text_string_empty() {
    let result = TextExtractor::decode_pdf_text_string(b"");
    assert_eq!(result, "");
}

#[test]
fn test_is_ligature_code() {
    assert!(TextExtractor::is_ligature_code(0xFB00)); // ff ~keep
    assert!(TextExtractor::is_ligature_code(0xFB01)); // fi ~keep
    assert!(TextExtractor::is_ligature_code(0xFB02)); // fl ~keep
    assert!(TextExtractor::is_ligature_code(0xFB03)); // ffi ~keep
    assert!(TextExtractor::is_ligature_code(0xFB04)); // ffl ~keep
}

#[test]
fn test_is_not_ligature_code() {
    assert!(!TextExtractor::is_ligature_code(0x41));
    assert!(!TextExtractor::is_ligature_code(0xFAFF)); // Before range ~keep
    assert!(!TextExtractor::is_ligature_code(0xFB05)); // After range ~keep
}

#[test]
fn test_bt_resets_text_matrix() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (A) Tj ET BT /F1 12 Tf (B) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 2);
    assert_eq!(chars[0].char, 'A');
    assert_eq!(chars[1].char, 'B');
    assert!(
        chars[1].bbox.x < 10.0,
        "Second BT should reset text matrix, x={}",
        chars[1].bbox.x
    );
}

#[test]
fn test_multiple_bt_et_blocks() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET BT /F1 12 Tf 100 680 Td (World) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Hello"), "Should contain Hello");
    assert!(text.contains("World"), "Should contain World");
}

#[test]
fn test_bmc_artifact_tracking() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    extractor
        .execute_operator_public(crate::content::operators::Operator::BeginMarkedContent {
            tag: "Artifact".to_string(),
        })
        .unwrap();

    assert!(
        extractor.inside_artifact,
        "Should be inside artifact after BMC Artifact"
    );

    extractor
        .execute_operator_public(crate::content::operators::Operator::EndMarkedContent)
        .unwrap();

    assert!(!extractor.inside_artifact, "Should be outside artifact after EMC");
}

#[test]
fn test_bmc_non_artifact() {
    let mut extractor = TextExtractor::new();

    extractor
        .execute_operator_public(crate::content::operators::Operator::BeginMarkedContent {
            tag: "Span".to_string(),
        })
        .unwrap();

    assert!(
        !extractor.inside_artifact,
        "Non-artifact BMC should not set inside_artifact"
    );
}

#[test]
fn test_font_switch_mid_stream() {
    let mut extractor = TextExtractor::new();
    let font1 = create_test_font();
    let mut font2_data = create_test_font();
    font2_data.base_font = "Helvetica".to_string();
    extractor.add_font("F1".to_string(), font1);
    extractor.add_font("F2".to_string(), font2_data);

    let stream = b"BT /F1 12 Tf 100 700 Td (Hello) Tj /F2 14 Tf (World) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    let text: String = chars.iter().map(|c| c.char).collect();
    assert!(text.contains("Hello"), "Should contain Hello");
    assert!(text.contains("World"), "Should contain World");
}

#[test]
fn test_font_switch_same_font_no_flush() {
    // Setting the same font twice should be a no-op (optimization) ~keep
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"BT /F1 12 Tf /F1 12 Tf 100 700 Td (Test) Tj ET";
    let spans = extractor.extract_text_spans(stream).unwrap();

    let text: String = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("");
    assert!(text.contains("Test"), "Should extract text, got: {}", text);
}

#[test]
fn test_cm_operator_translation() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"1 0 0 1 50 100 cm BT /F1 12 Tf (X) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!((chars[0].bbox.x - 50.0).abs() < 2.0, "X should be ~50");
    assert!((chars[0].bbox.y - 100.0).abs() < 2.0, "Y should be ~100");
}

#[test]
fn test_cm_operator_scaling() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    let stream = b"2 0 0 2 0 0 cm BT /F1 12 Tf 1 0 0 1 50 100 Tm (Y) Tj ET";
    let chars = extractor.extract(stream).unwrap();

    assert_eq!(chars.len(), 1);
    assert!(
        (chars[0].bbox.x - 100.0).abs() < 2.0,
        "X should be ~100 (got {})",
        chars[0].bbox.x
    );
}

#[test]
fn test_deduplicate_overlapping_chars() {
    let mut extractor = TextExtractor::new();
    let font = create_test_font();
    extractor.add_font("F1".to_string(), font);

    // Create overlapping chars (simulating bold rendering with duplicate glyphs) ~keep
    extractor.chars = vec![
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.5, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.5,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(extractor.chars.len(), 1, "Overlapping chars should be deduplicated");
}

#[test]
fn test_deduplicate_overlapping_chars_different_lines() {
    let mut extractor = TextExtractor::new();

    extractor.chars = vec![
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 700.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 700.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
        TextChar {
            char: 'A',
            bbox: Rect::new(100.0, 680.0, 6.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            is_italic: false,
            is_monospace: false,
            origin_x: 100.0,
            origin_y: 680.0,
            rotation_degrees: 0.0,
            advance_width: 6.0,
            rendered_advance: 6.0,
            ascent: 11.4,
            descent: -4.2,
            matrix: None,
        },
    ];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        2,
        "Chars on different lines should not be deduplicated"
    );
}

#[test]
fn test_deduplicate_overlapping_chars_empty() {
    let mut extractor = TextExtractor::new();
    extractor.deduplicate_overlapping_chars();
    assert!(extractor.chars.is_empty());
}

#[test]
fn test_deduplicate_keeps_distinct_close_chars() {
    // Distinct characters close together should NOT be dropped ~keep
    let mut extractor = TextExtractor::new();

    let make_char = |c: char, x: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, 6.0, 12.0),
        font_name: "F1".to_string(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 6.0,
        rendered_advance: 6.0,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    // 't' at x=100, ' ' at x=105, 'r' at x=106.5 (within 2pt of ' ' but different char) ~keep
    extractor.chars = vec![make_char('t', 100.0), make_char(' ', 105.0), make_char('r', 106.5)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        3,
        "Distinct characters close together must not be dropped"
    );
    assert_eq!(extractor.chars[0].char, 't');
    assert_eq!(extractor.chars[1].char, ' ');
    assert_eq!(extractor.chars[2].char, 'r');
}

#[test]
fn test_deduplicate_still_removes_same_char_duplicates() {
    let mut extractor = TextExtractor::new();

    let make_char = |c: char, x: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, 6.0, 12.0),
        font_name: "F1".to_string(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 6.0,
        rendered_advance: 6.0,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    extractor.chars = vec![make_char('A', 100.0), make_char('A', 100.5)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(extractor.chars.len(), 1, "Duplicate same char should still be deduped");
    assert_eq!(extractor.chars[0].char, 'A');
}

#[test]
fn test_deduplicate_keeps_narrow_glyph_doublets() {
    // Regression: `ll`, `rr`, `II`, `ii` in small-font body text were
    // wrongly collapsed to a single glyph because the dedup threshold
    // was a hardcoded 2 pt — larger than the advance width of narrow
    // glyphs at ≤ 9 pt in most fonts (Helvetica `l` ≈ 2.5 pt at 9 pt,
    // smaller below). This caused visible corruption like
    // `controller → controler` and `billed → biled`. ~keep
    //
    // Exercises the matrix of four narrow glyphs across three small
    // body-text sizes. Advance widths are the real Helvetica per-em
    // values (0.278 em for `l`/`i`, 0.333 em for `r`, 0.278 em for `I`). ~keep
    let narrow_char = |c: char, x: f32, font_size: f32, advance_em: f32| TextChar {
        char: c,
        bbox: Rect::new(x, 700.0, advance_em * font_size * 0.6, font_size),
        font_name: "Helvetica".to_string(),
        font_size,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: advance_em * font_size,
        rendered_advance: advance_em * font_size,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    let cases: &[(char, f32)] = &[('l', 0.278), ('r', 0.333), ('I', 0.278), ('i', 0.278)];
    // Body-text sizes where narrow-glyph advance falls at or below 2 pt. ~keep
    let sizes: &[f32] = &[7.0, 9.0, 11.0];

    for &(glyph, advance_em) in cases {
        for &font_size in sizes {
            let advance = advance_em * font_size;
            let mut extractor = TextExtractor::new();
            extractor.chars = vec![
                narrow_char(glyph, 100.0, font_size, advance_em),
                narrow_char(glyph, 100.0 + advance, font_size, advance_em),
            ];

            extractor.deduplicate_overlapping_chars();
            assert_eq!(
                extractor.chars.len(),
                2,
                "Adjacent narrow-glyph doublet ('{glyph}{glyph}') at {font_size} pt \
                     (advance = {advance:.2} pt) must not be collapsed",
            );
        }
    }
}

#[test]
fn test_deduplicate_still_collapses_narrow_glyph_stroke_fill_duplicates() {
    // Positive regression: even with the advance-scaled threshold,
    // stroke+fill render passes on narrow glyphs (two `l`s at ~0 pt
    // offset) must still be collapsed. The ratio (0.30) comfortably
    // catches real duplicates (< 5 % of one advance apart) while
    // staying below typical heaviest kerning (~20 %). ~keep
    let mut extractor = TextExtractor::new();

    let narrow_at = |x: f32| TextChar {
        char: 'l',
        bbox: Rect::new(x, 700.0, 1.5, 9.0),
        font_name: "Helvetica".to_string(),
        font_size: 9.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        is_italic: false,
        is_monospace: false,
        origin_x: x,
        origin_y: 700.0,
        rotation_degrees: 0.0,
        advance_width: 2.5, // 0.278 em × 9 pt ~keep
        rendered_advance: 2.5,
        ascent: 11.4,
        descent: -4.2,
        matrix: None,
    };

    // Stroke pass and fill pass typically land within 0.05 pt of each
    // other (2 % of advance at 9 pt Helvetica `l`). ~keep
    extractor.chars = vec![narrow_at(100.0), narrow_at(100.05)];

    extractor.deduplicate_overlapping_chars();
    assert_eq!(
        extractor.chars.len(),
        1,
        "Stroke+fill narrow-glyph duplicates (same char at ~0 pt offset) \
             must still be collapsed"
    );
}

#[test]
fn test_deduplicate_overlapping_spans_geometric() {
    let mut extractor = TextExtractor::new();
    extractor.spans = vec![
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(100.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 0,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
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
            mirrored: false,
            page_rotation_applied: 0,
        },
        TextSpan {
            provenance: None,
            text_rise: 0.0,
            artifact_type: None,
            text: "Hello".to_string(),
            bbox: Rect::new(101.0, 700.0, 30.0, 12.0),
            font_name: "F1".to_string(),
            font_size: 12.0,
            font_weight: FontWeight::Normal,
            color: Color::black(),
            mcid: None,
            mcid_scope: None,
            sequence: 1,
            split_boundary_before: false,
            offset_semantic: false,
            is_italic: false,
            is_monospace: false,
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
            mirrored: false,
            page_rotation_applied: 0,
        },
    ];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(extractor.spans.len(), 1, "Geometric duplicates should be removed");
}

#[test]
fn test_deduplicate_overlapping_spans_empty() {
    let mut extractor = TextExtractor::new();
    extractor.deduplicate_overlapping_spans();
    assert!(extractor.spans.is_empty());
}

#[test]
fn test_deduplicate_spans_keeps_narrow_glyph_doublets() {
    // Regression: PDFs that emit kerned text glyph-by-glyph produce
    // consecutive single-character spans. Two adjacent narrow-glyph
    // spans (`l`, `r`, `I`, `i` at ≤ 9 pt) sit roughly one advance-width
    // apart, which used to fall under the hardcoded 2 pt geometric
    // threshold and get collapsed. The threshold now scales with each
    // span's per-glyph width so legitimate doublets survive.
    //
    // Exercises the matrix of four narrow glyphs across three small
    // body-text sizes. ~keep
    let narrow_span = |glyph: char, x: f32, font_size: f32, advance: f32, seq: usize| TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: glyph.to_string(),
        bbox: Rect::new(x, 700.0, advance, font_size),
        font_name: "Helvetica".to_string(),
        font_size,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: seq,
        split_boundary_before: false,
        offset_semantic: false,
        is_italic: false,
        is_monospace: false,
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
        mirrored: false,
        page_rotation_applied: 0,
    };

    // (glyph, Helvetica per-em advance width) ~keep
    let cases: &[(char, f32)] = &[('l', 0.278), ('r', 0.333), ('I', 0.278), ('i', 0.278)];
    let sizes: &[f32] = &[7.0, 9.0, 11.0];

    for &(glyph, advance_em) in cases {
        for &font_size in sizes {
            let advance = advance_em * font_size;
            let mut extractor = TextExtractor::new();
            extractor.spans = vec![
                narrow_span(glyph, 100.0, font_size, advance, 0),
                narrow_span(glyph, 100.0 + advance, font_size, advance, 1),
            ];

            extractor.deduplicate_overlapping_spans();
            assert_eq!(
                extractor.spans.len(),
                2,
                "Adjacent single-glyph narrow-doublet spans ('{glyph}{glyph}') \
                     at {font_size} pt (advance = {advance:.2} pt) must not be collapsed",
            );
        }
    }
}

#[test]
fn test_deduplicate_spans_still_collapses_stroke_fill_narrow_glyphs() {
    // Positive regression: stroke+fill single-glyph narrow spans at
    // ~0 pt offset must still be collapsed by the geometric dedup
    // phase. The ratio (0.30) comfortably catches real duplicates
    // while preserving legitimate doublets. ~keep
    let mut extractor = TextExtractor::new();

    let narrow_at = |x: f32, seq: usize| TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: "l".to_string(),
        bbox: Rect::new(x, 700.0, 2.5, 9.0),
        font_name: "Helvetica".to_string(),
        font_size: 9.0,
        font_weight: FontWeight::Normal,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: seq,
        split_boundary_before: false,
        offset_semantic: false,
        is_italic: false,
        is_monospace: false,
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
        mirrored: false,
        page_rotation_applied: 0,
    };

    // Stroke pass + fill pass at ~2 % of advance apart. ~keep
    extractor.spans = vec![narrow_at(100.0, 0), narrow_at(100.05, 1)];

    extractor.deduplicate_overlapping_spans();
    assert_eq!(
        extractor.spans.len(),
        1,
        "Stroke+fill narrow-glyph duplicate spans (same text at ~0 pt offset) \
             must still be collapsed"
    );
}

#[test]
fn test_detect_span_columns_empty() {
    let extractor = TextExtractor::new();
    let columns = extractor.detect_span_columns();
    assert!(columns.is_empty());
}
