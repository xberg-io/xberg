use crate::cancellation::CancellationToken;
use crate::{Result, XbergError};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Default)]
pub(crate) struct ExcelReadCancellation<'a> {
    pub(crate) request: Option<&'a CancellationToken>,
    pub(crate) abandoned: Option<&'a AtomicBool>,
}

impl ExcelReadCancellation<'_> {
    pub(super) fn check(self) -> Result<()> {
        if self.request.is_some_and(CancellationToken::is_cancelled)
            || self.abandoned.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(XbergError::Cancelled);
        }
        Ok(())
    }

    pub(super) fn start_sheet(self, index: usize) -> Result<()> {
        tracing::debug!(target: "xberg::excel::worksheet", sheet_index = index, "reading workbook worksheet");
        self.check()
    }
}
