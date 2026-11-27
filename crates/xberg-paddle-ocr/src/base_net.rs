use crate::inference::{self, Backend, ModelBackend};
use crate::ocr_error::OcrError;

#[cfg(feature = "ort")]
use ort::session::builder::SessionBuilder;

pub trait BaseNet {
    fn new() -> Self;

    fn set_backend(&mut self, backend: Option<Box<dyn ModelBackend>>);

    /// Load a model from a file path onto the compile-time default backend (see
    /// [`inference::default_backend`]).
    fn init_model(&mut self, path: &str, num_thread: usize) -> Result<(), OcrError> {
        let model_bytes = std::fs::read(path)?;
        self.init_model_from_memory(&model_bytes, num_thread)
    }

    /// Load a model from ONNX bytes onto the compile-time default backend.
    fn init_model_from_memory(&mut self, model_bytes: &[u8], num_thread: usize) -> Result<(), OcrError> {
        self.init_model_from_memory_on(inference::default_backend(), model_bytes, num_thread)
    }

    /// Load a model from a file path onto an explicitly chosen backend.
    ///
    /// See [`Self::init_model_from_memory_on`] for why an explicit choice exists at all.
    fn init_model_on(&mut self, backend: Backend, path: &str, num_thread: usize) -> Result<(), OcrError> {
        let model_bytes = std::fs::read(path)?;
        self.init_model_from_memory_on(backend, &model_bytes, num_thread)
    }

    /// Load a model from ONNX bytes onto an explicitly chosen backend.
    ///
    /// [`inference::default_backend`] resolves at compile time and prefers `ort` whenever the
    /// `ort` feature is on, so a build with **both** engines compiled in (the native
    /// cross-engine parity configuration, and any consumer that lets its caller pick per
    /// request) cannot reach `tract` through the default path at all. Selecting the engine per
    /// load is the only way to exercise both in one binary.
    fn init_model_from_memory_on(
        &mut self,
        backend: Backend,
        model_bytes: &[u8],
        num_thread: usize,
    ) -> Result<(), OcrError> {
        let backend = inference::load_backend(backend, model_bytes, num_thread, None)?;
        self.set_backend(Some(backend));
        Ok(())
    }

    /// Load a model from a file path onto the `ort` backend with a custom session
    /// builder (e.g. for GPU execution providers). ONNX Runtime reads the file itself, so the
    /// model is not also held in a buffer while the session is built.
    #[cfg(feature = "ort")]
    fn init_model_with_ort_builder(
        &mut self,
        path: &str,
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        let backend =
            inference::ort_backend::OrtBackend::load_file(std::path::Path::new(path), num_thread, builder_fn)?;
        self.set_backend(Some(Box::new(backend)));
        Ok(())
    }

    /// Load a model from ONNX bytes onto the `ort` backend with a custom session
    /// builder.
    #[cfg(feature = "ort")]
    fn init_model_from_memory_with_ort_builder(
        &mut self,
        model_bytes: &[u8],
        num_thread: usize,
        builder_fn: Option<fn(SessionBuilder) -> Result<SessionBuilder, ort::Error>>,
    ) -> Result<(), OcrError> {
        let backend = inference::ort_backend::OrtBackend::load(model_bytes, num_thread, builder_fn)?;
        self.set_backend(Some(Box::new(backend)));
        Ok(())
    }
}
