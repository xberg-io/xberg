#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use lopdf::{Object, dictionary};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig};

const NOTE: &str = "Pinned tall guide annotation control";
const GUIDE: &str = "04c76db01e20d7641e886b2403bea2df5757dfe9b4be7a283ee44d1861081a06";

fn isolated(source: &[u8], number: u32, annotated: bool) -> Vec<u8> {
    let (mut pdf, page) = helpers::isolate_pdf_page(source, number);
    assert!(!pdf.get_object(page).unwrap().as_dict().unwrap().has(b"Annots"));
    if annotated {
        let annotation = pdf.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![10.into(), 10.into(), 30.into(), 30.into()],
            "Contents" => Object::string_literal(NOTE), "F" => 0,
        });
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", vec![Object::Reference(annotation)]);
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

async fn read(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let out = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .unwrap();
    assert!(out.errors.is_empty());
    assert_eq!(out.results.len(), 1);
    out.results.into_iter().next().unwrap()
}

#[tokio::test]
#[ignore = "requires pinned public guide and external evidence directory"]
async fn guide_tables_and_real_annotations_survive_paired_reads() {
    let guide = std::fs::read(std::env::var_os("XBERG_2502_GUIDE").unwrap()).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&guide)), GUIDE);
    let evidence = PathBuf::from(std::env::var_os("XBERG_2502_EVIDENCE").unwrap());
    let tessdata = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    for (number, count) in [(4, 1), (13, 3)] {
        let bytes = isolated(&guide, number, false);
        let annotated = isolated(&guide, number, true);
        let config = ExtractionConfig {
            use_cache: false,
            ocr: Some(OcrConfig {
                tessdata_path: Some(tessdata.path().to_path_buf()),
                backend_options: Some(serde_json::json!({"use_cache": false})),
                ..Default::default()
            }),
            pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
            ..Default::default()
        };
        let mut stock = config.clone();
        stock.use_cache = true;
        stock.ocr.as_mut().unwrap().backend_options = Some(serde_json::json!({"use_cache": true}));
        let mut annotations = config.clone();
        annotations.pdf_options =
            Some(serde_json::from_value(serde_json::json!({"extract_annotations": true})).unwrap());
        let primary = read(&bytes, &config).await;
        let stock = read(&bytes, &stock).await;
        let enabled = read(&bytes, &annotations).await;
        let injected = read(&annotated, &annotations).await;
        for (label, doc) in [
            ("primary", &primary),
            ("stock", &stock),
            ("enabled", &enabled),
            ("injected", &injected),
        ] {
            std::fs::write(
                evidence.join(format!("page-{number}-{label}.json")),
                serde_json::to_vec_pretty(doc).unwrap(),
            )
            .unwrap();
        }
        assert_eq!(primary.tables.len(), count);
        assert_eq!(
            serde_json::to_value(&primary.tables).unwrap(),
            serde_json::to_value(&stock.tables).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&enabled.tables).unwrap(),
            serde_json::to_value(&injected.tables).unwrap()
        );
        assert!(enabled.annotations.is_none());
        let [note] = injected.annotations.as_deref().expect("injected note must survive OCR") else {
            panic!("expected exactly one source annotation");
        };
        assert_eq!(note.content.as_deref(), Some(NOTE));
        assert_eq!(note.page_number, 1);
        assert_eq!(injected.content.matches(NOTE).count(), 1);
        assert_eq!(
            enabled.content, primary.content,
            "annotation opt-in dropped certified page {number} text"
        );
        assert_eq!(
            enabled.pages.as_ref().unwrap()[0].content,
            primary.pages.as_ref().unwrap()[0].content,
        );
        assert_eq!(
            serde_json::to_value(&primary.tables).unwrap(),
            serde_json::to_value(&enabled.tables).unwrap(),
        );
        assert_eq!(
            injected.content.split("\nAnnotations:").next().unwrap().trim_end(),
            stock.content.trim_end(),
            "annotated source must retain the stock body",
        );
    }
}
