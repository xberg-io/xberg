use crate::{
    base_net::BaseNet,
    constants::{IMAGENET_MEAN_VALUES, IMAGENET_NORM_VALUES},
    inference::{self, ModelBackend},
    ocr_error::OcrError,
    ocr_result::Angle,
    ocr_utils::OcrUtils,
};

const ANGLE_DST_WIDTH: u32 = 160;
const ANGLE_DST_HEIGHT: u32 = 80;
const ANGLE_COLS: usize = 2;

pub struct AngleNet {
    backend: Option<Box<dyn ModelBackend>>,
}

impl std::fmt::Debug for AngleNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AngleNet")
            .field("initialized", &self.backend.is_some())
            .field("backend", &self.backend.as_ref().map(|backend| backend.name()))
            .finish()
    }
}

impl BaseNet for AngleNet {
    fn new() -> Self {
        Self { backend: None }
    }

    fn set_backend(&mut self, backend: Option<Box<dyn ModelBackend>>) {
        self.backend = backend;
    }
}

impl AngleNet {
    /// Whether a classifier model is loaded. Without one, only `do_angle = false` reads succeed.
    pub fn is_initialized(&self) -> bool {
        self.backend.is_some()
    }

    pub fn get_angles(
        &self,
        part_imgs: &[image::RgbImage],
        do_angle: bool,
        most_angle: bool,
        cls_thresh: f32,
    ) -> Result<Vec<Angle>, OcrError> {
        // Checked before the loop so a read with no regions fails the same way as one with text.
        if do_angle && self.backend.is_none() {
            return Err(OcrError::SessionNotInitialized);
        }

        let mut angles = Vec::with_capacity(part_imgs.len());

        if do_angle {
            for img in part_imgs {
                let angle = self.get_angle(img, cls_thresh)?;
                angles.push(angle);
            }
        } else {
            angles.extend(part_imgs.iter().map(|_| Angle::default()));
        }

        if do_angle && most_angle {
            let sum: i32 = angles.iter().map(|x| x.index).sum();
            let half_percent = angles.len() as f32 / 2.0;
            let most_angle_index = if (sum as f32) < half_percent { 0 } else { 1 };

            for angle in angles.iter_mut() {
                angle.index = most_angle_index;
            }
        }

        Ok(angles)
    }

    fn get_angle(&self, img_src: &image::RgbImage, cls_thresh: f32) -> Result<Angle, OcrError> {
        let Some(backend) = &self.backend else {
            return Err(OcrError::SessionNotInitialized);
        };

        let angle_img = image::imageops::resize(
            img_src,
            ANGLE_DST_WIDTH,
            ANGLE_DST_HEIGHT,
            image::imageops::FilterType::Triangle,
        );

        let input_tensors =
            OcrUtils::substract_mean_normalize(&angle_img, &IMAGENET_MEAN_VALUES, &IMAGENET_NORM_VALUES);

        let (_, src_data) = inference::run_flat(backend.as_ref(), input_tensors.into_dyn())?;

        let mut angle = Self::score_to_angle(&src_data, ANGLE_COLS);

        if angle.score < cls_thresh {
            angle.index = 0;
        }

        Ok(angle)
    }

    fn score_to_angle(src_data: &[f32], angle_cols: usize) -> Angle {
        let mut angle = Angle::default();
        let mut max_value = f32::MIN;
        let mut angle_index = 0;

        for (i, value) in src_data.iter().take(angle_cols).enumerate() {
            if *value > max_value {
                max_value = *value;
                angle_index = i as i32;
            }
        }

        angle.index = angle_index;
        angle.score = max_value;
        angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UprightClassifier;

    impl ModelBackend for UprightClassifier {
        fn name(&self) -> &str {
            "stub"
        }

        fn run(&self, _input: inference::Tensor) -> Result<inference::Tensor, OcrError> {
            Ok(ndarray::arr2(&[[0.99_f32, 0.01]]).into_dyn())
        }
    }

    fn regions() -> Vec<image::RgbImage> {
        vec![image::RgbImage::new(64, 16), image::RgbImage::new(32, 16)]
    }

    #[test]
    fn a_read_without_angle_classification_needs_no_classifier() {
        let net = AngleNet::new();
        assert!(!net.is_initialized());

        let angles = net
            .get_angles(&regions(), false, false, 0.9)
            .expect("no classifier is consulted");

        assert_eq!(angles.len(), 2);
        assert!(angles.iter().all(|angle| angle.index == 0 && angle.score == 0.0));
    }

    #[test]
    fn angle_classification_without_a_classifier_fails_instead_of_skipping() {
        let error = AngleNet::new()
            .get_angles(&regions(), true, false, 0.9)
            .expect_err("a missing classifier must not read as upright text");

        assert!(matches!(error, OcrError::SessionNotInitialized));
    }

    #[test]
    fn angle_classification_without_a_classifier_fails_with_no_regions() {
        let error = AngleNet::new()
            .get_angles(&[], true, false, 0.9)
            .expect_err("a missing classifier fails whatever the page holds");

        assert!(matches!(error, OcrError::SessionNotInitialized));
    }

    #[test]
    fn a_loaded_classifier_scores_every_region() {
        let mut net = AngleNet::new();
        net.set_backend(Some(Box::new(UprightClassifier)));
        assert!(net.is_initialized());

        let angles = net
            .get_angles(&regions(), true, false, 0.9)
            .expect("the classifier runs");

        assert_eq!(angles.len(), 2);
        assert!(angles.iter().all(|angle| angle.index == 0 && angle.score == 0.99));
    }
}
