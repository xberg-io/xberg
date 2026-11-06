#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const SOURCE_ROWS: [[&str; 3]; 12] = [
    ["97", "623,093", "(8,538)"],
    ["66", "541,808", "709.58"],
    ["46", "948,387", "7,489"],
    ["21", "424,076", "(9,007)"],
    ["65", "847,601", "384.81"],
    ["83", "994,034", "67,217"],
    ["63", "65,008", "(224)"],
    ["18", "725,593", "34.41"],
    ["77", "164,840", "29,568"],
    ["49", "717,232", "(8,467)"],
    ["15", "290,279", "502.18"],
    ["44", "633,649", "14,758"],
];

const LABELS_WITH_LEADER_READS: [&str; 12] = [
    "Tree planting cee eae",
    "Signal timing tee eee",
    "Pool repairs ce eee",
    "Snow removal seen nea",
    "Field lights cee eae",
    "Parks and trails cena",
    "Water service seen",
    "Street repair tee ee",
    "Library hours wee",
    "Fleet fuel Cee eee",
    "Bridge paint we",
    "Storm drains cee ee eee",
];

fn literal_inventory(content: &str) -> Vec<&str> {
    let mut words = content
        .split(|character: char| character.is_whitespace() || character == '|')
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .collect::<Vec<_>>();
    words.sort_unstable();
    words
}

fn numeric_fields(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| character.is_whitespace() || character == '|')
        .filter(|token| {
            token.bytes().any(|byte| byte.is_ascii_digit())
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || b",.()+-".contains(&byte))
        })
}

fn extract(bytes: &[u8]) -> ExtractedDocument {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_path_buf()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap()
}

fn tinted_scan() -> Vec<u8> {
    match std::env::var_os("XBERG_2503_SCAN") {
        Some(path) => {
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(
                hex::encode(Sha256::digest(&bytes)),
                "646f94cfca1f44d2aae766cf7b694bf8e606445336f0fa548cff8636de6ec3a6"
            );
            bytes
        }
        None => {
            let bytes = include_bytes!("fixtures/ocr/dot_leader_scan_2503/scan-200-tinted.pdf").to_vec();
            assert_eq!(
                hex::encode(Sha256::digest(&bytes)),
                "b284ba53054ea40b23e00c79adeed3ac646645bdaeb12a0c67f9e515754c26c4"
            );
            bytes
        }
    }
}

#[test]
fn auto_ocr_keeps_all_twelve_tinted_dot_leader_scan_rows_together() {
    let result = extract(&tinted_scan());
    assert!(result.metadata.ocr_used);
    let [table] = result.tables.as_slice() else {
        panic!(
            "the dot-leader route must claim the rows as one table, found {}",
            result.tables.len()
        );
    };
    assert_eq!(table.cells.len(), 13);
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("expected one source page");
    };
    let expected = LABELS_WITH_LEADER_READS
        .iter()
        .zip(SOURCE_ROWS)
        .map(|(label, values)| format!("{label} {}", values.join(" ")))
        .collect::<Vec<_>>();
    let oracle = format!("Item Count Budget Change\n{}", expected.join("\n"));
    let inventory = literal_inventory(&oracle);
    assert_eq!(inventory.len(), 86);
    for (name, content) in [("document", &result.content), ("page", &page.content)] {
        assert_eq!(
            literal_inventory(content),
            inventory,
            "{name}: preserve every recognized literal"
        );
        assert!(
            numeric_fields(content).eq(SOURCE_ROWS.iter().flatten().copied()),
            "{name}: recognition must already contain every source value exactly once and in order"
        );
        let rows = content
            .lines()
            .map(|line| numeric_fields(line).collect::<Vec<_>>())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(rows, SOURCE_ROWS, "{name}: place each exact source triplet on one row");
    }
}

#[test]
fn regular_dashes_in_place_of_leader_dots_do_not_form_a_table() {
    let bytes = include_bytes!("fixtures/ocr/dot_leader_scan_2503/scan-200-dashes.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "8a7d75aea33be2c6adc464f012948fc84b55fe96e42c078ac4504d58e75a5ac1"
    );
    let result = extract(bytes);
    assert!(result.metadata.ocr_used);
    assert!(result.tables.is_empty(), "found {} tables", result.tables.len());
}
