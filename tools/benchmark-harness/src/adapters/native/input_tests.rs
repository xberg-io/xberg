use super::*;

fn base_config(force_ocr: bool, backend: Option<&str>) -> ExtractionConfig {
    let ocr = backend.map(|backend| xberg::OcrConfig {
        backend: backend.to_string(),
        language: vec!["eng".to_string()],
        ..Default::default()
    });
    ExtractionConfig {
        force_ocr,
        ocr,
        ..Default::default()
    }
}

#[test]
fn build_batch_input_does_not_disable_base_forced_ocr() {
    let base = base_config(true, Some("tesseract"));
    let input = build_batch_input(Path::new("/a.pdf"), false, Some("deu"), &base);
    let file_config = input.config.expect("file config should be set");
    assert_eq!(file_config.force_ocr, Some(true));
}

#[test]
fn build_batch_input_preserves_backend_and_canonicalizes_language() {
    let base = base_config(false, Some("paddle"));
    let input = build_batch_input(Path::new("/a.pdf"), false, Some("deu+eng"), &base);
    let ocr = input.config.expect("file config").ocr.expect("ocr override");
    assert_eq!(ocr.backend, "paddle");
    assert_eq!(ocr.language, vec!["deu".to_string(), "eng".to_string()]);
    assert!(ocr.tesseract_config.is_none());
}

#[test]
fn build_batch_input_updates_nested_tesseract_language_and_disables_cache() {
    let mut base = base_config(true, Some("tesseract"));
    base.ocr.as_mut().unwrap().tesseract_config = Some(xberg::TesseractConfig::default());

    let input = build_batch_input(Path::new("/a.pdf"), false, Some("deu+eng"), &base);
    let ocr = input.config.expect("file config").ocr.expect("ocr override");
    let tesseract = ocr.tesseract_config.expect("nested Tesseract config");

    assert_eq!(ocr.language, vec!["deu".to_string(), "eng".to_string()]);
    assert_eq!(tesseract.language, ocr.language);
    assert!(!tesseract.use_cache);
}

#[test]
fn extraction_config_for_sets_tesseract_language_without_materializing_nested_config() {
    let adapter = NativeAdapter::with_config(base_config(true, Some("tesseract")));

    let config = adapter.extraction_config_for(false, Some("deu+eng"));
    let ocr = config.ocr.expect("OCR config");

    assert_eq!(ocr.language, vec!["deu".to_string(), "eng".to_string()]);
    assert!(
        ocr.tesseract_config.is_none() && ocr.backend_options.is_none(),
        "either field would switch xberg off its default OCR route"
    );
}

#[test]
fn extraction_config_for_refreshes_an_existing_tesseract_config() {
    let mut base = base_config(true, Some("tesseract"));
    base.ocr.as_mut().unwrap().tesseract_config = Some(xberg::TesseractConfig::default());
    let adapter = NativeAdapter::with_config(base);

    let config = adapter.extraction_config_for(false, Some("deu+eng"));
    let ocr = config.ocr.expect("OCR config");
    let tesseract = ocr.tesseract_config.expect("nested Tesseract config");

    assert_eq!(tesseract.language, ocr.language);
    assert!(!tesseract.use_cache);
}

#[test]
fn extraction_config_for_preserves_non_tesseract_backend() {
    let adapter = NativeAdapter::with_config(base_config(false, Some("paddle")));

    let config = adapter.extraction_config_for(false, Some("deu+eng"));
    let ocr = config.ocr.expect("OCR config");

    assert_eq!(ocr.backend, "paddle");
    assert_eq!(ocr.language, vec!["deu".to_string(), "eng".to_string()]);
    assert!(ocr.tesseract_config.is_none());
}

#[test]
fn build_batch_input_without_overrides_inherits_base() {
    let base = base_config(false, Some("paddle"));
    let input = build_batch_input(Path::new("/a.pdf"), false, None, &base);
    assert!(input.config.is_none());
}

#[test]
fn build_batch_input_force_ocr_without_language_leaves_ocr_inherited() {
    let base = base_config(false, Some("paddle"));
    let input = build_batch_input(Path::new("/a.pdf"), true, None, &base);
    let file_config = input.config.expect("file config");
    assert_eq!(file_config.force_ocr, Some(true));
    assert!(file_config.ocr.is_none());
}
