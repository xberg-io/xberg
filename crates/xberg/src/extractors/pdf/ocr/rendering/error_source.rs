#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub(in crate::extractors::pdf::ocr) struct SourceImageBudgetExceeded(#[source] xberg_native_pdf::Error);

pub(super) fn render_error_source(
    error: xberg_native_pdf::Error,
    source_budget_exceeded: bool,
) -> Box<dyn std::error::Error + Send + Sync> {
    if source_budget_exceeded {
        Box::new(SourceImageBudgetExceeded(error))
    } else {
        Box::new(error)
    }
}
