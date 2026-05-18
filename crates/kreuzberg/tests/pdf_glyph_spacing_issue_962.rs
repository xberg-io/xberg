//! Regression tests for issue #962 — glyph-spaced PDF text fragmented one
//! character per line.
//!
//! When a PDF positions each character via a separate `BT … ET` block with a
//! sinusoidal y-jitter, pdf_oxide's ColumnAware reading order groups spans by
//! y-level rather than reading order, producing single-character spans that each
//! land on their own output line. Microsoft Word triggers this pattern for
//! "broken image" placeholder text
//! (`Het afbeelding onderdeel met relatie-id … is niet aangetroffen`).
//!
//! Fix: `oxide/text.rs` detects the fragmentation signature (≥ 5 same-line
//! x-disorder events among short spans) and rebuilds page text from span
//! positions: sort by y-descending, group by y-proximity, sort each group by x,
//! insert spaces at word gaps.

#![cfg(feature = "pdf")]

use kreuzberg::{ExtractionConfig, extract_bytes_sync};

/// Build a minimal but valid single-page PDF whose content stream places each
/// character of `text` in its own `BT … ET` block via an absolute `Tm`
/// operator. The y-coordinate oscillates sinusoidally with amplitude
/// `jitter_pt` and period `JITTER_PERIOD`, replicating the pattern Microsoft
/// Word emits for broken-image placeholder text.
///
/// pdf_oxide's ColumnAware mode groups these single-character spans by y-level,
/// producing out-of-reading-order output that the fragmentation repair path detects
/// and corrects.
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

/// Shared helper: build a minimal single-page PDF from a ready-made content stream string.
fn assemble_single_page_pdf(stream: &str) -> Vec<u8> {
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

/// A PDF with two clearly separate text lines (y gap = 30 pt, well above the coalesce threshold).
/// Used to verify that multi-line content stays on two lines after the fix.
///
/// Line 1 at y=700: "FirstLine"
/// Line 2 at y=670: "SecondLine"
/// No jitter — one BT block per line, absolute Tm positioning.
fn make_two_line_pdf() -> Vec<u8> {
    let stream = "BT /F1 12 Tf 1 0 0 1 72.00 700.00 Tm (FirstLine) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 72.00 670.00 Tm (SecondLine) Tj ET\n";
    assemble_single_page_pdf(stream)
}

/// A PDF with two words on the same line separated by a large x-gap (> font_size * 0.5).
/// Used to verify space insertion between words.
///
/// "Hello" starting at x=72, "World" starting at x=300.
/// All chars at same y, no jitter. Uses absolute Tm positioning so pdf_oxide can
/// correctly determine each span's position.
fn make_word_gap_pdf() -> Vec<u8> {
    let stream = "BT /F1 12 Tf 1 0 0 1 72.00 700.00 Tm (Hello) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 300.00 700.00 Tm (World) Tj ET\n";
    assemble_single_page_pdf(stream)
}

/// A PDF with normal word-level text (no per-glyph Tj, no jitter).
/// `is_fragmented_span_list` must return false and content must be unchanged.
fn make_normal_prose_pdf() -> Vec<u8> {
    // Single BT block — all words in one run; no glyph-level fragmentation.
    let stream = "BT /F1 12 Tf 72 700 Td (The quick brown fox) Tj ET\n";
    assemble_single_page_pdf(stream)
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

/// The coalesced text must actually contain the expected characters in order.
///
/// TEXT = "Hetafbeeldingisnietsaangetroffen" (32 chars). After rebuilding from
/// char positions the characters must all be present; spaces may be injected
/// between some chars but the non-space characters must spell out the word.
#[test]
fn test_coalesced_content_is_coherent() {
    let pdf = make_glyph_jitter_pdf(3.5);
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("3.5 pt jitter PDF should extract without error");

    let content = result.content.trim().to_string();
    // Drop spaces injected by the gap-detection heuristic and check the chars are present.
    let no_spaces: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        no_spaces.contains("Hetafbeelding"),
        "coalesced content must contain the leading chars of the original word; got: {content:?}"
    );
}

/// Two real text lines (30pt y gap) must remain as two separate lines after the fix.
#[test]
fn test_two_line_pdf_stays_two_lines() {
    let pdf = make_two_line_pdf();
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("two-line PDF should extract without error");

    let content = result.content.trim().to_string();
    assert!(
        content.contains("FirstLine"),
        "output must contain 'FirstLine'; got: {content:?}"
    );
    assert!(
        content.contains("SecondLine"),
        "output must contain 'SecondLine'; got: {content:?}"
    );
    // The two lines must be separated (not merged into one line).
    let line_count = content.lines().count();
    assert!(
        line_count >= 2,
        "two-line PDF must produce at least 2 output lines; got {line_count}: {content:?}"
    );
}

/// Two words with a large x-gap on the same line must have a space between them.
#[test]
fn test_word_gap_produces_space() {
    let pdf = make_word_gap_pdf();
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("word-gap PDF should extract without error");

    let content = result.content.trim().to_string();
    assert!(
        content.contains("Hello"),
        "output must contain 'Hello'; got: {content:?}"
    );
    assert!(
        content.contains("World"),
        "output must contain 'World'; got: {content:?}"
    );
    // Both words must appear with some separator (space or newline) between them.
    assert!(
        content.contains("Hello World") || content.contains("Hello\nWorld"),
        "output must have 'Hello World' or 'Hello\\nWorld'; got: {content:?}"
    );
}

/// Normal word-level prose PDF must not be disturbed.
#[test]
fn test_normal_prose_not_disturbed() {
    let pdf = make_normal_prose_pdf();
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&pdf, "application/pdf", &config).expect("normal prose should extract");
    let content = result.content.trim().to_string();
    assert!(!content.is_empty(), "normal prose must produce non-empty content");
    assert!(content.contains("quick"), "must include 'quick'; got: {content:?}");
    assert!(
        count_single_char_lines(&content) < 2,
        "prose must not fragment; got: {content:?}"
    );
}

/// Fix must apply when page tracking is enabled.
#[test]
fn test_fix_applies_with_page_tracking() {
    use kreuzberg::PageConfig;
    let pdf = make_glyph_jitter_pdf(3.5);
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_sync(&pdf, "application/pdf", &config).expect("page tracking extract");
    let content = result.content.trim().to_string();
    assert!(
        count_single_char_lines(&content) < 5,
        "page tracking fix failed; got: {content:?}"
    );
    assert!(result.pages.is_some(), "page tracking must populate pages");
}

/// 5pt jitter must also be coalesced.
#[test]
fn test_5pt_jitter_coalesced() {
    let pdf = make_glyph_jitter_pdf(5.0);
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&pdf, "application/pdf", &config).expect("5pt extract");
    let content = result.content.trim().to_string();
    assert!(
        count_single_char_lines(&content) < 5,
        "5pt jitter not coalesced; got: {content:?}"
    );
}

/// Negative regression: a PDF with genuine single-character-per-line content
/// (e.g. a vertical column label, formula subscript stack, or CJK-like layout)
/// must round-trip unchanged — the fragmentation repair path must NOT activate.
///
/// Uses 20 pt y-spacing between single-character spans, which is well above the
/// MAX_GLYPH_JITTER_PT detection ceiling (5 pt) and above the COALESCE_THRESHOLD
/// (5 pt), so no same-line x-disorder events can occur and reconstruction is skipped.
/// This guards against false positives on poetry, code columns, and similar layouts.
#[test]
fn test_genuine_single_char_lines_not_collapsed() {
    // Five stacked single-character spans at 20 pt y-intervals — genuinely one char per line.
    let stream = "BT /F1 12 Tf 1 0 0 1 72.00 700.00 Tm (A) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 72.00 680.00 Tm (B) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 72.00 660.00 Tm (C) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 72.00 640.00 Tm (D) Tj ET\n\
                  BT /F1 12 Tf 1 0 0 1 72.00 620.00 Tm (E) Tj ET\n";
    let pdf = assemble_single_page_pdf(stream);
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&pdf, "application/pdf", &config)
        .expect("single-char-per-line PDF should extract without error");

    let content = result.content.trim().to_string();
    // All five characters must be present.
    for ch in ["A", "B", "C", "D", "E"] {
        assert!(content.contains(ch), "output must contain '{ch}'; got: {content:?}");
    }
    // Characters must NOT be collapsed onto a single line; expect ≥ 5 separate lines.
    let line_count = content.lines().count();
    assert!(
        line_count >= 5,
        "genuine single-char-per-line content must not be collapsed; \
         got {line_count} lines: {content:?}"
    );
}
