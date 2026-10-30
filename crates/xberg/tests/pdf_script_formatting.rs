#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};

const SUPERSCRIPTS: &str = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (7,2 x 10) Tj \
    /F1 8 Tf 3 Ts (9) Tj /F1 12 Tf 0 Ts (/l) Tj ET\n\
    BT /F1 12 Tf 1 0 0 1 72 620 Tm (10) Tj \
    /F1 8 Tf 3 Ts (5) Tj /F1 12 Tf 0 Ts ( KBE/ml) Tj ET\n";
const POSITIONED_SUPERSCRIPTS: &str = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (7,2 x 10) Tj ET\n\
    BT /F1 8 Tf 1 0 0 1 114.696 703 Tm (9) Tj ET\n\
    BT /F1 12 Tf 1 0 0 1 119.144 700 Tm (/l) Tj ET\n\
    BT /F1 12 Tf 1 0 0 1 72 620 Tm (10) Tj ET\n\
    BT /F1 8 Tf 1 0 0 1 85.344 623 Tm (5) Tj ET\n\
    BT /F1 12 Tf 1 0 0 1 89.792 620 Tm ( KBE/ml) Tj ET\n";
const SUBSCRIPT: &str = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (H) Tj \
    /F1 8 Tf -2 Ts (2) Tj /F1 12 Tf 0 Ts (O) Tj ET\n";
const ON_BASELINE: &str = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (10) Tj \
    /F1 8 Tf (9) Tj /F1 12 Tf (/l) Tj ET\n";

fn script_pdf(content: &str) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let stream = pdf.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("script PDF serializes");
    bytes
}

fn markdown(bytes: &[u8]) -> String {
    markdown_with_tables(bytes, false)
}

fn markdown_with_tables(bytes: &[u8], extract_tables: bool) -> String {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("script PDF extracts through the public native pipeline")
        .content
}

#[test]
fn should_decode_script_fixture_with_real_font_and_baseline_changes() {
    let cases = [
        (SUPERSCRIPTS, 21, '9', 700.0),
        (POSITIONED_SUPERSCRIPTS, 21, '9', 703.0),
        (SUBSCRIPT, 3, '2', 700.0),
    ];
    assert_eq!(cases.len(), 3);
    for (content, expected_glyphs, script_char, expected_baseline) in cases {
        let native = xberg_native_pdf::PdfDocument::from_bytes(script_pdf(content)).expect("native parses script PDF");
        assert_eq!(native.page_count().expect("native counts pages"), 1);
        let chars = native.extract_chars(0).expect("native extracts script glyphs");
        assert_eq!(chars.len(), expected_glyphs, "fixture glyph census");
        let scripts = chars
            .iter()
            .filter(|glyph| glyph.char == script_char)
            .collect::<Vec<_>>();
        assert_eq!(scripts.len(), 1, "fixture script census");
        assert_eq!(scripts[0].font_size, 8.0);
        assert_eq!(scripts[0].origin_y, expected_baseline);
        let spans = native
            .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .expect("native extracts script spans");
        let script = spans
            .spans
            .iter()
            .find(|span| span.text == script_char.to_string())
            .expect("script span");
        let default_spans = native
            .extract_page_text_with_options(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
            .expect("default native spans");
        let default_script = default_spans
            .spans
            .iter()
            .find(|span| span.text == script_char.to_string())
            .expect("default script");
        assert_eq!(
            default_script.bbox.y, 700.0,
            "default native script snapping remains unchanged"
        );
        assert_eq!(script.font_size, 8.0);
        assert_eq!(
            script.bbox.y,
            if content == POSITIONED_SUPERSCRIPTS {
                703.0
            } else {
                700.0
            }
        );
        assert_eq!(
            script.text_rise,
            if content == SUPERSCRIPTS {
                0.375
            } else if content == SUBSCRIPT {
                -0.25
            } else {
                0.0
            }
        );
        assert!(scripts[0].origin_x > 72.0, "script follows its base");
    }
}

#[test]
fn should_preserve_numeric_superscripts_on_their_own_visual_lines() {
    let cases = [SUPERSCRIPTS, POSITIONED_SUPERSCRIPTS];
    assert_eq!(cases.len(), 2);
    for content in cases {
        let output = markdown(&script_pdf(content));
        let expected = ["7,2 x 10^9^/l", "10^5^ KBE/ml"];
        assert_eq!(expected.len(), 2);
        for value in expected {
            assert_eq!(
                output.matches(value).count(),
                1,
                "expected exponent exactly once: {output:?}"
            );
        }
        assert_eq!(
            output.matches('^').count(),
            4,
            "two paired exponent markers: {output:?}"
        );
    }
}

#[test]
fn should_preserve_subscript_between_base_and_suffix() {
    assert_eq!(markdown(&script_pdf(SUBSCRIPT)).trim(), "H~2~O");
}

#[test]
fn should_preserve_prose_scripts_when_heuristic_table_detection_is_enabled() {
    let cases = [
        (SUPERSCRIPTS, "7,2 x 10^9^/l", 4),
        (POSITIONED_SUPERSCRIPTS, "7,2 x 10^9^/l", 4),
        (SUBSCRIPT, "H~2~O", 2),
    ];
    assert_eq!(cases.len(), 3);
    for (content, expected, expected_markers) in cases {
        let bytes = script_pdf(content);
        let without_tables = markdown_with_tables(&bytes, false);
        let with_tables = markdown_with_tables(&bytes, true);
        assert_eq!(
            with_tables.matches(expected).count(),
            1,
            "table detection must preserve the script exactly once: {with_tables:?}"
        );
        let marker = if content == SUBSCRIPT { '~' } else { '^' };
        assert_eq!(with_tables.matches(marker).count(), expected_markers);
        assert_eq!(with_tables, without_tables, "table detection must not change prose");
    }
}

#[test]
fn should_keep_same_baseline_digits_plain_with_table_detection_enabled() {
    let bytes = script_pdf(ON_BASELINE);
    let without_tables = markdown_with_tables(&bytes, false);
    let with_tables = markdown_with_tables(&bytes, true);
    assert_eq!(with_tables, without_tables);
    assert_eq!(with_tables.matches('9').count(), 1);
    assert_eq!(with_tables.matches('^').count(), 0);
    assert_eq!(with_tables.matches('~').count(), 0);
}

#[test]
fn should_leave_a_smaller_digit_on_the_base_baseline_unannotated() {
    let output = markdown(&script_pdf(ON_BASELINE));
    assert_eq!(
        output.matches('^').count(),
        0,
        "same-baseline digit is not an exponent: {output:?}"
    );
    assert_eq!(
        output.matches('~').count(),
        0,
        "same-baseline digit is not a subscript: {output:?}"
    );
    assert_eq!(
        output.matches('9').count(),
        1,
        "same-baseline digit remains present: {output:?}"
    );
}

#[test]
fn should_preserve_multidigit_exponents_and_multiple_internal_subscripts() {
    let cases = [
        (
            "BT /F1 12 Tf 1 0 0 1 72 700 Tm (10) Tj /F1 8 Tf 3 Ts (12) Tj /F1 12 Tf 0 Ts (/l) Tj ET",
            "10^12^/l",
        ),
        (
            "BT /F1 12 Tf 1 0 0 1 72 700 Tm (H) Tj /F1 8 Tf -2 Ts (2) Tj /F1 12 Tf 0 Ts (SO) Tj /F1 8 Tf -2 Ts (4) Tj /F1 12 Tf 0 Ts ( solution) Tj ET",
            "H~2~SO~4~ solution",
        ),
    ];
    assert_eq!(cases.len(), 2);
    for (content, expected) in cases {
        assert_eq!(markdown(&script_pdf(content)).trim(), expected);
    }
}

#[test]
fn should_keep_displaced_exponent_on_its_wrapped_visual_line() {
    let content = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (First visual line) Tj ET\n\
        BT /F1 12 Tf 1 0 0 1 72 682 Tm (10) Tj /F1 8 Tf 4 Ts (9) Tj /F1 12 Tf 0 Ts (/l) Tj ET";
    let output = markdown(&script_pdf(content));
    assert_eq!(output.matches("10^9^/l").count(), 1, "upper-range shift: {output:?}");
    assert!(output.find("First visual line").unwrap() < output.find("10^9^/l").unwrap());
}

#[test]
fn should_keep_confirmed_numeric_note_separate_from_a_true_exponent() {
    let content = "BT /F1 12 Tf 1 0 0 1 72 700 Tm (Clause 123) Tj /F1 8 Tf 3 Ts (1) Tj /F1 12 Tf 0 Ts ( continues.) Tj ET\n\
        BT /F1 12 Tf 1 0 0 1 72 660 Tm (Rate 10) Tj /F1 8 Tf 3 Ts (1) Tj /F1 12 Tf 0 Ts (+ 2 units) Tj ET\n\
        BT /F1 6 Tf 1 0 0 1 72 100 Tm (1) Tj /F1 8 Tf ( footnote body) Tj ET";
    let output = markdown(&script_pdf(content));
    assert!(output.contains("Clause 123 1 continues."), "note reference: {output:?}");
    assert_eq!(
        output.matches("Rate 10^1^+ 2 units").count(),
        1,
        "expression: {output:?}"
    );
    assert_eq!(
        output.matches('^').count(),
        2,
        "only the exponent is annotated: {output:?}"
    );
}

#[test]
fn should_preserve_scripts_inside_a_detected_ruled_pdf_table_gh2275() {
    let mut content = String::from("0.5 w 0 0 0 RG\n");
    for y in [660, 690, 720, 750] {
        content.push_str(&format!("50 {y} m 550 {y} l S\n"));
    }
    for x in [50, 200, 400, 550] {
        content.push_str(&format!("{x} 660 m {x} 750 l S\n"));
    }
    for (x, y, text) in [
        (60, 730, "Parameter"),
        (210, 730, "Value"),
        (410, 730, "Status"),
        (60, 700, "Count"),
        (410, 700, "Approved"),
        (60, 670, "Compound"),
        (410, 670, "Verified"),
    ] {
        content.push_str(&format!("BT /F1 12 Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET\n"));
    }
    content.push_str("BT /F1 12 Tf 1 0 0 1 210 700 Tm (10) Tj /F1 8 Tf 3 Ts (9) Tj /F1 12 Tf 0 Ts (/l) Tj ET\n");
    content.push_str("BT /F1 12 Tf 1 0 0 1 210 670 Tm (H) Tj /F1 8 Tf -2 Ts (2) Tj /F1 12 Tf 0 Ts (O) Tj ET\n");
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&script_pdf(&content), "application/pdf", &config)
        .expect("ruled script table extracts");
    assert_eq!(result.tables.len(), 1, "fixture must produce one actual table");
    assert_eq!(result.tables[0].cells.len(), 3);
    assert!(result.tables[0].cells.iter().all(|row| row.len() == 3));
    assert_eq!(result.content.matches("10^9^/l").count(), 1, "{}", result.content);
    assert_eq!(result.content.matches("H~2~O").count(), 1, "{}", result.content);
}
