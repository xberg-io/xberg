//! Process-wide ONNX Runtime session options shared by every ORT-backed feature.

use serde::{Deserialize, Serialize};

/// Process-wide memory and threading options applied to the ONNX Runtime sessions of
/// embeddings, reranking, sparse embeddings, late-interaction engines and PaddleOCR.
///
/// The pure-Rust tract PaddleOCR backend ignores these options.
///
/// PaddleOCR applies `memory_pattern` and `cpu_arena` only; its intra-op thread count
/// is not changed by `max_threads`.
///
/// Other ORT users (layout detection, SLANet, TATR, the inference backend, Whisper,
/// GLiNER) build their own sessions and are not affected.
///
/// The defaults reproduce the historical behaviour: ORT's memory-pattern planner and
/// CPU arena stay enabled and the intra-op thread count follows the default thread
/// budget. Both ORT features retain allocations sized by the largest batch seen, which
/// is what makes resident memory grow with variable-length embedding batches and never
/// shrink. Turning them off trades some throughput for a bounded footprint.
///
/// Options are read when a session is built. [`set_ort_session_options`] clears the
/// engine caches when the options change, so resident engines are rebuilt on next use;
/// a caller still holding an engine keeps its old session until it drops it. PaddleOCR
/// engines are pooled per option set, so a changed option set builds new engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OrtSessionOptions {
    /// Enable ORT's memory-pattern optimization. Disable for variable input sizes.
    /// Default `true`.
    pub memory_pattern: bool,
    /// Enable the CPU execution provider's arena allocator. Disable so freed tensors
    /// return to the system allocator instead of being retained. Applies to the CPU
    /// execution provider only; CUDA/TensorRT device arenas are unaffected. Default `true`.
    pub cpu_arena: bool,
    /// Intra-op thread count. `None` (default) uses the standard thread budget
    /// (`min(cores, 8)` or the cgroup quota). Values below 1 are raised to 1. Only the
    /// ORT intra-op pool is changed; `EMBED_SEMAPHORE` and other `resolve_thread_budget`
    /// callers are unaffected. ONNX Runtime builds that use OpenMP ignore this setting;
    /// configure `OMP_NUM_THREADS` for those builds.
    pub max_threads: Option<usize>,
}

impl OrtSessionOptions {
    pub(crate) const DEFAULT: Self = Self {
        memory_pattern: true,
        cpu_arena: true,
        max_threads: None,
    };
}

impl Default for OrtSessionOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

static SESSION_OPTIONS: std::sync::RwLock<OrtSessionOptions> = std::sync::RwLock::new(OrtSessionOptions::DEFAULT);

/// Set the [`OrtSessionOptions`] used for ONNX sessions built from now on.
///
/// When the options actually change, the embedding, reranking, sparse-embedding and
/// late-interaction engine caches are cleared so resident engines are rebuilt with the
/// new options on next use. PaddleOCR engines already resident stay in their pool under
/// the previous options and are no longer selected.
pub fn set_ort_session_options(options: OrtSessionOptions) {
    let changed = {
        let mut guard = SESSION_OPTIONS.write().unwrap_or_else(|e| e.into_inner());
        let changed = *guard != options;
        *guard = options;
        changed
    };
    if changed {
        #[cfg(feature = "onnx-runtime")]
        crate::clear_onnx_engine_caches();
        #[cfg(feature = "paddle-ocr-ort")]
        crate::paddle_ocr::clear_paddle_engine_caches();
    }
}

/// The [`OrtSessionOptions`] currently in effect.
pub fn ort_session_options() -> OrtSessionOptions {
    *SESSION_OPTIONS.read().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RestoreOptions(OrtSessionOptions);
    impl Drop for RestoreOptions {
        fn drop(&mut self) {
            set_ort_session_options(self.0);
        }
    }

    #[test]
    #[serial_test::serial]
    fn set_ort_session_options_round_trips() {
        let _restore = RestoreOptions(ort_session_options());
        let bounded = OrtSessionOptions {
            memory_pattern: false,
            cpu_arena: false,
            max_threads: Some(3),
        };
        set_ort_session_options(bounded);
        assert_eq!(ort_session_options(), bounded);
    }
}
