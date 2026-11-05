#![cfg(feature = "pdf")]
mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, OutputFormat};

#[test]
fn original_posted_panels_keep_eight_distinct_lines_without_a_table() {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let (bytes, expected_rows) = helpers::pdf_panels::issue_2323_panels(1.0, 2);
    let source = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("native reads source operators");
    let source_text =
        String::from_utf8(source.get_page_content_data(0).expect("source content")).expect("ASCII operators");
    assert_eq!(source_text.matches("BT /F1 8 Tf").count(), 54);
    let decoded = source
        .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::ColumnAware)
        .expect("native source spans");
    assert!(!decoded.spans.is_empty(), "real native spans must be decoded");
    let document =
        extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("original panel extraction");
    assert!(document.tables.is_empty(), "these are prose panels");
    let lines = document
        .content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(
        lines.len(),
        8,
        "each panel's source lines must stay distinct: {}",
        document.content
    );
    let letters = |text: &str| text.chars().filter(char::is_ascii_alphabetic).collect::<String>();
    let actual = lines.iter().map(|line| letters(line)).collect::<Vec<_>>();
    let mut positions = Vec::new();
    for expected in expected_rows {
        let expected = letters(&expected);
        assert_eq!(actual.iter().filter(|line| **line == expected).count(), 1);
        positions.push(actual.iter().position(|line| *line == expected).expect("source row"));
    }
    assert!(positions[..4].windows(2).all(|pair| pair[0] < pair[1]));
    assert!(positions[4..].windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(positions, (0..8).collect::<Vec<_>>());
}

#[test]
fn page_marker_prefix_does_not_shift_or_duplicate_repaired_prose() {
    let (bytes, expected) = helpers::pdf_panels::issue_2323_panels(1.0, 2);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            marker_format: "\n\nBoundary {page_num}\n\n".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(
        document.content.matches("Boundary 1").count(),
        1,
        "{}",
        document.content
    );
    let body = document
        .content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.contains("Boundary 1"))
        .collect::<Vec<_>>();
    assert_eq!(body.len(), 8, "{}", document.content);
    let letters = |text: &str| text.chars().filter(char::is_ascii_alphabetic).collect::<String>();
    assert_eq!(
        body.iter().map(|line| letters(line)).collect::<Vec<_>>(),
        expected.iter().map(|line| letters(line)).collect::<Vec<_>>()
    );
    assert!(document.tables.is_empty());
    assert_eq!(document.pages.as_ref().expect("native page").len(), 1);
}

#[test]
fn tilted_single_column_wraps_remain_one_ordinary_paragraph() {
    let (bytes, expected) = helpers::pdf_panels::issue_2323_panels(1.0, 1);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(
        document.content.lines().filter(|line| !line.trim().is_empty()).count(),
        1,
        "{}",
        document.content
    );
    let letters = |text: &str| text.chars().filter(char::is_ascii_alphabetic).collect::<String>();
    assert_eq!(letters(&document.content), letters(&expected.join(" ")));
    assert!(document.tables.is_empty());
}

#[test]
fn level_two_columns_keep_their_existing_paragraphs() {
    let (bytes, expected) = helpers::pdf_panels::issue_2323_panels(0.0, 2);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(
        document.content.lines().filter(|line| !line.trim().is_empty()).count(),
        2,
        "{}",
        document.content
    );
    let letters = |text: &str| text.chars().filter(char::is_ascii_alphabetic).collect::<String>();
    assert_eq!(letters(&document.content), letters(&expected.join(" ")));
    assert!(document.tables.is_empty());
}

#[cfg(feature = "layout-detection")]
#[test]
fn an_explicit_reading_order_request_keeps_normal_paragraph_assembly() {
    let (bytes, _) = helpers::pdf_panels::issue_2323_panels(1.0, 2);
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(xberg::PdfConfig {
            reading_order: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(
        document.content.lines().filter(|line| !line.trim().is_empty()).count(),
        2,
        "{}",
        document.content
    );
    assert!(document.tables.is_empty());
}

fn panels_with_metadata(tagged: bool) -> Vec<u8> {
    use lopdf::{Document, Object, dictionary};
    let (bytes, _) = helpers::pdf_panels::issue_2323_panels(1.0, 2);
    let mut pdf = Document::load_mem(&bytes).unwrap();
    if tagged {
        let root = pdf.trailer.get(b"Root").unwrap().as_reference().unwrap();
        pdf.get_object_mut(root)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("MarkInfo", dictionary! {"Marked" => true});
    } else {
        let annotation = pdf.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link",
        "Rect" => vec![72.into(), 690.into(), 150.into(), 710.into()],
        "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://example.invalid/source") }
        });
        let page = *pdf.get_pages().get(&1).unwrap();
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![annotation.into()]);
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn tagged_catalogs_and_link_annotations_keep_their_existing_paragraphs() {
    for tagged in [false, true] {
        let bytes = panels_with_metadata(tagged);
        let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
        assert_eq!(native.get_annotations(0).unwrap().len(), usize::from(!tagged));
        let config = ExtractionConfig {
            output_format: OutputFormat::Markdown,
            disable_ocr: true,
            use_cache: false,
            enable_quality_processing: false,
            pdf_options: Some(xberg::PdfConfig {
                extract_annotations: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
        assert_eq!(
            document
                .content
                .split("## Annotations")
                .next()
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            2,
            "{}",
            document.content
        );
        if !tagged {
            assert_eq!(document.annotations.as_ref().unwrap().len(), 1);
        }
    }
}

#[test]
fn accurate_helvetica_rows_preserve_every_word_with_and_without_page_markers() {
    for insert_page_markers in [false, true] {
        let (bytes, expected) = helpers::pdf_panels::accurate_helvetica_panels(1.0);
        let config = ExtractionConfig {
            output_format: OutputFormat::Markdown,
            disable_ocr: true,
            use_cache: false,
            enable_quality_processing: false,
            pages: Some(xberg::core::config::PageConfig {
                insert_page_markers,
                marker_format: "\n\nBoundary {page_num}\n\n".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
        let rows = document
            .content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != "Boundary 1")
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>();
        assert_eq!(rows, expected, "exact AFM source rows, markers={insert_page_markers}");
        assert_eq!(
            document.content.matches("Boundary 1").count(),
            usize::from(insert_page_markers)
        );
        assert!(document.tables.is_empty());
    }
}
