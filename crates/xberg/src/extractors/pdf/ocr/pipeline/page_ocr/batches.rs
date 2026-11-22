use super::super::*;
use super::*;

impl PageOcrContext<'_> {
    /// Render and read each batch of pages, post-processing every page it read, then hand the
    /// accumulated state to the assembly phase.
    pub(super) async fn run_batches(self) -> OcrRunResult<Self> {
        let total_pages = self.total_pages;
        let batch_size = self.batch_size;
        let mut this = self;
        for batch_start in (0..total_pages).step_by(batch_size) {
            let (next, rendered) = this.render_batch(batch_start).await?;
            let (next, mut reads) = next.read_batch(batch_start, &rendered).await?;
            this = next.process_batch(batch_start, &rendered, &mut reads).await?;
        }
        #[cfg(feature = "layout-detection")]
        if let Some(model) = this.tatr_model.take() {
            crate::layout::return_tatr(model);
        }
        Ok(this)
    }
}
