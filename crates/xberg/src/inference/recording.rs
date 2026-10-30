use std::sync::{Arc, Mutex};

use super::backend::InferenceError;
use super::{InferenceSession, InferenceTensor};

pub(crate) struct ForwardCapture {
    pub(crate) inputs: Vec<(String, InferenceTensor)>,
    pub(crate) outputs: Vec<(String, InferenceTensor)>,
}

pub(crate) type Captures = Arc<Mutex<Vec<ForwardCapture>>>;

struct RecordingSession {
    inner: Box<dyn InferenceSession>,
    captures: Captures,
}

pub(crate) fn record_session(inner: Box<dyn InferenceSession>) -> (Box<dyn InferenceSession>, Captures) {
    let captures = Arc::new(Mutex::new(Vec::new()));
    let session = RecordingSession {
        inner,
        captures: Arc::clone(&captures),
    };
    (Box::new(session), captures)
}

impl InferenceSession for RecordingSession {
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        let recorded_inputs = inputs.clone();
        let outputs = self.inner.run(inputs)?;
        self.captures.lock().expect("capture mutex").push(ForwardCapture {
            inputs: recorded_inputs,
            outputs: outputs.clone(),
        });
        Ok(outputs)
    }

    fn input_names(&self) -> &[String] {
        self.inner.input_names()
    }
}

thread_local! {
    static PROFILE_PREFIX: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn profile_prefix() -> Option<std::path::PathBuf> {
    PROFILE_PREFIX.with(|path| path.borrow().clone())
}

pub(crate) fn with_profile<T>(path: &std::path::Path, create: impl FnOnce() -> T) -> T {
    struct ClearProfile;
    impl Drop for ClearProfile {
        fn drop(&mut self) {
            PROFILE_PREFIX.with(|path| *path.borrow_mut() = None);
        }
    }
    PROFILE_PREFIX.with(|slot| {
        assert!(slot.borrow().is_none(), "profiling scopes cannot nest");
        *slot.borrow_mut() = Some(path.to_path_buf());
    });
    let _guard = ClearProfile;
    create()
}
