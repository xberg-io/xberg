#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use xberg::types::{ExtractedDocument, OcrBoundingGeometry, OcrElementConfig, OcrElementLevel, PageContent};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig, OutputFormat, PageConfig, TesseractConfig};

const REPORTER: &[u8] = include_bytes!("fixtures/ocr_spanish_layer_2951/spanish.pdf");
const BODY: &str = "La dirección del proyecto presentó el informe anual ante el consejo.
El niño y la niña visitaron la biblioteca del barrio cada mañana.
Este año la población creció y la administración abrió una oficina nueva.
La educación pública recibió más atención en la sesión de hoy.
El señor Peña explicó la situación económica de la región.
Los técnicos revisaron la instalación eléctrica del pabellón.
También se aprobó la compra de cañerías para la estación del norte.
La comisión recomendó una reunión extraordinaria después de la inspección.
El jardín de la plaza tendrá un sistema de riego automático.
Mañana el ayuntamiento publicará la lista de contratación.
La información completa estará disponible en la página oficial.
Gracias por su participación y por su colaboración.";

#[tokio::test]
#[ignore = "matched #2951 acceptance replay requires a dedicated OCR build slot"]
async fn english_ocr_preserves_the_reporters_spanish_body() {
    replay(REPORTER, false, false).await;
}

#[tokio::test]
#[ignore = "matched #2951 acceptance replay requires a dedicated OCR build slot"]
async fn correction_does_not_rewrite_the_same_spelling_at_a_disjoint_position() {
    replay(
        include_bytes!("fixtures/ocr_spanish_layer_2951/position-control.pdf"),
        true,
        false,
    )
    .await;
}

#[tokio::test]
#[ignore = "matched #2951 acceptance replay requires a dedicated OCR build slot"]
async fn recovery_does_not_publish_invisible_native_text() {
    replay(
        include_bytes!("fixtures/ocr_spanish_layer_2951/invisible-control.pdf"),
        true,
        true,
    )
    .await;
}

async fn replay(pdf: &[u8], position_control: bool, invisible_control: bool) {
    assert_fixture_provenance(pdf, position_control, invisible_control);
    let tessdata = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    use sha2::{Digest, Sha256};
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    let mut config = ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_owned()),
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                ..Default::default()
            }),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        config.output_format = format.clone();
        let result = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), &config)
            .await
            .expect("extract reporter fixture");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let [doc] = result.results.as_slice() else {
            panic!("expected one document")
        };
        assert!(
            matches!(
                doc.extraction_method,
                Some(xberg::types::ExtractionMethod::Mixed | xberg::types::ExtractionMethod::Ocr)
            ),
            "{format:?}"
        );
        let [page] = doc.pages.as_deref().expect("page output") else {
            panic!("expected one page")
        };
        assert_eq!(page.page_number, 1);
        if position_control {
            assert_positioned_words(doc, page, &format);
        }
        let expected = normalized(BODY);
        for (surface, text) in [("document", &doc.content), ("page", &page.content)] {
            let text = normalized(text);
            if position_control {
                assert_eq!(
                    text.matches("Control visible ano").count(),
                    1,
                    "{format:?} {surface}: {text}"
                );
                assert_eq!(
                    text.matches("Este año la población").count(),
                    1,
                    "{format:?} {surface}: {text}"
                );
            }
            if invisible_control {
                assert!(!text.contains("invisibilización"), "{format:?} {surface}: {text}");
            }
            assert_eq!(text.matches(&expected).count(), 1, "{format:?} {surface}: {text}");
        }
    }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_fixture_provenance(pdf: &[u8], position_control: bool, invisible_control: bool) {
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(pdf.to_vec()).expect("parse source fixture");
    assert_eq!(pdf.page_count().unwrap(), 1);
    assert_eq!(pdf.get_page_media_box(0).unwrap(), (0.0, 0.0, 612.0, 792.0));
    let native = normalized(&pdf.extract_text(0).expect("native transcript"));
    let painted = normalized(
        &pdf.extract_text_without_nonpainting_glyphs(0)
            .expect("painting transcript"),
    );
    for text in [&native, &painted] {
        assert_eq!(text.matches(&normalized(BODY)).count(), 1, "{text}");
        assert_eq!(
            text.matches("Control visible ano").count(),
            usize::from(position_control),
            "{text}"
        );
    }
    assert_eq!(
        native.matches("invisibilización").count(),
        usize::from(invisible_control),
        "{native}"
    );
    assert!(!painted.contains("invisibilización"), "{painted}");
}

fn assert_positioned_words(doc: &ExtractedDocument, page: &PageContent, format: &OutputFormat) {
    let preprocessing = page
        .image_preprocessing
        .as_ref()
        .or(doc.metadata.image_preprocessing.as_ref())
        .expect("OCR preprocessing geometry");
    let dimensions = preprocessing
        .new_dimensions
        .as_ref()
        .unwrap_or(&preprocessing.original_dimensions);
    assert!(dimensions.width > 0 && dimensions.height > 0, "{format:?}");
    let mut body = Vec::new();
    let mut footer = Vec::new();
    for word in doc.ocr_elements.as_deref().expect("public OCR words") {
        if word.level != OcrElementLevel::Word || !matches!(word.text.as_str(), "ano" | "año") {
            continue;
        }
        assert_eq!(word.page_number, 1);
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = &word.geometry
        else {
            panic!("{format:?}: expected Tesseract rectangle for {word:?}");
        };
        assert!(*width > 0 && *height > 0, "{word:?}");
        let x = (f64::from(*left) + f64::from(*width) / 2.0) * 612.0 / dimensions.width as f64;
        let y = (f64::from(*top) + f64::from(*height) / 2.0) * 792.0 / dimensions.height as f64;
        if (95.0..125.0).contains(&x) && (373.0..395.0).contains(&y) {
            body.push(word.text.as_str());
        } else if (158.0..194.0).contains(&x) && (571.0..596.0).contains(&y) {
            footer.push(word.text.as_str());
        } else {
            panic!("{format:?}: unexpected occurrence {:?} at ({x}, {y})", word.text);
        }
    }
    assert_eq!(body, ["año"], "{format:?} body occurrence");
    assert_eq!(footer, ["ano"], "{format:?} disjoint occurrence");
    assert!(
        doc.ocr_elements
            .iter()
            .flatten()
            .all(|word| !word.text.contains("invisibilización")),
        "{format:?}"
    );
}
