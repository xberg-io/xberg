#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use std::fmt::Write;
use xberg::core::config::{ExtractionConfig, OutputFormat, PageConfig, PdfBackend, PdfConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};

const WRAPPED_LINE: &str = "that wraps to a second line";
const CAPTION: &str = "Labels and amounts for the current reporting period";

struct Row {
    label: String,
    value: Option<String>,
    continuation: bool,
    y: i32,
}

fn table_rows() -> Vec<Row> {
    let mut y = 750;
    (1..=40)
        .map(|index| {
            let continuation = index % 7 == 0;
            let note = if continuation { " with a long note" } else { "" };
            let row = Row {
                label: format!("Item {index}{note}"),
                value: (index % 8 != 0).then(|| format!("Value {}", index * 11)),
                continuation,
                y,
            };
            y -= 12 * (if continuation { 2 } else { 1 }) + 2;
            row
        })
        .collect()
}

fn draw(operators: &mut String, x: i32, y: i32, text: &str) {
    writeln!(operators, "BT 1 0 0 1 {x} {y} Tm ({text}) Tj T* ET").expect("write fixture operation");
}

fn draw_label(operators: &mut String, row: &Row) {
    draw(operators, 72, row.y, &row.label);
    if row.continuation {
        draw(operators, 72, row.y - 12, WRAPPED_LINE);
    }
}

fn draw_value(operators: &mut String, row: &Row) {
    if let Some(value) = &row.value {
        draw(operators, 360, row.y, value);
    }
}

fn table_pdf(rows: &[Row], paint_by_column: bool, include_caption: bool) -> Vec<u8> {
    let mut operators = String::new();
    if include_caption {
        operators.push_str("BT /F1 14 Tf 16 TL ET\n");
        draw(&mut operators, 72, 775, CAPTION);
    }
    operators.push_str("BT /F1 10 Tf 12 TL ET\n");
    for row in rows {
        draw_label(&mut operators, row);
        if !paint_by_column {
            draw_value(&mut operators, row);
        }
    }
    if paint_by_column {
        for row in rows {
            draw_value(&mut operators, row);
        }
    }
    one_page_pdf(operators)
}

fn one_page_pdf(operators: String) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let content = pdf.add_object(Stream::new(dictionary! {}, operators.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "Contents" => content,
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("write synthetic PDF");
    bytes
}

fn extract(pdf: &[u8], output_format: OutputFormat) -> ExtractedDocument {
    let result = extract_bytes_document_blocking(
        pdf,
        "application/pdf",
        &ExtractionConfig {
            disable_ocr: true,
            use_cache: false,
            output_format,
            pages: Some(PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            pdf_options: Some(PdfConfig {
                backend: PdfBackend::Native,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("native PDF extraction");
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
    assert!(result.tables.is_empty(), "the fixture consists of unruled text rows");
    result
}

fn texts(result: &ExtractedDocument) -> [&str; 2] {
    let pages = result.pages.as_deref().expect("tracked native page");
    assert_eq!(pages.len(), 1);
    [&result.content, &pages[0].content]
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect()
}

fn expected_rows(rows: &[Row], join_continuation: bool) -> Vec<String> {
    let mut expected = Vec::new();
    for row in rows {
        let mut line = row.label.clone();
        if let Some(value) = &row.value {
            write!(line, " {value}").expect("write expected row");
        }
        if row.continuation && join_continuation {
            write!(line, " {WRAPPED_LINE}").expect("write expected continuation");
        }
        expected.push(line);
        if row.continuation && !join_continuation {
            expected.push(WRAPPED_LINE.to_owned());
        }
    }
    expected
}

fn assert_word_inventory(text: &str, expected_lines: &[String]) {
    let mut actual: Vec<_> = text.split_whitespace().collect();
    let mut expected: Vec<_> = expected_lines.iter().flat_map(|line| line.split_whitespace()).collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn row_painted_native_table_keeps_each_value_with_its_label() {
    let rows = table_rows();
    let result = extract(&table_pdf(&rows, false, false), OutputFormat::Plain);
    let expected = expected_rows(&rows, false);
    for text in texts(&result) {
        assert_word_inventory(text, &expected);
        assert_eq!(lines(text), expected, "value must remain on its source row");
    }
}

#[test]
fn column_painted_text_with_the_same_geometry_keeps_legacy_order() {
    let rows = table_rows();
    let result = extract(&table_pdf(&rows, true, false), OutputFormat::Plain);
    for text in texts(&result) {
        assert_word_inventory(text, &expected_rows(&rows, false));
        assert_eq!(lines(text)[..2], ["Item 1", "Item 2"]);
    }
}

#[test]
fn a_caption_keeps_the_mixed_page_on_its_legacy_path() {
    let rows = table_rows();
    let result = extract(&table_pdf(&rows, false, true), OutputFormat::Plain);
    let mut expected = expected_rows(&rows, false);
    expected.push(CAPTION.to_owned());
    for text in texts(&result) {
        assert_word_inventory(text, &expected);
        assert_eq!(lines(text)[..3], [CAPTION, "Item 1", "Item 2"]);
    }
}

#[test]
fn markdown_keeps_its_existing_wrapped_row_grouping() {
    let rows = table_rows();
    let result = extract(&table_pdf(&rows, false, false), OutputFormat::Markdown);
    for text in texts(&result) {
        assert_eq!(lines(text), expected_rows(&rows, true));
    }
}

#[test]
fn aligned_or_offset_two_column_prose_stays_column_major() {
    let left = [
        "The narrow trail climbs into the hills.",
        "A small group paused beside the brook.",
        "Leaves shifted softly in the evening.",
        "The guide opened a folded trail map.",
        "Each marker named a different valley.",
        "The path turned sharply after the bend.",
        "A low stone wall bordered the meadow.",
        "The hikers could see the distant ridge.",
    ];
    let right = [
        "The museum hired a new archivist.",
        "Her first task was a box of records.",
        "The papers described an old factory.",
        "Several letters mentioned a strike.",
        "One envelope contained a short note.",
        "The ink had faded near the edges.",
        "A ledger gave dates for the orders.",
        "The names agreed with the accounts.",
    ];
    for offset in [0, 2] {
        let mut operators = String::from("BT /F1 10 Tf 12 TL ET\n");
        for (index, (a, b)) in left.iter().zip(right).enumerate() {
            let y = 750 - 15 * i32::try_from(index).expect("small fixture");
            draw(&mut operators, 72, y, a);
            draw(&mut operators, 337, y - offset, b);
        }
        let result = extract(&one_page_pdf(operators), OutputFormat::Plain);
        let expected: Vec<_> = left.into_iter().chain(right).collect();
        for text in texts(&result) {
            assert_eq!(lines(text), expected);
        }
    }
}
