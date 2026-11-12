//! GH#3027: layer-token recovery must not turn page 4 of `a_course_in_machine_learning_ciml_v0_9_all.pdf`
//! into its text layer's question-mark lines. The page's layer is decodable only in its title and its
//! page numbers; the OCR read has every table-of-contents entry and none of the title. Recovery used to
//! add `OF CONTENTS` from the layer, which made the read "confirm" that layer and put its lines first.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;
use helpers::get_test_documents_dir;
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, PageConfig};

const DOCUMENT: &str = "pdf/a_course_in_machine_learning_ciml_v0_9_all.pdf";
const PAGE: usize = 4;

/// Entries the OCR read gets right on every run; the artefacts of the read (`~The`) are not pinned.
const STABLE_ENTRIES: [&str; 4] = [
    "1 Decision Trees 8",
    "2 Geometry and Nearest Neighbors 26",
    "6 Linear Models 86",
    "10 Learning Theory 141",
];

#[tokio::test]
async fn the_table_of_contents_page_keeps_its_clean_read_without_the_layers_question_marks() {
    let path = get_test_documents_dir().join(DOCUMENT);
    if !path.exists() {
        assert!(
            std::env::var_os("CI").is_none(),
            "{} is missing from the corpus",
            path.display()
        );
        eprintln!("skipping: {} is not fetched", path.display());
        return;
    }
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(
        tessdata.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("bundled English model"),
    )
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_path_buf()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = xberg::extract(
        ExtractInput::from_bytes(std::fs::read(&path).unwrap(), "application/pdf", None),
        &config,
    )
    .await
    .expect("extract the document");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let pages = output.results[0].pages.as_ref().expect("per-page output requested");
    let text = &pages[PAGE - 1].content;
    assert!(!text.contains('?'), "the layer's unmapped glyphs came back:\n{text}");
    assert!(
        !text.contains("OF CONTENTS"),
        "a title word was added from the layer:\n{text}"
    );
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    for entry in STABLE_ENTRIES {
        assert!(lines.contains(&entry), "`{entry}` is missing:\n{text}");
    }
}
