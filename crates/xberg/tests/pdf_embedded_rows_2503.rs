#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use xberg::core::config::{ExtractInput, ExtractionConfig};

const HEADING: &str = "The table below lists the yearly service figures.";
const ROWS: [(&str, &str); 8] = [
    ("Snow removal", "68 594,898 (8,684)"),
    ("Field lights", "95 985,652 141.97"),
    ("Parks and trails", "45 901,761 70,139"),
    ("Water service", "26 61,656 (3,409)"),
    ("Street repair", "16 181,811 126.32"),
    ("Library hours", "28 801,525 54,396"),
    ("Fleet fuel", "16 863,842 (5,273)"),
    ("Bridge paint", "39 756,157 176.66"),
];

#[tokio::test]
#[ignore = "requires XBERG_2503_EMBEDDED_PDF and pinned English tessdata"]
async fn embedded_markdown_preserves_table_rows_and_native_heading() {
    let bytes = std::fs::read(std::env::var_os("XBERG_2503_EMBEDDED_PDF").unwrap()).unwrap();
    for psm in [None, Some(6)] {
        let mut config = serde_json::json!({
            "use_cache": false,
            "output_format": "markdown",
            "ocr": {"enabled": true, "backend": "tesseract", "language": ["eng"]},
            "pages": {"extract_pages": true}
        });
        if let Some(psm) = psm {
            config["ocr"]["tesseract_config"] = serde_json::json!({"psm": psm});
        }
        let config: ExtractionConfig = serde_json::from_value(config).unwrap();
        let mut batch = xberg::extract(
            ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
            &config,
        )
        .await
        .unwrap();
        assert!(batch.errors.is_empty());
        let result = batch.results.remove(0);
        for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
            assert_eq!(text.matches(HEADING).count(), 1);
            let rows: Vec<_> = text
                .lines()
                .filter(|line| ROWS.iter().any(|(_, values)| line.ends_with(values)))
                .collect();
            assert_eq!(rows.len(), ROWS.len(), "PSM {psm:?}\n{text}");
            for (row, (label, values)) in rows.iter().zip(ROWS) {
                let leaders = row.strip_prefix(label).and_then(|tail| tail.strip_suffix(values));
                assert!(
                    leaders.is_some_and(|marks| marks.trim().bytes().all(|byte| byte == b'.')),
                    "{row}"
                );
            }
        }
    }
}
