//! Native OCR support for WASM via kreuzberg-tesseract
//!
//! Provides Tesseract OCR functionality compiled directly into the WASM binary.
//! Uses `TessBaseAPIInit5` to load trained data from memory (no filesystem needed).

use wasm_bindgen::prelude::*;

/// Perform OCR on raw image pixel data using Tesseract.
///
/// This function accepts pre-decoded image pixels (RGB format) along with
/// tessdata loaded into memory. No filesystem access is needed.
///
/// # Arguments
///
/// * `image_data` - Raw pixel data in RGB format (3 bytes per pixel)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `bytes_per_pixel` - Bytes per pixel (typically 3 for RGB, 1 for grayscale)
/// * `bytes_per_line` - Bytes per scan line (typically width * bytes_per_pixel)
/// * `tessdata` - Raw `.traineddata` file content loaded into memory
/// * `language` - Tesseract language code (e.g., "eng")
///
/// # Returns
///
/// The recognized text as a string.
#[cfg(feature = "ocr-wasm")]
#[wasm_bindgen(js_name = ocrRecognizeRaw)]
pub fn ocr_recognize_raw(
    image_data: &[u8],
    width: i32,
    height: i32,
    bytes_per_pixel: i32,
    bytes_per_line: i32,
    tessdata: &[u8],
    language: &str,
) -> Result<String, JsValue> {
    let api = kreuzberg_tesseract::TesseractAPI::new()
        .map_err(|e| JsValue::from_str(&format!("Failed to allocate Tesseract engine: {e}")))?;

    api.init_5(tessdata, tessdata.len() as i32, language, 3, &[])
        .map_err(|e| JsValue::from_str(&format!("Tesseract initialization failed: {e}")))?;

    api.set_image(image_data, width, height, bytes_per_pixel, bytes_per_line)
        .map_err(|e| JsValue::from_str(&format!("Failed to set image: {e}")))?;

    api.get_utf8_text()
        .map_err(|e| JsValue::from_str(&format!("OCR recognition failed: {e}")))
}

/// Perform OCR on encoded image bytes (PNG, JPEG, BMP, GIF, TIFF).
///
/// Automatically decodes the image to RGB pixels before running Tesseract.
/// This is the primary function for OCR in WASM - it handles image decoding
/// internally so the caller doesn't need browser APIs like `createImageBitmap`.
///
/// # Arguments
///
/// * `image_bytes` - Encoded image data (PNG, JPEG, BMP, GIF, TIFF)
/// * `tessdata` - Raw `.traineddata` file content loaded into memory
/// * `language` - Tesseract language code (e.g., "eng")
///
/// # Returns
///
/// The recognized text as a string.
#[cfg(feature = "ocr-wasm")]
#[wasm_bindgen(js_name = ocrRecognize)]
pub fn ocr_recognize(image_bytes: &[u8], tessdata: &[u8], language: &str) -> Result<String, JsValue> {
    let img =
        kreuzberg::extraction::image::load_image_for_ocr(image_bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();
    let raw_pixels = rgb.as_raw();
    let bytes_per_pixel = 3i32;
    let bytes_per_line = width as i32 * bytes_per_pixel;

    ocr_recognize_raw(
        raw_pixels,
        width as i32,
        height as i32,
        bytes_per_pixel,
        bytes_per_line,
        tessdata,
        language,
    )
}

/// Get the Tesseract version string compiled into this WASM binary.
///
/// Returns the version of the statically linked Tesseract library.
#[cfg(feature = "ocr-wasm")]
#[wasm_bindgen(js_name = ocrTesseractVersion)]
pub fn ocr_tesseract_version() -> String {
    kreuzberg_tesseract::TesseractAPI::version()
}

/// Check if OCR support is available in this WASM build.
///
/// Returns `true` if the `ocr-wasm` feature was enabled at build time.
#[wasm_bindgen(js_name = ocrIsAvailable)]
pub fn ocr_is_available() -> bool {
    cfg!(feature = "ocr-wasm")
}
