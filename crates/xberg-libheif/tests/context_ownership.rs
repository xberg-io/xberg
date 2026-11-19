//! Ownership and lifecycle contracts of `HeifContext` and the handles derived from it.
//!
//! Decoding tests read a `test_documents/` fixture at runtime and skip with a greppable
//! message when the bucket-fetched corpus is absent (see `decode_smoke.rs`).

#![allow(clippy::print_stderr)] // ~keep: skip notices go to stderr by design

use std::io::{self, Cursor};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use xberg_libheif::{
    AuxiliaryImagesFilter, ColorSpace, HeifContext, HeifErrorCode, LibHeif, Reader, ReaderGrowStatus, RgbChroma,
    StreamReader,
};

fn load_fixture(relative: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents")
        .join(relative);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) => {
            eprintln!(
                "SKIP: fixture {} not available ({e}); run `python3 test_documents/scripts/fetch_corpus.py` to fetch it",
                path.display()
            );
            None
        }
    }
}

fn decode_len(handle: &xberg_libheif::ImageHandle<'_>) -> usize {
    let image = LibHeif::new()
        .decode(handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .expect("decode through the handle");
    image.planes().interleaved.expect("interleaved RGBA plane").data.len()
}

#[test]
fn a_handle_decodes_after_its_context_is_dropped() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let context = HeifContext::read_from_bytes(&bytes).expect("read fixture");
    let handle = context.primary_image_handle().expect("primary image");
    drop(context);
    assert!(handle.width() > 0 && handle.height() > 0);
    assert!(decode_len(&handle) >= (handle.width() as usize) * (handle.height() as usize) * 4);
}

#[test]
fn a_derived_handle_outlives_its_parent_handle_and_context() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let context = HeifContext::read_from_bytes(&bytes).expect("read fixture");
    let handles = context.top_level_image_handles();
    assert!(!handles.is_empty());
    let auxiliary = handles[0].auxiliary_images(AuxiliaryImagesFilter::new());
    drop(handles);
    drop(context);
    for handle in &auxiliary {
        assert!(handle.width() > 0);
    }
}

#[test]
fn a_reader_backed_context_decodes_after_the_context_is_dropped() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let size = bytes.len() as u64;
    let context = HeifContext::read_from_reader(Box::new(StreamReader::new(Cursor::new(bytes), size)))
        .expect("read fixture through a reader");
    let handle = context.primary_image_handle().expect("primary image");
    drop(context);
    assert!(decode_len(&handle) > 0);
}

#[test]
fn a_context_with_live_handles_refuses_a_new_source() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let mut context = HeifContext::read_from_bytes(&bytes).expect("read fixture");
    let handle = context.primary_image_handle().expect("primary image");
    let error = context
        .read_bytes(&bytes)
        .expect_err("a live handle must block re-reading");
    assert!(matches!(error.code, HeifErrorCode::UsageError));
    drop(handle);
    context.read_bytes(&bytes).expect("re-read once the handle is gone");
}

#[test]
fn a_handle_from_another_context_is_rejected() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let first = HeifContext::read_from_bytes(&bytes).expect("read fixture");
    let mut second = HeifContext::read_from_bytes(&bytes).expect("read fixture");
    let foreign = first.primary_image_handle().expect("primary image");
    let error = second
        .add_exif_metadata(&foreign, b"exif")
        .expect_err("a foreign handle must be refused");
    assert!(matches!(error.code, HeifErrorCode::UsageError));
}

struct CountedFailingReader(Arc<AtomicUsize>);

impl Drop for CountedFailingReader {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl Reader for CountedFailingReader {
    fn position(&mut self) -> u64 {
        0
    }

    #[allow(deprecated)]
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::UnexpectedEof.into())
    }

    fn seek(&mut self, position: u64) -> io::Result<u64> {
        Ok(position)
    }

    fn wait_for_file_size(&mut self, _target_size: u64) -> ReaderGrowStatus {
        ReaderGrowStatus::SizeBeyondEof
    }
}

#[test]
fn a_failing_reader_is_freed_once_when_the_read_fails() {
    let drops = Arc::new(AtomicUsize::new(0));
    let result = HeifContext::read_from_reader(Box::new(CountedFailingReader(Arc::clone(&drops))));
    assert!(result.is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
