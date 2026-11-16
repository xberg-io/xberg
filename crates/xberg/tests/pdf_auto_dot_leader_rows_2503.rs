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

const LABELS: [&str; 12] = [
    "Tree planting",
    "Signal timing",
    "Pool repairs",
    "Snow removal",
    "Field lights",
    "Parks and trails",
    "Water service",
    "Street repair",
    "Library hours",
    "Fleet fuel",
    "Bridge paint",
    "Storm drains",
];

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
    extract_with_psm(bytes, None)
}

fn extract_with_psm(bytes: &[u8], psm: Option<i32>) -> ExtractedDocument {
    extract_as(bytes, psm, OutputFormat::Plain)
}

fn extract_as(bytes: &[u8], psm: Option<i32>, output_format: OutputFormat) -> ExtractedDocument {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_path_buf()),
            tesseract_config: psm.map(|psm| xberg::types::TesseractConfig {
                psm: Some(psm),
                ..Default::default()
            }),
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
    for (name, content) in [("document", &result.content), ("page", &page.content)] {
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
        for (label, values) in LABELS.iter().zip(SOURCE_ROWS) {
            let row = content
                .lines()
                .find(|line| line.contains(label))
                .unwrap_or_else(|| panic!("{name}: no row for {label}"));
            assert!(
                numeric_fields(row).eq(values),
                "{name}: row {label} must carry its own values, got {row}"
            );
        }
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

#[test]
fn explicit_psm6_scan_publishes_each_owned_leader_row_once() {
    let result = extract_with_psm(&tinted_scan(), Some(6));
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("one scan page");
    };
    for content in [&result.content, &page.content] {
        for (label, literal) in [
            ("Tree planting", "Tree planting .......... 97 623,093 (8,538)"),
            ("Parks and trails", "Parks and trails ........ 83 994,034 67,217"),
        ] {
            let rows: Vec<_> = content.lines().filter(|line| line.starts_with(label)).collect();
            assert_eq!(rows, [literal], "{content}");
        }
    }
}

#[test]
#[ignore = "requires the pinned #2432 reporter PDF"]
fn plain_200_dpi_psm6_keeps_source_leader_occurrences_in_order() {
    let bytes = std::fs::read(std::env::var_os("XBERG_2503_PLAIN_200_PDF").unwrap()).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "362b728c7dd8e4eabc61f0070f29a006fd43540d70116453cba66f7396b263db"
    );
    let result = extract_with_psm(&bytes, Some(6));
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("one scan page");
    };
    let [table] = result.tables.as_slice() else {
        panic!("one accepted table");
    };
    assert_eq!(table.cells.len(), 13);
    assert_eq!(page.tables[0].cells, table.cells);
    let expected = table.cells.iter().map(|row| row.join(" ")).collect::<Vec<_>>();
    for content in [&result.content, &page.content] {
        let rows = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(rows, expected, "each accepted source row occurs once and in order");
        assert_eq!(rows[1], "Transit passes ......... 80 344,768 (2,258)");
        assert_eq!(rows[2], "Code review ........... 46 443,988 590.07");
        assert_eq!(rows[3], "Tree planting .......... 77 774,883 95,023");
        assert_eq!(rows[8], "Parks and trails ........ 56 86,301 600.09");
        assert_eq!(rows[12], "Fleet fuel. ..........., 53 562,485 54,237");
    }
}

#[test]
#[ignore = "requires the pinned #2432 scan and the shifted-heading control"]
fn psm6_html_preserves_scan_rows_only_with_owned_columns() {
    let pinned = |variable: &str, sha: &str| {
        let bytes = std::fs::read(std::env::var_os(variable).expect(variable)).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)), sha);
        bytes
    };
    let original = pinned(
        "XBERG_2503_PLAIN_150_PDF",
        "9f427df6284fe295ea131e46211844b24b0a8693b2c36da1ce004439148abc6c",
    );
    let shifted = pinned(
        "XBERG_2503_PLAIN_150_SHIFTED_HEADER_PDF",
        "5d6a45358a6824f9003ead2d6bb0aed701e98a4e72826511848c38819bc56b60",
    );
    let plain = extract_with_psm(&original, Some(6));
    let control_plain = extract_with_psm(&shifted, Some(6));
    assert_eq!(
        plain.content, control_plain.content,
        "the geometry control has identical stock text"
    );
    let stock = plain
        .content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(stock.len(), 13);
    assert_eq!(stock[0], "Item Count Budget Change");
    assert_eq!(stock[1], "Library hours .......... 11 481,115 (6,536)");
    let paragraphs = stock
        .iter()
        .map(|line| format!("<p>{line}</p>"))
        .collect::<Vec<_>>()
        .join("\n");
    let flattened = format!("<p>{}</p>", stock.join(" "));
    for (input, expected) in [(&original, paragraphs), (&shifted, flattened)] {
        let html = extract_as(input, Some(6), OutputFormat::Html);
        assert!(
            html.tables.is_empty(),
            "the proof uses the stock lines without rebuilding a table"
        );
        let [page] = html.pages.as_deref().unwrap() else {
            panic!("one source page");
        };
        assert_eq!(html.content.trim(), expected);
        assert_eq!(page.content.trim(), expected);
    }
}
