use super::super::*;
use super::common::*;

/// Build a tagged one-page PDF with three MCID-tagged paragraphs
/// (ALPHA/BRAVO/CHARLIE, MCID 0/1/2) drawn top-to-bottom and a minimal
/// struct tree whose `/P` kids reference those MCIDs in order.
fn build_mcid_structure_tree_pdf() -> Vec<u8> {
    let content = b"BT /F1 12 Tf\n\
            /P <</MCID 0>> BDC 1 0 0 1 72 700 Tm (ALPHA) Tj EMC\n\
            /P <</MCID 1>> BDC 1 0 0 1 72 600 Tm (BRAVO) Tj EMC\n\
            /P <</MCID 2>> BDC 1 0 0 1 72 500 Tm (CHARLIE) Tj EMC\n\
            ET\n";
    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 9];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    let stream = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, data: &[u8]| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n<< /Length {} >>\nstream\n", data.len()).as_bytes());
        buf.extend_from_slice(data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    };
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(
        &mut buf,
        &mut off,
        1,
        "<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 7 0 R >>",
    );
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 >>",
    );
    stream(&mut buf, &mut off, 4, content);
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    // Minimal struct tree: three /P kids referencing MCID 0/1/2. ~keep
    obj(&mut buf, &mut off, 7, "<< /Type /StructTreeRoot /K [8 0 R] >>");
    obj(
        &mut buf,
        &mut off,
        8,
        "<< /Type /StructElem /S /Document /K [<< /Type /StructElem /S /P /Pg 3 0 R /K 0 >> \
             << /Type /StructElem /S /P /Pg 3 0 R /K 1 >> \
             << /Type /StructElem /S /P /Pg 3 0 R /K 2 >>] >>",
    );
    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 9\n0000000000 65535 f \n");
    for id in 1..=8 {
        if id == 6 {
            buf.extend_from_slice(b"0000000000 65535 f \n");
            continue;
        }
        buf.extend_from_slice(format!("{:010} 00000 n \n", off[id]).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());

    buf
}

/// A span injected into a tagged page via `extract_text_with_extra_spans`
/// and carrying the MCID of a middle block must be emitted at that block's
/// position in structure order — not appended after the page. This is the
/// primitive the Auto extractor uses to drop OCR'd image text into the
/// figure's reading-order slot instead of after the whole page.
#[test]
fn extra_span_with_borrowed_mcid_lands_in_structure_order() {
    let doc = PdfDocument::from_bytes(build_mcid_structure_tree_pdf()).unwrap();
    let plain = doc.extract_text(0).unwrap();
    assert!(
        plain.find("ALPHA") < plain.find("BRAVO") && plain.find("BRAVO") < plain.find("CHARLIE"),
        "baseline structure order wrong: {plain:?}"
    );

    // Inject a span carrying MCID 1 (BRAVO's block) positioned at BRAVO's
    // y. It must land within BRAVO's group — after BRAVO, before CHARLIE —
    // NOT appended after CHARLIE. ~keep
    let extra = crate::layout::TextSpan {
        text: "INSERTED".to_string(),
        bbox: crate::geometry::Rect::new(72.0, 590.0, 50.0, 12.0),
        font_size: 12.0,
        mcid: Some(1),
        ..Default::default()
    };
    let opts = crate::converters::ConversionOptions {
        extract_tables: true,
        ..Default::default()
    };
    let out = doc.extract_text_with_extra_spans(0, vec![extra], &opts).unwrap();
    let (a, b, ins, c) = (
        out.find("ALPHA"),
        out.find("BRAVO"),
        out.find("INSERTED"),
        out.find("CHARLIE"),
    );
    assert!(ins.is_some(), "injected span dropped: {out:?}");
    assert!(
        a < b && b < ins && ins < c,
        "injected span not placed in MCID-1 slot (expected ALPHA<BRAVO<INSERTED<CHARLIE): {out:?}"
    );
}

/// Helper to create a TextSpan with minimal required fields for testing.
#[test]
fn test_try_assemble_vertical_cjk_orders_columns_right_to_left() {
    // Three columns of CJK glyphs; the right column (x=116) is read first,
    // top-to-bottom, then the next column to the left, etc. ~keep
    let mk = |t: &str, x: f32, y: f32| make_test_span(t, x, y, 18.0, 18.0);
    let spans = vec![
        mk("\u{4E00}", 116.0, 719.0),
        mk("\u{4E8C}", 116.0, 701.0),
        mk("\u{4E09}", 116.0, 683.0),
        mk("\u{56DB}", 89.0, 719.0),
        mk("\u{4E94}", 89.0, 701.0),
        mk("\u{516D}", 89.0, 683.0),
        mk("\u{4E03}", 62.0, 719.0),
        mk("\u{516B}", 62.0, 701.0),
        mk("\u{4E5D}", 62.0, 683.0),
    ];
    assert_eq!(
        PdfDocument::try_assemble_vertical_cjk(&spans).as_deref(),
        Some("\u{4E00}\u{4E8C}\u{4E09}\u{56DB}\u{4E94}\u{516D}\u{4E03}\u{516B}\u{4E5D}")
    );
}

#[test]
fn vertical_cjk_assembly_excludes_hangul_syllables() {
    let spans: Vec<TextSpan> = "가나다라마바사아자"
        .chars()
        .enumerate()
        .map(|(i, glyph)| make_test_span(&glyph.to_string(), 116.0, 719.0 - i as f32 * 18.0, 18.0, 18.0))
        .collect();

    assert_eq!(PdfDocument::try_assemble_vertical_cjk(&spans), None);
}

#[test]
fn test_try_assemble_vertical_cjk_horizontal_returns_none() {
    // A horizontal CJK row (glyphs advance in X at a fixed Y) must NOT be
    // detected as vertical — horizontal documents stay on the normal path. ~keep
    let mk = |t: &str, x: f32| make_test_span(t, x, 700.0, 18.0, 18.0);
    let spans = vec![
        mk("\u{4E00}", 62.0),
        mk("\u{4E8C}", 80.0),
        mk("\u{4E09}", 98.0),
        mk("\u{56DB}", 116.0),
        mk("\u{4E94}", 134.0),
        mk("\u{516D}", 152.0),
        mk("\u{4E03}", 170.0),
        mk("\u{516B}", 188.0),
    ];
    assert!(PdfDocument::try_assemble_vertical_cjk(&spans).is_none());
}

#[test]
fn test_try_assemble_vertical_cjk_multichar_runs_returns_none() {
    // Horizontal CJK emitted as multi-character RUNS (a whole line per show
    // op), stacked top-to-bottom. Each run's nearest neighbour is the run on
    // the line above/below (vertical) — but these are horizontal lines, not
    // tategaki columns. The single-glyph-span gate must keep this on the
    // horizontal path so the reading order is not shredded. ~keep
    let mk = |t: &str, y: f32| make_test_span(t, 60.0, y, 200.0, 18.0);
    let spans = vec![
        mk("標準マーケットモデルは", 700.0),
        mk("次元で説明することは", 680.0),
        mk("取引できない商品がマ", 660.0),
        mk("クロ経済変数などである", 640.0),
        mk("モデルを考えることは", 620.0),
        mk("可能で非完備の場合は", 600.0),
        mk("リスク中立確率は一意", 580.0),
        mk("ではなく価格も一意でない", 560.0),
    ];
    assert!(PdfDocument::try_assemble_vertical_cjk(&spans).is_none());
}

#[test]
fn test_try_assemble_vertical_cjk_latin_returns_none() {
    // A Latin page is not CJK-majority → None (never vertical). ~keep
    let spans: Vec<TextSpan> = "the quick brown fox jumps over a lazy dog today"
        .split(' ')
        .enumerate()
        .map(|(i, w)| make_test_span(w, 60.0 + i as f32 * 40.0, 700.0, 30.0, 12.0))
        .collect();
    assert!(PdfDocument::try_assemble_vertical_cjk(&spans).is_none());
}
