use super::{MarkedWords, PipelineTrial, TrialPage};
use crate::core::config::ExtractionConfig;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

type PageTaskOutput = super::super::PageTaskOutput;

/// A declined trial's normal-density retry render: the pages it rendered, the pages it could not,
/// the raster DPIs it observed, and the render warnings drained on the blocking worker that did
/// the rendering.
pub(in super::super) type RetryRenderOutcome = crate::Result<(
    Vec<(usize, image::DynamicImage)>,
    Vec<(usize, crate::XbergError)>,
    crate::pdf::render::OcrRasterDpis,
    Vec<crate::types::ProcessingWarning>,
)>;

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

/// The render a declined trial's page needs, captured so the task owns everything it reads.
pub(in super::super) struct RetryRenderRequest {
    pub(in super::super) pdf: Arc<PdfDocument>,
    pub(in super::super) index: usize,
    pub(in super::super) rotations: Vec<u32>,
    pub(in super::super) limits: crate::extractors::security::SecurityLimits,
    pub(in super::super) images_config: Option<crate::core::config::ImageExtractionConfig>,
    pub(in super::super) preprocessing_dpi: Option<i32>,
}

/// Spawn a declined trial's page render at normal density into the batch's page task set.
///
/// The render runs on the blocking pool from inside the task, never inline in the collector: a
/// render that stalls just before the extraction deadline must not stop the collector from
/// draining the reads that already finished or from honouring the stop request. `capture_render_warnings`
/// drains the warnings on the blocking worker that rendered, because the buffer is thread-local
/// and the collector would never see them.
pub(in super::super) fn spawn_retry_render(
    tasks: &mut tokio::task::JoinSet<PageTaskOutput>,
    request: RetryRenderRequest,
) {
    let RetryRenderRequest {
        pdf,
        index,
        rotations,
        limits,
        images_config,
        preprocessing_dpi,
    } = request;
    let task = async move {
        let rendered = super::super::super::super::spawn_pdf_blocking(move || {
            crate::pdf::render::capture_render_warnings(|| {
                Ok(super::super::super::rendering::render_selected_pages_with_image_config(
                    &pdf,
                    &rotations,
                    &[index],
                    &limits,
                    |_| images_config.as_ref(),
                    preprocessing_dpi,
                ))
            })
        })
        .await;
        PageTaskOutput::RetryRender(
            index,
            Box::new(rendered.map(|(render, warnings)| (render.0, render.1, render.2, warnings))),
        )
    };
    tasks.spawn(crate::core::extractor::cacheability::inherit(task));
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
