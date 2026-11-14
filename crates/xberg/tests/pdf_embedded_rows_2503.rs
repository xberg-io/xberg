#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use xberg::core::config::{ExtractInput, ExtractionConfig};

const HEADING: &str = "The table below lists the yearly service figures.";
const TABLE: [&str; 9] = [
    "Item Count Budget Change",
    "Snow removal......... 68 594,898 (8,684)",
    "Field lights ............ 95 985,652 141.97",
    "Parks and trails ........ 45 901,761 70,139",
    "Water service ......... 26 61,656 (3,409)",
    "Street repair.......... 16 181,811 126.32",
    "Library hours .......... 28 801,525 54,396",
    "Fleet fuel............. 16 863,842 (5,273)",
    "Bridge paint........... 39 756,157 176.66",
];

#[tokio::test]
#[ignore = "requires XBERG_2503_EMBEDDED_PDF and pinned English tessdata"]
async fn embedded_markdown_preserves_table_rows_and_native_heading() {
    let bytes = std::fs::read(std::env::var_os("XBERG_2503_EMBEDDED_PDF").unwrap()).unwrap();
    for psm in [None, Some(6)] {
        for (inject_placeholders, ocr_text_only) in [(false, false), (true, true), (true, false)] {
            let mut config = serde_json::json!({
                "use_cache": false,
                "output_format": "markdown",
                "ocr": {"enabled": true, "backend": "tesseract", "language": ["eng"]},
                "pages": {"extract_pages": true}
            });
            if inject_placeholders {
                config["images"] = serde_json::json!({
                    "extract_images": true,
                    "inject_placeholders": true,
                    "ocr_text_only": ocr_text_only,
                    "append_ocr_text": !ocr_text_only
                });
            }
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
            let mut table = TABLE;
            if psm == Some(6) {
                table[4] = "Water service......... 26 61,656 (3,409)";
            }
            let expect_placeholder = inject_placeholders && !ocr_text_only;
            for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
                assert_table(text, table, psm, inject_placeholders, expect_placeholder);
            }
        }
    }
}

fn assert_table(text: &str, table: [&str; 9], psm: Option<u8>, explicit_image: bool, expect_placeholder: bool) {
    let mut lines: Vec<_> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let context = format!("PSM {psm:?}, explicit image {explicit_image}, placeholder {expect_placeholder}");
    if expect_placeholder {
        assert!(
            lines
                .get(1)
                .is_some_and(|line| line.starts_with("![") && line.ends_with(')')),
            "{context}\n{text}"
        );
        lines.remove(1);
    }
    let expected: Vec<_> = std::iter::once(HEADING).chain(table).collect();
    assert_eq!(lines, expected, "{context}");
    if explicit_image {
        assert!(text.contains(&table.join("\n\n")), "{context}\n{text}");
    }
}
