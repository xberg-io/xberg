//! Loading a [`PaddleOcrEngine`] with or without its textline-orientation classifier.

#[cfg(feature = "ort")]
use ort::session::builder::SessionBuilder;

use super::PaddleOcrEngine;
use crate::{base_net::BaseNet, ocr_error::OcrError};

impl PaddleOcrEngine {
    /// [`Self::init_models_with_dict_on`], loading the textline-orientation classifier only when
    /// `cls_path` is given.
    ///
    /// An engine loaded without it serves reads with `do_angle = false`, which never consult
    /// the classifier. A `do_angle = true` read on it fails with
    /// [`OcrError::SessionNotInitialized`] rather than reading every line as upright.
    pub fn init_models_with_optional_classifier_on(
        &mut self,
        backend: crate::inference::Backend,
        det_path: &str,
        cls_path: Option<&str>,
        rec_path: &str,
        dict_path: &str,
        num_thread: usize,
    ) -> Result<(), OcrError> {
        self.db_net.init_model_on(backend, det_path, num_thread)?;
        match cls_path {
            Some(cls_path) => self.angle_net.init_model_on(backend, cls_path, num_thread)?,
            None => self.angle_net.set_backend(None),
        }
        self.crnn_net
            .init_model_dict_file_on(backend, rec_path, num_thread, dict_path)?;
        Ok(())
    }

    /// [`Self::init_models_with_dict_custom`], loading the textline-orientation classifier only
    /// when `cls_path` is given; see [`Self::init_models_with_optional_classifier_on`].
    #[cfg(feature = "ort")]
    pub fn init_models_with_optional_classifier_custom(
        &mut self,
        det_path: &str,
        cls_path: Option<&str>,
        rec_path: &str,
        dict_path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        self.db_net
            .init_model_with_ort_builder(det_path, num_thread, builder_fn)?;
        match cls_path {
            Some(cls_path) => self
                .angle_net
                .init_model_with_ort_builder(cls_path, num_thread, builder_fn)?,
            None => self.angle_net.set_backend(None),
        }
        self.crnn_net
            .init_model_dict_file_with_ort_builder(rec_path, num_thread, builder_fn, dict_path)?;
        Ok(())
    }

    /// Whether the textline-orientation classifier is loaded, so `do_angle = true` reads can run.
    pub fn has_angle_classifier(&self) -> bool {
        self.angle_net.is_initialized()
    }
}
