use crate::cancellation::CancellationToken;
use crate::{Result, XbergError};
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Default)]
pub(crate) struct PptReadCancellation<'a> {
    pub(crate) request: Option<&'a CancellationToken>,
    pub(crate) abandoned: Option<&'a AtomicBool>,
}

impl PptReadCancellation<'_> {
    pub(crate) fn check(self) -> Result<()> {
        if self.request.is_some_and(CancellationToken::is_cancelled)
            || self.abandoned.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(XbergError::Cancelled);
        }
        Ok(())
    }

    pub(crate) fn read_into(self, mut reader: impl Read, output: &mut Vec<u8>) -> io::Result<()> {
        if self.request.is_none() && self.abandoned.is_none() {
            reader.read_to_end(output)?;
            return Ok(());
        }
        let mut buffer = [0; 16 * 1024];
        loop {
            self.check().map_err(|_| io::Error::other("PPT read cancelled"))?;
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}
