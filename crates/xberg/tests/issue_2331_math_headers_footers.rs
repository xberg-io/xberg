//! Issue #2331: Markdown keeps math and keeps running headers/footers out of the body.
//!
//! Three structural properties, each from a fixture built in the test:
//! - an equation line becomes display math (`$$...$$`),
//! - an equation inside a sentence becomes inline math (`$...$`),
//! - a running header whose only change is its page number does not reach the body.

#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const PDF_MIME: &str = "application/pdf";

/// One A4 page per content stream, Helvetica, no embedded font.
///
/// Each page is a list of `(x, y, size, text)` runs in PDF points.
fn build_pdf(pages: &[Vec<(f32, f32, f32, String)>]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: Vec<String> = (0..pages.len()).map(|index| format!("{} 0 R", 4 + 2 * index)).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    for runs in pages {
        let stream: String = runs
            .iter()
            .map(|(x, y, size, text)| format!("BT /F1 {size} Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET\n"))
            .collect();
        let contents_id = objects.len() + 2;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {contents_id} 0 R \
                 /Resources << /Font << /F1 3 0 R >> >> >>"
            )
            .into_bytes(),
        );
        objects.push(format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()).into_bytes());
    }
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn line(x: f32, y: f32, size: f32, text: &str) -> (f32, f32, f32, String) {
    (x, y, size, text.to_string())
}

fn extract(pages: &[Vec<(f32, f32, f32, String)>], output_format: OutputFormat) -> String {
    let config = ExtractionConfig {
        output_format,
        use_cache: false,
        ..Default::default()
    };
    let result =
        extract_bytes_document_blocking(&build_pdf(pages), PDF_MIME, &config).expect("the fixture must extract");
    result.content
}

/// An equation on its own line is display math.
#[test]
fn an_equation_line_becomes_display_math() {
    let markdown = extract(
        &[vec![
            line(72.0, 760.0, 16.0, "A Study of Motion"),
            line(72.0, 700.0, 12.0, "x = y^2 + 1"),
            line(72.0, 660.0, 12.0, "The object moves with constant acceleration."),
        ]],
        OutputFormat::Markdown,
    );

    assert_eq!(
        markdown.trim(),
        "A Study of Motion\n\n$$\nx = y^2 + 1\n$$\n\nThe object moves with constant acceleration.",
        "an equation line must be written as display math"
    );
}

/// An equation inside a sentence is inline math, and Plain keeps the source text.
#[test]
fn an_equation_inside_a_sentence_becomes_inline_math() {
    let pages = [vec![
        line(72.0, 760.0, 16.0, "A Study of Circles"),
        line(72.0, 700.0, 12.0, "The area of a circle is A = pi r^2 for a radius r."),
    ]];

    let markdown = extract(&pages, OutputFormat::Markdown);
    assert_eq!(
        markdown.trim(),
        "A Study of Circles\n\nThe area of a circle is $A = pi r^2$ for a radius r.",
        "an equation inside a sentence must be written as inline math"
    );

    let plain = extract(&pages, OutputFormat::Plain);
    assert_eq!(
        plain.trim(),
        "A Study of Circles\n\nThe area of a circle is A = pi r^2 for a radius r.",
        "plain output keeps the source text without math delimiters"
    );
}

/// A running header that changes only its page number is still one running line.
#[test]
fn a_repeated_running_header_stays_out_of_the_body() {
    let repeated_body = "A sentence that repeats on every page of this report.";
    let pages: Vec<Vec<(f32, f32, f32, String)>> = (1..=3)
        .map(|page| {
            vec![
                line(72.0, 800.0, 9.0, &format!("ACME ANNUAL REPORT 2026 - PAGE {page}")),
                line(72.0, 700.0, 11.0, &format!("Page {page} body line one of the report.")),
                line(72.0, 680.0, 11.0, "The quick brown fox jumps over the lazy dog."),
                line(72.0, 660.0, 11.0, "It continues for several lines of ordinary text."),
                line(72.0, 400.0, 11.0, repeated_body),
            ]
        })
        .collect();

    let markdown = extract(&pages, OutputFormat::Markdown);

    assert!(
        !markdown.contains("ACME"),
        "the running header must not reach the body: {markdown}"
    );
    for page in 1..=3 {
        assert!(
            markdown.contains(&format!("Page {page} body line one of the report.")),
            "page {page} body must stay: {markdown}"
        );
    }
    assert_eq!(
        markdown.matches(repeated_body).count(),
        3,
        "a line repeated in the body is content, not furniture: {markdown}"
    );
}
