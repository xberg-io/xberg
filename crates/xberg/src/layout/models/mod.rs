#[cfg(feature = "layout-detection")]
/// PP-DocLayout-V3 layout detection model. ORT-only: engine-neutral on the seam, but a
/// tract 0.23.4 `LayerNormalization` op-translation bug leaves it unrunnable under tract
/// (see `docs-site/src/content/docs/concepts/tract-inference.md`), so it is gated out of `layout-tract` builds.
pub mod pp_doclayout_v3;
/// RT-DETR v2 layout detection model. Engine-neutral (runs on the `crate::inference`
/// seam) — available under both `layout-detection` (ORT) and `layout-tract` (tract).
pub mod rtdetr;
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
/// SLANeXT table structure recognition model. ORT-only (bare `ort::Session`; tract's
/// `Loop` op is unimplemented).
pub mod slanet;
#[cfg(feature = "pdf")]
/// Binary classifier for distinguishing wired vs wireless tables. Engine-neutral (runs
/// on the `crate::inference` seam) — available under both `layout-detection` (ORT) and
/// `layout-tract` (tract).
pub mod table_classifier;
#[cfg(all(feature = "layout-detection", feature = "pdf"))]
/// Table Transformer (TATR) table structure recognition model. ORT-only (bare
/// `ort::Session`; quantized export tract cannot unify — see `docs-site/src/content/docs/concepts/tract-inference.md`).
pub mod tatr;
#[cfg(feature = "layout-detection")]
/// YOLO-based layout detection models (DocLayNet, DocStructBench, YOLOX variants). ORT-only
/// (bare `ort::Session`).
pub mod yolo;

use image::RgbImage;

use crate::layout::error::LayoutError;
use crate::layout::types::LayoutDetection;

/// Common interface for all layout detection model backends.
#[cfg_attr(alef, alef(skip))]
pub trait LayoutModel: Send {
    /// Run layout detection on an image using the default confidence threshold.
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError>;

    /// Run layout detection with a custom confidence threshold.
    fn detect_with_threshold(&mut self, img: &RgbImage, threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError>;

    /// Run layout detection on a batch of images in a single model call.
    ///
    /// Returns one `Vec<LayoutDetection>` per input image (same order).
    /// `threshold` overrides the model's default confidence cutoff when `Some`.
    ///
    /// The default implementation is a sequential fallback: models that support
    /// true batched inference (e.g. [`rtdetr::RtDetrModel`]) override this.
    fn detect_batch(
        &mut self,
        images: &[&RgbImage],
        threshold: Option<f32>,
    ) -> Result<Vec<Vec<LayoutDetection>>, LayoutError> {
        images
            .iter()
            .map(|img| match threshold {
                Some(t) => self.detect_with_threshold(img, t),
                None => self.detect(img),
            })
            .collect()
    }

    /// Human-readable model name.
    fn name(&self) -> &str;
}
