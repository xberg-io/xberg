use super::*;
use crate::structure::spatial_table_detector::{TableDetectionConfig, TableStrategy, header_source_diagnostic};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

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
