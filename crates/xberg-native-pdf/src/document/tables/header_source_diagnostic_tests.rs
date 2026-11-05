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
        .filter(|table| table["source_span_ids"][0] == json!([[174, 185], [], [186, 187, 188]]))
        .collect();
    assert_eq!(fragment.len(), 1);
    assert_eq!(fragment[0]["rows"].as_array().unwrap().len(), 6);
    assert_eq!(fragment[0]["source_span_ids"][1], json!([[175], [], [189, 190]]));
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
