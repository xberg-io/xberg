#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use xberg::core::config::{ExtractInput, ExtractionConfig, ImageExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};

// Synthetic RGB, drawn with bundled DejaVu Sans using Pillow 12.3.0. At x=55,
// lines start at y=66,101,130 with sizes 17,8,17 on RGB(218,214,196).
// Strong ink is RGB(35,34,31); the middle line is RGB(112,112,112). ~keep
const IMAGE: &[u8] = include_bytes!("fixtures/ocr/embedded_psm6_panel_2327.png");
const NATIVE: [&str; 4] = [
    "Archive inventory memorandum for the branch office and municipal staff.",
    "The original image and paper sources remain available in the reference room.",
    "Readers may request a copy from the records desk during business hours.",
    "Page 7",
];
const FIRST: &str = "Annual calibration records are available for review.";
const MISSING: &str = "Relay service starts Monday";
const LAST: &str = "Deliver all equipment by Friday afternoon.";

fn panel_pdf(overpaint: bool, duplicate: bool) -> Vec<u8> {
    panel_pdf_with_decode(overpaint, duplicate, false)
}

fn panel_pdf_with_decode(overpaint: bool, duplicate: bool, unproved_decode: bool) -> Vec<u8> {
    let raster = image::load_from_memory(IMAGE)
        .expect("load synthetic panel")
        .into_rgb8();
    assert_eq!(raster.dimensions(), (612, 396));

    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut image_dictionary = dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => 612, "Height" => 396,
    };
    if unproved_decode {
        image_dictionary.set(
            "Decode",
            vec![0.into(), 1.into(), 0.into(), 1.into(), 0.into(), 1.into()],
        );
    }
    let image = pdf.add_object(Stream::new(image_dictionary, raster.into_raw()));
    let mut drawing = String::new();
    for (index, line) in NATIVE.into_iter().enumerate() {
        writeln!(
            drawing,
            "BT /F1 12 Tf 1 0 0 1 72 {} Tm ({line}) Tj ET",
            740 - index * 24
        )
        .expect("write native line");
    }
    drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
    if duplicate {
        drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
    }
    if overpaint {
        drawing.push_str("BT /F1 24 Tf 1 0 0 1 55 284 Tm (Inspection overlay) Tj ET\n");
    }
    let content = pdf.add_object(Stream::new(Dictionary::new(), drawing.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font }, "XObject" => dictionary! { "Panel" => image } },
        "Contents" => content,
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    pdf.compress();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize PDF");
    bytes
}

fn config(psm: impl Into<Option<i32>>, use_cache: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            inject_placeholders: false,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: psm.into(),
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn extract(bytes: Vec<u8>, config: &ExtractionConfig) -> ExtractedDocument {
    extract_input(ExtractInput::from_bytes(bytes, "application/pdf", None), config).await
}

async fn extract_input(input: ExtractInput, config: &ExtractionConfig) -> ExtractedDocument {
    let mut batch = xberg::extract(input, config).await.unwrap();
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    let document = batch.results.remove(0);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Native));
    document
}

#[tokio::test]
async fn explicit_psm6_adds_only_the_missing_embedded_line() {
    use sha2::{Digest, Sha256};
    assert_eq!(
        hex::encode(Sha256::digest(IMAGE)),
        "04c680cdd7c95eb6ca7df11a60c93b07d6b902306d6b658c37c0b5a44b74821f"
    );
    let result = extract(panel_pdf(false, false), &config(6, false)).await;
    let [image] = result.images.as_deref().expect("retained images") else {
        panic!("expected exactly one retained image");
    };
    let original = image.ocr_result.as_ref().expect("original nested OCR");
    assert_eq!(original.content, format!("{FIRST}\n{LAST}"));
    assert_eq!(original.metadata.additional.get("psm"), Some(&serde_json::json!("6")));
    assert!(original.tables.is_empty());
    assert!(original.ocr_elements.is_none());
    let [page] = result.pages.as_deref().expect("tracked page") else {
        panic!("expected one page");
    };
    assert_eq!(page.page_number, 1);
    let expected = format!("{}\n\n{FIRST}\n{MISSING}\n{LAST}", NATIVE.join("\n\n"));
    assert_eq!(result.content, expected);
    assert_eq!(page.content, expected);
    let mut automatic = config(6, false);
    automatic.images = None;
    let ordinary = extract(panel_pdf(false, false), &automatic).await;
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.pages.as_deref().unwrap()[0].content, expected);

    let mut placeholders = config(6, false);
    placeholders.images.as_mut().unwrap().inject_placeholders = true;
    let untouched = extract(panel_pdf(false, false), &placeholders).await;
    assert!(!untouched.content.contains(MISSING));
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(untouched.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    );
}

#[tokio::test]
async fn omitted_psm_keeps_actual_psm3_and_adds_only_the_missing_embedded_line() {
    let mut reference_config = config(None, false);
    reference_config.images.as_mut().unwrap().inject_placeholders = true;
    let reference = extract(panel_pdf(false, false), &reference_config).await;
    let [reference_image] = reference.images.as_deref().unwrap() else {
        panic!("one reference image")
    };
    let reference_nested = reference_image.ocr_result.as_ref().unwrap();
    assert_eq!(reference_nested.content, format!("{FIRST}\n\n{LAST}"));
    assert_eq!(
        reference_nested.metadata.additional.get("psm"),
        Some(&serde_json::json!("3"))
    );
    assert!(!reference.content.contains(MISSING));

    let result = extract(panel_pdf(false, false), &config(None, false)).await;
    let [image] = result.images.as_deref().unwrap() else {
        panic!("one retained image")
    };
    let nested = image.ocr_result.as_ref().expect("original nested OCR");
    assert_eq!(
        serde_json::to_value(nested).unwrap(),
        serde_json::to_value(reference_nested).unwrap()
    );
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("one extracted page")
    };
    assert_eq!(page.page_number, 1);
    let expected = format!("{}\n\n{FIRST}\n\n{MISSING}\n\n{LAST}", NATIVE.join("\n\n"));
    assert_eq!(
        result.content, expected,
        "recover the literal missing line under actual PSM 3"
    );
    assert_eq!(page.content, expected);
}

fn reencode_panel_with_png_up_predictor(ordinary_pdf: &[u8]) -> Vec<u8> {
    let mut predictor_pdf = Document::load_mem(ordinary_pdf).unwrap();
    let panel = predictor_pdf
        .objects
        .values_mut()
        .filter_map(|object| object.as_stream_mut().ok())
        .find(|stream| stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
        .expect("find the single synthetic panel");
    let raster = image::load_from_memory(IMAGE).unwrap().into_rgb8();
    let row_bytes = raster.width() as usize * 3;
    let mut previous = vec![0; row_bytes];
    let mut tagged = Vec::new();
    for row in raster.as_raw().chunks_exact(row_bytes) {
        tagged.push(2);
        tagged.extend(
            row.iter()
                .zip(&previous)
                .map(|(sample, above)| sample.wrapping_sub(*above)),
        );
        previous.copy_from_slice(row);
    }
    panel.dict.remove(b"Filter");
    panel.set_content(tagged);
    panel.compress().unwrap();
    assert_eq!(
        panel.dict.get(b"Filter").and_then(Object::as_name).unwrap(),
        b"FlateDecode"
    );
    panel.dict.set(
        "DecodeParms",
        dictionary! { "Predictor" => 12, "Columns" => 612, "Colors" => 3, "BitsPerComponent" => 8 },
    );
    let mut bytes = Vec::new();
    predictor_pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn png_predictor_panel_recovers_missing_line_without_changing_primary_ocr() {
    let ordinary_pdf = panel_pdf(false, false);
    let predictor_pdf = reencode_panel_with_png_up_predictor(&ordinary_pdf);
    let config = config(None, false);
    let ordinary = extract(ordinary_pdf, &config).await;
    let predicted = extract(predictor_pdf, &config).await;
    let [ordinary_image] = ordinary.images.as_deref().unwrap() else {
        panic!("one ordinary panel");
    };
    let [predicted_image] = predicted.images.as_deref().unwrap() else {
        panic!("one predictor panel");
    };
    assert_eq!(ordinary_image.data, predicted_image.data);
    assert_eq!(
        serde_json::to_value(&ordinary_image.ocr_result).unwrap(),
        serde_json::to_value(&predicted_image.ocr_result).unwrap()
    );
    let expected = format!("{}\n\n{FIRST}\n\n{MISSING}\n\n{LAST}", NATIVE.join("\n\n"));
    assert_eq!(ordinary.content, expected);
    assert_eq!(predicted.content, expected);
    assert_eq!(predicted.pages.as_deref().unwrap()[0].content, expected);
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn cached_default_recovers_embedded_line_in_cold_and_warm_reads() {
    const CHILD: &str = "XBERG_EMBEDDED_PROSE_2327_CACHE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        run_isolated_cache_test(CHILD);
        return;
    }

    use tracing_subscriber::layer::SubscriberExt as _;
    let lookups = CacheLookups::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(lookups.clone())).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cached-panel.pdf");
    let bytes = panel_pdf(false, false);
    std::fs::write(&path, &bytes).unwrap();

    let mut cached = config(None, true);
    cached.ocr = Some(OcrConfig::default());
    let expected = format!("{}\n\n{FIRST}\n\n{MISSING}\n\n{LAST}", NATIVE.join("\n\n"));
    let cold = extract(bytes.clone(), &cached).await;
    assert_eq!(cold.content, expected);
    assert_eq!(cold.pages.as_deref().unwrap()[0].content, expected);
    let cold_lookups = lookups.entries();
    let [(key, false)] = cold_lookups.as_slice() else {
        panic!("one cold primary OCR lookup: {cold_lookups:?}");
    };
    let inner_warm = extract(bytes, &cached).await;
    assert_eq!(
        serde_json::to_value(&cold).unwrap(),
        serde_json::to_value(inner_warm).unwrap()
    );
    assert_eq!(lookups.entries(), [(key.clone(), false), (key.clone(), true)]);

    let uri = ExtractInput::from_uri(path.to_string_lossy().into_owned());
    let uri_cold = extract_input(uri.clone(), &cached).await;
    assert_eq!(uri_cold.content, expected);
    assert_eq!(uri_cold.pages.as_deref().unwrap()[0].content, expected);
    let before_outer_warm = [(key.clone(), false), (key.clone(), true), (key.clone(), true)];
    assert_eq!(lookups.entries(), before_outer_warm);
    let outer_warm = extract_input(uri, &cached).await;
    assert_eq!(
        serde_json::to_value(&uri_cold).unwrap(),
        serde_json::to_value(outer_warm).unwrap()
    );
    assert_eq!(lookups.entries(), before_outer_warm, "a warm extraction skips OCR");

    cached.images.as_mut().unwrap().inject_placeholders = true;
    let reference = extract(panel_pdf(false, false), &cached).await;
    let nested = |doc: &ExtractedDocument| {
        serde_json::to_value(doc.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    };
    assert_eq!(nested(&cold), nested(&reference));
    assert_eq!(nested(&uri_cold), nested(&reference));
    assert!(!reference.content.contains(MISSING));

    cached.images = None;
    let ordinary = extract(panel_pdf(false, false), &cached).await;
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.pages.as_deref().unwrap()[0].content, expected);
}

#[cfg(feature = "bundle-tessdata-eng")]
fn run_isolated_cache_test(child: &str) {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().expect("isolated OCR cache");
    let tessdata = root.path().join("tessdata");
    std::fs::create_dir(&tessdata).unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English fixture");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.join("eng.traineddata"), english).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cached_default_recovers_embedded_line_in_cold_and_warm_reads",
            "--nocapture",
        ])
        .env(child, "1")
        .env("TESSDATA_PREFIX", tessdata)
        .env("XBERG_CACHE_DIR", root.path().join("cache"))
        .output()
        .expect("run isolated cache test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(feature = "bundle-tessdata-eng")]
#[derive(Clone, Default)]
struct CacheLookups(std::sync::Arc<std::sync::Mutex<Vec<(String, bool)>>>);

#[cfg(feature = "bundle-tessdata-eng")]
impl CacheLookups {
    fn entries(&self) -> Vec<(String, bool)> {
        self.0.lock().unwrap().clone()
    }
}

#[cfg(feature = "bundle-tessdata-eng")]
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CacheLookups {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() == "xberg::ocr::cache" {
            let mut lookup = CacheLookup::default();
            event.record(&mut lookup);
            if let (Some(key), Some(hit)) = (lookup.key, lookup.hit) {
                self.0.lock().unwrap().push((key, hit));
            }
        }
    }
}

#[cfg(feature = "bundle-tessdata-eng")]
#[derive(Default)]
struct CacheLookup {
    key: Option<String>,
    hit: Option<bool>,
}

#[cfg(feature = "bundle-tessdata-eng")]
impl tracing::field::Visit for CacheLookup {
    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}

    fn record_bool(&mut self, field: &tracing::field::Field, hit: bool) {
        if field.name() == "xberg.cache.hit" {
            self.hit = Some(hit);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, key: &str) {
        if field.name() == "xberg.cache.key" {
            self.key = Some(key.to_owned());
        }
    }
}

#[tokio::test]
async fn embedded_line_recovery_leaves_other_modes_and_unproved_placements_untouched() {
    for (psm, late, duplicate) in [(11, false, false), (6, true, false), (6, false, true)] {
        let result = extract(panel_pdf(late, duplicate), &config(psm, false)).await;
        assert_eq!(result.images.as_ref().unwrap().len(), if duplicate { 2 } else { 1 });
        assert!(
            !result.content.contains(MISSING),
            "psm={psm} late={late} duplicate={duplicate}"
        );
        if late {
            assert!(result.content.contains("Inspection overlay"));
        }
    }
}

#[tokio::test]
async fn uri_recovery_keeps_the_nested_result_with_or_without_extraction_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("embedded-panel.pdf");
    std::fs::write(&path, panel_pdf(false, false)).unwrap();
    let uri = ExtractInput::from_uri(path.to_string_lossy().into_owned());
    let enabled = extract_input(uri.clone(), &config(6, false)).await;
    let cached = extract_input(uri, &config(6, true)).await;
    assert_eq!(enabled.content.matches(MISSING).count(), 1);
    assert_eq!(enabled.pages.as_ref().unwrap()[0].content.matches(MISSING).count(), 1);
    assert_eq!(cached.content, enabled.content);
    assert_eq!(
        cached.pages.as_ref().unwrap()[0].content,
        enabled.pages.as_ref().unwrap()[0].content
    );
    assert_eq!(
        serde_json::to_value(enabled.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap(),
        serde_json::to_value(cached.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    );
}

#[tokio::test]
async fn explicit_decode_remains_outside_the_pixel_proof() {
    let result = extract(panel_pdf_with_decode(false, false, true), &config(6, false)).await;
    assert!(!result.content.contains(MISSING));
    let image = &result.images.as_deref().unwrap()[0];
    assert_eq!(image.ocr_result.as_ref().unwrap().content, format!("{FIRST}\n{LAST}"));
}
