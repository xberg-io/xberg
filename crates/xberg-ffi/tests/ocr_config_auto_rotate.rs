use std::ffi::CString;
use xberg_ffi::{
    xberg_ocr_config_auto_rotate, xberg_ocr_config_free, xberg_ocr_config_from_json, xberg_ocr_config_has_auto_rotate,
};

/// Returns `(has_auto_rotate, auto_rotate)` for an `OcrConfig` parsed from `json`.
fn read(json: &str) -> (i32, i32) {
    let json = CString::new(json).unwrap();
    // SAFETY: `json` is a valid NUL-terminated string; the handle is freed before returning.
    unsafe {
        let handle = xberg_ocr_config_from_json(json.as_ptr());
        assert_ne!(handle, 0, "the config JSON must parse");
        let observed = (
            xberg_ocr_config_has_auto_rotate(handle),
            xberg_ocr_config_auto_rotate(handle),
        );
        xberg_ocr_config_free(handle);
        observed
    }
}

#[test]
fn has_auto_rotate_distinguishes_an_unset_policy_from_an_explicit_false() {
    assert_eq!(read(r#"{"backend":"tesseract"}"#), (0, 0), "unset");
    assert_eq!(read(r#"{"auto_rotate":false}"#), (1, 0), "explicit false");
    assert_eq!(read(r#"{"auto_rotate":true}"#), (1, 1), "explicit true");
}
