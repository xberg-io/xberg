#![cfg(feature = "ocr")]

use xberg::ocr::TesseractConfig;

#[test]
fn downstream_callers_can_construct_tesseract_config_with_struct_update() {
    let config = TesseractConfig {
        language: "deu".into(),
        ..Default::default()
    };
    assert_eq!(config.language, "deu");
}
