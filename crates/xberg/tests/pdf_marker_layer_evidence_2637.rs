//! Review points 1 and 2 of #2637: a long covered marker layer and a short correct visible read.
#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use sha2::{Digest, Sha256};
use xberg::core::config::OutputFormat;
use xberg::types::ExtractionMethod;
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const OUTLINED: &[u8] = include_bytes!("fixtures/ocr/outlined_prose_2313.pdf");
// Source sentences of the tracked DejaVu outline fixture, not an OCR transcript. ~keep
const VISIBLE: [&str; 5] = [
    "The committee met on Tuesday to review the plans for the library.",
    "Members asked about the schedule, budget, and planned parking.",
    "The architect said work would start after the winter season ends.",
    "A vote is planned for the next regular meeting of the council.",
    "Residents may send written comments before the end of the month.",
];
const CONTROL_LINES: [&str; 3] = [
    "Independent native control remains readable beside the checklist.",
    "Every control word comes from this authored source sentence.",
    "The neighbouring page must not replace or duplicate these words.",
];
const COVERED_ROWS: usize = 20;

/// Twenty distinct plausible sentences of the covered layer: together more than twice the letters
/// and digits of the visible sentences, so the visible read keeps under half of the layer. ~keep
fn covered_lines() -> Vec<String> {
    let who = ["warehouse workers", "gardener", "captain", "musician", "baker"];
    let what = ["engine", "furniture", "roses", "melody", "ropes"];
    let place = ["river", "harbour", "station", "orchard"];
    (0..COVERED_ROWS)
        .map(|n| {
            format!(
                "The {} checked the {} near the {} on day {} of the long survey.",
                who[n % 5],
                what[(n / 5 + n) % 5],
                place[n % 4],
                n + 1
            )
        })
        .collect()
}

fn alnum(text: &str) -> usize {
    text.chars().filter(|c| c.is_alphanumeric()).count()
}

fn marker_font(pdf: &mut Document) -> ObjectId {
    let program = include_bytes!("../../xberg-native-pdf/tests/fixtures/fonts/DejaVuSansMono.ttf");
    assert_eq!(
        hex::encode(Sha256::digest(program)),
        "0f5db4f1749979d961019838b160bec74abdf7f9eca69553fe1aa856bbff49a4"
    );
    let file = pdf.add_object(Stream::new(
        dictionary! { "Length1" => program.len() as i64 },
        program.to_vec(),
    ));
    let descriptor = pdf.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "ABCDEF+Probe", "Flags" => 4, "FontFile2" => file,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 800.into()],
        "ItalicAngle" => 0, "Ascent" => 800, "Descent" => -200, "CapHeight" => 700, "StemV" => 80,
    });
    let cid = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+Probe",
        "CIDSystemInfo" => dictionary! { "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"), "Supplement" => 0 },
        "FontDescriptor" => descriptor, "DW" => 600, "CIDToGIDMap" => "Identity",
    });
    let cmap = b"begincmap /CMapName /Probe def /CMapType 2 def \
        1 begincodespacerange <0000> <FFFF> endcodespacerange \
        1 beginbfchar <0044> <0000> endbfchar endcmap";
    let unicode = pdf.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
    pdf.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+Probe",
        "Encoding" => "Identity-H", "DescendantFonts" => vec![cid.into()], "ToUnicode" => unicode },
    )
}

/// The covered layer: the twenty sentences and eight unmapped Type0 glyphs. ~keep
fn covered_layer_form(pdf: &mut Document) -> ObjectId {
    let ordinary = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let markers = marker_font(pdf);
    let mut content = "BT /F1 11 Tf 60 740 Td ".to_owned();
    for line in covered_lines() {
        content.push_str(&format!("({line}) Tj 0 -18 Td "));
    }
    content.push_str("ET BT /F2 12 Tf 60 360 Td <00440044004400440044004400440044> Tj ET");
    pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
            "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => ordinary, "F2" => markers } },
        },
        content.into_bytes(),
    ))
}

/// The outline page with the covered layer drawn under opaque white before the visible source. ~keep
fn covered_page() -> Vec<u8> {
    assert_eq!(
        hex::encode(Sha256::digest(OUTLINED)),
        "106b3839dbb131d5b0ee8bd5c065eeb10247e59dcf4b3927cf7f83a668bd82f6"
    );
    let mut pdf = Document::load_mem(OUTLINED).unwrap();
    let page = pdf.get_pages()[&1];
    let form = covered_layer_form(&mut pdf);
    let resources = pdf.get_dictionary(page).unwrap().get(b"Resources").unwrap().clone();
    let mut resources = match resources {
        Object::Reference(id) => pdf.get_dictionary(id).unwrap().clone(),
        Object::Dictionary(dictionary) => dictionary,
        _ => panic!("source page resources"),
    };
    assert!(!resources.has(b"XObject"));
    resources.set("XObject", dictionary! { "Covered" => form });
    let mut contents = vec![Object::Reference(pdf.add_object(Stream::new(
        dictionary! {},
        b"q /Covered Do 1 1 1 rg 0 0 612 792 re f Q".to_vec(),
    )))];
    contents.extend(pdf.get_page_contents(page).into_iter().map(Object::Reference));
    let page = pdf.get_dictionary_mut(page).unwrap();
    page.set("Resources", resources);
    page.set("Contents", contents);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

/// The covered page and a clean native sibling, in either order. ~keep
fn two_page_document(covered_first: bool) -> Vec<u8> {
    let mut pdf = Document::load_mem(&covered_page()).unwrap();
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
    pdf.get_dictionary_mut(source_root).unwrap().set("Parent", root);
    let children = if covered_first {
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

/// The oracle of the fixture, from the native extractor and the renderer, not from the code under
/// test: the covered page holds the twenty sentences and eight markers, no visible sentence, more
/// than twice the letters and digits of the visible sentences, and paints the original page. ~keep
fn assert_fixture(bytes: &[u8], covered: usize) {
    let source = xberg_native_pdf::PdfDocument::from_bytes(OUTLINED.to_vec()).unwrap();
    let mixed = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(mixed.page_count().unwrap(), 2);
    let text: String = mixed
        .extract_page_text(covered)
        .unwrap()
        .spans
        .into_iter()
        .map(|span| span.text)
        .collect();
    assert_eq!(text.matches(char::REPLACEMENT_CHARACTER).count(), 8);
    for line in covered_lines() {
        assert_eq!(text.matches(line.as_str()).count(), 1, "{line}");
    }
    for line in VISIBLE {
        assert_eq!(text.matches(line).count(), 0, "{line}");
    }
    assert!(
        2 * alnum(&VISIBLE.concat()) < alnum(&covered_lines().concat()),
        "the visible read keeps under half of the covered layer"
    );
    let options = RenderOptions::with_dpi(150);
    let original = render_page(&source, 0, &options).unwrap();
    let painted = render_page(&mixed, covered, &options).unwrap();
    assert_eq!((painted.width, painted.height), (original.width, original.height));
    assert_eq!(
        Sha256::digest(&painted.data),
        Sha256::digest(&original.data),
        "the covered layer must not change the visible ink"
    );
}

fn pinned_config(models: &std::path::Path) -> ExtractionConfig {
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
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

#[derive(Clone, Copy, Debug)]
enum Mode {
    Native,
    Automatic,
    Selected(u32),
}

fn extract(bytes: &[u8], base: &ExtractionConfig, format: OutputFormat, mode: Mode) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: format,
        disable_ocr: matches!(mode, Mode::Native),
        force_ocr_pages: match mode {
            Mode::Selected(page) => Some(vec![page]),
            _ => None,
        },
        ..base.clone()
    };
    helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .unwrap_or_else(|error| panic!("{mode:?}: {error}"))
}

/// The failures of one result: on the covered page and in the whole content, each visible
/// sentence once, no covered sentence and no marker. ~keep
fn read_wins(label: &str, document: &ExtractedDocument, covered: usize) -> Vec<String> {
    let page = &document.pages.as_ref().expect("requested pages")[covered];
    let mut failures = Vec::new();
    for (scope, content) in [("page", &page.content), ("content", &document.content)] {
        for line in VISIBLE {
            let count = content.matches(line).count();
            if count != 1 {
                failures.push(format!("{label} {scope}: visible {line:?} printed {count} times"));
            }
        }
        let kept = covered_lines()
            .iter()
            .filter(|line| content.contains(line.as_str()))
            .count();
        if kept > 0 {
            failures.push(format!("{label} {scope}: {kept} covered sentences printed"));
        }
        if content.contains(char::REPLACEMENT_CHARACTER) {
            failures.push(format!("{label} {scope}: a marker printed"));
        }
    }
    failures
}

fn short_visible_read_wins(covered_first: bool) {
    let bytes = two_page_document(covered_first);
    let covered = usize::from(!covered_first);
    assert_fixture(&bytes, covered);
    let models = tempfile::tempdir().unwrap();
    let config = pinned_config(models.path());
    let mut failures = Vec::new();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let native = extract(&bytes, &config, format.clone(), Mode::Native);
        let source = &native.pages.as_ref().unwrap()[covered].content;
        let held = covered_lines()
            .iter()
            .filter(|line| source.contains(line.as_str()))
            .count();
        assert_eq!(
            held, COVERED_ROWS,
            "{format:?}: the native route prints the covered layer"
        );
        let automatic = extract(&bytes, &config, format.clone(), Mode::Automatic);
        if automatic.extraction_method != Some(ExtractionMethod::Mixed) {
            failures.push(format!(
                "{format:?} automatic: method {:?}, expected Mixed",
                automatic.extraction_method
            ));
        }
        failures.extend(read_wins(&format!("{format:?} automatic"), &automatic, covered));
        let sibling = |document: &ExtractedDocument| document.pages.as_ref().unwrap()[1 - covered].content.clone();
        if sibling(&automatic) != sibling(&native) {
            failures.push(format!("{format:?} automatic: the clean sibling changed"));
        }
        let prefix = format!("Page {} retained native text", covered + 1);
        if automatic
            .processing_warnings
            .iter()
            .any(|warning| warning.message.starts_with(&prefix))
        {
            failures.push(format!(
                "{format:?} automatic: the covered page reports native retention"
            ));
        }
        let selected = extract(&bytes, &config, format.clone(), Mode::Selected(covered as u32 + 1));
        failures.extend(read_wins(&format!("{format:?} selected"), &selected, covered));
    }
    assert!(failures.is_empty(), "the short visible read must win: {failures:#?}");
}

#[test]
#[serial_test::serial]
fn a_short_visible_read_replaces_a_long_covered_marker_layer_when_it_is_first() {
    short_visible_read_wins(true);
}

#[test]
#[serial_test::serial]
fn a_short_visible_read_replaces_a_long_covered_marker_layer_when_it_is_second() {
    short_visible_read_wins(false);
}
