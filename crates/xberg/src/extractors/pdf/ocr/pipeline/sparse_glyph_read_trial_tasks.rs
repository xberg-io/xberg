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

/// Runs `work` while `guard` lives; the guard records its page unless `work` completes. The guard is
/// moved in whole: a bare `guard.armed = false` in a `move` block captures only that field and
/// leaves the guard to drop armed at the end of the spawning scope.
async fn guarded<T>(guard: PanicGuard, work: impl std::future::Future<Output = T>) -> T {
    let mut guard = guard;
    let output = work.await;
    guard.armed = false;
    output
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
        let guard = PanicGuard {
            index,
            panicked: panicked.clone(),
            armed: true,
        };
        let task = guarded(guard, async move { PageTaskOutput::Trial(index, trial.run().await) });
        #[cfg(test)]
        let task = crate::plugins::registry::inherit_test_ocr_backend(task);
        tasks.spawn(crate::core::extractor::cacheability::inherit(task));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn run(panicked: &PanickedTrials, index: usize, fail: bool) {
        let guard = PanicGuard {
            index,
            panicked: panicked.clone(),
            armed: true,
        };
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(guarded(guard, async move {
            assert!(!fail, "intentional trial panic");
        }));
        assert_eq!(tasks.join_next().await.unwrap().is_err(), fail);
    }

    #[tokio::test]
    async fn only_a_panicking_task_records_its_page() {
        let panicked = PanickedTrials::default();
        run(&panicked, 3, false).await;
        assert_eq!(panicked.take(), None, "a completed trial must not be recorded");
        run(&panicked, 5, true).await;
        assert_eq!(panicked.take(), Some(5));
        assert_eq!(panicked.take(), None);
    }
}
