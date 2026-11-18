use super::*;
use crate::cancellation::CancellationToken;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};

const DECK: &[u8] = include_bytes!("../../../../tests/fixtures/ppt/embedded_word_object.ppt");
const FIXED_HEADER_BYTES: u64 = 512;

struct DirectoryRead<'a> {
    inner: Cursor<&'a [u8]>,
    stop: Box<dyn Fn() + 'a>,
    reached_sectors: bool,
    accesses_after_stop: usize,
}

impl Read for DirectoryRead<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.reached_sectors {
            self.accesses_after_stop += 1;
        }
        let position = self.inner.position();
        let count = self.inner.read(output)?;
        if !self.reached_sectors && position >= FIXED_HEADER_BYTES && count > 0 {
            self.reached_sectors = true;
            (self.stop)();
        }
        Ok(count)
    }
}

impl Seek for DirectoryRead<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.reached_sectors {
            self.accesses_after_stop += 1;
        }
        self.inner.seek(position)
    }
}

#[test]
fn native_ppt_cfb_scan_cancels_after_entering_allocation_sectors() {
    let mut unguarded = DirectoryRead {
        inner: Cursor::new(DECK),
        stop: Box::new(|| {}),
        reached_sectors: false,
        accesses_after_stop: 0,
    };
    cfb::CompoundFile::open(&mut unguarded).expect("valid signed fixture container");
    assert!(unguarded.reached_sectors);
    assert!(unguarded.accesses_after_stop > 0, "unguarded CFB must keep scanning");

    for drop_request in [false, true] {
        let token = CancellationToken::new();
        let abandoned = AtomicBool::new(false);
        let mut source = DirectoryRead {
            inner: Cursor::new(DECK),
            stop: Box::new(|| {
                if drop_request {
                    abandoned.store(true, Ordering::Relaxed);
                } else {
                    token.cancel();
                }
            }),
            reached_sectors: false,
            accesses_after_stop: 0,
        };
        let cancel = PptReadCancellation {
            request: Some(&token),
            abandoned: Some(&abandoned),
        };
        assert!(matches!(
            PptFileStreams::read_from(&mut source, false, cancel),
            Err(XbergError::Cancelled)
        ));
        assert!(
            source.reached_sectors,
            "the cancellation must happen inside the real CFB scan"
        );
        assert_eq!(source.accesses_after_stop, 0);
    }

    assert!(matches!(
        PptFileStreams::read_from(Cursor::new(b"not a CFB"), false, PptReadCancellation::default()),
        Err(XbergError::Parsing { .. })
    ));
}
