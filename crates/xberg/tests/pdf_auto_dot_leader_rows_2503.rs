#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

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

fn numeric_fields(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| character.is_whitespace() || character == '|')
        .filter(|token| {
            token.bytes().any(|byte| byte.is_ascii_digit())
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || b",.()+-".contains(&byte))
        })
}

#[test]
fn auto_ocr_keeps_all_twelve_tinted_dot_leader_scan_rows_together() {
    let (bytes, expected_sha) = match std::env::var_os("XBERG_2503_SCAN") {
        Some(path) => (
            std::fs::read(path).unwrap(),
            "646f94cfca1f44d2aae766cf7b694bf8e606445336f0fa548cff8636de6ec3a6",
        ),
        None => (
            include_bytes!("fixtures/ocr/dot_leader_scan_2503/scan-200-tinted.pdf").to_vec(),
            "b284ba53054ea40b23e00c79adeed3ac646645bdaeb12a0c67f9e515754c26c4",
        ),
    };
    assert_eq!(hex::encode(Sha256::digest(&bytes)), expected_sha);
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
    let result = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert!(result.metadata.ocr_used);
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
    }
}
