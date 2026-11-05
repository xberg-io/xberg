use super::*;
use crate::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy, header_source_diagnostic};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

#[test]
fn table_configurations_share_one_page_input_and_keep_independent_results() {
    let bytes =
        include_bytes!("../../../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    let document = PdfDocument::from_bytes(bytes.to_vec()).expect("parse real ruled-table fixture");
    let strict = TableDetectionConfig::strict();
    let disabled = TableDetectionConfig {
        enabled: false,
        ..strict.clone()
    };
    let relaxed = TableDetectionConfig::relaxed();
    let configs = [&strict, &relaxed, &disabled];
    let separate = configs.map(|config| {
        document
            .extract_tables_with_config(0, config.clone())
            .expect("detect independently")
    });
    assert!(!separate[0].is_empty(), "fixture must exercise actual table extraction");
    assert!(separate[2].is_empty());

    let (combined, events) = header_source_diagnostic::capture(|| {
        document
            .extract_tables_with_configs(0, configs)
            .expect("detect all configurations from one input")
    });
    assert_eq!(
        combined
            .each_ref()
            .map(|tables| header_source_diagnostic::tables(tables)),
        separate
            .each_ref()
            .map(|tables| header_source_diagnostic::tables(tables)),
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event["stage"] == "table_word_spans")
            .count(),
        1
    );
    assert_eq!(
        events.iter().filter(|event| event["stage"] == "filtered_paths").count(),
        1
    );
}

#[test]
#[ignore = "opt-in native source capture; requires an empty /tmp output directory"]
fn capture_wildomar_native_header_sources() {
    let output = std::env::var_os("XBERG_WILDOMAR_DIAGNOSTIC_DIR").expect("set XBERG_WILDOMAR_DIAGNOSTIC_DIR");
    let output = Path::new(&output)
        .canonicalize()
        .expect("create output directory before capture");
    let temporary = Path::new("/tmp").canonicalize().expect("/tmp exists");
    assert!(
        output.starts_with(&temporary) && output != temporary,
        "output must be inside /tmp"
    );
    assert!(
        output.read_dir().expect("read output directory").next().is_none(),
        "output must be empty"
    );
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    let bytes = std::fs::read(&fixture).expect("tracked Wildomar fixture");
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(sha, "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe");
    let document = PdfDocument::from_bytes(bytes).expect("parse real fixture");
    assert_eq!(document.page_count().expect("real page count"), 1);
    let bordered = TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: 15,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    };
    let mut runs = Vec::new();
    for (name, config) in [("strict", TableDetectionConfig::strict()), ("bordered", bordered)] {
        let (tables, events) = header_source_diagnostic::capture(|| {
            document
                .extract_tables_with_config(0, config)
                .expect("actual native table producer")
        });
        assert!(!tables.is_empty(), "{name}: actual tables required");
        for required in [
            "table_word_spans",
            "raw_path",
            "filtered_paths",
            "intersection_grid",
            "intersection_assigned",
            "intersection_finalized",
            "intersection_before_merge",
            "intersection_merged",
            "intersection_divider_edges",
            "intersection_section_split",
            "intersection_final",
        ] {
            assert!(
                events.iter().any(|event| event["stage"] == required),
                "{name}: missing actual {required}"
            );
        }
        runs.push(json!({"name": name, "events": events, "public_tables": header_source_diagnostic::tables(&tables)}));
    }
    let receipt = json!({"fixture": fixture, "fixture_sha256": sha, "page_count": 1, "runs": runs,
        "identity_limit": "source indices are recorded at assignment/split; later cloned span payloads do not retain source IDs"});
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("receipt.json"))
        .expect("create new diagnostic receipt");
    serde_json::to_writer_pretty(&mut file, &receipt).expect("serialize actual source receipt");
    file.write_all(b"\n").expect("finish receipt");
}

fn event_payload<'a>(events: &'a [serde_json::Value], stage: &str) -> &'a serde_json::Value {
    let matching: Vec<_> = events.iter().filter(|event| event["stage"] == stage).collect();
    assert_eq!(matching.len(), 1, "one actual {stage} event");
    &matching[0]["payload"]
}

fn cell_inventory(tables: &serde_json::Value) -> Vec<String> {
    let mut cells: Vec<_> = tables
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|table| table["rows"].as_array().unwrap())
        .flat_map(|row| {
            row["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(move |cell| serde_json::to_string(&json!([row["is_header"], cell])).unwrap())
        })
        .collect();
    cells.sort();
    cells
}

fn assert_wildomar_source_words(events: &[serde_json::Value]) {
    let words = event_payload(events, "table_word_spans").as_array().unwrap();
    assert_eq!(words.len(), 303);
    for (id, text) in [
        (112, "Tests"),
        (113, "Test"),
        (114, " Method"),
        (115, "Requirements"),
        (174, "Un-Extracted"),
        (175, "Aggregate"),
        (185, "RAP"),
        (186, "Extracted"),
        (187, " RAP"),
        (188, " Aggregate"),
        (189, "(ASTM"),
        (190, " D1856)"),
    ] {
        assert_eq!(words[id]["id"], id);
        assert_eq!(words[id]["span"]["text"], text);
        assert_eq!(words[id]["span"]["font_weight"], "Bold");
        assert_eq!(words[id]["span"]["font_name"], "QGLBFH+Arial,Bold");
    }
    for word in &words[116..144] {
        assert_eq!(word["span"]["font_weight"], "Normal");
        assert_eq!(word["span"]["font_name"], "MJCNMM+ArialMT");
    }
}

fn assert_first_table_assignment(assigned: &[&serde_json::Value]) {
    let first: Vec<_> = assigned
        .iter()
        .filter(|table| table["source_span_ids"][0] == json!([[112], [113, 114], [115]]))
        .collect();
    assert_eq!(first.len(), 1);
    let rows: Vec<Vec<_>> = first[0]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|cell| cell["text"].as_str().unwrap())
                .collect()
        })
        .collect();
    assert_eq!(
        rows,
        [
            vec!["Tests", "Test Method", "Requirements"],
            vec!["Percentage Wear, 500 Revolutions 1", "ASTM C131", "35% Maximum"],
            vec!["Sand Equivalent", "ASTM D2419", "60 Minimum"],
            vec!["Soundness (5 Cycles) 1", "ASTM C88", "15% Maximum"],
            vec!["Durability", "CTM 229", "55 Minimum"],
        ]
    );
}

fn assert_wildomar_source_assignment(events: &[serde_json::Value]) {
    let assigned: Vec<_> = events
        .iter()
        .filter(|event| event["stage"] == "intersection_assigned")
        .map(|event| &event["payload"])
        .collect();
    assert_first_table_assignment(&assigned);
    let fragment: Vec<_> = assigned
        .iter()
        .filter(|table| table["source_span_ids"][0] == json!([[], [174, 175, 185], [186, 187, 188, 189, 190]]))
        .collect();
    assert_eq!(fragment.len(), 1);
    assert_eq!(fragment[0]["rows"].as_array().unwrap().len(), 11);
    let ids: Vec<_> = assigned
        .iter()
        .flat_map(|table| table["source_span_ids"].as_array().unwrap())
        .flat_map(|row| row.as_array().unwrap())
        .flat_map(|cell| cell.as_array().unwrap())
        .map(|id| id.as_u64().unwrap())
        .collect();
    for caption in [110, 111, 171, 172] {
        assert!(
            !ids.contains(&caption),
            "bold caption remains outside actual assignment"
        );
    }
}

fn wildomar_section_conservation(config: TableDetectionConfig) {
    let bytes =
        include_bytes!("../../../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    let sha = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(sha, "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe");
    let document = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(document.page_count().unwrap(), 1);
    let (tables, events) =
        header_source_diagnostic::capture(|| document.extract_tables_with_config(0, config).unwrap());
    assert_wildomar_source_words(&events);
    assert_wildomar_source_assignment(&events);
    let before = cell_inventory(event_payload(&events, "intersection_merged"));
    assert!(!before.is_empty());
    let after = cell_inventory(event_payload(&events, "intersection_section_split"));
    // Only global payload conservation is compared after cloning; source IDs are never reassociated. ~keep
    assert_eq!(
        after, before,
        "section handling must retain every admitted cell and its metadata"
    );
    assert_eq!(cell_inventory(&header_source_diagnostic::tables(&tables)), before);
}

#[test]
fn wildomar_strict_section_handling_conserves_admitted_labels_and_body() {
    wildomar_section_conservation(TableDetectionConfig::strict());
}

#[test]
fn wildomar_bordered_section_handling_conserves_admitted_labels_and_body() {
    wildomar_section_conservation(TableDetectionConfig {
        enabled: true,
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        column_tolerance: 3.0,
        row_tolerance: 2.0,
        min_table_cells: 4,
        min_table_columns: 2,
        regular_row_ratio: 0.5,
        max_table_columns: 15,
        column_merge_threshold: 12.0,
        v_split_gap: 4.0,
        text_fallback: false,
    });
}

#[test]
fn wildomar_complete_bold_header_keeps_source_roles_without_changing_cells() {
    let bytes =
        include_bytes!("../../../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    assert_eq!(
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe"
    );
    let document = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let (tables, events) = header_source_diagnostic::capture(|| {
        document
            .extract_tables_with_config(0, TableDetectionConfig::strict())
            .unwrap()
    });
    assert_wildomar_source_words(&events);
    assert_wildomar_source_assignment(&events);
    let matches: Vec<_> = tables
        .iter()
        .filter(|table| {
            table.rows.first().is_some_and(|row| {
                row.cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>()
                    == ["Tests", "Test Method", "Requirements"]
            })
        })
        .collect();
    assert_eq!(matches.len(), 1, "one independently identified source table");
    let table = matches[0];
    assert_eq!((table.rows.len(), table.col_count), (5, 3));
    assert!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.rowspan == 1 && cell.colspan == 1)
    );
    assert!(table.has_header, "the bold shaded source header must retain its role");
    assert!(table.rows[0].is_header);
    assert!(table.rows[0].cells.iter().all(|cell| cell.is_header));
    assert!(
        table.rows[1..]
            .iter()
            .all(|row| !row.is_header && row.cells.iter().all(|cell| !cell.is_header))
    );
}

#[test]
fn wildomar_multiline_header_backgrounds_do_not_create_phantom_columns() {
    let bytes =
        include_bytes!("../../../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    let document = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let (tables, events) = header_source_diagnostic::capture(|| {
        document
            .extract_tables_with_config(0, TableDetectionConfig::strict())
            .unwrap()
    });
    assert_wildomar_source_words(&events);
    let matches: Vec<_> = tables
        .iter()
        .filter(|table| {
            table
                .rows
                .iter()
                .any(|row| row.cells.iter().any(|cell| cell.text.contains("No. 200")))
        })
        .collect();
    assert_eq!(matches.len(), 1, "one source-backed body table");
    assert_eq!(
        matches[0].col_count, 3,
        "the source draws three columns, not text-padding columns"
    );
    assert_lower_table_source_inventory(matches[0], &events);
    assert_lower_table_source_records(matches[0]);
}

fn assert_lower_table_source_inventory(table: &crate::structure::Table, events: &[serde_json::Value]) {
    let source = event_payload(events, "table_word_spans").as_array().unwrap();
    assert_eq!(source[173]["span"]["text"], "Requirements");
    assert_eq!(source[277]["span"]["text"], " %");
    let mut expected: Vec<_> = source[173..278]
        .iter()
        .map(|word| serde_json::to_string(&word["span"]).unwrap())
        .collect();
    let mut actual: Vec<_> = table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .flat_map(|cell| &cell.spans)
        .map(|span| serde_json::to_value(span).unwrap())
        .map(|span| serde_json::to_string(&span).unwrap())
        .collect();
    expected.sort();
    actual.sort();
    assert_eq!(
        expected.len(),
        105,
        "all words in the independently identified source table"
    );
    assert_eq!(
        actual, expected,
        "every original word, geometry and style occurs exactly once"
    );
}

fn assert_lower_table_source_records(table: &crate::structure::Table) {
    let records: Vec<Vec<_>> = table
        .rows
        .iter()
        .filter(|row| {
            row.cells.first().is_some_and(|cell| {
                cell.text.starts_with("No.") || cell.text.starts_with("3/8") || cell.text.starts_with("Residual")
            })
        })
        .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect())
        .collect();
    assert_eq!(
        records,
        [
            vec!["3/8” (9.5 mm)", "100", ""],
            vec!["No. 4 (4.74 mm)", "90 - 100", "100"],
            vec!["No. 8 (2.36 mm)", "60 - 90", "90 - 100"],
            vec!["No. 16 (1.18 mm)", "35 - 60", "65 - 90"],
            vec!["No. 30 (600 µm)", "23 - 45", "40 - 60"],
            vec!["No. 50 (300 µm)", "12 - 30", "25 - 42"],
            vec!["No. 100 (150 µm)", "5 - 20", "15 - 30"],
            vec!["No. 200 (75 µm)", "0.5 -10", "10 – 20"],
            vec!["Residual Asphalt Content\n(ASTM D6307), %", "", "6.0 – 8.5"],
        ]
    );
}
