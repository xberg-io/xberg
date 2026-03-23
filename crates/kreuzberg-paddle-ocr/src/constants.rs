//! Shared normalization constants for PaddleOCR preprocessing.
//!
//! Two normalization schemes are used:
//!
//! - **ImageNet** (`IMAGENET_MEAN_VALUES` / `IMAGENET_NORM_VALUES`): used by the text
//!   detection network (`DbNet`) and the angle classifier (`AngleNet`).
//!   Formula: `(pixel - mean * 255) * (1 / (std * 255))`.
//!
//! - **CRNN** (`CRNN_MEAN_VALUES` / `CRNN_NORM_VALUES`): used by the text recognition
//!   network (`CrnnNet`).
//!   Formula: `(pixel - 127.5) * (1 / 127.5)`.

/// ImageNet channel means (R, G, B), pre-multiplied by 255.
///
/// Derived from `[0.485, 0.456, 0.406]` (per-channel ImageNet means).
/// Used by `DbNet` (text detection) and `AngleNet` (angle classification).
pub(crate) const IMAGENET_MEAN_VALUES: [f32; 3] = [0.485 * 255.0, 0.456 * 255.0, 0.406 * 255.0];

/// ImageNet channel normalization factors (R, G, B), equal to `1 / (std * 255)`.
///
/// Derived from `[0.229, 0.224, 0.225]` (per-channel ImageNet standard deviations).
/// Used by `DbNet` (text detection) and `AngleNet` (angle classification).
pub(crate) const IMAGENET_NORM_VALUES: [f32; 3] = [1.0 / (0.229 * 255.0), 1.0 / (0.224 * 255.0), 1.0 / (0.225 * 255.0)];

/// CRNN channel means (R, G, B): `127.5` for all channels.
///
/// Used by `CrnnNet` (text recognition).
pub(crate) const CRNN_MEAN_VALUES: [f32; 3] = [127.5, 127.5, 127.5];

/// CRNN channel normalization factors (R, G, B): `1 / 127.5` for all channels.
///
/// Used by `CrnnNet` (text recognition).
pub(crate) const CRNN_NORM_VALUES: [f32; 3] = [1.0 / 127.5, 1.0 / 127.5, 1.0 / 127.5];
