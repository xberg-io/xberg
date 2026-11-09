#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use lopdf::{Document, Object, Stream, dictionary};
use sha2::{Digest, Sha256};
use xberg::core::config::OutputFormat;
use xberg::types::ExtractionMethod;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};

const SOURCE_SHA256: &str = "71c2a76af332a9b796899a78e99ce2d835026d624048cf239069276fcacce79b";
const MODEL_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
const MAPPED_EXPRESSION: &str = "Vmax = 1000 V";
// Poppler on the pinned public NIST page reads U+00BA, not U+00B0, six times. ~keep
const CELSIUS_UNIT: &str = "ºC";
const CONTROL_LINES: [&str; 3] = [
    "Independent native control remains readable beside the checklist.",
    "Every control word comes from this authored source sentence.",
    "The neighbouring page must not replace or duplicate these words.",
];

#[derive(Clone, Copy, Debug)]
enum Mode {
    Native,
    Automatic,
    Forced,
}

fn pinned_source() -> Vec<u8> {
    let path = std::env::var_os("XBERG_NIST_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_file_path("pdf/regression_pages/nist-sp811-2008-p7.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required {}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SOURCE_SHA256);
    bytes
}

fn mixed_pdf(nist_first: bool) -> Vec<u8> {
    let mut pdf = Document::load_mem(&pinned_source()).unwrap();
    assert_eq!(pdf.get_pages().len(), 1);
    let source_root = pdf.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
    let root = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"});
    let text = format!(
        "BT /F1 12 Tf 60 700 Td ({}) Tj 0 -24 Td ({}) Tj 0 -24 Td ({}) Tj ET",
        CONTROL_LINES[0], CONTROL_LINES[1], CONTROL_LINES[2]
    );
    let content = pdf.add_object(Stream::new(dictionary! {}, text.into_bytes()));
    let control = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => root,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}}, "Contents" => content,
    });
    // Keep the original page tree intact so inherited resources and boxes do not change. ~keep
    pdf.get_dictionary_mut(source_root).unwrap().set("Parent", root);
    let children = if nist_first {
        [source_root, control]
    } else {
        [control, source_root]
    };
    pdf.objects.insert(
        root,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => 2,
            "Kids" => children.into_iter().map(Object::Reference).collect::<Vec<_>>(),
        }),
    );
    pdf.catalog_mut().unwrap().set("Pages", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn assert_source_glyphs(bytes: &[u8], nist_index: usize) {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    let source = xberg_native_pdf::PdfDocument::from_bytes(pinned_source()).unwrap();
    assert_eq!(pdf.page_count().unwrap(), 2);
    let span_texts = |document: &xberg_native_pdf::PdfDocument, page| {
        document
            .extract_page_text(page)
            .unwrap()
            .spans
            .into_iter()
            .map(|span| span.text)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        span_texts(&pdf, nist_index),
        span_texts(&source, 0),
        "unchanged NIST glyph-derived text"
    );
    for index in 0..2 {
        let markers: usize = pdf
            .extract_page_text(index)
            .unwrap()
            .spans
            .iter()
            .map(|span| span.text.matches(char::REPLACEMENT_CHARACTER).count())
            .sum();
        assert_eq!(
            markers,
            if index == nist_index { 8 } else { 0 },
            "source page {}",
            index + 1
        );
    }
}

fn pinned_config(models: &std::path::Path) -> ExtractionConfig {
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(hex::encode(Sha256::digest(english)), MODEL_SHA256);
    std::fs::write(models.join("eng.traineddata"), english).unwrap();
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(models.to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(
    bytes: &[u8],
    base: &ExtractionConfig,
    format: OutputFormat,
    mode: Mode,
) -> xberg::Result<ExtractedDocument> {
    let config = ExtractionConfig {
        output_format: format,
        disable_ocr: matches!(mode, Mode::Native),
        force_ocr: matches!(mode, Mode::Forced),
        ..base.clone()
    };
    helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config)
}

fn assert_public_pages(result: &ExtractedDocument, format: &OutputFormat, mode: Mode) {
    let pages = result.pages.as_ref().expect("requested public pages");
    eprintln!(
        "{mode:?} {format:?}: method={:?}, page expression counts={:?}, page units counts={:?}",
        result.extraction_method,
        pages
            .iter()
            .map(|page| page.content.matches(MAPPED_EXPRESSION).count())
            .collect::<Vec<_>>(),
        pages
            .iter()
            .map(|page| page.content.matches(CELSIUS_UNIT).count())
            .collect::<Vec<_>>()
    );
    assert_eq!(pages.iter().map(|page| page.page_number).collect::<Vec<_>>(), [1, 2]);
    let paint_failures = result
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.contains("glyph ink is missing"))
        .count();
    assert_eq!(paint_failures, 0, "{mode:?}: this is the no-paint-loss scope case");
}

fn record_equal<T: PartialEq + std::fmt::Debug>(errors: &mut Vec<String>, label: &str, actual: T, expected: T) {
    if actual != expected {
        errors.push(format!("{label}: expected {expected:?}, got {actual:?}"));
    }
}

fn assert_native_oracles(native: &ExtractedDocument, nist_index: usize) {
    assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
    for content in [&native.content, &native.pages.as_ref().unwrap()[nist_index].content] {
        assert_eq!(
            content.matches(MAPPED_EXPRESSION).count(),
            1,
            "independently mapped source expression"
        );
        assert_eq!(
            content.matches(CELSIUS_UNIT).count(),
            6,
            "independent Poppler units oracle"
        );
    }
    let sibling = &native.pages.as_ref().unwrap()[1 - nist_index].content;
    for line in CONTROL_LINES {
        assert_eq!(sibling.matches(line).count(), 1, "authored control source page");
        assert_eq!(
            native.content.matches(line).count(),
            1,
            "authored control source document"
        );
    }
}

fn compare_automatic(
    errors: &mut Vec<String>,
    native: &ExtractedDocument,
    automatic: &ExtractedDocument,
    nist_index: usize,
) {
    for (source, selected) in native
        .pages
        .as_ref()
        .unwrap()
        .iter()
        .zip(automatic.pages.as_ref().unwrap())
    {
        record_equal(
            errors,
            &format!("page {} exact native text", source.page_number),
            selected.content.as_str(),
            source.content.as_str(),
        );
    }
    record_equal(
        errors,
        "document exact native text",
        automatic.content.as_str(),
        native.content.as_str(),
    );
    record_equal(
        errors,
        "automatic extraction method",
        automatic.extraction_method,
        Some(ExtractionMethod::Native),
    );
    for content in [
        &automatic.content,
        &automatic.pages.as_ref().unwrap()[nist_index].content,
    ] {
        compare_projection(errors, content);
    }
}

fn compare_projection(errors: &mut Vec<String>, content: &str) {
    for (label, actual, expected) in [
        (
            "automatic mapped expression",
            content.matches(MAPPED_EXPRESSION).count(),
            1,
        ),
        ("automatic exact units", content.matches(CELSIUS_UNIT).count(), 6),
        (
            "automatic replacement markers",
            content.matches(char::REPLACEMENT_CHARACTER).count(),
            0,
        ),
    ] {
        record_equal(errors, label, actual, expected);
    }
}

type ExtractionRow = (OutputFormat, [xberg::Result<ExtractedDocument>; 3]);

fn extract_matrix(bytes: &[u8], config: &ExtractionConfig) -> Vec<ExtractionRow> {
    let mut extractions = 0;
    let matrix: Vec<_> = [OutputFormat::Plain, OutputFormat::Markdown]
        .into_iter()
        .map(|format| {
            let results =
                [Mode::Native, Mode::Automatic, Mode::Forced].map(|mode| extract(bytes, config, format.clone(), mode));
            extractions += results.len();
            (format, results)
        })
        .collect();
    assert_eq!(extractions, 6);
    eprintln!("Completed {extractions} public extractions");
    matrix
}

fn assert_mixed_retention(nist_first: bool) {
    let bytes = mixed_pdf(nist_first);
    let nist_index = usize::from(!nist_first);
    assert_source_glyphs(&bytes, nist_index);
    let models = tempfile::tempdir().unwrap();
    let config = pinned_config(models.path());
    let matrix = extract_matrix(&bytes, &config);
    let mut errors = Vec::new();
    for (format, results) in matrix {
        let [native, automatic, forced] = results.map(|result| result.expect("public PDF extraction"));
        for (document, mode) in [
            (&native, Mode::Native),
            (&automatic, Mode::Automatic),
            (&forced, Mode::Forced),
        ] {
            assert_public_pages(document, &format, mode);
        }
        assert_native_oracles(&native, nist_index);
        assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
        assert_eq!(forced.metadata.ocr_used, true);
        for page in forced.pages.as_ref().unwrap() {
            assert_ne!(page.content.trim(), "", "forced OCR page {}", page.page_number);
        }
        let mut format_errors = Vec::new();
        compare_automatic(&mut format_errors, &native, &automatic, nist_index);
        errors.extend(format_errors.into_iter().map(|error| format!("{format:?}: {error}")));
    }
    assert_eq!(
        errors,
        Vec::<String>::new(),
        "automatic selection must preserve both source pages"
    );
}

#[test]
fn should_preserve_unrecovered_type0_page_when_first_in_mixed_document() {
    assert_mixed_retention(true);
}

#[test]
fn should_preserve_unrecovered_type0_page_when_second_in_mixed_document() {
    assert_mixed_retention(false);
}
