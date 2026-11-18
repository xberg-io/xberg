use crate::cancellation::CancellationToken;
use crate::{Result, XbergError};
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Default)]
pub(crate) struct PptReadCancellation<'a> {
    pub(crate) request: Option<&'a CancellationToken>,
    pub(crate) abandoned: Option<&'a AtomicBool>,
}

impl<'a> PptReadCancellation<'a> {
    pub(crate) fn check(self) -> Result<()> {
        if self.request.is_some_and(CancellationToken::is_cancelled)
            || self.abandoned.is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(XbergError::Cancelled);
        }
        Ok(())
    }

    fn check_io(self) -> io::Result<()> {
        self.check().map_err(|_| io::Error::other("PPT read cancelled"))
    }

    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    pub(super) fn guard_reader<R: Read + io::Seek>(self, reader: R) -> CancellablePptReader<'a, R> {
        CancellablePptReader { reader, cancel: self }
    }

    pub(crate) fn read_into(self, mut reader: impl Read, output: &mut Vec<u8>) -> io::Result<()> {
        if self.request.is_none() && self.abandoned.is_none() {
            reader.read_to_end(output)?;
            return Ok(());
        }
        let mut buffer = [0; 16 * 1024];
        loop {
            self.check_io()?;
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(count) => output.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
pub(super) struct CancellablePptReader<'a, R> {
    reader: R,
    cancel: PptReadCancellation<'a>,
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
impl<R: Read> Read for CancellablePptReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancel.check_io()?;
        self.reader.read(buffer)
    }
}

#[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
impl<R: io::Seek> io::Seek for CancellablePptReader<'_, R> {
    fn seek(&mut self, position: io::SeekFrom) -> io::Result<u64> {
        self.cancel.check_io()?;
        self.reader.seek(position)
    }
}
