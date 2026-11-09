use super::{Mode, assert_public_pages, extract_matrix, mixed_source_pdf, pinned_config, record_equal};
use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use sha2::{Digest, Sha256};
use xberg::types::ExtractionMethod;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const OUTLINED: &[u8] = include_bytes!("../fixtures/ocr/outlined_prose_2313.pdf");
// Source sentences of the tracked DejaVu outline fixture, not an OCR transcript. ~keep
const VISIBLE: [&str; 5] = [
    "The committee met on Tuesday to review the plans for the library.",
    "Members asked about the schedule, budget, and planned parking.",
    "The architect said work would start after the winter season ends.",
    "A vote is planned for the next regular meeting of the council.",
    "Residents may send written comments before the end of the month.",
];
const BAD: [&str; 5] = [
    "The warehouse workers inspected the broken engine beside the river.",
    "Visitors requested a refund for the damaged furniture and plates.",
    "The gardener planted roses behind the wooden gate last summer.",
    "A musician played a quiet melody during the evening concert.",
    "The captain ordered every sailor to secure the ropes before sunrise.",
];

fn marker_font(pdf: &mut Document) -> ObjectId {
    let program = include_bytes!("../../../xberg-native-pdf/tests/fixtures/fonts/DejaVuSansMono.ttf");
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

fn bad_layer_form(pdf: &mut Document) -> ObjectId {
    let ordinary = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let markers = marker_font(pdf);
    let mut content = "BT /F1 12 Tf 72 640 Td ".to_owned();
    for line in BAD {
        content.push_str(&format!("({line}) Tj 0 -20 Td "));
    }
    content.push_str("ET BT /F2 12 Tf 72 520 Td <00440044004400440044004400440044> Tj ET");
    pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "FormType" => 1,
            "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => ordinary, "F2" => markers } },
        },
        content.into_bytes(),
    ))
}

fn contradicted_page() -> Vec<u8> {
    assert_eq!(
        hex::encode(Sha256::digest(OUTLINED)),
        "106b3839dbb131d5b0ee8bd5c065eeb10247e59dcf4b3927cf7f83a668bd82f6"
    );
    let mut pdf = Document::load_mem(OUTLINED).unwrap();
    let page = pdf.get_pages()[&1];
    let form = bad_layer_form(&mut pdf);
    let resources = pdf.get_dictionary(page).unwrap().get(b"Resources").unwrap().clone();
    let mut resources = match resources {
        Object::Reference(id) => pdf.get_dictionary(id).unwrap().clone(),
        Object::Dictionary(dictionary) => dictionary,
        _ => panic!("source page resources"),
    };
    assert_eq!(resources.has(b"XObject"), false);
    resources.set("XObject", dictionary! { "Bad" => form });
    let mut contents = vec![Object::Reference(pdf.add_object(Stream::new(
        dictionary! {},
        b"q /Bad Do 1 1 1 rg 0 0 612 792 re f Q".to_vec(),
    )))];
    contents.extend(pdf.get_page_contents(page).into_iter().map(Object::Reference));
    let page = pdf.get_dictionary_mut(page).unwrap();
    page.set("Resources", resources);
    page.set("Contents", contents);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn assert_independent_source(bytes: &[u8], bad_index: usize) {
    let source = xberg_native_pdf::PdfDocument::from_bytes(OUTLINED.to_vec()).unwrap();
    let mixed = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(mixed.page_count().unwrap(), 2);
    let spans = mixed.extract_page_text(bad_index).unwrap().spans;
    let text: String = spans.into_iter().map(|span| span.text).collect();
    assert_eq!(text.matches(char::REPLACEMENT_CHARACTER).count(), 8);
    for line in BAD {
        assert_eq!(text.matches(line).count(), 1);
    }
    for line in VISIBLE {
        assert_eq!(text.matches(line).count(), 0);
    }
    let options = RenderOptions::with_dpi(150);
    let original = render_page(&source, 0, &options).unwrap();
    let painted = render_page(&mixed, bad_index, &options).unwrap();
    assert_eq!((painted.width, painted.height), (original.width, original.height));
    assert_eq!(
        Sha256::digest(&painted.data),
        Sha256::digest(&original.data),
        "covered bad text must not change visible source ink"
    );
}

fn compare_read_wins(errors: &mut Vec<String>, document: &xberg::ExtractedDocument, bad_index: usize) {
    let page = &document.pages.as_ref().unwrap()[bad_index];
    for content in [&page.content, &document.content] {
        for line in VISIBLE {
            record_equal(
                errors,
                &format!("independent visible source: {line}"),
                content.matches(line).count(),
                1,
            );
        }
        for line in BAD {
            record_equal(
                errors,
                &format!("covered native source: {line}"),
                content.matches(line).count(),
                0,
            );
        }
        record_equal(
            errors,
            "no replacement markers",
            content.matches(char::REPLACEMENT_CHARACTER).count(),
            0,
        );
    }
}

fn compare_automatic_choice(
    errors: &mut Vec<String>,
    native: &xberg::ExtractedDocument,
    automatic: &xberg::ExtractedDocument,
    bad_index: usize,
) {
    record_equal(
        errors,
        "successful read must replace bad marker layer",
        automatic.extraction_method,
        Some(ExtractionMethod::Mixed),
    );
    record_equal(errors, "real OCR used", automatic.metadata.ocr_used, true);
    compare_read_wins(errors, automatic, bad_index);
    record_equal(
        errors,
        "unselected sibling",
        automatic.pages.as_ref().unwrap()[1 - bad_index].content.as_str(),
        native.pages.as_ref().unwrap()[1 - bad_index].content.as_str(),
    );
    let prefix = format!("Page {} retained native text;", bad_index + 1);
    let kept = automatic
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.starts_with(&prefix))
        .count();
    record_equal(errors, "bad marker page not certified for retention", kept, 0);
}

fn contradicted_marker_page_uses_ocr(bad_first: bool) {
    let bytes = mixed_source_pdf(&contradicted_page(), bad_first);
    let bad_index = usize::from(!bad_first);
    assert_independent_source(&bytes, bad_index);
    let models = tempfile::tempdir().unwrap();
    let config = pinned_config(models.path());
    let matrix = extract_matrix(&bytes, &config);
    let mut errors = Vec::new();
    for (format, results) in matrix {
        let [native, automatic, forced] = results.map(|result| result.unwrap());
        for (document, mode) in [
            (&native, Mode::Native),
            (&automatic, Mode::Automatic),
            (&forced, Mode::Forced),
        ] {
            assert_public_pages(document, &format, mode);
        }
        let source = &native.pages.as_ref().unwrap()[bad_index].content;
        for line in BAD {
            assert_eq!(source.matches(line).count(), 1);
        }
        for line in VISIBLE {
            assert_eq!(source.matches(line).count(), 0);
        }
        assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
        let mut forced_errors = Vec::new();
        compare_read_wins(&mut forced_errors, &forced, bad_index);
        assert_eq!(
            forced_errors,
            Vec::<String>::new(),
            "{format:?}: independent readable-word prerequisite"
        );
        let mut format_errors = Vec::new();
        compare_automatic_choice(&mut format_errors, &native, &automatic, bad_index);
        errors.extend(format_errors.into_iter().map(|error| format!("{format:?}: {error}")));
    }
    assert_eq!(errors, Vec::<String>::new(), "successful read must win in both formats");
}

#[test]
fn should_choose_visible_outline_words_when_bad_marker_layer_is_first() {
    contradicted_marker_page_uses_ocr(true);
}

#[test]
fn should_choose_visible_outline_words_when_bad_marker_layer_is_second() {
    contradicted_marker_page_uses_ocr(false);
}
