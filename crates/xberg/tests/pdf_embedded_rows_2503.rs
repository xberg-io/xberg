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

#[tokio::test]
#[ignore = "requires XBERG_2503_EMBEDDED_96_PDF and pinned English tessdata"]
async fn embedded_markdown_keeps_recognized_96_dpi_table_rows() {
    let bytes = std::fs::read(std::env::var_os("XBERG_2503_EMBEDDED_96_PDF").unwrap()).unwrap();
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
        let batch = xberg::extract(
            ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
            &config,
        )
        .await
        .unwrap();
        assert!(batch.errors.is_empty());
        let [result] = batch.results.as_slice() else {
            panic!("expected one extraction");
        };
        for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
            assert_eq!(text.matches(HEADING).count(), 1);
            assert_eq!(
                text.lines()
                    .filter(|line| line.contains("Library hours") && line.contains("25."))
                    .count(),
                1,
                "PSM {psm:?}: keep the literal OCR value"
            );
            assert!(
                text.lines().any(|line| line.contains("Signal timing")
                    && line.contains("432,439")
                    && !line.contains("Pool repairs")),
                "PSM {psm:?}: adjacent table rows must remain separate"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires XBERG_2503_EMBEDDED_PDF and pinned English tessdata"]
async fn embedded_html_preserves_stock_rows_and_native_heading() {
    let bytes = std::fs::read(std::env::var_os("XBERG_2503_EMBEDDED_PDF").unwrap()).unwrap();
    for psm in [None, Some(6)] {
        let mut config = serde_json::json!({
            "use_cache": false,
            "output_format": "html",
            "ocr": {"backend": "tesseract", "language": ["eng"]},
            "pages": {"extract_pages": true}
        });
        if let Some(psm) = psm {
            config["ocr"]["tesseract_config"] = serde_json::json!({"psm": psm});
        }
        let config: ExtractionConfig = serde_json::from_value(config).unwrap();
        let batch = xberg::extract(
            ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
            &config,
        )
        .await
        .unwrap();
        assert!(batch.errors.is_empty(), "{:?}", batch.errors);
        let [result] = batch.results.as_slice() else {
            panic!("expected one extraction")
        };
        let [page] = result.pages.as_deref().unwrap() else {
            panic!("expected one page")
        };
        let mut rows = TABLE;
        if psm == Some(6) {
            rows[4] = "Water service......... 26 61,656 (3,409)";
        }
        let expected: String = std::iter::once(HEADING)
            .chain(rows)
            .map(|row| format!("<p>{row}</p>\n"))
            .collect();
        assert_eq!(result.content.trim(), expected.trim(), "document, PSM {psm:?}");
        assert_eq!(page.content.trim(), expected.trim(), "page, PSM {psm:?}");
    }
}

const PLAIN_96_STOCK: &str = "Item Count Budget Change

Signal timing .......... 91 432,439 (1,764)
Pool repairs ........... 69 384,044 614.44
Snow removal......... 44 728,337 77,394
Field lights ............ 61 229,955 (1,482)
Parks and trails........ 65 540,252 399.47
Water service......... 78 932,702 63,329
Street repair.......... 23 257,117 (2,199)

Library hours.......... 25. 791,403 702.23";

const TINTED_96_STOCK: &str = "Item, Count Budget Change
Pool repairs ........+.. 36 266,394 (524)
Snow removal .......-. 82 6,661 702.45
Field lights ..........2. 32 982,834 10,661
Parks and trails ........ 56 9,110 (7,044)
Water service ......... 22 889,401 604.38
Street repair... 18 472,441 75,099
Library hours .........+ 10 742,725 (845)
Fleet fuel... eee eeee 89 493,633 830.40";

#[tokio::test]
#[ignore = "requires pinned 96 DPI reporter PDFs and English tessdata"]
async fn embedded_html_keeps_owned_96_dpi_stock_rows() {
    use sha2::{Digest, Sha256};

    let fixtures = std::path::PathBuf::from(std::env::var_os("XBERG_2503_REPORTER_FIXTURES").unwrap());
    const PLAIN_SHA: &str = "3ee1109e03ce48d718a2f33b05095f50fb121f3c43be9806889f31148c0f698d";
    const TINTED_SHA: &str = "51d6cb0d6c62bd3b7a23dd3863ebc65e216dcb72569652c39df66863b3049ded";
    let cases = [
        ("pic_96_plain_lead.pdf", PLAIN_SHA, None, PLAIN_96_STOCK),
        ("pic_96_plain_lead.pdf", PLAIN_SHA, Some(6), PLAIN_96_STOCK),
        ("pic_96_tint_lead.pdf", TINTED_SHA, Some(6), TINTED_96_STOCK),
    ];
    for (fixture, digest, psm, stock) in cases {
        let input = std::fs::read(fixtures.join(fixture)).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&input)), digest);
        let mut config = serde_json::json!({
            "use_cache": false,
            "output_format": "html",
            "ocr": {"backend": "tesseract", "language": ["eng"], "backend_options": {"use_cache": false}},
            "pages": {"extract_pages": true}
        });
        if let Some(psm) = psm {
            config["ocr"]["tesseract_config"] = serde_json::json!({"psm": psm});
        }
        let config: ExtractionConfig = serde_json::from_value(config).unwrap();
        let batch = xberg::extract(ExtractInput::from_bytes(input, "application/pdf", None), &config)
            .await
            .unwrap();
        assert!(batch.errors.is_empty(), "{:?}", batch.errors);
        let [result] = batch.results.as_slice() else {
            panic!("one result")
        };
        let [page] = result.pages.as_deref().unwrap() else {
            panic!("one page");
        };
        let expected: String = std::iter::once(HEADING)
            .chain(stock.lines().filter(|row| !row.is_empty()))
            .map(|row| format!("<p>{row}</p>\n"))
            .collect();
        for text in [&result.content, &page.content] {
            assert_eq!(text.trim(), expected.trim(), "{fixture}, PSM {psm:?}");
        }
    }
}
