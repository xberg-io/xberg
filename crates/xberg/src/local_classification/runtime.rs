use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::control::{WorkerControl, check_control, run_controlled, run_controlled_with_factory};
use super::policy::{invalid, model_identity, validate_config, validate_request};
use super::preparation::{PreparedModel, load_model, validate_cpu_runtime};
use super::{
    LocalClassificationConfig, LocalClassificationModelInfo, LocalClassificationOptions, LocalClassificationResult,
};

const MAX_CONCURRENT_REQUESTS: usize = 1;

struct ClassifierState {
    config: LocalClassificationConfig,
    pool: crate::model_cache::ModelCache<PreparedModel>,
    prepared_options: Mutex<Option<crate::onnx::OrtSessionOptions>>,
    semaphore: Arc<tokio::sync::Semaphore>,
}

/// Experimental Rust-only CPU classifier for trusted, pre-staged ONNX and tokenizer artifacts.
///
/// Checksums pin the input bytes and per-asset limits cap those bytes. They do not
/// sandbox the model or tokenizer, or bound their parsed or native runtime memory.
/// The async methods require a Tokio runtime with time enabled.
#[cfg_attr(alef, alef(skip))]
pub struct LocalTextClassifier {
    state: Arc<ClassifierState>,
}

impl LocalTextClassifier {
    /// Validate an experimental classifier configuration without resolving artifacts or loading a model.
    pub fn new(config: LocalClassificationConfig) -> crate::Result<Self> {
        validate_config(&config)?;
        Ok(Self {
            state: Arc::new(ClassifierState {
                config,
                pool: crate::model_cache::ModelCache::with_capacity(MAX_CONCURRENT_REQUESTS),
                prepared_options: Mutex::new(None),
                semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            }),
        })
    }

    /// Prepare the experimental CPU session from a pre-staged, checksum-pinned immutable cache.
    ///
    /// Neither preparation nor classification fetches missing assets from the network.
    /// Cancellation and deadlines are checked before and between load stages. An already
    /// running filesystem read, tokenizer parse or ONNX session build may finish before
    /// this method returns cancellation or timeout.
    pub async fn prepare(&self, options: &LocalClassificationOptions) -> crate::Result<LocalClassificationModelInfo> {
        let started = Instant::now();
        check_control(options, started)?;
        let state = Arc::clone(&self.state);
        let worker_options = options.clone();
        run_controlled(Arc::clone(&state.semaphore), options.clone(), started, move |request| {
            let control = WorkerControl {
                state: request,
                options: worker_options,
                started,
            };
            control.check()?;
            validate_cpu_runtime()?;
            let session_options = crate::onnx::ort_session_options();
            let identity = model_identity(&state.config);
            let model = state.pool.take_or_create_matching(
                |model| model.identity == identity && model.session_options == session_options,
                || load_model(&state.config, &control),
            )?;
            control.check()?;
            let info = model.info.clone();
            *state
                .prepared_options
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(model.session_options);
            Ok(info)
        })
        .await
    }

    /// Classify using an explicitly prepared local model without provider fallback.
    ///
    /// Cancellation asks ONNX Runtime to terminate active inference and waits for the
    /// blocking worker. A deadline does not guarantee a maximum wall-clock return time.
    pub async fn classify(
        &self,
        text: &str,
        labels: &[String],
        options: &LocalClassificationOptions,
    ) -> crate::Result<LocalClassificationResult> {
        let started = Instant::now();
        check_control(options, started)?;
        validate_request(text, labels, &self.state.config)?;
        if self
            .state
            .prepared_options
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
        {
            return Err(invalid(
                "Local classification requires explicit prepare before inference",
            ));
        }
        let state = Arc::clone(&self.state);
        run_controlled_with_factory(Arc::clone(&state.semaphore), options.clone(), started, || {
            // Copy bounded input only after admission, so queued futures do not own duplicate request buffers. ~keep
            let text = text.to_owned();
            let labels = labels.to_vec();
            let options = options.clone();
            move |request| {
                let control = WorkerControl {
                    state: request,
                    options,
                    started,
                };
                classify_admitted(&state, &text, &labels, &control)
            }
        })
        .await
    }
}

fn classify_admitted(
    state: &ClassifierState,
    text: &str,
    labels: &[String],
    control: &WorkerControl,
) -> crate::Result<LocalClassificationResult> {
    control.check()?;
    validate_cpu_runtime()?;
    let session_options = crate::onnx::ort_session_options();
    let prepared = *state
        .prepared_options
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if prepared != Some(session_options) {
        return Err(invalid(
            "ORT options changed; explicitly prepare the local NLI classifier again",
        ));
    }
    let identity = model_identity(&state.config);
    let mut model = state.pool.take_or_create_matching(
        |model| model.identity == identity && model.session_options == session_options,
        || Err(invalid("Local NLI session is not cached; explicitly prepare again")),
    )?;
    control.check()?;
    super::execution::classify_prepared(&mut model, &state.config, text, labels, control)
}
