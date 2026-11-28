#![cfg(all(feature = "build-tesseract", not(target_arch = "wasm32")))]

use xberg_tesseract::TesseractAPI;

#[test]
fn should_link_the_pinned_tesseract_554_runtime() {
    assert_eq!(TesseractAPI::version(), "5.5.5");
}
