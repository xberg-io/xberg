#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat};

const IMAGE: &[u8] = include_bytes!("fixtures/ocr_source_row_layout_2432/image-11-no-leaders.png");
const TINTED: &[u8] = include_bytes!("fixtures/ocr_source_row_layout_2432/image-20-tinted.png");

fn config(format: OutputFormat) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "output_format": format,
        "pages": {"extract_pages": true},
        "ocr": {"backend": "tesseract", "language": ["eng"], "element_config": {
            "include_elements": true, "min_level": "word", "build_hierarchy": true
        }}
    }))
    .unwrap()
}

fn pinned_english_model() -> tempfile::TempDir {
    use sha2::{Digest, Sha256};

    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    models
}

fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    extract_with_mime(bytes, "image/png", config)
}

fn extract_with_mime(bytes: &[u8], mime: &str, config: &ExtractionConfig) -> ExtractedDocument {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut batch = runtime
        .block_on(xberg::extract(
            ExtractInput::from_bytes(bytes.to_vec(), mime, None),
            config,
        ))
        .unwrap();
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    assert_eq!(batch.results.len(), 1);
    batch.results.pop().unwrap()
}

fn expected_rows(tinted: bool) -> Vec<Vec<&'static str>> {
    let source = if tinted {
        include_str!("fixtures/ocr_source_row_layout_2432/image-20-tinted-source-rows.tsv")
    } else {
        include_str!("fixtures/ocr_source_row_layout_2432/image-11-source-rows.tsv")
    };
    let mut rows: Vec<Vec<_>> = source.lines().map(|line| line.split('\t').collect()).collect();
    assert_eq!(rows.len(), 13);
    if tinted {
        assert_eq!(rows[1], ["Signal timing", "59", "860,596", "(3,091)"]);
        rows[1][1] = "BY)";
    } else {
        assert_eq!(rows[7], ["Storm drains", "86", "346,028", "(1,463)"]);
        rows[7][3] = "(1.463)";
    }
    rows
}

fn assert_spatial_rows(doc: &ExtractedDocument, expected: &[Vec<&str>]) {
    use xberg::types::{OcrBoundingGeometry, OcrElementLevel};
    let mut words: Vec<_> = doc
        .ocr_elements
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|element| {
            if element.level != OcrElementLevel::Word {
                return None;
            }
            let OcrBoundingGeometry::Rectangle { left, top, height, .. } = element.geometry else {
                panic!("fixture words must retain rectangular geometry");
            };
            Some((top + height / 2, left, height, element.text.as_str()))
        })
        .collect();
    let mut heights: Vec<_> = words.iter().map(|word| word.2).collect();
    heights.sort_unstable();
    let tolerance = heights[heights.len() / 2];
    words.sort_by_key(|word| (word.0, word.1));
    let mut rows: Vec<Vec<(u32, u32, u32, &str)>> = Vec::new();
    for word in words {
        if rows.last().is_none_or(|row| word.0.abs_diff(row[0].0) > tolerance) {
            rows.push(Vec::new());
        }
        rows.last_mut().unwrap().push(word);
    }
    assert_eq!(rows.len(), expected.len(), "source row geometry");
    for (row, cells) in rows.iter_mut().zip(expected) {
        row.sort_by_key(|word| word.1);
        let recognized: Vec<_> = row.iter().map(|word| word.3).collect();
        let source: Vec<_> = cells.iter().flat_map(|cell| cell.split_whitespace()).collect();
        assert_eq!(recognized, source, "literal words on each source row, left to right");
    }
}

fn assert_html_table(doc: &ExtractedDocument, expected: &[Vec<&str>]) {
    assert!(doc.metadata.ocr_used);
    assert_eq!(doc.metadata.additional.get("psm").unwrap(), "11");
    let [table] = doc.tables.as_slice() else {
        panic!("expected one certified HTML table; got {}", doc.tables.len());
    };
    assert_eq!(table.cells, expected);
    let [page] = doc.pages.as_deref().unwrap() else {
        panic!("one source page")
    };
    assert_eq!(page.tables.len(), 1);
    assert_eq!(page.tables[0].cells, expected);
    assert_eq!(doc.content.matches("<table>").count(), 1);
    assert!(!doc.content.contains("<p>"), "no duplicate cell paragraphs");
    assert_eq!(doc.content.matches("<tr>").count(), 13);
    assert_eq!(doc.content.matches("<th>").count(), 4);
    assert_eq!(doc.content.matches("<td>").count(), 48);
    for (index, row) in expected.iter().enumerate() {
        let tag = if index == 0 { "th" } else { "td" };
        let cells = row
            .iter()
            .map(|cell| format!("<{tag}>{cell}</{tag}>\n"))
            .collect::<String>();
        let html = format!("<tr>\n{cells}</tr>");
        assert_eq!(
            doc.content.matches(&html).count(),
            1,
            "source row {index}: {}",
            doc.content
        );
    }
}

#[test]
fn sparse_html_keeps_all_source_rows_and_literal_word_geometry() {
    let expected = expected_rows(false);
    let selected = extract(IMAGE, &config(OutputFormat::Html));
    assert_spatial_rows(&selected, &expected);
    assert_html_table(&selected, &expected);
    let mut explicit = config(OutputFormat::Html);
    explicit.ocr.as_mut().unwrap().tesseract_config = Some(xberg::types::TesseractConfig {
        psm: Some(11),
        ..Default::default()
    });
    let ordinary = extract(IMAGE, &explicit);
    assert!(ordinary.tables.is_empty());
    assert_eq!(
        serde_json::to_value(selected.ocr_elements).unwrap(),
        serde_json::to_value(ordinary.ocr_elements).unwrap()
    );
}

#[test]
fn tinted_html_keeps_header_separate_and_preserves_the_recognized_cell() {
    let expected = expected_rows(true);
    let result = extract(TINTED, &config(OutputFormat::Html));
    assert_spatial_rows(&result, &expected);
    assert_html_table(&result, &expected);
}

#[test]
fn html_explicit_segmentation_disabled_tables_and_structure_keep_their_route() {
    for mode in ["explicit", "disabled", "structured"] {
        let mut config = config(OutputFormat::Html);
        match mode {
            "explicit" => {
                config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::types::TesseractConfig {
                    psm: Some(11),
                    ..Default::default()
                })
            }
            "disabled" => {
                config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::types::TesseractConfig {
                    enable_table_detection: false,
                    ..Default::default()
                })
            }
            _ => config.include_document_structure = true,
        }
        let result = extract(IMAGE, &config);
        assert!(result.tables.is_empty(), "{mode}");
        assert!(!result.content.contains("<table>"), "{mode}");
        assert_spatial_rows(&result, &expected_rows(false));
    }
}

#[test]
fn html_columned_prose_stays_prose_and_json_remains_unassociated() {
    let prose = include_bytes!("fixtures/ocr_dot_leader_2503/two_column.png");
    let result = extract(prose, &config(OutputFormat::Html));
    assert!(!result.content.trim().is_empty());
    assert!(result.tables.is_empty());
    assert!(!result.content.contains("<table>"));
    let json = extract(IMAGE, &config(OutputFormat::Json));
    assert!(json.tables.is_empty());
    assert_spatial_rows(&json, &expected_rows(false));
}

#[test]
fn html_page_transcript_keeps_every_certified_row_on_one_line() {
    let result = extract(IMAGE, &config(OutputFormat::Html));
    let expected = expected_rows(false);
    assert_spatial_rows(&result, &expected);
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("one source page")
    };
    let text = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(page.content.trim(), text);
}

#[cfg(feature = "html")]
#[test]
fn styled_html_uses_the_same_certified_source_cells() {
    let mut config = config(OutputFormat::Html);
    config.html_output = Some(Default::default());
    let result = extract(IMAGE, &config);
    let expected = expected_rows(false);
    assert_spatial_rows(&result, &expected);
    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells, expected);
    assert!(result.content.contains("kb-table"));
    assert_eq!(result.content.matches("<tr").count(), 13);
    for (index, row) in expected.iter().enumerate() {
        let tag = if index == 0 { "th" } else { "td" };
        let cells = row
            .iter()
            .map(|cell| format!(r#"<{tag} class="kb-{tag}">{cell}</{tag}>"#))
            .collect::<String>();
        let html = format!(r#"<tr class="kb-tr">{cells}</tr>"#);
        assert_eq!(result.content.matches(&html).count(), 1, "source row {index}");
    }
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("one source page")
    };
    assert_eq!(page.tables[0].cells, expected);
    let text = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    assert_eq!(page.content.trim(), text);
}

#[cfg(feature = "html")]
#[test]
fn html_modes_escape_literal_table_cells() {
    let csv = b"Field,<header> & value\nrow,<body> & value\n";
    for styled in [false, true] {
        let config = ExtractionConfig {
            output_format: OutputFormat::Html,
            html_output: styled.then(Default::default),
            use_cache: false,
            ..Default::default()
        };
        let result = extract_with_mime(csv, "text/csv", &config);
        let [table] = result.tables.as_slice() else {
            panic!("expected one source table");
        };
        assert_eq!(table.cells, [["Field", "<header> & value"], ["row", "<body> & value"]]);
        for (tag, label) in [("th", "header"), ("td", "body")] {
            let attributes = if styled {
                format!(r#" class="kb-{tag}""#)
            } else {
                String::new()
            };
            let cell = format!("<{tag}{attributes}>&lt;{label}&gt; &amp; value</{tag}>");
            assert_eq!(
                result.content.matches(&cell).count(),
                1,
                "styled={styled}: {}",
                result.content
            );
            assert!(
                !result.content.contains(&format!("<{label}> & value")),
                "styled={styled}"
            );
        }
    }
}

#[test]
fn explicit_sparse_mode_publishes_stock_rows_in_plain_page_text() {
    use sha2::{Digest, Sha256};

    const CLEAN: &[u8] = include_bytes!("fixtures/ocr_explicit_sparse_pages_2503/img_20_plain_nolead.png");
    const ROWS: &str = include_str!("fixtures/ocr_explicit_sparse_pages_2503/rows.tsv");
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let cases = [
        (CLEAN, ROWS.lines().map(|line| line.split('\t').collect()).collect()),
        (IMAGE, expected_rows(false)),
    ];
    for (bytes, rows) in cases {
        let mut config = config(OutputFormat::Plain);
        let ocr = config.ocr.as_mut().unwrap();
        ocr.tessdata_path = Some(models.path().to_path_buf());
        ocr.tesseract_config.get_or_insert_default().psm = Some(11);

        let with_words = extract(bytes, &config);
        assert_spatial_rows(&with_words, &rows);
        config.ocr.as_mut().unwrap().element_config = None;
        let doc = extract(bytes, &config);
        assert!(doc.ocr_elements.is_none());
        assert_eq!(doc.metadata.additional["psm"], "11");
        let expected = rows.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.tables[0].cells, rows);
        assert_eq!(doc.content.trim(), expected);
        let pages = doc.pages.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].content.trim(), expected, "complete stock rows on the page");
        assert_eq!(with_words.pages.unwrap()[0].content, pages[0].content);
    }
}

#[test]
fn explicit_auto_mode_publishes_existing_table_rows_in_plain_page_text() {
    const CLEAN: &[u8] = include_bytes!("fixtures/ocr_explicit_sparse_pages_2503/img_20_plain_nolead.png");
    const ROWS: &str = include_str!("fixtures/ocr_explicit_sparse_pages_2503/rows.tsv");
    let mut config = config(OutputFormat::Plain);
    let ocr = config.ocr.as_mut().unwrap();
    ocr.element_config = None;
    ocr.tesseract_config.get_or_insert_default().psm = Some(3);
    let doc = extract(CLEAN, &config);
    assert_eq!(doc.metadata.additional["psm"], "3");
    let expected = ROWS.replacen("Item", "ltem", 1).replace('\t', " ");
    assert_eq!(doc.content.trim(), expected.trim());
    assert_eq!(
        doc.pages.unwrap()[0].content.trim(),
        expected.trim(),
        "published table rows on the page"
    );
}

const TINTED_LEADERS: &[u8] = include_bytes!("fixtures/ocr_source_row_layout_2432/image-11-tinted-leaders.png");

#[test]
fn implicit_sparse_mode_certifies_tinted_leader_rows_in_plain_and_html() {
    let expected: Vec<Vec<_>> =
        include_str!("fixtures/ocr_source_row_layout_2432/image-11-tinted-leaders-recognized-rows.tsv")
            .lines()
            .map(|row| row.split('\t').collect())
            .collect();
    assert_eq!(expected.len(), 13);
    assert!(expected.iter().all(|row| row.len() == 4));
    assert_eq!(expected[6], ["Code review", "al", "822,978", "17,187"]);
    let transcript = expected.iter().map(|row| row.join(" ")).collect::<Vec<_>>().join("\n");
    let models = pinned_english_model();
    for format in [OutputFormat::Plain, OutputFormat::Html] {
        let mut extraction = config(format.clone());
        let ocr = extraction.ocr.as_mut().unwrap();
        ocr.element_config = None;
        ocr.tessdata_path = Some(models.path().to_path_buf());
        let result = extract(TINTED_LEADERS, &extraction);
        assert!(result.metadata.ocr_used);
        assert_eq!(result.metadata.additional["psm"], "11");
        let [table] = result.tables.as_slice() else {
            panic!("expected exactly one document table");
        };
        assert_eq!(table.cells, expected);
        let [page] = result.pages.as_deref().unwrap() else {
            panic!("expected exactly one page");
        };
        let [page_table] = page.tables.as_slice() else {
            panic!("expected exactly one page table");
        };
        assert_eq!(page_table.cells, expected);
        assert_eq!(page.content.trim(), transcript);
        match format {
            OutputFormat::Html => assert_html_table(&result, &expected),
            _ => assert_eq!(result.content.trim(), transcript),
        }
    }
}

#[test]
fn explicit_sparse_and_single_block_modes_keep_the_stock_tinted_leader_transcript() {
    const SPARSE: &str = include_str!("fixtures/ocr_source_row_layout_2432/image-11-tinted-leaders-psm11-plain.txt");
    const BLOCK: &str = include_str!("fixtures/ocr_source_row_layout_2432/image-11-tinted-leaders-psm6-plain.txt");
    let models = pinned_english_model();
    for (psm, expected_page, separator) in [(11, SPARSE, "\n\n"), (6, BLOCK, "\n")] {
        for format in [OutputFormat::Plain, OutputFormat::Html] {
            let mut extraction = config(format.clone());
            let ocr = extraction.ocr.as_mut().unwrap();
            ocr.element_config = None;
            ocr.tessdata_path = Some(models.path().to_path_buf());
            ocr.tesseract_config.get_or_insert_default().psm = Some(psm);
            let result = extract(TINTED_LEADERS, &extraction);
            assert!(result.metadata.ocr_used);
            assert_eq!(result.metadata.additional["psm"], psm.to_string());
            assert!(result.tables.is_empty());
            let [page] = result.pages.as_deref().unwrap() else {
                panic!("expected exactly one page");
            };
            assert!(page.tables.is_empty());
            assert_eq!(page.content.trim(), expected_page.trim());
            let expected = match (format, psm) {
                (OutputFormat::Html, 11) => expected_page
                    .trim()
                    .split(separator)
                    .map(|cell| format!("<p>{cell}</p>\n"))
                    .collect(),
                (OutputFormat::Html, _) => format!("<p>{}</p>\n", expected_page.trim().replace(separator, " ")),
                _ => expected_page.trim().to_owned(),
            };
            assert_eq!(result.content, expected);
        }
    }
}
