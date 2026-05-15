//! Regression tests for issue #962 — glyph-spaced PDF text fragmented one
//! character per line.
//!
//! When a PDF positions each character via a separate `BT … ET` block using an
//! absolute `Tm` operator with a y-coordinate that differs from adjacent glyphs
//! by more than pdfium's internal line-break threshold (~3.2 pt), pdfium inserts
//! a `\r\n` between them rather than keeping them on the same text line.
//! Microsoft Word triggers this pattern for "broken image" placeholder text
//! (`Het afbeelding onderdeel met relatie-id … is niet aangetroffen`).
//!
//! Fix: `text.rs` detects the fragmentation signature (≥ 5 consecutive
//! `\r\n`-separated lines of ≤ 3 chars) and rebuilds the page text from
//! per-character position data via `rebuild_page_text_from_char_positions`.

#![cfg(feature = "pdf")]

use kreuzberg::{ExtractionConfig, extract_bytes_sync};

/// Build a minimal but valid single-page PDF whose content stream places each
/// character of `text` in its own `BT … ET` block via an absolute `Tm`
/// operator.  The y-coordinate oscillates sinusoidally with amplitude
/// `jitter_pt` and period `JITTER_PERIOD`, replicating the pattern Microsoft
/// Word emits for broken-image placeholder text.
///
/// At jitter_pt ≥ ~3.2 pt pdfium emits `\r\n` between adjacent glyphs; below
/// that threshold pdfium coalesces them into a single text line.
fn make_glyph_jitter_pdf(jitter_pt: f32) -> Vec<u8> {
    const TEXT: &str = "Hetafbeeldingisnietsaangetroffen";
    const FONT_SIZE: f32 = 12.0;
    const JITTER_PERIOD: usize = 6;
    const X_START: f32 = 72.0;
    const X_STEP: f32 = 7.0;
    const Y_BASE: f32 = 700.0;

    let mut stream = String::new();
    for (i, ch) in TEXT.chars().enumerate() {
        let x = X_START + i as f32 * X_STEP;
        let angle = std::f64::consts::TAU * i as f64 / JITTER_PERIOD as f64;
        let y = Y_BASE + angle.sin() as f32 * jitter_pt;
        let escaped = match ch {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            c => c.to_string(),
        };
        stream.push_str(&format!(
            "BT /F1 {FONT_SIZE} Tf 1 0 0 1 {x:.2} {y:.4} Tm ({escaped}) Tj ET\n"
        ));
    }

    // Assemble PDF object by object, recording byte offsets for the xref table.
    let mut pdf: Vec<u8> = Vec::new();

    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push!("%PDF-1.4\n");

    let off1 = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    push!(
        "3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\
         \n   /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
         endobj\n"
    );

    let off4 = pdf.len();
    let stream_bytes = stream.as_bytes();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len()));
    pdf.extend_from_slice(stream_bytes);
    push!("\nendstream\nendobj\n");

    let off5 = pdf.len();
    push!("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    let xref_off = pdf.len();
    push!(format!(
        "xref\n0 6\n\
         0000000000 65535 f \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n",
        off1, off2, off3, off4, off5
    ));
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

fn count_single_char_lines(text: &str) -> usize {
    text.lines().filter(|l| l.trim().chars().count() == 1).count()
}

/// After the fix, 3.5 pt jitter must produce no more than 4 single-character
/// lines (a few isolated chars are acceptable — runs of 18 are not).
#[test]
fn test_3_5pt_jitter_coalesced() {
    let pdf = make_glyph_jitter_pdf(3.5);
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("3.5 pt jitter PDF should extract without error");

    let content = result.content.trim().to_string();
    let single_char_lines = count_single_char_lines(&content);

    assert!(
        single_char_lines < 5,
        "issue #962 regression (3.5 pt): {single_char_lines} single-char lines after fix.\n\
         Content: {content:?}"
    );
}

/// 4.0 pt jitter: same guarantee as 3.5 pt.
#[test]
fn test_4_0pt_jitter_coalesced() {
    let pdf = make_glyph_jitter_pdf(4.0);
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("4.0 pt jitter PDF should extract without error");

    let content = result.content.trim().to_string();
    let single_char_lines = count_single_char_lines(&content);

    assert!(
        single_char_lines < 5,
        "issue #962 regression (4.0 pt): {single_char_lines} single-char lines after fix.\n\
         Content: {content:?}"
    );
}

/// 3.0 pt jitter: pdfium already coalesces these — the fix must not disturb them.
#[test]
fn test_3_0pt_jitter_unchanged() {
    let pdf = make_glyph_jitter_pdf(3.0);
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("3.0 pt jitter PDF should extract without error");

    let content = result.content.trim().to_string();
    let single_char_lines = count_single_char_lines(&content);

    assert!(
        single_char_lines < 5,
        "3.0 pt jitter (already-coalesced) regressed: {single_char_lines} single-char lines.\n\
         Content: {content:?}"
    );
    assert!(!content.is_empty(), "3.0 pt jitter PDF must produce non-empty content");
}

/// The fix must not panic or return an error on any generated fixture.
#[test]
fn test_all_fixtures_loadable() {
    let config = ExtractionConfig::default();
    for (label, jitter) in [("3.5pt", 3.5f32), ("4.0pt", 4.0), ("3.0pt", 3.0)] {
        let pdf = make_glyph_jitter_pdf(jitter);
        let result = extract_bytes_sync(&pdf, "application/pdf", &config);
        assert!(
            result.is_ok(),
            "[{label}] extraction should not error: {:?}",
            result.err()
        );
        let r = result.unwrap();
        assert!(!r.content.trim().is_empty(), "[{label}] must produce non-empty content");
    }
}
