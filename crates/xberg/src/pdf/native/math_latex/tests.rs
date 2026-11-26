use super::glyphs::{MathFont, math_font, spell_glyph};
use super::{is_equation_number, rewrite_math_spans};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

fn span(text: &str, font: &str, size: f32, x: f32, width: f32, baseline: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, baseline, width, size),
        font_name: font.to_string(),
        font_size: size,
        ..TextSpan::default()
    }
}

fn texts(spans: &[TextSpan]) -> Vec<&str> {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn should_classify_tex_math_families_and_leave_text_fonts_alone() {
    assert_eq!(math_font("ABCDEF+CMMI10"), Some(MathFont::Italic));
    assert_eq!(math_font("CMEX10"), Some(MathFont::Italic));
    assert_eq!(math_font("CMSY10"), Some(MathFont::Symbol));
    assert_eq!(math_font("MSBM10"), Some(MathFont::Blackboard));
    assert_eq!(math_font("EUFM10"), Some(MathFont::Fraktur));
    assert_eq!(math_font("LatinModernMath-Regular"), Some(MathFont::Italic));
    assert_eq!(math_font("Cambria Math"), Some(MathFont::Italic));
    for text_font in [
        "CMR10",
        "ABCDEF+CMTI10",
        "CMBX12",
        "SymbolMT",
        "Mathematica1Mono-Bold",
        "Times-Roman",
    ] {
        assert_eq!(math_font(text_font), None, "{text_font}");
    }
}

#[test]
fn should_spell_glyphs_as_latex() {
    let spell = |c: char, font: Option<MathFont>| spell_glyph(c, font).tex;
    assert_eq!(spell('α', Some(MathFont::Italic)), "\\alpha");
    assert!(spell_glyph('α', Some(MathFont::Italic)).command);
    assert_eq!(spell('≤', Some(MathFont::Symbol)), "\\leq");
    assert_eq!(spell('∂', Some(MathFont::Italic)), "\\partial");
    assert_eq!(spell('R', Some(MathFont::Blackboard)), "\\mathbb{R}");
    assert_eq!(spell('g', Some(MathFont::Fraktur)), "\\mathfrak{g}");
    assert_eq!(spell('L', Some(MathFont::Symbol)), "\\mathcal{L}");
    assert_eq!(spell('ℝ', None), "\\mathbb{R}");
    assert_eq!(spell('−', Some(MathFont::Symbol)), "-");
    assert_eq!(spell('′', Some(MathFont::Symbol)), "'");
    assert_eq!(spell('x', Some(MathFont::Italic)), "x");
    assert_eq!(spell('\u{FFFD}', Some(MathFont::Italic)), "");
}

#[test]
fn should_recognize_equation_numbers() {
    for number in ["(4)", "(2.13)", "(A.1)", "(3b)"] {
        assert!(is_equation_number(number), "{number}");
    }
    for text in ["4", "(a)", "()", "(x+1)"] {
        assert!(!is_equation_number(text), "{text}");
    }
}

#[test]
fn should_leave_a_page_without_math_fonts_untouched() {
    let spans = vec![
        span("Throughout this paper, x = 2 and", "CMR10", 10.0, 100.0, 150.0, 700.0),
        span("y2 holds.", "CMR10", 10.0, 100.0, 40.0, 688.0),
    ];
    assert!(rewrite_math_spans(&spans).is_none());
}

#[test]
fn should_write_a_display_equation_with_a_superscript() {
    let spans = vec![
        span(
            "Throughout this paper, we use the definition",
            "CMR10",
            10.0,
            100.0,
            200.0,
            730.0,
        ),
        span("z", "ABCDEF+CMMI10", 10.0, 200.0, 5.0, 700.0),
        span("≡", "ABCDEF+CMSY10", 10.0, 208.0, 8.0, 700.0),
        span("2", "ABCDEF+CMR10", 10.0, 219.0, 5.0, 700.0),
        span("r", "ABCDEF+CMMI10", 10.0, 224.0, 4.5, 700.0),
        span("2", "ABCDEF+CMR7", 7.0, 228.8, 4.0, 703.6),
        span("−", "ABCDEF+CMSY10", 10.0, 235.0, 7.8, 700.0),
        span("1,", "ABCDEF+CMR10", 10.0, 245.0, 7.0, 700.0),
        span("(4)", "ABCDEF+CMR10", 10.0, 400.0, 13.0, 700.0),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("display equation is rewritten");
    assert_eq!(
        texts(&rewritten),
        [
            "Throughout this paper, we use the definition",
            "$$z\\equiv2r^{2}-1,$$",
            "(4)"
        ]
    );
    assert_eq!(rewritten[1].bbox.y, 700.0);
    assert_eq!(rewritten[1].bbox.x, 200.0);
}

#[test]
fn should_wrap_an_inline_subscript_and_keep_the_prose() {
    let spans = vec![
        span("where", "CMR10", 10.0, 100.0, 25.0, 700.0),
        span("δ", "CMMI10", 10.0, 128.0, 5.0, 700.0),
        span("n,n", "CMMI7", 7.0, 133.0, 10.0, 698.5),
        span("represents the Kronecker delta.", "CMR10", 10.0, 146.0, 130.0, 700.0),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("inline formula is rewritten");
    assert_eq!(
        texts(&rewritten),
        ["where", "$\\delta_{n,n}$", "represents the Kronecker delta."]
    );
}

#[test]
fn should_stack_a_numerator_over_a_denominator() {
    let spans = vec![
        span("x", "CMMI10", 10.0, 200.0, 5.0, 700.0),
        span("=", "CMR10", 10.0, 208.0, 7.8, 700.0),
        span("1", "CMR10", 10.0, 221.0, 5.0, 706.8),
        span("2", "CMR10", 10.0, 221.0, 5.0, 693.1),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("fraction is rewritten");
    assert_eq!(texts(&rewritten), ["$$x=\\frac{1}{2}$$"]);
}

#[test]
fn should_keep_aligned_rows_as_separate_equations() {
    let spans = vec![
        span("a", "CMMI10", 10.0, 200.0, 5.0, 700.0),
        span("=", "CMR10", 10.0, 208.0, 7.8, 700.0),
        span("b", "CMMI10", 10.0, 219.0, 5.0, 700.0),
        span("c", "CMMI10", 10.0, 200.0, 5.0, 689.0),
        span("=", "CMR10", 10.0, 208.0, 7.8, 689.0),
        span("d", "CMMI10", 10.0, 219.0, 5.0, 689.0),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("rows are rewritten");
    assert_eq!(texts(&rewritten), ["$$a=b$$", "$$c=d$$"]);
}

#[test]
fn should_leave_a_lone_inline_symbol_and_list_bullets_as_they_are() {
    let symbol = vec![
        span("where", "CMR10", 10.0, 100.0, 25.0, 700.0),
        span("r", "CMMI10", 10.0, 128.0, 4.5, 700.0),
        span("is the radius.", "CMR10", 10.0, 135.0, 60.0, 700.0),
    ];
    assert!(rewrite_math_spans(&symbol).is_none());
    let bullet = vec![
        span("•", "CMSY10", 10.0, 100.0, 5.0, 700.0),
        span("First item of the list", "CMR10", 10.0, 110.0, 90.0, 700.0),
    ];
    assert!(rewrite_math_spans(&bullet).is_none());
}

#[test]
fn should_ignore_rotated_math() {
    let mut rotated = span("x", "CMMI10", 10.0, 200.0, 5.0, 700.0);
    rotated.rotation_degrees = 90.0;
    let mut script = span("2", "CMR7", 7.0, 205.0, 4.0, 703.6);
    script.rotation_degrees = 90.0;
    assert!(rewrite_math_spans(&[rotated, script]).is_none());
}

fn fixture_spans(relative: &str, page: usize) -> Option<Vec<TextSpan>> {
    let bytes = crate::utils::read_test_fixture(relative)?;
    let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("fixture PDF must parse");
    Some(
        document
            .extract_spans_with_reading_order(page, xberg_native_pdf::document::ReadingOrder::ColumnAware)
            .expect("fixture page must extract"),
    )
}

#[test]
fn should_rewrite_the_definition_on_an_arxiv_page() {
    // Page 6 of arXiv 1509.07624 sets `z ≡ 2r² − 1, (4)` as a display equation in CMMI/CMSY.
    let Some(spans) = fixture_spans(".corpus-cache/pdf/1509.07624.pdf", 5) else {
        return;
    };
    let rewritten = rewrite_math_spans(&spans).expect("a TeX math page has formulas");
    let display = rewritten
        .iter()
        .find(|span| span.text.starts_with("$$") && span.text.contains("\\equiv"))
        .unwrap_or_else(|| panic!("no display equation in {:?}", texts(&rewritten)));
    assert!(display.text.contains("r^{2}"), "{}", display.text);
    assert!(
        rewritten.iter().any(|span| span.text.trim() == "(4)"),
        "equation number kept"
    );
    for span in rewritten.iter().filter(|span| !span.text.starts_with('$')) {
        assert!(
            spans
                .iter()
                .any(|source| source.text == span.text && source.bbox == span.bbox),
            "a span outside any formula changed: {:?}",
            span.text
        );
    }
}

#[test]
fn should_not_touch_a_page_without_tex_math() {
    let Some(spans) = fixture_spans("pdf/embedded_images_tables.pdf", 0) else {
        return;
    };
    assert!(rewrite_math_spans(&spans).is_none());
}

#[test]
fn should_leave_a_table_column_of_negative_numbers_untouched() {
    let mut spans = Vec::new();
    for (row, value) in ["3.4", "12.5", "0.7"].iter().enumerate() {
        let baseline = 700.0 - 15.0 * row as f32;
        spans.push(span("Row", "CMR10", 10.0, 100.0, 20.0, baseline));
        spans.push(span("−", "CMSY10", 10.0, 200.0, 7.8, baseline));
        spans.push(span(value, "CMR10", 10.0, 208.0, 16.0, baseline));
        spans.push(span("1.25", "CMR10", 10.0, 236.0, 18.0, baseline));
        spans.push(span("−", "CMSY10", 10.0, 266.0, 7.8, baseline));
        spans.push(span("4.0", "CMR10", 10.0, 274.0, 12.0, baseline));
    }
    assert!(rewrite_math_spans(&spans).is_none());
}
