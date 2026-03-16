pub mod rtdetr;
pub mod tatr;
pub mod yolo;

use image::RgbImage;

use crate::layout::error::LayoutError;
use crate::layout::types::LayoutDetection;

/// Common interface for all layout detection model backends.
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
