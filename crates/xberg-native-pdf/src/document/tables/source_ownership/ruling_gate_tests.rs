use super::*;
use crate::structure::spatial_table_detector::header_source_diagnostic;

const HEADER_AND_FOOTER_RULES: &str = "20 380 m 280 380 l S\n20 20 m 280 20 l S\n";

/// One page of aligned two-column text between the given rules.
fn page(rules: &str) -> PdfDocument {
    let mut content = rules.to_string();
    for (row, (label, value)) in [
        ("Item", "Status"),
        ("8", "Correct"),
        ("27", "Incomplete"),
        ("29", "Missing"),
    ]
    .into_iter()
    .enumerate()
    {
        content.push_str(&format!(
            "BT /F1 9 Tf 40 {} Td ({label}) Tj 120 0 Td ({value}) Tj ET\n",
            300 - row * 20
        ));
    }
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
    ];
    let mut pdf = "%PDF-1.4\n".to_string();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }
    let xref = pdf.len();
    pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    PdfDocument::from_bytes(pdf.into_bytes()).expect("parse synthetic ruled page")
}

fn production_configs() -> [TableDetectionConfig; 2] {
    let bordered = TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: DEFAULT_MAX_TABLE_COLUMNS,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    };
    [TableDetectionConfig::strict(), bordered]
}

fn word_reads(events: &[serde_json::Value]) -> usize {
    events
        .iter()
        .filter(|event| event["stage"] == "table_word_spans")
        .count()
}

fn tables_and_ownership(results: &[RuledSourceTables; 2]) -> serde_json::Value {
    serde_json::json!(
        results
            .iter()
            .map(|(tables, ownership)| serde_json::json!({
                "tables": header_source_diagnostic::tables(tables),
                "owned": ownership.is_some(),
            }))
            .collect::<Vec<_>>()
    )
}

/// Runs the gated production entry and the word-reading detector on the same
/// paths, returning both outcomes and how often each read the page's words.
fn gated_and_detected(document: &PdfDocument) -> ((serde_json::Value, usize), (serde_json::Value, usize), usize) {
    let [strict, bordered] = production_configs();
    let configs = [&strict, &bordered];
    let ((paths, gated), gated_events) = header_source_diagnostic::capture(|| {
        document
            .extract_tables_with_wide_source_recovery_and_paths(0, configs)
            .expect("gated ruled detection")
    });
    let (detected, detected_events) = header_source_diagnostic::capture(|| {
        document
            .extract_tables_with_wide_source_recovery_from_paths(0, configs, &paths)
            .expect("ruled detection with words")
    });
    (
        (tables_and_ownership(&gated), word_reads(&gated_events)),
        (tables_and_ownership(&detected), word_reads(&detected_events)),
        paths.len(),
    )
}

#[test]
fn header_and_footer_rules_skip_the_table_words_without_changing_the_result() {
    let document = page(HEADER_AND_FOOTER_RULES);
    let lines = document.table_ruling_paths(0, &document.extract_paths(0).expect("paths"));
    assert_eq!(lines.len(), 2, "both rules must reach the ruled detectors");
    let ((gated, gated_reads), (detected, detected_reads), paths) = gated_and_detected(&document);
    assert_eq!(paths, 2, "the gate still returns the page's paths for later passes");
    assert_eq!(
        gated_reads, 0,
        "rules that cannot bound a cell must not read the page's words"
    );
    assert_eq!(
        detected_reads, 1,
        "the reference run must read the words it detects from"
    );
    assert_eq!(gated, detected);
    assert!(
        gated
            .as_array()
            .expect("two routes")
            .iter()
            .all(|route| route["tables"].as_array().is_some_and(Vec::is_empty) && route["owned"] == false)
    );
}

#[test]
fn a_vertical_rule_keeps_the_word_read() {
    let document = page(&format!("{HEADER_AND_FOOTER_RULES}100 30 m 100 370 l S\n"));
    let ((gated, gated_reads), (detected, _), _) = gated_and_detected(&document);
    assert_eq!(gated_reads, 1);
    assert_eq!(gated, detected);
}

#[test]
fn enough_horizontal_rules_for_painted_cells_keep_the_word_read() {
    let document = page(&format!(
        "{HEADER_AND_FOOTER_RULES}20 310 m 280 310 l S\n20 230 m 280 230 l S\n"
    ));
    let ((gated, gated_reads), (detected, _), _) = gated_and_detected(&document);
    assert_eq!(gated_reads, 1);
    assert_eq!(gated, detected);
}

#[test]
fn a_ruled_grid_is_detected_through_the_gate() {
    let mut rules = String::new();
    for y in [312, 292, 272, 252, 232] {
        rules.push_str(&format!("30 {y} m 280 {y} l S\n"));
    }
    for x in [30, 150, 280] {
        rules.push_str(&format!("{x} 232 m {x} 312 l S\n"));
    }
    let document = page(&rules);
    let ((gated, gated_reads), (detected, _), _) = gated_and_detected(&document);
    assert_eq!(gated_reads, 1);
    assert_eq!(gated, detected);
    assert!(
        gated
            .as_array()
            .expect("two routes")
            .iter()
            .any(|route| route["tables"].as_array().is_some_and(|tables| !tables.is_empty())),
        "the fixture must exercise real ruled detection: {gated}"
    );
}

#[test]
fn a_page_error_is_reported_before_the_gate() {
    let document = page(HEADER_AND_FOOTER_RULES);
    let [strict, bordered] = production_configs();
    let missing = document.page_count().expect("page count");
    let Err(error) = document.extract_tables_with_wide_source_recovery_and_paths(missing, [&strict, &bordered]) else {
        panic!("the page after the last page must fail");
    };
    let Err(word_error) = document.extract_table_word_spans(missing) else {
        panic!("the page after the last page has no words");
    };
    assert_eq!(error.to_string(), word_error.to_string());
}
