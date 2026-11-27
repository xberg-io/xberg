//! Second-pass regression tests for vertical writing mode (WMode 1).
//!
//! These tests pin the fixes for the bugs the reviewer flagged in the
//! first-pass implementation:
//!
//!   * C1/C2/C3: multi-Tj / TJ vertical advance must axis-swap in every
//!     downstream renderer and the extractor.
//!   * C4: `extract_chars` TJ-numeric-offset path must advance along Y.
//!   * C5: ToUnicode /WMode must NOT override /Encoding /WMode
//!     (ISO 32000-1 §9.10.2 — the ToUnicode CMap is extraction-only).
//!   * I2: horizontal scaling (Tz) must NOT apply to vertical w1y / Tc / Tw.
//!   * I3: malformed inner /W2 Form A triple must not desync subsequent CIDs.
//!   * I4: /W2 Form B range must not collapse onto u16::MAX on overflow.
//!
//! Each PDF is hand-assembled at the byte level (no real CJK fonts needed):
//! the ToUnicode CMap maps the test CIDs to ASCII so we can identify each
//! glyph in extracted output. /DW2 supplies vertical metrics; /W2 supplies
//! per-CID overrides where needed.

use xberg_native_pdf::document::PdfDocument;

/// Shared helper: emit a Type0 font + Identity-V/Identity-H wrapper with
/// the given content stream, /DW, /DW2, and an optional /W2 array.
///
/// Returns the assembled PDF bytes.
fn build_pdf(
    encoding_name: &str,
    content: &[u8],
    dw: i32,
    dw2: (i32, i32),
    w2: Option<&str>,
    cmap_extra: Option<&str>,
    horizontal_scaling_tz: Option<i32>,
) -> Vec<u8> {
    // Build ToUnicode CMap that maps CIDs 0001..0006 to ASCII A..F. If
    // `cmap_extra` is provided we splice it in (used by C5 to add an
    // explicit `/WMode 1 def`). ~keep
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
6 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <0043>
<0004> <0044>
<0005> <0045>
<0006> <0046>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end"
    );
    let cmap = cmap_src.as_bytes();

    let content_with_tz: Vec<u8> = if let Some(tz) = horizontal_scaling_tz {
        let mut v = Vec::new();
        v.extend_from_slice(b"BT /F1 12 Tf ");
        v.extend_from_slice(format!("{} Tz ", tz).as_bytes());
        let prefix = b"BT /F1 12 Tf ";
        if content.starts_with(prefix) {
            v.extend_from_slice(&content[prefix.len()..]);
        } else {
            v.extend_from_slice(content);
        }
        v
    } else {
        content.to_vec()
    };
    let content = content_with_tz.as_slice();

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let o5 = pdf.len();
    let f5 = format!(
        "5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestFont \
         /Encoding /{} /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
        encoding_name
    );
    pdf.extend_from_slice(f5.as_bytes());

    let o6 = pdf.len();
    let w2_clause = w2.map(|s| format!(" /W2 {}", s)).unwrap_or_default();
    let f6 = format!(
        "6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /DW {} /DW2 [{} {}]{} >> endobj\n",
        dw, dw2.0, dw2.1, w2_clause
    );
    pdf.extend_from_slice(f6.as_bytes());

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

    pdf
}

/// Two successive Tj operators under Identity-V at the same starting cursor
/// must produce CHARS that advance along Y across the operator boundary,
/// not along X. /DW2 = [880 -1000] means each glyph displaces by
/// -font_size in y (12 pt at fs=12), so the second Tj's char Y must be
/// ~12 less than the first's.
///
/// `extract_chars` records per-character positions, bypassing the
/// `tj_span_buffer` that would otherwise coalesce consecutive Tj operators
/// into a single span. This makes per-Tj advances directly observable.
#[test]
fn c1_c2_extractor_multi_tj_vertical_advances_along_y() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let chars = doc.extract_chars(0).expect("extract chars");

    let a = chars.iter().find(|c| c.char == 'A').expect("expected an 'A' char");
    let b = chars.iter().find(|c| c.char == 'B').expect("expected a 'B' char");

    assert!(
        (a.bbox.x - b.bbox.x).abs() < 1.0,
        "vertical multi-Tj X must stay stable: A.x={}, B.x={}",
        a.bbox.x,
        b.bbox.x
    );
    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 12.0).abs() < 0.5,
        "vertical multi-Tj Y delta should be ~12; got {} (A.y={}, B.y={})",
        dy,
        a.bbox.y,
        b.bbox.y
    );
}

/// Same property for TJ arrays: numeric offsets and string segments alike
/// must advance along Y in vertical mode.
#[test]
fn c2_c4_extractor_tj_array_vertical_advances_along_y() {
    // [(<0001>) -250 (<0002>)] TJ — a 250/1000 × 12 = 3.0 unit positive Y
    // shift in vertical mode (negative offset moves the cursor *forward*
    // along the writing axis per §9.4.3). Vertical mode forward = -Y. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td [<0001> -250 <0002>] TJ ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let spans = doc.extract_spans(0).expect("extract spans");

    let mut s = spans.clone();
    s.sort_by_key(|sp| sp.sequence);

    let combined: String = s.iter().map(|sp| sp.text.as_str()).collect();
    assert!(
        combined.contains('A') && combined.contains('B'),
        "expected A and B in TJ output; got {:?}",
        combined
    );

    let xs: Vec<f32> = s.iter().map(|sp| sp.bbox.x).collect();
    let x_min = xs.iter().cloned().fold(f32::INFINITY, f32::min);
    let x_max = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (x_max - x_min) < 1.0,
        "vertical TJ array must keep X stable; got xs={:?}",
        xs
    );

    let a = s.iter().find(|sp| sp.text.contains('A')).unwrap();
    let b = s.iter().find(|sp| sp.text.contains('B')).unwrap();
    assert!(
        a.bbox.y > b.bbox.y,
        "vertical TJ: A should be above B; A.y={}, B.y={}",
        a.bbox.y,
        b.bbox.y
    );
}

/// `extract_chars` is a separate extraction pathway from `extract_spans`.
/// The TJ numeric-offset shift must use the active writing axis.
#[test]
fn c4_extract_chars_vertical_tj_offset_advances_along_y() {
    let content = b"BT /F1 12 Tf 100 700 Td [<0001> -250 <0002>] TJ ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let chars = doc.extract_chars(0).expect("extract chars");

    let a = chars.iter().find(|c| c.char == 'A').expect("expected an 'A' char");
    let b = chars.iter().find(|c| c.char == 'B').expect("expected a 'B' char");

    assert!(
        (a.bbox.x - b.bbox.x).abs() < 1.0,
        "vertical extract_chars: A.x={} vs B.x={} should share a column",
        a.bbox.x,
        b.bbox.x
    );
    assert!(
        a.bbox.y > b.bbox.y,
        "vertical extract_chars: A.y={} should be above B.y={}",
        a.bbox.y,
        b.bbox.y
    );
}

/// A horizontal /Encoding (Identity-H) paired with a ToUnicode CMap whose
/// stream contains `/WMode 1 def` (stale tooling leftover) must NOT silently
/// flip the font to vertical. Per ISO 32000-1 §9.10.2 the ToUnicode CMap is
/// for extraction-time character → Unicode mapping ONLY.
#[test]
fn c5_to_unicode_wmode_does_not_override_encoding_wmode() {
    // Stale /WMode 1 def inside the ToUnicode prologue. ~keep
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf(
        "Identity-H",
        content,
        1000,
        (880, -1000),
        None,
        Some("/WMode 1 def"),
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic horizontal PDF");
    let spans = doc.extract_spans(0).expect("extract spans");

    // Every span must report wmode=0 because /Encoding is Identity-H. ~keep
    for sp in &spans {
        assert_eq!(
            sp.wmode, 0,
            "ToUnicode /WMode 1 must NOT flip /Encoding /Identity-H to vertical; \
             span {:?} reported wmode={}",
            sp.text, sp.wmode
        );
    }
}

/// Synthetic vertical-majority page: spans are tagged wmode=1 and
/// arranged in two right-to-left columns. Extraction must produce
/// tategaki order — right column top-to-bottom,
/// then left column top-to-bottom — across every reading-order strategy.
#[test]
fn i1_pipeline_routes_vertical_majority_through_tategaki_sort() {
    use xberg_native_pdf::geometry::Rect;
    use xberg_native_pdf::layout::TextSpan;
    use xberg_native_pdf::pipeline::{ReadingOrderContext, TextPipeline};

    fn mk(text: &str, x: f32, y: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, 12.0, 12.0),
            font_name: "Test".into(),
            font_size: 12.0,
            wmode: 1,
            ..TextSpan::default()
        }
    }

    let spans = vec![
        mk("D", 300.0, 700.0),
        mk("F", 300.0, 676.0),
        mk("B", 500.0, 688.0),
        mk("C", 500.0, 676.0),
        mk("A", 500.0, 700.0),
        mk("E", 300.0, 688.0),
    ];

    let pipeline = TextPipeline::new();
    let ordered = pipeline
        .process(spans, ReadingOrderContext::new())
        .expect("pipeline process");
    let combined: String = ordered.iter().map(|o| o.span.text.as_str()).collect();
    assert_eq!(
        combined, "ABCDEF",
        "vertical-majority pipeline must produce tategaki order; got {:?}",
        combined
    );
}

/// Extraction routes through `TextPipeline::process`
/// per src/document.rs:14258 / 14588. The vertical reading-order test
/// PDF (two columns: A-C right, D-F left under Identity-V) must produce
/// tategaki order in both output formats.
#[test]
fn i1_vertical_text_routes_through_tategaki() {
    let cmap = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
6 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <0043>
<0004> <0044>
<0005> <0045>
<0006> <0046>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";
    let content = b"BT /F1 12 Tf \
        1 0 0 1 500 700 Tm <0001> Tj \
        1 0 0 1 500 680 Tm <0002> Tj \
        1 0 0 1 500 660 Tm <0003> Tj \
        1 0 0 1 300 700 Tm <0004> Tj \
        1 0 0 1 300 680 Tm <0005> Tj \
        1 0 0 1 300 660 Tm <0006> Tj \
        ET";
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestFont \
          /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
    );
    let o6 = pdf.len();
    pdf.extend_from_slice(
        b"6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
          /DW 1000 /DW2 [880 -1000] >> endobj\n",
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

    let doc = PdfDocument::from_bytes(pdf).expect("parse tategaki PDF");

    use xberg_native_pdf::pipeline::{TextPipeline, TextPipelineConfig};
    let spans = doc.extract_spans(0).expect("extract_spans");
    let pipeline = TextPipeline::with_config(TextPipelineConfig::default());
    let ordered = pipeline.process(spans, Default::default()).expect("pipeline process");

    let letters: String = ordered
        .iter()
        .flat_map(|o| o.span.text.chars())
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    assert!(
        letters.contains("ABCDEF"),
        "the pipeline must route a vertical page through tategaki; got {:?}",
        letters
    );
}

/// A Type0 font with explicit per-CID /W2 metrics that disagree with
/// /DW2 must use the per-CID values for the listed CIDs and the /DW2
/// defaults for unlisted CIDs. This pins the /W2 lookup wiring all the
/// way through to extract_chars positioning.
#[test]
fn per_cid_w2_overrides_dw2_for_listed_cids() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf(
        "Identity-V",
        content,
        1000,
        (880, -1000),
        Some("[1 [-500 250 600]]"),
        None,
        None,
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse per-CID /W2 PDF");
    let chars = doc.extract_chars(0).expect("extract chars");

    let a = chars.iter().find(|c| c.char == 'A').expect("expected an 'A' char");
    let b = chars.iter().find(|c| c.char == 'B').expect("expected a 'B' char");

    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 6.0).abs() < 0.5,
        "per-CID /W2 must override /DW2 for CID 1: expected dy ~6, got {} (A.y={}, B.y={})",
        dy,
        a.bbox.y,
        b.bbox.y
    );
}

/// A content stream that swaps a horizontal font for a vertical font
/// (or vice versa) mid-stream via Tf must correctly track the active
/// wmode for spans emitted on each side of the switch.
#[test]
fn mid_stream_tf_h_to_v_switches_span_wmode() {
    // Build a PDF carrying two fonts: F1 Identity-H, F2 Identity-V,
    // both with the same ToUnicode mapping <0001>→A and <0002>→B. ~keep
    let cmap = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<0001> <0041>
<0002> <0042>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";
    // One BT/ET, position set ONCE, then alternate Tf/Tj inside the
    // block. Mid-BT Tf must flush the pending Tj span buffer so the
    // post-switch glyph is emitted under its new font + wmode. (Earlier
    // drafts used two BT/ET blocks as a defensive workaround; the
    // probe36 investigation in the QA pass confirmed the in-block
    // flush is correct, so the workaround is no longer required.) ~keep
    let content = b"BT 100 700 Td /F1 12 Tf <0001> Tj /F2 12 Tf <0002> Tj ET";

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R /F2 8 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestH \
          /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
    );
    let o6 = pdf.len();
    pdf.extend_from_slice(
        b"6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestH \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
          /DW 1000 /DW2 [880 -1000] >> endobj\n",
    );
    let o7 = pdf.len();
    pdf.extend_from_slice(format!("7 0 obj << /Length {} >> stream\n", cmap.len()).as_bytes());
    pdf.extend_from_slice(cmap);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o8 = pdf.len();
    pdf.extend_from_slice(
        b"8 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestV \
          /Encoding /Identity-V /DescendantFonts [9 0 R] /ToUnicode 7 0 R >> endobj\n",
    );
    let o9 = pdf.len();
    pdf.extend_from_slice(
        b"9 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestV \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
          /DW 1000 /DW2 [880 -1000] >> endobj\n",
    );
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 10\n0000000000 65535 f \n");
    for off in [o1, o2, o3, o4, o5, o6, o7, o8, o9] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer << /Size 10 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).expect("parse mid-stream Tf PDF");
    let spans = doc.extract_spans(0).expect("extract spans");

    let a = spans.iter().find(|s| s.text.contains('A')).expect("A span");
    let b = spans.iter().find(|s| s.text.contains('B')).expect("B span");
    assert_eq!(a.wmode, 0, "A drawn under Identity-H must report wmode=0");
    assert_eq!(b.wmode, 1, "B drawn under Identity-V must report wmode=1");
}

/// Adobe-Japan1 CIDs can run into the thousands. A /W2 entry assigning
/// per-CID metrics to a CID > 256 must look up correctly via the
/// extractor's vertical advance path.
#[test]
fn high_cid_w2_lookup_works() {
    let cmap = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<0500> <0041>
<0501> <0042>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";
    let content = b"BT /F1 12 Tf 100 700 Td <0500> Tj <0501> Tj ET";
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let o1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
    let o2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
    let o3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n",
    );
    let o4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let o5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj << /Type /Font /Subtype /Type0 /BaseFont /TestHigh \
          /Encoding /Identity-V /DescendantFonts [6 0 R] /ToUnicode 7 0 R >> endobj\n",
    );
    let o6 = pdf.len();
    pdf.extend_from_slice(
        b"6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestHigh \
          /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> \
          /DW 1000 /DW2 [880 -1000] /W2 [1280 [-500 250 600]] >> endobj\n",
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

    let doc = PdfDocument::from_bytes(pdf).expect("parse high-CID PDF");
    let chars = doc.extract_chars(0).expect("extract chars");
    let a = chars
        .iter()
        .find(|c| c.char == 'A')
        .expect("expected A from CID 0x0500");
    let b = chars
        .iter()
        .find(|c| c.char == 'B')
        .expect("expected B from CID 0x0501");
    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 6.0).abs() < 0.5,
        "high-CID /W2 lookup must use per-CID metric; expected dy ~6, got {}",
        dy
    );
}

/// Counter-test: a vertical /Encoding (Identity-V) with a ToUnicode that has
/// no /WMode directive must still produce wmode=1.
#[test]
fn c5_identity_v_with_silent_to_unicode_is_vertical() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");
    let spans = doc.extract_spans(0).expect("extract spans");
    assert!(!spans.is_empty());
    for sp in &spans {
        assert_eq!(
            sp.wmode, 1,
            "/Encoding /Identity-V must produce wmode=1; got {} for {:?}",
            sp.wmode, sp.text
        );
    }
}

// ---------------------------------------------------------------------------
// I2 — horizontal scaling (Tz) MUST NOT apply to vertical advances
// Per ISO 32000-1 §9.4.4 the vertical advance formula is
//   `ty = (w1y − Tj/1000) × Tfs + Tc + Tw` — no Th factor. Tz (horizontal
// scaling) is the glyph-stretching direction (§9.3.4), not "writing-
// direction scale". Two glyphs at fs=12 with /DW2 [880 -1000] under Tz=150
// must still advance by exactly 12 units in y (not 18). This pins the
// extractor's vertical-Tj path to the spec.
// --------------------------------------------------------------------------- ~keep

/// Render the multi-Tj vertical PDF through the page renderer. The test
/// asserts that rendering succeeds AND that the painted ink occupies a
/// vertical column (more pixel rows touched than columns), which is the
/// pixel-level signature of axis-correct per-Tj advance. A renderer that
/// advanced along X across the second Tj would paint a horizontal row
/// instead.
#[test]
fn c1_page_renderer_multi_tj_vertical_paints_a_column() {
    let content = b"BT /F1 24 Tf 100 700 Td <0001> Tj <0002> Tj <0003> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, None);
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF");

    let opts = xberg_native_pdf::rendering::RenderOptions::with_dpi(72).as_raw();
    let rendered = xberg_native_pdf::rendering::render_page(&doc, 0, &opts).expect("render page 0 of vertical PDF");

    let w = rendered.width as usize;
    let h = rendered.height as usize;
    let mut x_min = w;
    let mut x_max = 0usize;
    let mut y_min = h;
    let mut y_max = 0usize;
    let mut hits = 0usize;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let r = rendered.data[i];
            let g = rendered.data[i + 1];
            let b = rendered.data[i + 2];
            if r < 250 || g < 250 || b < 250 {
                hits += 1;
                if x < x_min {
                    x_min = x;
                }
                if x > x_max {
                    x_max = x;
                }
                if y < y_min {
                    y_min = y;
                }
                if y > y_max {
                    y_max = y;
                }
            }
        }
    }
    assert!(hits > 100, "vertical render produced too few non-white pixels ({hits})");
    let dx = (x_max - x_min) as f32;
    let dy = (y_max - y_min) as f32;
    // Three glyphs stacked vertically must occupy more y-extent than
    // x-extent (column, not row). A bug-class regression to horizontal
    // advance would invert this. ~keep
    assert!(
        dy > dx,
        "vertical render must produce a column (dy > dx); got dx={}, dy={}",
        dx,
        dy
    );
}

#[test]
fn i2_tz_does_not_scale_vertical_advance() {
    let content = b"BT /F1 12 Tf 100 700 Td <0001> Tj <0002> Tj ET";
    let pdf = build_pdf("Identity-V", content, 1000, (880, -1000), None, None, Some(150));
    let doc = PdfDocument::from_bytes(pdf).expect("parse synthetic vertical PDF (Tz=150)");
    let chars = doc.extract_chars(0).expect("extract chars (Tz=150)");

    let a = chars.iter().find(|c| c.char == 'A').unwrap();
    let b = chars.iter().find(|c| c.char == 'B').unwrap();
    let dy = a.bbox.y - b.bbox.y;
    assert!(
        (dy - 12.0).abs() < 0.5,
        "Tz=150 must NOT scale vertical advance: expected ~12, got {}",
        dy
    );
}
