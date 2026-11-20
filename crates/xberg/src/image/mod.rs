/// DPI detection and normalization for scanned images.
pub mod dpi;
/// Image preprocessing pipeline: denoising, deskew, binarization, rotation.
///
/// Needs the `ocr-pipeline` dependency set; the model preprocessors use only
/// [`resize`], which `layout-detection` and `auto-rotate` also pull. ~keep
#[cfg(feature = "ocr-pipeline")]
pub mod preprocessing;
/// Image resize helpers used before OCR to normalize resolution, and by the
/// layout/orientation model preprocessors.
#[cfg(any(
    feature = "ocr-pipeline",
    feature = "layout-detection",
    feature = "layout-tract",
    feature = "auto-rotate",
    feature = "auto-rotate-tract"
))]
pub mod resize;
/// Text-size estimate used to pick a resample factor for a raster of unknown resolution.
#[cfg(feature = "ocr-pipeline")]
pub(crate) mod text_height;

// Re-exported only for the Tesseract processor (`ocr::processor::execution`); the
// standalone-image path calls `preprocessing::normalize_image_dpi_owned` by full path,
// so under `ocr-pipeline` alone this alias has no consumer.
#[cfg(feature = "ocr")]
pub(crate) use preprocessing::normalize_image_dpi_owned;
