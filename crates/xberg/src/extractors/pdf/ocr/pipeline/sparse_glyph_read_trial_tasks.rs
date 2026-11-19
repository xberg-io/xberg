use super::{MarkedWords, PipelineTrial, TrialPage};
use crate::core::config::ExtractionConfig;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

type PageTaskOutput = super::super::PageTaskOutput;

/// Indices of trial tasks that panicked. A task that panics yields a `JoinError` without its page,
/// and a panicked trial is retried at normal density like a declined one.
#[derive(Clone, Default)]
pub(in super::super) struct PanickedTrials(Arc<std::sync::Mutex<Vec<usize>>>);

impl PanickedTrials {
    pub(in super::super) fn take(&self) -> Option<usize> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pop()
    }
}

struct PanicGuard {
    index: usize,
    panicked: PanickedTrials,
    armed: bool,
}

impl Drop for PanicGuard {
    fn drop(&mut self) {
        if self.armed {
            self.panicked
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(self.index);
        }
    }
}

/// Spawn one trial per page into the batch's page task set.
pub(in super::super) struct TrialContext<'a> {
    pub(in super::super) pdf: &'a Arc<PdfDocument>,
    pub(in super::super) config: &'a ExtractionConfig,
    pub(in super::super) pipeline: &'a crate::core::config::OcrPipelineConfig,
}

pub(in super::super) fn spawn_trials<'a>(
    tasks: &mut tokio::task::JoinSet<PageTaskOutput>,
    panicked: &PanickedTrials,
    context: &TrialContext<'_>,
    pages: impl IntoIterator<Item = (usize, &'a MarkedWords)>,
    page: impl Fn(usize) -> TrialPage,
) {
    for (index, words) in pages {
        let trial = PipelineTrial {
            pdf: Arc::clone(context.pdf),
            index,
            config: context.config.clone(),
            pipeline: context.pipeline.clone(),
            page: page(index),
            words: words.clone(),
        };
        let mut guard = PanicGuard {
            index,
            panicked: panicked.clone(),
            armed: true,
        };
        let task = async move {
            let outcome = trial.run().await;
            guard.armed = false;
            PageTaskOutput::Trial(index, outcome)
        };
        #[cfg(test)]
        let task = crate::plugins::registry::inherit_test_ocr_backend(task);
        tasks.spawn(crate::core::extractor::cacheability::inherit(task));
    }
}
