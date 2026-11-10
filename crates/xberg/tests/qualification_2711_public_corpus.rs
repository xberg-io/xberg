#![allow(clippy::print_stdout)]
#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]
#[path = "ocr_pixel_phase_2711/fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
use xberg::types::ImagePreprocessingConfig;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};

const CID: [&str; 4] = [
    "The committee met on Tuesday to review the plan for the new library.",
    "Members asked about the schedule and the budget for the parking area.",
    "The architect said that building work would start after the winter.",
    "A vote on the final design is planned for the next regular meeting.",
];
const HEADER: &str = "Minutes of the municipal library planning committee and public consultation working group";
const OUTLINE: [&str; 5] = [
    "The committee met on Tuesday to review the plans for the library.",
    "Members asked about the schedule, budget, and planned parking.",
    "The architect said work would start after the winter season ends.",
    "A vote is planned for the next regular meeting of the council.",
    "Residents may send written comments before the end of the month.",
];
const LIGATURE: [&str; 3] = [
    "The financial plan shows a careful reserve for the first year.",
    "Each office reflects the effort of the staff and the final review.",
    "The influence of the flow on the efficient system is specific.",
];

fn font_cases() -> Vec<(&'static str, Vec<u8>, String)> {
    let mut cases = vec![
        (
            "narrow-gap",
            fixture::localized_pdf("gap-left"),
            fixture::LINES.join(" "),
        ),
        ("I-right", fixture::localized_pdf("I-right"), fixture::LINES.join(" ")),
        ("origin-base", fixture::pdf(7, 0), fixture::LINES.join(" ")),
        ("origin-shift", fixture::pdf(7, 1), fixture::LINES.join(" ")),
    ];
    for (name, bytes) in [
        (
            "cid-good",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/good_full.pdf").as_slice(),
        ),
        (
            "cid-nul",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/unicode_nul_mapping.pdf")
                .as_slice(),
        ),
        (
            "cid-missing",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/missing.pdf").as_slice(),
        ),
        (
            "cid-partial",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/partial.pdf").as_slice(),
        ),
    ] {
        cases.push((name, bytes.to_vec(), CID.repeat(5).join(" ")));
    }
    cases
}

fn corpus() -> Vec<(&'static str, Vec<u8>, String)> {
    let mut cases = font_cases();
    let prose = std::iter::once(HEADER)
        .chain(OUTLINE)
        .chain(std::iter::once("Page 7"))
        .collect::<Vec<_>>()
        .join(" ");
    for (name, bytes) in [
        (
            "outlined-prose",
            include_bytes!("fixtures/ocr/outlined_prose_2313.pdf").as_slice(),
        ),
        (
            "raster-stubs",
            include_bytes!("fixtures/ocr/raster_prose_stubs_2313.pdf").as_slice(),
        ),
        (
            "icc-raster-stubs",
            include_bytes!("fixtures/ocr/raster_prose_icc_stubs_2313.pdf").as_slice(),
        ),
    ] {
        cases.push((name, bytes.to_vec(), prose.clone()));
    }
    let ligature = LIGATURE
        .iter()
        .chain(LIGATURE.iter())
        .chain(std::iter::once(&LIGATURE[1]))
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    for (name, bytes) in [
        (
            "ligature-letters",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/ligature_advance_2322/letters.pdf").as_slice(),
        ),
        (
            "ligature-codepoint",
            include_bytes!("../../xberg-native-pdf/tests/fixtures/ligature_advance_2322/codepoint.pdf").as_slice(),
        ),
    ] {
        cases.push((name, bytes.to_vec(), ligature.clone()));
    }
    assert_eq!(cases.len(), 13);
    cases
}

fn pinned_corpus() -> Vec<(&'static str, Vec<u8>, String)> {
    let pins: serde_json::Value =
        serde_json::from_str(include_str!("ocr_pixel_phase_2711/qualification-pins.json")).unwrap();
    assert_eq!(pins["font_sha256"], fixture::FONT_SHA256);
    assert_eq!(
        pins["tessdata_eng_sha256"],
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    let cases = corpus();
    let expected = pins["cases"].as_array().unwrap();
    assert_eq!(expected.len(), cases.len());
    for ((name, bytes, source), pin) in cases.iter().zip(expected) {
        assert_eq!(pin["name"], *name);
        assert_eq!(pin["pdf_sha256"], hex::encode(Sha256::digest(bytes)), "{name}");
        assert_eq!(pin["expected_words"], source.split_whitespace().count(), "{name}");
        assert_eq!(
            pin["source_text_sha256"],
            hex::encode(Sha256::digest(source.as_bytes())),
            "{name}"
        );
    }
    cases
}

#[test]
fn qualification_pins_cover_the_thirteen_source_cases() {
    assert_eq!(pinned_corpus().len(), 13);
}

fn configuration(models: &std::path::Path) -> ExtractionConfig {
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                preprocessing: Some(ImagePreprocessingConfig {
                    denoise: std::env::var_os("XBERG_2711_DENOISE").is_some(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "paired OCR qualification requires XBERG_2711_QUALIFICATION and XBERG_2711_ARM"]
async fn paired_public_source_word_qualification() {
    let directory =
        std::path::PathBuf::from(std::env::var_os("XBERG_2711_QUALIFICATION").expect("external qualification dir"));
    let arm = std::env::var("XBERG_2711_ARM").expect("named baseline or treatment");
    let models = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), model).unwrap();
    let config = configuration(models.path());
    let mut rows = Vec::new();
    for (name, bytes, expected) in pinned_corpus() {
        let hash = hex::encode(Sha256::digest(&bytes));
        let output = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config)
            .await
            .unwrap();
        assert!(output.errors.is_empty(), "{name}: {:?}", output.errors);
        let [doc]: [ExtractedDocument; 1] = output.results.try_into().unwrap();
        assert!(
            doc.extraction_method.is_some_and(|method| method.used_ocr()),
            "{name} actual OCR"
        );
        assert!(!doc.content.is_empty());
        println!("{arm} {name} words={}", doc.content.split_whitespace().count());
        rows.push(serde_json::json!({"name":name,"sha256":hash,"expected":expected,"text":doc.content,"pages":doc.pages,"method":doc.extraction_method}));
        std::fs::write(
            directory.join(format!("{arm}.json")),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(rows.len(), 13);
}
