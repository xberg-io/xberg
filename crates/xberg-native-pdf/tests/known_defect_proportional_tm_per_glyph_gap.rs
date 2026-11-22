//! KNOWN DEFECT, not a passing feature: pins a general-purpose bug found
//! while investigating GH#1770 but deliberately NOT fixed there (out of
//! scope — a much larger blast radius than the narrow fixed-pitch gate).
//!
//! `extractors/text/operators.rs`'s `Operator::Tm` "batch character-by-
//! character Tm+Tj patterns" buffer-continuation optimization glues two
//! same-baseline, same-transform glyph runs together with NO horizontal gap
//! check at all — see the match guard at `operators.rs:69-86`: it tests
//! baseline proximity (`f`), identical `a`/`b`/`c`/`d`, and forward
//! progression (`e >= start.e`), and nothing else. This holds for ANY font,
//! not just monospace, and for ANY gap size, not just a fixed-pitch cell.
//!
//! This test documents the defect on a PROPORTIONAL font (unlike GH#1770's
//! monospace repro) across several gap distances, including an extreme
//! 100pt gap that is obviously not intra-word kerning by any measure. It
//! asserts the CURRENT (buggy) glued output on purpose, so a future fix for
//! this general case turns it red — that is the intended trigger to update
//! or remove this test, not a regression in this test itself.
//!
//! GH#1778 investigation (no-ship): a fix routing this arm through the same
//! corpus-tuned geometric threshold `should_insert_space` uses for its
//! "strong geometric signal alone" rule (50% of the font's space-glyph
//! width) was implemented and found UNSAFE — it broke
//! `tests/table_cell_word_seam.rs`, whose fixture exists specifically to
//! prove no gap-magnitude constant can separate a real word space from an
//! intra-word fragment seam: a producer that draws one word as several
//! Tm+Tj fragments can position them with a seam (there: 1.75pt) WIDER than
//! a legitimate word space elsewhere on the same line (there: 1.53pt). Any
//! threshold that keeps such fragments fused also swallows real spaces, and
//! any threshold that keeps real spaces also splits such words — "Credit"
//! became "Cre d it". `should_insert_space`'s cross-span path avoids this
//! because it runs AFTER both texts are known and can lean on
//! following-text-dependent signals (source-order/glyph evidence via the
//! span-level merger); the Tm-continuation arm decides before the *next*
//! Tj's text is decoded (Tm arrives before its Tj), so it structurally
//! cannot consult those signals without a deferred-decision redesign of the
//! per-glyph append path — out of scope for a gap-check fix. Left open.

use xberg_native_pdf::document::PdfDocument;

/// `A` and `B`, each 600/1000 em wide, in a proportional Helvetica font,
/// shown via two separate `Tm`+`Tj` operators on the same baseline,
/// separated by `gap` points measured from the end of `A`'s declared advance
/// to the start of `B`.
fn two_glyphs_pdf(gap: f32) -> Vec<u8> {
    const FONT_SIZE: f32 = 12.0;
    const GLYPH_WIDTH_PT: f32 = 7.2;
    let x_a = 100.0f32;
    let x_b = x_a + GLYPH_WIDTH_PT + gap;

    let mut content = Vec::new();
    content.extend_from_slice(format!("BT /F1 {FONT_SIZE} Tf\n").as_bytes());
    content.extend_from_slice(format!("1 0 0 1 {x_a} 500 Tm (A) Tj\n").as_bytes());
    content.extend_from_slice(format!("1 0 0 1 {x_b} 500 Tm (B) Tj\n").as_bytes());
    content.extend_from_slice(b"ET");

    build_minimal_pdf(&content)
}

/// Hand-assemble a one-page PDF with a single Type1 Helvetica (proportional,
/// NOT monospace) font whose glyphs `A` (65) and `B` (66) are both declared
/// at 600/1000 em, matching `tests/adjacent_span_merge.rs`'s fixture.
fn build_minimal_pdf(content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 66 /Widths [600 600] >>\nendobj\n",
    );

    let xref_pos = pdf.len();
    let offsets = [0usize, off1, off2, off3, off4, off5];
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(format!("{:010} 65535 f\r\n", 0).as_bytes());
    for &off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_pos
        )
        .as_bytes(),
    );
    pdf
}

/// KNOWN DEFECT: a proportional-font Tm-per-glyph run glues across any gap
/// under the column gap (`max(3 x font size, 30 pt)` = 36 pt here). This is the
/// current, unfixed behaviour for those gaps; a jump past the column gap now
/// ends the run (see `jump_past_the_column_gap_ends_the_run`).
#[test]
fn known_defect_proportional_tm_per_glyph_glues_across_any_gap() {
    for gap in [7.2f32, 20.0] {
        let doc = PdfDocument::from_bytes(two_glyphs_pdf(gap)).expect("open fixture");
        let text = doc.extract_text(0).expect("extract_text");
        assert_eq!(
            text.trim(),
            "AB",
            "expected the current (buggy) glued single token \"AB\" at gap={gap}pt, got {text:?} — \
             if this now reads \"A B\", the sub-column Tm-continuation gap defect has been fixed and \
             this test (and the issue it documents) should be closed/updated, not silently left red"
        );
    }
}

/// A jump past the column gap is a new run, so the glyphs read as two tokens.
#[test]
fn jump_past_the_column_gap_ends_the_run() {
    for gap in [50.0f32, 100.0] {
        let doc = PdfDocument::from_bytes(two_glyphs_pdf(gap)).expect("open fixture");
        let text = doc.extract_text(0).expect("extract_text");
        assert_eq!(text.trim(), "A B", "gap={gap}pt must not glue, got {text:?}");
    }
}
