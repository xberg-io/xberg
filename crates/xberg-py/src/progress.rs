use std::sync::Arc;

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use xberg::engine::seams::{ProgressEvent, ProgressSink};

struct PythonProgressSink {
    sender: std::sync::mpsc::Sender<PythonProgressEvent>,
}

enum PythonProgressEvent {
    OcrPage {
        page: usize,
        total: usize,
        completed: usize,
        backend: String,
        input_index: Option<usize>,
    },
}

impl ProgressSink for PythonProgressSink {
    fn emit(&self, _event: ProgressEvent) {}

    fn emit_ocr_page(&self, page: usize, total: usize, completed: usize, backend: &str, input_index: Option<usize>) {
        let _ = self.sender.send(PythonProgressEvent::OcrPage {
            page,
            total,
            completed,
            backend: backend.to_string(),
            input_index,
        });
    }
}

fn validate_listener(on_progress: &Py<PyAny>) -> PyResult<()> {
    Python::attach(|py| {
        if on_progress.bind(py).hasattr("on_progress")? {
            Ok(())
        } else {
            Err(PyAttributeError::new_err("progress listener is missing on_progress"))
        }
    })
}

fn progress_forwarder(on_progress: Py<PyAny>) -> (Arc<PythonProgressSink>, tokio::task::JoinHandle<()>) {
    let (sender, receiver) = std::sync::mpsc::channel::<PythonProgressEvent>();
    let worker = tokio::task::spawn_blocking(move || {
        while let Ok(event) = receiver.recv() {
            let PythonProgressEvent::OcrPage {
                page,
                total,
                completed,
                backend,
                input_index,
            } = event;
            Python::attach(|py| {
                if let Err(error) = on_progress.call_method1(
                    py,
                    "on_progress",
                    (
                        "ocr_page",
                        Some(page),
                        Some(total),
                        Some(completed),
                        Some(backend),
                        input_index,
                    ),
                ) {
                    tracing::warn!(%error, "progress callback raised; ignoring");
                }
            });
        }
    });
    (Arc::new(PythonProgressSink { sender }), worker)
}

async fn finish_progress_forwarder(worker: tokio::task::JoinHandle<()>) {
    if let Err(error) = worker.await {
        tracing::warn!(%error, "progress callback worker failed");
    }
}

#[pyfunction]
#[pyo3(signature = (input, config, on_progress))]
pub fn extract_with_progress<'py>(
    py: Python<'py>,
    input: crate::ExtractInput,
    config: crate::ExtractionConfig,
    on_progress: Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    validate_listener(&on_progress)?;
    let input = input.into();
    let config = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let (sink, worker) = progress_forwarder(on_progress);
        let engine = xberg::engine::Engine::builder()
            .with_progress_sink(sink.clone())
            .build();
        let result = engine.extract(input, &config).await;
        drop(engine);
        drop(sink);
        finish_progress_forwarder(worker).await;
        result
            .map(crate::ExtractionResult::from)
            .map_err(crate::xberg_error_to_py_err)
    })
}

#[pyfunction]
#[pyo3(signature = (inputs, config, on_progress))]
pub fn extract_batch_with_progress<'py>(
    py: Python<'py>,
    inputs: Vec<crate::ExtractInput>,
    config: crate::ExtractionConfig,
    on_progress: Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    validate_listener(&on_progress)?;
    let inputs = inputs.into_iter().map(Into::into).collect();
    let config = config.into();
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let (sink, worker) = progress_forwarder(on_progress);
        let engine = xberg::engine::Engine::builder()
            .with_progress_sink(sink.clone())
            .build();
        let result = engine.extract_batch(inputs, &config).await;
        drop(engine);
        drop(sink);
        finish_progress_forwarder(worker).await;
        result
            .map(crate::ExtractionResult::from)
            .map_err(crate::xberg_error_to_py_err)
    })
}

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use pyo3::types::PyModule;

    use super::*;

    #[tokio::test]
    async fn should_deliver_exact_progress_values_to_python() {
        Python::initialize();
        let (listener, module, extraction_thread) = Python::attach(|py| {
            let module = PyModule::from_code(
                py,
                c_str!(
                    r#"
import threading
events = []
class Listener:
    def on_progress(self, *args):
        events.append((args, threading.get_ident()))
listener = Listener()
"#
                ),
                c_str!("progress_test.py"),
                c_str!("progress_test"),
            )
            .expect("test Python module should compile");
            let listener = module.getattr("listener").expect("listener should exist").unbind();
            let extraction_thread = py
                .import("threading")
                .expect("threading should import")
                .call_method0("get_ident")
                .expect("thread identifier should be available")
                .extract::<u64>()
                .expect("thread identifier should be numeric");
            (listener, module.unbind(), extraction_thread)
        });
        let (sink, worker) = progress_forwarder(listener);

        sink.emit_ocr_page(3, 9, 4, "tesseract", Some(2));
        sink.emit_ocr_page(5, 9, 5, "paddleocr", Some(2));
        drop(sink);
        finish_progress_forwarder(worker).await;

        Python::attach(|py| {
            let events = module.bind(py).getattr("events").expect("events should exist");
            let first: ((String, usize, usize, usize, String, usize), u64) = events
                .get_item(0)
                .expect("first event should be recorded")
                .extract()
                .expect("first event should contain typed values");
            let second: ((String, usize, usize, usize, String, usize), u64) = events
                .get_item(1)
                .expect("second event should be recorded")
                .extract()
                .expect("second event should contain typed values");
            assert_eq!(first.0, ("ocr_page".to_string(), 3, 9, 4, "tesseract".to_string(), 2));
            assert_eq!(second.0, ("ocr_page".to_string(), 5, 9, 5, "paddleocr".to_string(), 2));
            assert_ne!(first.1, extraction_thread);
            assert_eq!(first.1, second.1);
            assert_eq!(events.len().expect("event count should be available"), 2);
        });
    }

    #[tokio::test]
    async fn should_ignore_python_callback_exception() {
        Python::initialize();
        let listener = Python::attach(|py| {
            let module = PyModule::from_code(
                py,
                c_str!(
                    r#"
class Listener:
    def on_progress(self, *args):
        raise RuntimeError("callback failed")
listener = Listener()
"#
                ),
                c_str!("progress_error_test.py"),
                c_str!("progress_error_test"),
            )
            .expect("test Python module should compile");
            module.getattr("listener").expect("listener should exist").unbind()
        });
        let (sink, worker) = progress_forwarder(listener);

        sink.emit_ocr_page(1, 1, 1, "tesseract", None);
        drop(sink);
        finish_progress_forwarder(worker).await;
    }

    #[test]
    fn should_reject_listener_without_progress_method() {
        Python::initialize();
        Python::attach(|py| {
            let listener = PyModule::from_code(
                py,
                c_str!("listener = object()"),
                c_str!("invalid_progress_test.py"),
                c_str!("invalid_progress_test"),
            )
            .expect("test Python module should compile")
            .getattr("listener")
            .expect("listener should exist")
            .unbind();

            let error = validate_listener(&listener).expect_err("listener should be rejected");
            assert!(error.is_instance_of::<PyAttributeError>(py));
        });
    }
}
