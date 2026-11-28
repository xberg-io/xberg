use super::glyphs::{MathFont, math_font, spell_glyph};
use super::{Delimiters, is_equation_number, rewrite_math_spans_with, tex_is_reliable};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

/// Rewrite the math of one page as `$`-delimited LaTeX, or `None` when nothing changes.
fn rewrite_math_spans(spans: &[TextSpan]) -> Option<Vec<TextSpan>> {
    rewrite_math_spans_with(spans, Delimiters::Dollars)
}

fn span(text: &str, font: &str, size: f32, x: f32, width: f32, baseline: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, baseline, width, size),
        font_name: font.into(),
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

/// The page's spans as the plain path reads them, and the page's math rewrite.
fn fixture_page(relative: &str, page: usize) -> Option<(Vec<TextSpan>, Option<Vec<TextSpan>>)> {
    let bytes = crate::utils::read_test_fixture(relative)?;
    let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("fixture PDF must parse");
    let spans = document
        .extract_spans_with_reading_order(page, xberg_native_pdf::document::ReadingOrder::ColumnAware)
        .expect("fixture page must extract");
    let rewritten = super::rewrite_page_math(&document, page, &spans);
    Some((spans, rewritten))
}

#[test]
fn should_rewrite_the_definition_on_an_arxiv_page() {
    // Page 6 of arXiv 1509.07624 sets `z ≡ 2r² − 1, (4)` as a display equation in CMMI/CMSY.
    let Some((spans, rewritten)) = fixture_page(".corpus-cache/pdf/1509.07624.pdf", 5) else {
        return;
    };
    let rewritten = rewritten.expect("a TeX math page has formulas");
    let display = rewritten
        .iter()
        .find(|span| span.text.starts_with("$$") && span.text.contains("\\equiv"))
        .unwrap_or_else(|| panic!("no display equation in {:?}", texts(&rewritten)));
    assert!(display.text.contains("r^{2}"), "{}", display.text);
    assert!(
        rewritten.iter().any(|span| span.text.trim() == "(4)"),
        "equation number kept"
    );
    // A span outside every formula is unchanged, or lost only the closing bracket a formula took.
    for span in rewritten.iter().filter(|span| !span.text.starts_with('$')) {
        assert!(
            spans
                .iter()
                .any(|source| source.text == span.text && source.bbox == span.bbox
                    || (source.text.ends_with(span.text.as_str()) && source.text.starts_with([')', ']']))),
            "a span outside any formula changed: {:?}",
            span.text
        );
    }
}

#[test]
fn should_not_touch_a_page_without_tex_math() {
    let Some((_, rewritten)) = fixture_page("pdf/embedded_images_tables.pdf", 0) else {
        return;
    };
    assert!(rewritten.is_none());
}

#[test]
fn should_delimit_formulas_with_sentinels_for_structure_assembly() {
    use super::sentinels::{INLINE_CLOSE, INLINE_OPEN};
    let spans = vec![
        span("where", "CMR10", 10.0, 100.0, 25.0, 700.0),
        span("δ", "CMMI10", 10.0, 128.0, 5.0, 700.0),
        span("n,n", "CMMI7", 7.0, 133.0, 10.0, 698.5),
        span("represents the Kronecker delta.", "CMR10", 10.0, 146.0, 130.0, 700.0),
    ];
    let rewritten = super::rewrite_math_spans_with(&spans, super::Delimiters::Sentinels).expect("inline formula");
    assert_eq!(rewritten[1].text, format!("{INLINE_OPEN}\\delta_{{n,n}}{INLINE_CLOSE}"));
}

#[test]
fn should_take_the_closing_bracket_from_the_following_prose() {
    let mut closing = span(") is", "CMR10", 10.0, 129.5, 14.0, 700.0);
    closing.char_widths = vec![4.0, 3.0, 3.0, 4.0];
    let spans = vec![
        span("the", "CMR10", 10.0, 90.0, 15.0, 700.0),
        span("G", "CMMI10", 10.0, 110.0, 7.0, 700.0),
        span("(", "CMR10", 10.0, 117.0, 4.0, 700.0),
        span("t", "CMMI10", 10.0, 121.0, 4.0, 700.0),
        span("σ", "CMMI7", 7.0, 125.0, 4.0, 703.6),
        closing,
    ];
    let rewritten = rewrite_math_spans(&spans).expect("inline formula is rewritten");
    assert_eq!(texts(&rewritten), ["the", "$G(t^{\\sigma})$", " is"]);
    assert_eq!(rewritten[2].bbox.x, 133.5);
    assert_eq!(rewritten[2].char_widths, [3.0, 3.0, 4.0]);
}

#[test]
fn should_leave_a_footnote_mark_after_inline_math_outside_it() {
    let spans = vec![
        span("rules choose", "CMR10", 10.0, 70.0, 55.0, 700.0),
        span("a", "CMMI10", 10.0, 128.0, 5.0, 700.0),
        span(".", "CMMI10", 10.0, 133.0, 3.0, 700.0),
        span("2", "CMR7", 7.0, 136.0, 4.0, 704.0),
        span("We only discuss", "CMR10", 10.0, 143.0, 70.0, 700.0),
    ];
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

#[test]
fn should_map_annotation_offsets_across_resolved_sentinels() {
    use super::sentinels::{INLINE_CLOSE, INLINE_OPEN, resolve, strip};
    let text = format!("a {INLINE_OPEN}x^{{2}}{INLINE_CLOSE} b");
    let resolved = resolve(&text, true);
    assert_eq!(resolved.text, "a $x^{2}$ b");
    assert_eq!(resolved.math.len(), 1);
    assert_eq!(&resolved.text[resolved.math[0].start..resolved.math[0].end], "$x^{2}$");
    let b = text.rfind('b').unwrap();
    assert_eq!(&resolved.text[resolved.remap(b)..], "b");
    assert_eq!(resolve(&text, false).text, "a x^{2} b");
    assert_eq!(strip(&text), "a x^{2} b");
}

/// The definition `z ≡ 2r² − 1,` with every glyph run drawn twice, a fake-bold overdraw.
#[test]
fn should_write_an_overdrawn_formula_once() {
    let mut spans = Vec::new();
    for offset in [0.0, 0.3] {
        spans.extend([
            span("z", "CMMI10", 10.0, 200.0 + offset, 5.0, 700.0),
            span("≡", "CMSY10", 10.0, 208.0 + offset, 8.0, 700.0),
            span("2", "CMR10", 10.0, 219.0 + offset, 5.0, 700.0),
            span("r", "CMMI10", 10.0, 224.0 + offset, 4.5, 700.0),
            span("2", "CMR7", 7.0, 228.8 + offset, 4.0, 703.6),
            span("−", "CMSY10", 10.0, 235.0 + offset, 7.8, 700.0),
            span("1,", "CMR10", 10.0, 245.0 + offset, 7.0, 700.0),
        ]);
    }
    let rewritten = rewrite_math_spans(&spans).expect("display equation is rewritten");
    assert_eq!(texts(&rewritten), ["$$z\\equiv2r^{2}-1,$$"]);
}

/// Different glyphs drawn over each other on one baseline do not resolve into a formula, so
/// the source text is kept rather than spelled as TeX.
#[test]
fn should_keep_the_source_when_axis_glyphs_collide() {
    let spans = vec![
        span("x", "CMMI10", 10.0, 200.0, 5.0, 700.0),
        span("y", "CMMI10", 10.0, 200.5, 5.0, 700.0),
        span("=", "CMR10", 10.0, 208.0, 7.8, 700.0),
        span("a", "CMMI10", 10.0, 219.0, 5.0, 700.0),
        span("2", "CMR7", 7.0, 224.0, 4.0, 703.6),
    ];
    assert!(rewrite_math_spans(&spans).is_none());
}

/// A raised script with no glyph of the row in reach keeps its place on an empty base; the
/// formula is still written rather than left as loose source glyphs.
#[test]
fn should_write_a_script_with_no_base_on_an_empty_base() {
    let spans = vec![
        span("2", "CMR7", 7.0, 204.0, 4.0, 703.6),
        span("x", "CMMI10", 10.0, 210.0, 5.0, 700.0),
        span("=", "CMR10", 10.0, 218.0, 7.8, 700.0),
        span("y", "CMMI10", 10.0, 229.0, 5.0, 700.0),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("formula with a baseless script is rewritten");
    assert_eq!(texts(&rewritten), ["$${}^{2}x=y$$"]);
}

/// Two aligned equations put their scripts at the same x; each script's raw baseline is the
/// one recorded at its own row, not the first row's.
#[test]
fn should_restore_each_row_its_own_script_baseline() {
    let row = |baseline: f32| {
        [
            span("a", "CMMI10", 10.0, 200.0, 5.0, baseline),
            span("=", "CMR10", 10.0, 208.0, 7.8, baseline),
            span("r", "CMMI10", 10.0, 219.0, 4.5, baseline),
            span("2", "CMR7", 7.0, 223.5, 4.0, baseline),
        ]
    };
    let spans: Vec<TextSpan> = row(700.0).into_iter().chain(row(680.0)).collect();
    let recorded = [(223.5, 7.0, 700.0, 703.6), (223.5, 7.0, 680.0, 683.6)];
    let (rewritten, _) =
        super::rewrite_with_baselines(&spans, Delimiters::Dollars, &super::RawBaselines::new(&recorded))
            .expect("both rows are rewritten");
    assert_eq!(texts(&rewritten), ["$$a=r^{2}$$", "$$a=r^{2}$$"]);
}

#[test]
fn rebuilt_tex_with_misplaced_scripts_is_not_published() {
    for broken in [
        "=_{2}\\sqrt{1+v1}",
        "\\frac{}{b}",
        "^{x^{3}}",
        "a+_{3}b",
        "\\frac{dy}{dx",
    ] {
        assert!(!tex_is_reliable(broken), "{broken}");
    }
    for sound in [
        "\\frac{dy}{dx}=x^{2}+1",
        "e^{-ax}",
        "f_{j+1}'",
        "{}^{14}\\mathrm{C}",
        "x_{1}^{2}",
        "(a+b)^{2}",
    ] {
        assert!(tex_is_reliable(sound), "{sound}");
    }
}

const MMI: &str = "RRQMNM+CMMI12";
const MR: &str = "TRYITZ+CMR12";
const MSY12: &str = "LKAHCF+CMSY10";

/// Gutenberg 33283 p139: `dy/dx = 1/(dx/dy)`, the inverse function's reciprocal written as a
/// fraction whose denominator is itself a fraction.
#[test]
fn should_write_a_fraction_whose_denominator_is_a_fraction() {
    let spans = vec![
        span("dy", MMI, 11.96, 137.59, 11.79, 314.65),
        span("1", MR, 11.96, 176.74, 5.85, 314.65),
        span("=", MR, 11.96, 154.58, 9.11, 306.57),
        span("dx", MMI, 11.96, 137.33, 12.73, 298.36),
        span("dx", MMI, 11.96, 173.30, 12.73, 299.58),
        span("dy", MMI, 11.96, 173.56, 11.79, 283.29),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("the equation is rewritten");
    assert_eq!(texts(&rewritten), ["$$\\frac{dy}{dx}=\\frac{1}{\\frac{dx}{dy}}$$"]);
}

/// Gutenberg 33283 p139: a problem number `(15)` printed above a row of full-size text is not
/// a script of the expression below it.
#[test]
fn should_not_script_a_full_size_row_label_onto_a_formula() {
    let spans = vec![
        span("(15)", MR, 11.96, 47.71, 20.81, 519.55),
        span("(", MR, 11.96, 75.57, 4.55, 511.35),
        span("x", MMI, 11.96, 80.12, 6.65, 511.35),
        span("+ 2)(", MR, 11.96, 89.43, 26.72, 511.35),
        span("x", MMI, 11.96, 116.15, 6.65, 511.35),
        span("2", "PSEHXM+CMR8", 7.97, 122.80, 4.23, 511.35),
        span("+", MR, 11.96, 130.19, 9.11, 511.35),
        span("x", MMI, 11.96, 141.95, 6.65, 511.35),
        span("+ 1)", MR, 11.96, 151.26, 22.16, 511.35),
        span("2", "PSEHXM+CMR8", 7.97, 173.42, 4.23, 511.35),
    ];
    if let Some(rewritten) = rewrite_math_spans(&spans) {
        for formula in texts(&rewritten) {
            assert!(!formula.contains("{}_") && !formula.contains("{}^"), "{formula}");
        }
    }
}

/// Gutenberg 33283 p161: `y = ∛x`, a CMEX radical drawn above the axis with its degree over
/// the radical's left edge.
#[test]
fn should_write_the_degree_of_a_radical() {
    let spans = vec![
        span("y", MMI, 11.96, 78.27, 5.71, 429.39),
        span("=", MR, 11.96, 87.73, 9.11, 429.39),
        span("p", "QTXDXT+CMEX10", 11.96, 100.80, 11.96, 439.37),
        span("3", "PXXQUV+CMR7", 6.97, 103.47, 3.97, 436.0),
        span("x", MMI, 11.96, 114.0, 6.65, 429.39),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("the equation is rewritten");
    assert_eq!(texts(&rewritten), ["$$y=\\sqrt[3]{x}$$"]);
}

#[test]
fn rebuilt_tex_with_scripts_that_are_labels_or_punctuation_is_not_published() {
    for broken in [
        "{}_{9.}x^{2}y_{2}^{2}+3xy_{\\times}2c+1_{.}",
        "{}^{12.}16a^{2}-9b^{2}{}^{\\div}4a-3b^{.}",
        "4440.3\\pm1.3_{+4^{-}}^{+4.1}{}_{4}{}_{.}{}_{.}{}_{1}",
        "{}_{(16)}5x^{2}+8x-12",
    ] {
        assert!(!tex_is_reliable(broken), "{broken}");
    }
}

/// arXiv 2312.13591 p5: a bound-state label and a dagger stacked at script size after a mass are
/// that mass's scripts, not a fraction `B / †`.
#[test]
fn should_not_write_two_script_size_marks_over_each_other_as_a_fraction() {
    let spans = vec![
        span("[4486.6, 4517.3]", "YBENFD+CMR10", 9.96, 100.0, 70.0, 500.0),
        span("B", "VAYNGT+CMR7", 6.97, 171.0, 4.5, 497.0),
        span("\u{2020}", "PJJXZJ+CMSY7", 6.97, 171.0, 4.5, 503.5),
    ];
    if let Some(rewritten) = rewrite_math_spans(&spans) {
        assert!(
            !texts(&rewritten).iter().any(|tex| tex.contains("\\frac")),
            "{:?}",
            texts(&rewritten)
        );
    }
}

/// Gutenberg 36670 p140, exercise 10: two fractions joined by a `×` on the middle line, with
/// no relation to anchor the axis. The line of operators between the numerators and the
/// denominators is the axis, not the widest line.
#[test]
fn should_take_the_operator_line_between_two_fraction_rows_as_the_axis() {
    let cmr8 = "GFCZPI+CMR8";
    let cmr12 = "FQBHCU+CMR12";
    let mmi = "PDPVDU+CMMI12";
    let spans = vec![
        span("a", mmi, 11.96, 210.34, 6.14, 416.54),
        span("2", cmr8, 7.97, 216.49, 4.23, 420.1),
        span("\u{2212}", MSY12, 11.96, 223.88, 9.30, 416.54),
        span(" 100", cmr12, 11.96, 233.18, 20.21, 416.54),
        span("10.", "XULGOR+CMBX12", 11.96, 184.16, 17.19, 408.45),
        span("a", mmi, 11.96, 273.32, 6.14, 416.54),
        span(" \u{2212}", MSY12, 11.96, 279.47, 11.95, 416.54),
        span(" 3", cmr12, 11.96, 291.42, 8.51, 416.54),
        span("\u{d7}", MSY12, 11.96, 257.24, 9.30, 408.45),
        span(".", cmr12, 11.96, 304.05, 3.25, 408.45),
        span("a", mmi, 11.96, 216.20, 6.14, 400.25),
        span("2", cmr8, 7.97, 222.34, 4.23, 403.8),
        span("\u{2212}", MSY12, 11.96, 229.73, 9.30, 400.25),
        span(" 9", cmr12, 11.96, 239.03, 8.51, 400.25),
        span("a", mmi, 11.96, 270.39, 6.14, 400.25),
        span(" \u{2212}", MSY12, 11.96, 276.54, 11.95, 400.25),
        span(" 10", cmr12, 11.96, 288.49, 14.36, 400.25),
    ];
    let rewritten = rewrite_math_spans(&spans).expect("the exercise is rewritten");
    assert_eq!(
        texts(&rewritten),
        ["$$10.\\frac{a^{2}-100}{a^{2}-9}\\times\\frac{a-3}{a-10}.$$"]
    );
}
