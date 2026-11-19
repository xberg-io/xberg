//! Ownership and lifecycle contracts of `HeifContext` and the handles derived from it.
//!
//! Decoding tests read a `test_documents/` fixture at runtime and skip with a greppable
//! message when the bucket-fetched corpus is absent (see `decode_smoke.rs`).

#![allow(clippy::print_stderr)] // ~keep: skip notices go to stderr by design

use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use xberg_libheif::{
    AuxiliaryImagesFilter, ColorSpace, HeifContext, HeifErrorCode, LibHeif, Reader, ReaderGrowStatus, RgbChroma,
};

/// Corpus root: `TEST_DOCUMENTS_DIR` when set (a worktree's `test_documents/` is an empty
/// submodule directory), otherwise the repository's `test_documents/`.
fn corpus_root() -> PathBuf {
    std::env::var_os("TEST_DOCUMENTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_documents"))
}

/// Read a corpus fixture. A missing fixture skips the test, or fails it when
/// `XBERG_REQUIRE_TEST_FIXTURES=1` (the strict policy of the shared Rust test helper).
fn load_fixture(relative: &str) -> Option<Vec<u8>> {
    let path = corpus_root().join(relative);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) => {
            assert!(
                std::env::var_os("XBERG_REQUIRE_TEST_FIXTURES").is_none_or(|value| value != "1"),
                "required fixture {} is unavailable: {e}",
                path.display()
            );
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

struct TrackedReader {
    data: Cursor<Vec<u8>>,
    reads: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}

impl Drop for TrackedReader {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl Reader for TrackedReader {
    fn position(&mut self) -> u64 {
        self.data.position()
    }

    #[allow(deprecated)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.data.read_exact(buf)
    }

    fn seek(&mut self, position: u64) -> io::Result<u64> {
        self.data.seek(SeekFrom::Start(position))
    }

    fn wait_for_file_size(&mut self, target_size: u64) -> ReaderGrowStatus {
        if target_size > self.data.get_ref().len() as u64 {
            ReaderGrowStatus::SizeBeyondEof
        } else {
            ReaderGrowStatus::SizeReached
        }
    }
}

/// The context is dropped while a handle lives: the reader must stay alive and keep serving
/// the handle's decode, and be released only with the last handle.
#[test]
fn the_reader_lives_until_the_last_handle_is_dropped() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let reads = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicBool::new(false));
    let reader = TrackedReader {
        data: Cursor::new(bytes),
        reads: Arc::clone(&reads),
        dropped: Arc::clone(&dropped),
    };
    let context = HeifContext::read_from_reader(Box::new(reader)).expect("read fixture through the reader");
    let handle = context.primary_image_handle().expect("primary image");
    drop(context);
    assert!(
        !dropped.load(Ordering::SeqCst),
        "the reader was released while a handle still uses it"
    );
    let reads_before_decode = reads.load(Ordering::SeqCst);
    assert!(decode_len(&handle) > 0);
    assert!(
        reads.load(Ordering::SeqCst) > reads_before_decode,
        "decoding through the handle did not read through the context's reader"
    );
    drop(handle);
    assert!(
        dropped.load(Ordering::SeqCst),
        "the reader leaked after the last handle was dropped"
    );
}
