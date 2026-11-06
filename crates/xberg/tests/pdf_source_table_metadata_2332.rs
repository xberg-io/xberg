#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use sha2::{Digest, Sha256};
use xberg::types::document_structure::NodeContent;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat, PdfConfig};

fn source_document(output_format: OutputFormat) -> ExtractedDocument {
    let bytes = include_bytes!("../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        include_document_structure: true,
        pdf_options: Some(PdfConfig {
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap()
}

#[test]
fn source_headers_and_separate_table_records_reach_the_public_result() {
    let result = source_document(OutputFormat::Markdown);
    assert_eq!(result.tables.len(), 2, "the source page contains two physical tables");
    let upper = result
        .tables
        .iter()
        .find(|table| {
            table.cells.first().is_some_and(|row| {
                row.iter()
                    .map(String::as_str)
                    .eq(["Tests", "Test Method", "Requirements"])
            })
        })
        .expect("source header row survives");
    assert_eq!(
        upper.columns.as_deref(),
        Some(["Tests".into(), "Test Method".into(), "Requirements".into()].as_slice())
    );
    assert_eq!(upper.cells.len(), 5);
    assert!(upper.cells.iter().all(|row| row.len() == 3));
    assert_eq!(upper.cells[2], ["Sand Equivalent", "ASTM D2419", "60 Minimum"]);
    assert_eq!(upper.cells[4], ["Durability", "CTM 229", "55 Minimum"]);
    let lower = result.tables.iter().find(|table| !std::ptr::eq(*table, upper)).unwrap();
    assert!(lower.cells.iter().all(|row| row.len() == 3));
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 8 (2.36 mm)", "60 - 90", "90 - 100"])
    );
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 100 (150 µm)", "5 - 20", "15 - 30"])
    );
}

#[test]
fn source_header_bands_and_merged_label_reach_the_html_and_public_grid() {
    let result = source_document(OutputFormat::Html);
    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.content.matches("<table").count(), 2);
    assert!(result.content.contains("<th rowspan=\"2\">Requirements</th>"));
    let document = result.document.as_ref().expect("requested public structure");
    let lower = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .find(|grid| {
            grid.cells
                .iter()
                .any(|cell| cell.row == 0 && cell.col == 0 && cell.content == "Requirements")
        })
        .expect("source lower table grid");
    assert_eq!((lower.rows, lower.cols), (11, 3));
    let label = lower.cells.iter().find(|cell| cell.row == 0 && cell.col == 0).unwrap();
    assert_eq!((label.row_span, label.col_span, label.is_header), (2, 1, true));
    assert_eq!(lower.cells.iter().filter(|cell| cell.is_header).count(), 5);
    assert!(
        lower
            .cells
            .iter()
            .filter(|cell| cell.is_header)
            .all(|cell| cell.row < 2)
    );
    assert!(!lower.cells.iter().any(|cell| cell.row == 1 && cell.col == 0));
    assert_eq!(lower.cells.iter().filter(|cell| cell.row >= 2).count(), 27);
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_survey_leaf_band_remains_a_header_below_group_labels() {
    let bytes = parsebench_source();
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let [_, survey] = result.tables.as_slice() else {
        panic!("expected exactly the two source tables");
    };
    assert_eq!(survey.cells.len(), 9);
    assert_eq!(
        survey.cells[1],
        ["Department", "2024", "2025", "Voluntary Turnover", "Avg Tenure (yrs)"]
    );
    assert_eq!(survey.cells[2], ["Engineering", "4.2", "4.5", "8.1%", "3.8"]);
    assert_eq!(survey.cells[8], ["Company-wide", "3.9", "4.1", "9.9%", "3.9"]);
    let survey_html = survey_html(&result.content);
    assert!(survey_html.contains("<th colspan=\"2\">Satisfaction Score</th>"));
    assert!(survey_html.contains("<th colspan=\"2\">Retention</th>"));
    assert!(survey_html.contains("<td>Engineering</td>"));
    assert!(survey_html.contains("<td>Company-wide</td>"));
    for label in &survey.cells[1] {
        assert_eq!(
            survey_html.matches(&format!(">{label}</th>")).count(),
            1,
            "source leaf label {label:?} must be a header in survey HTML:\n{survey_html}"
        );
    }
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_survey_bold_leaves_without_a_visible_separator_stay_body_cells() {
    let mut pdf = lopdf::Document::load_mem(&parsebench_source()).unwrap();
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
    let source = String::from_utf8(pdf.get_page_content(page).unwrap()).unwrap();
    let separator = ".7 .7 .7 RG\nn 0 154 m 406.8 154 l S";
    assert_eq!(source.matches(separator).count(), 1);
    let white = source.replace(separator, "1 1 1 RG\nn 0 154 m 406.8 154 l S");
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(white.into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();

    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let survey_html = survey_html(&result.content);
    assert!(survey_html.contains("<th colspan=\"2\">Satisfaction Score</th>"));
    assert!(survey_html.contains("<td>Department</td>"));
    assert!(survey_html.contains("<td>Company-wide</td>"));
}

fn parsebench_source() -> Vec<u8> {
    let path = std::env::var_os("XBERG_PARSEBENCH_2332_SOURCE").expect("provide the pinned ParseBench source PDF");
    let bytes = std::fs::read(path).expect("read the ParseBench reference PDF");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "021397de9fc37d73c1412c47627e19dc3292dfa47d4b521610f653c4366254a4"
    );
    bytes
}

fn parsebench_html_config() -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Html,
        ..Default::default()
    }
}

fn survey_html(content: &str) -> &str {
    let tables = content.split("<table").skip(1).collect::<Vec<_>>();
    assert_eq!(tables.len(), 2);
    tables[1].split_once("</table>").expect("complete survey HTML table").0
}
