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

    pub(super) fn guard_reader<R: std::io::Read>(self, reader: R) -> impl std::io::Read {
        CancelledRead { reader, cancel: self }
    }

    pub(super) fn read_to_end(self, mut reader: impl std::io::Read) -> Result<Vec<u8>> {
        let mut contents = Vec::new();
        if self.request.is_none() && self.abandoned.is_none() {
            reader.read_to_end(&mut contents)?;
            return Ok(contents);
        }
        let mut chunk = [0; 16 * 1024];
        loop {
            self.check()?;
            let count = match reader.read(&mut chunk) {
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            if count == 0 {
                return Ok(contents);
            }
            contents.extend_from_slice(&chunk[..count]);
        }
    }

    pub(super) fn start_sheet(self, index: usize) -> Result<()> {
        tracing::debug!(target: "xberg::excel::worksheet", sheet_index = index, "reading workbook worksheet");
        self.check()
    }
}

struct CancelledRead<'a, R> {
    reader: R,
    cancel: ExcelReadCancellation<'a>,
}

impl<R: std::io::Read> std::io::Read for CancelledRead<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.cancel.check().map_err(std::io::Error::other)?;
        self.reader.read(buffer)
    }
}
