#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

extern crate xberg;

use lopdf::{Document, Object, dictionary};
use xberg::core::config::ExtractInput;

#[path = "embedded_psm6_2327/fixture.rs"]
mod fixture;
use fixture::*;

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
