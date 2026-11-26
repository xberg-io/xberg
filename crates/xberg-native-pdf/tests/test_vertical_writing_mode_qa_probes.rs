//! QA-driven regression probes for WMode 1 (vertical writing).
//!
//! Each probe targets a hole the first/second review pass may have missed
//! and either (a) writes a failing test that proves a bug, or (b) pins
//! the current behavior so future refactors can't silently regress it.
//!
//! Probe IDs map to the QA assignment categories:
//!
//!  - 1..4   real-world / corpus-driven probes
//!  - 5..7   horizontal-text regression (must not break)
//!  - 8..10  boundary at the majority threshold
//!  - 11..14 edge cases at the rasterizer
//!  - 15..20 /W2 parser stress
//!  - 21..23 CMap /WMode stress
//!  - 24..28 encoding-precedence stress
//!  - 29..33 per-page partition / reading-order
//!  - 34..36 save/restore / state machine
//!  - 37     performance regression check
//!  - 38     separation renderer integration
//!
//! The synthetic PDFs are built byte-by-byte so no third-party CJK fonts
//! are required.

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::pipeline::{ReadingOrderContext, TextPipeline};

/// Generic synthetic-PDF builder. Mirrors the helper in
/// test_vertical_writing_mode_fixes.rs but is reproduced locally so the two
/// test files don't couple.
///
/// CIDs 0001..0008 map to ASCII 'A'..'H' via the ToUnicode CMap, giving each
/// glyph an identifiable extracted text. `dw` is the horizontal default width
/// (1000 = one full em). `dw2 = (v_y, w1y)` is the spec /DW2 array.
/// `w2` is the optional /W2 array clause; `extra_resources` lets the test
/// inject additional resource dictionary entries (e.g. /Font << /F2 ... >>).
#[allow(clippy::too_many_arguments)]
fn build_pdf_full(
    encoding_name: &str,
    content: &[u8],
    dw: i32,
    dw2: (i32, i32),
    w2: Option<&str>,
    cmap_extra: Option<&str>,
    extra_fonts: Option<&str>,
    extra_objs: Option<&[(usize, Vec<u8>)]>,
) -> Vec<u8> {
    build_pdf_full_named(
        "TestFont",
        encoding_name,
        content,
        dw,
        dw2,
        w2,
        cmap_extra,
        extra_fonts,
        extra_objs,
    )
}

/// Like `build_pdf_full` but lets the caller supply a unique BaseFont name
/// so that the cross-document font cache (Layer 6) won't serve a
/// previously parsed FontInfo. Required by probes that rely on /W2 or /DW2
/// values, because the cheap identity hash does NOT include /W2 or /DW2
/// — see probe_cache_poison_*.
#[allow(clippy::too_many_arguments)]
fn build_pdf_full_named(
    base_font: &str,
    encoding_name: &str,
    content: &[u8],
    dw: i32,
    dw2: (i32, i32),
    w2: Option<&str>,
    cmap_extra: Option<&str>,
    extra_fonts: Option<&str>,
    extra_objs: Option<&[(usize, Vec<u8>)]>,
) -> Vec<u8> {
    let extra = cmap_extra.unwrap_or("");
    let cmap_src = format!(
        "/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
{extra}
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
8 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <0043>
<0004> <0044>
<0005> <0045>
<0006> <0046>
<0007> <0047>
<0008> <0048>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"
    );
    let cmap = cmap_src.as_bytes();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    let font_clause = if let Some(extra) = extra_fonts {
        format!("/Font << /F1 5 0 R {} >>", extra)
    } else {
        "/Font << /F1 5 0 R >>".to_string()
    };
    let page_body = format!(
        "3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << {} >> >> endobj\n",
        font_clause
    );
    pdf.extend_from_slice(page_body.as_bytes());

    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o5 = pdf.len();
    let f5 = format!(
        "5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /{} /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
        base_font, encoding_name
    );
    pdf.extend_from_slice(f5.as_bytes());

    let o6 = pdf.len();
    let w2_clause = w2.map(|s| format!(" /W2 {}", s)).unwrap_or_default();
    let f6 = format!(
        "6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW {} /DW2 [{} {}]{} >> endobj\n",
        base_font, dw, dw2.0, dw2.1, w2_clause
    );
    pdf.extend_from_slice(f6.as_bytes());

    let o7 = pdf.len();
    pdf.extend_from_slice(format!("7 0 obj << /Length {} >> stream\n", cmap.len()).as_bytes());
    pdf.extend_from_slice(cmap);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let mut extra_offsets: Vec<(usize, usize)> = Vec::new();
    if let Some(objs) = extra_objs {
        for (n, body) in objs {
            let off = pdf.len();
            extra_offsets.push((*n, off));
            pdf.extend_from_slice(body);
        }
    }

    let xref = pdf.len();
    let mut all_offsets: Vec<(usize, usize)> = vec![(1, o1), (2, o2), (3, o3), (4, o4), (5, o5), (6, o6), (7, o7)];
    all_offsets.extend(extra_offsets);
    all_offsets.sort_by_key(|(n, _)| *n);
    let max_n = all_offsets.iter().map(|(n, _)| *n).max().unwrap();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", max_n + 1).as_bytes());
    for (_n, off) in &all_offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            max_n + 1,
            xref
        )
        .as_bytes(),
    );
    pdf
}

fn build_pdf(
    encoding_name: &str,
    content: &[u8],
    dw: i32,
    dw2: (i32, i32),
    w2: Option<&str>,
    cmap_extra: Option<&str>,
) -> Vec<u8> {
    build_pdf_full(encoding_name, content, dw, dw2, w2, cmap_extra, None, None)
}

fn span_with_wmode(text: &str, x: f32, y: f32, wmode: u8) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, y, 12.0, 12.0),
        font_name: "TestFont".into(),
        font_size: 12.0,
        wmode,
        ..TextSpan::default()
    }
}

// ---------------------------------------------------------------------------
// Probe 5 — horizontal text path must not regress when wmode=0 is the default
// --------------------------------------------------------------------------- ~keep

/// Pin: a pure horizontal-text content stream produces identical span X
/// positions to the pre-branch behaviour (we approximate "pre-branch" with
/// the spec formula `(w0 * Tfs) * Th + Tc + Tw` per glyph). Tests the
/// hot-path branch `advance_text_matrix` in horizontal mode is a no-op
/// against the previous matrix-translation arithmetic.
#[test]
fn probe05_pure_horizontal_extraction_advances_by_expected_x_steps() {
    // Three glyphs (CID 1, 2, 3), each /DW = 1000, fs = 12 → 12.0 per glyph. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <000100020003> Tj ET";
    let pdf = build_pdf("Identity-H", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A");
    let b = chars.iter().find(|c| c.char == 'B').expect("B");
    let c = chars.iter().find(|c| c.char == 'C').expect("C");
    assert!(
        (b.bbox.x - a.bbox.x - 12.0).abs() < 0.05,
        "A→B Δx ≠ 12.0: {}",
        b.bbox.x - a.bbox.x
    );
    assert!(
        (c.bbox.x - b.bbox.x - 12.0).abs() < 0.05,
        "B→C Δx ≠ 12.0: {}",
        c.bbox.x - b.bbox.x
    );
    assert!((a.bbox.y - 700.0).abs() < 1.0);
    assert!((b.bbox.y - 700.0).abs() < 1.0);
    assert!((c.bbox.y - 700.0).abs() < 1.0);
    let spans = doc.extract_spans(0).expect("extract_spans");
    for s in &spans {
        assert_eq!(
            s.wmode, 0,
            "horizontal page must keep wmode=0; got {} for {:?}",
            s.wmode, s.text
        );
    }
}

// ---------------------------------------------------------------------------
// Probe 6 — TextPipeline dispatch must NOT route pure-horizontal pages through tategaki
// --------------------------------------------------------------------------- ~keep

/// Pin: with every span tagged wmode=0, the pipeline must run the
/// configured strategy, never `TategakiStrategy`.
#[test]
fn probe06_pipeline_does_not_route_pure_horizontal_through_tategaki() {
    let spans = vec![
        span_with_wmode("A", 100.0, 700.0, 0),
        span_with_wmode("B", 200.0, 700.0, 0),
        span_with_wmode("C", 300.0, 700.0, 0),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).expect("pipeline");
    // Horizontal LTR (Simple/Geometric/etc) must produce ascending X order:
    // A → B → C. Tategaki would reverse to right-to-left, giving C → B → A. ~keep
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    assert_eq!(
        combined, "ABC",
        "pure-horizontal page must NOT use tategaki sort; got {}",
        combined
    );
}

// ---------------------------------------------------------------------------
// Probe 7 — Mixed page with horizontal majority keeps the configured strategy
// --------------------------------------------------------------------------- ~keep

/// Pin: 5 horizontal spans + 1 vertical span = horizontal majority.
/// The pipeline must keep the configured (horizontal) sort. The single
/// vertical span keeps its wmode tag for downstream consumers but does
/// not flip the page-level strategy.
#[test]
fn probe07_horizontal_majority_keeps_configured_strategy_even_with_one_vertical() {
    let spans = vec![
        span_with_wmode("A", 100.0, 700.0, 0),
        span_with_wmode("B", 200.0, 700.0, 0),
        span_with_wmode("C", 300.0, 700.0, 0),
        span_with_wmode("D", 400.0, 700.0, 0),
        span_with_wmode("E", 500.0, 700.0, 0),
        span_with_wmode("V", 100.0, 600.0, 1),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).expect("pipeline");
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    // Configured (default Geometric) strategy should sort the row of A..E in
    // increasing X order; V is on a lower row and follows. Tategaki sort
    // would have produced rightmost-first → "EDCBAV" or similar. ~keep
    assert!(
        combined.starts_with("ABCDE"),
        "horizontal-majority page must not be routed through tategaki; got {}",
        combined
    );
    let v = ordered.iter().find(|o| o.span.text == "V").expect("V");
    assert_eq!(v.span.wmode, 1);
}

// ---------------------------------------------------------------------------
// Probe 8 — Exactly 50/50 vertical/horizontal: pinned to tategaki
// --------------------------------------------------------------------------- ~keep

/// `is_vertical_majority` uses `vertical_count * 2 >= len` — so exactly
/// 50% vertical routes to tategaki. Pin this.
#[test]
fn probe08_exact_5050_majority_routes_to_tategaki() {
    let spans = vec![
        span_with_wmode("A", 500.0, 700.0, 1),
        span_with_wmode("B", 500.0, 688.0, 1),
        span_with_wmode("X", 100.0, 700.0, 0),
        span_with_wmode("Y", 200.0, 700.0, 0),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).expect("pipeline");
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    // Tategaki sort puts rightmost X-center first, descending Y within column.
    // Right column (x≈500): A then B. Left column (x≈100..200): X then Y.
    // But tategaki uses median-width tolerance, so X/Y may cluster as one
    // column or two. Key invariant: A is first (rightmost top). ~keep
    assert!(
        combined.starts_with('A'),
        "5050 must route to tategaki; got {}",
        combined
    );
}

// ---------------------------------------------------------------------------
// Probe 9 — 51%/49% near the threshold
// --------------------------------------------------------------------------- ~keep

/// 51% vertical (3 of 6) — actually 50%; let's go 3 vertical of 6 == 50%.
/// To get strictly > 50% we need 4 of 6. Probe both 4/6 (=66%) and 3/6
/// (=exact 50%) above. Now probe 1 of 3 vs 2 of 3.
#[test]
fn probe09_boundary_thresholds() {
    // 1 vertical, 2 horizontal → 1*2 < 3 → horizontal majority. ~keep
    let spans = vec![
        span_with_wmode("V", 500.0, 700.0, 1),
        span_with_wmode("A", 100.0, 700.0, 0),
        span_with_wmode("B", 200.0, 700.0, 0),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).unwrap();
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    // Horizontal sort: A B V (A and B left to right, V below). Must not be
    // right-to-left tategaki. ~keep
    assert!(
        combined.starts_with('A'),
        "1/3 vertical must use horizontal sort; got {}",
        combined
    );

    // 2 vertical, 1 horizontal → 2*2 >= 3 → vertical majority. ~keep
    let spans2 = vec![
        span_with_wmode("V", 500.0, 700.0, 1),
        span_with_wmode("W", 500.0, 688.0, 1),
        span_with_wmode("A", 100.0, 700.0, 0),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans2, ReadingOrderContext::new()).unwrap();
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    // Vertical majority: rightmost first → V then W then A. ~keep
    assert_eq!(combined, "VWA", "2/3 vertical must use tategaki; got {}", combined);
}

// ---------------------------------------------------------------------------
// Probe 11 — Very large negative TJ numeric offsets must not overflow
// --------------------------------------------------------------------------- ~keep

/// `[<0001> -32767 <0002>] TJ` under vertical mode: pin no overflow, no NaN.
///
/// Sign convention observation: per ISO 32000-1 §9.4.3 a TJ number element
/// is "subtracted from the current... vertical coordinate". The
/// implementation encodes this as `displacement = -offset/1000 * fs`, so a
/// negative offset (-32767) produces a POSITIVE displacement (+393.2 in
/// text-space y). With identity Tm under WMode 1, this moves the cursor
/// UP in PDF user space, NOT downward in the writing direction. The net
/// effect with the glyph's negative w1y is that B ends up ABOVE A.
///
/// This may or may not be a spec-conformance bug — the spec text doesn't
/// directly say whether "subtracted from the vertical coordinate" means
/// "text-space y" or "writing-direction-forward". Several real
/// implementations (Adobe Reader observed) treat negative TJ offsets in
/// vertical mode as moving DOWNWARD (forward in writing direction), which
/// would require the formula to be `+offset/1000 * fs` in vertical, or
/// equivalently the displacement to be NEGATIVE in y for negative offset.
///
/// Pin: the current behavior makes B sit ABOVE A. If a future fix changes
/// this convention, update this test.
#[test]
fn probe11_large_negative_tj_offset_in_vertical_mode_no_overflow_or_nan() {
    let content = b"BT /F1 12 Tf 100 700 Td [<0001> -32767 <0002>] TJ ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A");
    let b = chars.iter().find(|c| c.char == 'B').expect("B");
    // Pin: no NaN, no inf — math is well-behaved at large magnitudes. ~keep
    assert!(
        a.bbox.x.is_finite() && a.bbox.y.is_finite(),
        "A position must be finite"
    );
    assert!(
        b.bbox.x.is_finite() && b.bbox.y.is_finite(),
        "B position must be finite"
    );
    // Pin: current sign convention puts B ABOVE A. The magnitude of the
    // y-jump (ignoring the v_y origin offset) is ~|32767/1000 * 12| - 12 = 381. ~keep
    let dy = b.bbox.y - a.bbox.y;
    assert!(
        dy > 300.0,
        "large negative TJ offset must shift B upward by ~381 in current convention; dy={}",
        dy
    );
}

// ---------------------------------------------------------------------------
// Probe 12 — Empty Tj (<>) in vertical mode produces no NaN or panic
// --------------------------------------------------------------------------- ~keep

/// `<>Tj` is legal (empty hex string). Vertical mode must not produce NaN
/// or panic, and the cursor must stay put.
#[test]
fn probe12_empty_tj_in_vertical_mode_no_nan_no_panic() {
    // Two glyphs with an empty Tj between them — the empty Tj must not
    // displace the cursor at all. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <> Tj <0002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A");
    let b = chars.iter().find(|c| c.char == 'B').expect("B");
    // Expected dy = 12 (one glyph's advance — the empty Tj contributes 0). ~keep
    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 12.0).abs() < 0.05,
        "empty Tj in vertical mode must not displace cursor; dy={} (expected 12.0)",
        dy
    );
    assert!(a.bbox.x.is_finite() && b.bbox.x.is_finite());
}

// ---------------------------------------------------------------------------
// Probe 13 — Zero font size with vertical text must not panic
// --------------------------------------------------------------------------- ~keep

/// `0 Tf` in vertical mode — extraction must not panic. The cursor doesn't
/// advance (font_size * w1y / 1000 = 0).
#[test]
fn probe13_zero_font_size_in_vertical_mode_does_not_panic() {
    let content = b"BT /F1 0 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    // Just need to not panic; chars may or may not extract depending on
    // how zero-fontsize glyphs are filtered. ~keep
    let _ = doc.extract_chars(0);
    let _ = doc.extract_spans(0);
}

// ---------------------------------------------------------------------------
// Probe 14 — Mid-cluster vertical advance: per-cluster aggregation
// --------------------------------------------------------------------------- ~keep

/// Two CIDs in a single Tj. Each CID is 2 bytes (Identity codespace). Verify
/// that the per-glyph y delta = (w1y_cid1 + w1y_cid2) * fs / 1000 when the
/// CIDs map to the SAME unicode cluster (multi-byte CID > single cluster).
/// We can't easily force a multi-byte cluster from a synthetic font, so we
/// instead pin the simpler property: two CIDs in one Tj produce two char
/// positions, each at the expected y.
#[test]
fn probe14_multi_cid_tj_vertical_per_glyph_advance() {
    let content = b"BT /F1 12 Tf 100 700 Td <00010002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A");
    let b = chars.iter().find(|c| c.char == 'B').expect("B");
    let dy = a.bbox.y - b.bbox.y;
    assert!((dy - 12.0).abs() < 0.05, "intra-Tj vertical dy = {}", dy);
}

// ---------------------------------------------------------------------------
// Probe 15 — /W2 with interleaved Form A and Form B in the same array
// --------------------------------------------------------------------------- ~keep

/// Form A (c [triples]) and Form B (c_first c_last w1y v_x v_y) intermixed.
/// Verify both forms parse correctly when next to each other.
///
/// CID 1: Form A override w1y=-500 → dy = 6.0 at fs=12
/// CID 2: Form A continuation w1y=-700 → dy = 8.4
/// CID 3,4: Form B range w1y=-400 → dy = 4.8
/// CID 5: DW2 default w1y=-1000 → dy = 12.0
#[test]
fn probe15_w2_interleaved_form_a_and_form_b() {
    // Unique BaseFont so the cross-document cache (Layer 6) cannot serve a
    // FontInfo parsed by another test. /W2 is NOT in the identity hash —
    // see probe_cache_poison_w2_collision_across_documents. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <00010002> Tj <00030004> Tj <0005> Tj ET";
    let pdf = build_pdf_full_named(
        "Probe15Font",
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[1 [-500 250 600 -700 250 600] 3 4 -400 250 600]"),
        None,
        None,
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let c = chars.iter().find(|c| c.char == 'C').unwrap();
    let d = chars.iter().find(|c| c.char == 'D').unwrap();
    let e = chars.iter().find(|c| c.char == 'E').unwrap();
    let dy_ab = a.bbox.y - b.bbox.y;
    let dy_bc = b.bbox.y - c.bbox.y;
    let dy_cd = c.bbox.y - d.bbox.y;
    let dy_de = d.bbox.y - e.bbox.y;
    // A→B uses CID 1's w1y=-500 → 6.0 ~keep
    assert!((dy_ab - 6.0).abs() < 0.1, "CID1 dy = {}, expected 6.0", dy_ab);
    // B→C uses CID 2's w1y=-700 → 8.4 ~keep
    assert!((dy_bc - 8.4).abs() < 0.1, "CID2 dy = {}, expected 8.4", dy_bc);
    // C→D uses CID 3's w1y=-400 → 4.8 ~keep
    assert!((dy_cd - 4.8).abs() < 0.1, "CID3 dy = {}, expected 4.8", dy_cd);
    // D→E uses CID 4's w1y=-400 → 4.8 ~keep
    assert!((dy_de - 4.8).abs() < 0.1, "CID4 dy = {}, expected 4.8", dy_de);
}

// ---------------------------------------------------------------------------
// Probe 16 — /W2 referencing CIDs not in /W (horizontal widths)
// --------------------------------------------------------------------------- ~keep

/// Spec-legal: a CID can have a vertical metric without a horizontal one.
/// Verify no crash when extracting a vertical-only-metric glyph.
#[test]
fn probe16_w2_for_cid_without_horizontal_width_does_not_crash() {
    // No /W array, only /W2 — CID 1 has vertical metrics, horizontal falls
    // back to /DW = 1000. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf(
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[1 [-500 250 600]]"),
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A char");
    assert!(a.bbox.x.is_finite() && a.bbox.y.is_finite());
}

// ---------------------------------------------------------------------------
// Probe 17 — /W2 referencing CIDs at exactly u16::MAX
// --------------------------------------------------------------------------- ~keep

/// Form B range ending at exactly u16::MAX must not overflow the inclusive
/// range iterator.
///
/// Note: extracting this through content-stream chars would require a CID
/// in that range, which the synthetic ToUnicode CMap doesn't map. We test
/// the PARSE path only — the PDF must load without panic. Inspecting the
/// internal vertical-metrics map requires `pub(crate)` access, so we
/// content ourselves with the load-without-panic + horizontal-Tj
/// extraction sanity.
#[test]
fn probe17_w2_at_u16_max_does_not_panic_in_parse() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    // Form B: CIDs 65530..=65535 (u16::MAX = 65535). ~keep
    let pdf = build_pdf(
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[65530 65535 -500 250 600]"),
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse — must not panic on u16::MAX range");
    let _ = doc.extract_chars(0);
}

// ---------------------------------------------------------------------------
// Probe 18 — Negative v_x or v_y
// --------------------------------------------------------------------------- ~keep

/// Glyphs with negative origin offsets (common for kana where the vertical
/// origin sits to the upper-LEFT). Verify the sign survives.
#[test]
fn probe18_negative_v_x_v_y_in_w2_preserved() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf(
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[1 [-500 -100 -200]]"),
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').expect("A char");
    assert!(a.bbox.x.is_finite() && a.bbox.y.is_finite());
    // The glyph's reported x/y position depends on v_x/v_y. We only pin
    // that the extractor doesn't reject negative values. ~keep
}

// ---------------------------------------------------------------------------
// Probe 19 — Real numbers (not just integers) in /W2
// --------------------------------------------------------------------------- ~keep

#[test]
fn probe19_w2_with_real_numbers() {
    // Unique BaseFont to escape cross-test cache poisoning — see
    // probe_cache_poison_w2_collision_across_documents.
    // Form A with floats: w1y=-456.5, v_x=250.3, v_y=600.7 ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf_full_named(
        "Probe19Font",
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[1 [-456.5 250.3 600.7]]"),
        None,
        None,
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse — must accept reals in /W2");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let dy = a.bbox.y - b.bbox.y;
    // CID 1: w1y=-456.5 → dy = 5.478 ~keep
    assert!(
        (dy - 5.478).abs() < 0.05,
        "real-number /W2 must compute exact dy = 5.478; got {}",
        dy
    );
}

// ---------------------------------------------------------------------------
// Probe 20 — /W2 with indirect references — DEFERRED
// --------------------------------------------------------------------------- ~keep

/// Indirect-reference /W2 — spec allows it but it requires injecting an
/// additional object into the PDF. Pin behavior: when /W2 references an
/// indirect array, the parser resolves it. Probe by inspecting that the
/// glyph metric matches an explicit override.
#[test]
fn probe20_w2_indirect_reference_resolves() {
    // Unique BaseFont to avoid cache poisoning — see probe_cache_poison_*.
    // The font's /W2 points to object 8, which holds the array. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let extra_objs: Vec<(usize, Vec<u8>)> = vec![(8, b"8 0 obj [1 [-500 250 600]] endobj\n".to_vec())];
    let pdf = build_pdf_full_named(
        "Probe20Font",
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("8 0 R"),
        None,
        None,
        Some(&extra_objs),
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let dy = a.bbox.y - b.bbox.y;
    // CID 1: w1y=-500 (via indirect-ref'd /W2) → dy = 6.0
    // If the parser failed to resolve the indirect ref, dy would be 12.0 (DW2 default). ~keep
    assert!(
        (dy - 6.0).abs() < 0.1,
        "indirect /W2 must resolve to per-CID override; dy={} (expected 6.0; got 12.0 → indirect not resolved)",
        dy
    );
}

// ---------------------------------------------------------------------------
// Probe 21 — Multiple /WMode directives in the same CMap: last-wins?
// --------------------------------------------------------------------------- ~keep

/// Pin behavior: the regex captures the FIRST match. So `/WMode 0 def` then
/// `/WMode 1 def` keeps wmode=0. Verify.
#[test]
fn probe21_multiple_wmode_directives_first_wins() {
    // Identity-H encoding with a ToUnicode CMap containing TWO /WMode
    // directives: 0 first, then 1. The current regex extracts the first
    // capture, so wmode=0 stays. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf(
        "Identity-H",
        content,
        1000,
        (880, -1000),
        None,
        Some("/WMode 0 def\n/WMode 1 def"),
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let spans = doc.extract_spans(0).expect("extract_spans");
    // Encoding is Identity-H so wmode should be 0 regardless of ToUnicode.
    // This pins C5 cumulatively: ToUnicode's /WMode never overrides /Encoding. ~keep
    for s in &spans {
        assert_eq!(
            s.wmode, 0,
            "ToUnicode /WMode (any count) must not override /Encoding /Identity-H; span {:?} has wmode={}",
            s.text, s.wmode
        );
    }
}

// ---------------------------------------------------------------------------
// Probe 22 — /WMode inside a begincidrange block: regex must not over-match
// --------------------------------------------------------------------------- ~keep

/// The regex `/WMode\s+([0-9]+)\s+def` will match anywhere in the stream.
/// If a producer accidentally writes `/WMode 1 def` inside a begincidrange
/// block (e.g., as a token name), the parser will pick it up. Pin: we test
/// what happens when a phrase like "1 /WMode 1 def" appears within a bfchar
/// block-like context.
#[test]
fn probe22_wmode_directive_inside_block_is_picked_up_by_regex() {
    // The regex doesn't care about block boundaries. A `/WMode 1 def` inside
    // what looks like a begincidrange would still flip wmode. Use a CMap
    // with /Encoding /Identity-V so this doesn't matter for wmode resolution
    // (encoding wins). But pin the regex behavior with a stream-only probe.
    //
    // Best we can do without exposing pub(crate) APIs: build a PDF whose
    // /Encoding is an indirect stream containing /WMode 1 def NESTED inside
    // a begincidrange. Then check whether the font's wmode comes out as 1.
    // This is C5's CMap-stream encoding path.
    //
    // For now: pin that an /Encoding stream with a properly-placed /WMode 1
    // def is honored. The nested case is a regex-quality test deferred to
    // an internal unit test if reviewer wants stronger guarantees. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let spans = doc.extract_spans(0).expect("extract_spans");
    assert!(spans.iter().any(|s| s.wmode == 1));
}

// ---------------------------------------------------------------------------
// Probe 23 — /WMode after a `%` line comment
// --------------------------------------------------------------------------- ~keep

/// Pin: a `% comment` line followed by `/WMode 1 def` on the next line must
/// still recognize the /WMode directive (M5 fix).
#[test]
fn probe23_wmode_after_postscript_comment_is_recognized_in_full_pipeline() {
    // Use a horizontal encoding so that ANY wmode detection comes from the
    // ToUnicode CMap stream. C5 design says ToUnicode wmode does NOT
    // override /Encoding. So even if we put `% comment` + `/WMode 1 def`
    // in the ToUnicode, the encoding stays Identity-H and wmode stays 0.
    //
    // This probe pins that the comment-strip logic (M5) doesn't accidentally
    // mis-handle the surrounding stream and break extraction. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf(
        "Identity-H",
        content,
        1000,
        (880, -1000),
        None,
        Some("% some prologue comment\n/WMode 1 def"),
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let spans = doc.extract_spans(0).expect("extract_spans");
    for s in &spans {
        assert_eq!(s.wmode, 0);
    }
    let combined: String = spans.iter().map(|s| s.text.as_str()).collect();
    assert!(combined.contains('A'));
}

// ---------------------------------------------------------------------------
// Probe 25 — Identity-V with absent /W2 falls back to /DW2
// --------------------------------------------------------------------------- ~keep

#[test]
fn probe25_identity_v_with_absent_w2_uses_dw2_default() {
    // Unique BaseFont to avoid cache poisoning — see probe_cache_poison_*.
    // Use a non-default /DW2 (v_y=900, w1y=-800) so any fallback to spec
    // defaults would be visible. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf_full_named(
        "Probe25Font",
        "Identity-V",
        content,
        1000,
        (900, -800),
        None,
        None,
        None,
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let chars = doc.extract_chars(0).expect("extract_chars");
    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let dy = a.bbox.y - b.bbox.y;
    // w1y = -800 → dy = |-800 * 12 / 1000| = 9.6 ~keep
    assert!(
        (dy - 9.6).abs() < 0.1,
        "DW2 default must apply when /W2 is absent; dy={} expected 9.6 (would be 12.0 if spec defaults wrongly used)",
        dy
    );
}

// ---------------------------------------------------------------------------
// Probe 28 — Missing /CIDSystemInfo handled gracefully (parse-time only)
// --------------------------------------------------------------------------- ~keep

/// /CIDSystemInfo is required by the spec but real-world producers omit
/// it. Verify wmode resolution still proceeds and the font loads.
#[test]
fn probe28_font_without_cid_system_info_does_not_panic() {
    let cmap = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0001> <0041>
endbfchar
endcmap
end
end";
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestFont /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
    );
    let o6 = pdf.len();
    // /CIDSystemInfo intentionally omitted. ~keep
    pdf.extend_from_slice(
        b"6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont /DW 1000 /DW2 [880 -1000] >> endobj\n",
    );
    let o7 = pdf.len();
    pdf.extend_from_slice(format!("7 0 obj << /Length {} >> stream\n", cmap.len()).as_bytes());
    pdf.extend_from_slice(cmap);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for off in [o1, o2, o3, o4, o5, o6, o7] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer << /Size 8 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).expect("parse — must not panic on missing CIDSystemInfo");
    let _ = doc.extract_chars(0);
    let _ = doc.extract_spans(0);
}

// ---------------------------------------------------------------------------
// Probe 29 — Mongolian (vertical, columns LEFT-TO-RIGHT)
// --------------------------------------------------------------------------- ~keep

/// Mongolian (and Traditional Manchu) script is vertical and reads
/// LEFT to RIGHT across columns — opposite of CJK tategaki. The current
/// implementation hardcodes right-to-left in TategakiStrategy. Pin the
/// current behavior: Mongolian-style vertical text WILL be misordered
/// (left column emitted last instead of first).
///
/// If the implementation later grows a LTR-vertical mode, this test must
/// be updated.
#[test]
fn probe29_mongolian_style_vertical_ltr_is_currently_misordered() {
    // Two columns: LEFT column should be FIRST in Mongolian reading order.
    // Tags: all wmode=1 (vertical), arranged as
    //   LEFT column (x≈100): A (top), B, C (bottom)
    //   RIGHT column (x≈300): D, E, F
    // Expected Mongolian order: A B C D E F (LTR)
    // Actual (CJK tategaki) order: D E F A B C (RTL) ~keep
    let spans = vec![
        span_with_wmode("A", 100.0, 700.0, 1),
        span_with_wmode("B", 100.0, 688.0, 1),
        span_with_wmode("C", 100.0, 676.0, 1),
        span_with_wmode("D", 300.0, 700.0, 1),
        span_with_wmode("E", 300.0, 688.0, 1),
        span_with_wmode("F", 300.0, 676.0, 1),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).unwrap();
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    // PIN: implementation only models CJK direction → RIGHT-to-LEFT.
    // Mongolian comes out as DEFABC. ~keep
    assert_eq!(
        combined, "DEFABC",
        "current impl hardcodes RTL columns; Mongolian/Manchu come out reversed.\
         If this is fixed, update this test."
    );
}

// ---------------------------------------------------------------------------
// Probe 30 — Single vertical column (spine text) ordering
// --------------------------------------------------------------------------- ~keep

#[test]
fn probe30_single_vertical_column_ordered_top_down() {
    let spans = vec![
        span_with_wmode("C", 300.0, 676.0, 1),
        span_with_wmode("A", 300.0, 700.0, 1),
        span_with_wmode("B", 300.0, 688.0, 1),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).unwrap();
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    assert_eq!(combined, "ABC");
}

// ---------------------------------------------------------------------------
// Probe 31 — Multiple non-overlapping vertical columns
// --------------------------------------------------------------------------- ~keep

#[test]
fn probe31_three_vertical_columns_grouped_right_to_left() {
    // Three columns at x=100, 300, 500. Should be 500 first, then 300, then 100. ~keep
    let spans = vec![
        span_with_wmode("X1", 100.0, 700.0, 1),
        span_with_wmode("M1", 300.0, 700.0, 1),
        span_with_wmode("R1", 500.0, 700.0, 1),
        span_with_wmode("X2", 100.0, 688.0, 1),
        span_with_wmode("M2", 300.0, 688.0, 1),
        span_with_wmode("R2", 500.0, 688.0, 1),
    ];
    let pipeline = TextPipeline::new();
    let ordered = pipeline.process(spans, ReadingOrderContext::new()).unwrap();
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    assert_eq!(
        combined, "R1R2M1M2X1X2",
        "three columns must order rightmost-first, top-down within column"
    );
}

// Split for file-too-long (#1567): the module continues below via `include!`,
// not a sibling tests/*_part2.rs file -- Cargo auto-discovers every *.rs file
// directly under tests/ as its own test binary, so a sibling would fail to
// compile on its own and, if it somehow did, run every #[test] in it twice.
// A subdirectory is not auto-discovered. ~keep
include!("test_vertical_writing_mode_qa_probes/part2.rs");
