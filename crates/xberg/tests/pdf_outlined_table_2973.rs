#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use sha2::{Digest, Sha256};
use xberg::types::ExtractionMethod;
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OcrStrategy, OutputFormat, TesseractConfig};

const TABLE: &[u8] = include_bytes!("fixtures/pdf_outlined_table_2973/table.pdf");
const SHORT: &[u8] = include_bytes!("fixtures/pdf_outlined_table_2973/short-heading.pdf");
const GEOMETRY: &[u8] = include_bytes!("fixtures/pdf_outlined_table_2973/geometry.pdf");
const HEADLESS: &[u8] = include_bytes!("fixtures/pdf_outlined_table_2973/headless.pdf");
const PROSE: &[u8] = include_bytes!("fixtures/pdf_outlined_table_2973/prose.pdf");
const PROSE_BODY: &str = "The committee met on Tuesday to review the plan for the new library building.
Members asked about the schedule, the budget and the plan for the parking area.
The architect said that building work would start after the winter season ends.
A vote on the final design is planned for the next regular meeting of the council.
Residents may send written comments to the office before the end of the month.";
const HEADING: &str = "Water Department Fund Statement\nAnnual budget summary, page 1";
const ROWS: [&str; 7] = [
    "Revenues 2025 2026 2027",
    "Water sales 296,345 200,000 245,000",
    "Connection fees 34,051 20,000 20,000",
    "Total revenues 330,396 220,000 265,000",
    "Personnel 97,311 102,515 109,251",
    "Supplies 28,659 35,000 31,000",
    "Total expenditures 168,030 237,623 264,485",
];

fn tokens(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == ','))
        .map(|token| token.trim_matches(','))
        .filter(|token| !token.is_empty())
        .collect()
}

async fn read(pdf: &[u8], output_format: OutputFormat, force_ocr: bool, disable_ocr: bool) -> xberg::ExtractedDocument {
    let tessdata = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    let output = xberg::extract(
        ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None),
        &ExtractionConfig {
            output_format,
            force_ocr,
            disable_ocr,
            ocr_strategy: OcrStrategy::Auto,
            use_cache: false,
            ocr: Some(OcrConfig {
                backend: "tesseract".into(),
                language: vec!["eng".into()],
                tessdata_path: Some(tessdata.path().to_owned()),
                tesseract_config: Some(TesseractConfig {
                    use_cache: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .expect("extract synthetic outlined table");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    document
}

#[tokio::test]
async fn auto_recovers_outlined_table_behind_native_heading() {
    let expected = format!("{HEADING}\n{}", ROWS.join("\n"));
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let document = read(TABLE, format.clone(), false, false).await;
        assert_eq!(tokens(&document.content), tokens(&expected), "{format:?}");
        assert_eq!(document.extraction_method, Some(ExtractionMethod::Mixed), "{format:?}");
    }
}

#[tokio::test]
async fn outlined_table_controls_preserve_order_and_multiplicity() {
    let body = ROWS.join("\n");
    let expected = format!("{HEADING}\n{body}");
    let short = format!("Header\n{body}\nPage 7");
    let prose = format!("{HEADING}\n{PROSE_BODY}");
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        for (label, pdf, force, disable, method, transcript) in [
            ("forced", TABLE, true, false, ExtractionMethod::Ocr, expected.as_str()),
            (
                "short heading",
                SHORT,
                false,
                false,
                ExtractionMethod::Mixed,
                short.as_str(),
            ),
            ("OCR off", TABLE, false, true, ExtractionMethod::Native, HEADING),
            ("headless", HEADLESS, false, false, ExtractionMethod::Ocr, body.as_str()),
            (
                "outlined prose",
                PROSE,
                false,
                false,
                ExtractionMethod::Mixed,
                prose.as_str(),
            ),
            (
                "filled geometry",
                GEOMETRY,
                false,
                false,
                ExtractionMethod::Native,
                HEADING,
            ),
        ] {
            let document = read(pdf, format.clone(), force, disable).await;
            assert_eq!(tokens(&document.content), tokens(transcript), "{format:?} {label}");
            assert_eq!(document.extraction_method, Some(method), "{format:?} {label}");
        }
    }
}
