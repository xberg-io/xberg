/// DPI detection and normalization for scanned images.
pub mod dpi;
/// Image preprocessing pipeline: denoising, deskew, binarization, rotation.
pub mod preprocessing;
/// Image resize helpers used before OCR to normalize resolution.
pub mod resize;

pub(crate) use preprocessing::normalize_image_dpi_owned;
