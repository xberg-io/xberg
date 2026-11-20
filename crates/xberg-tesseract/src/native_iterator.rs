use crate::api::TesseractAPI;
use crate::enums::TessPageIteratorLevel;
use crate::error::{Result, TesseractError};
use std::ops::Deref;
use std::os::raw::{c_int, c_void};
use std::sync::{Mutex, MutexGuard};

/// A native iterator allocation tied to the engine that produced it and to one recognition
/// result of that engine.
///
/// The `'api` borrow keeps the engine alive for as long as the iterator exists, so the
/// native allocation never outlives the `TessBaseAPI` that owns the `PAGE_RES` it reads.
/// Every query goes through [`Self::lock`], which holds the engine's own lock and refuses to
/// run once Init, SetImage, Recognize, Clear, End, DetectOS, layout analysis or any other
/// operation that replaces the result has bumped the engine's result epoch. Because the
/// engine lock is held for the whole native call, no such operation can start while a query
/// reads the result.
pub(crate) struct NativeIterator<'api> {
    api: &'api TesseractAPI,
    epoch: u64,
    raw: Mutex<*mut c_void>,
}

// SAFETY: The wrapper owns the native allocation and its deletion obligation. All native access
// goes through `lock`, which serializes on the iterator mutex and the engine mutex, so moving
// the wrapper to another thread does not by itself touch native data. ~keep
unsafe impl Send for NativeIterator<'_> {}
// SAFETY: Shared access is serialized by the iterator mutex and the engine mutex taken in `lock`. ~keep
unsafe impl Sync for NativeIterator<'_> {}

/// Exclusive access to a live iterator for the duration of one native call.
///
/// Dereferences to the raw iterator pointer. The engine lock held alongside it keeps the
/// recognition result the iterator reads from being replaced.
pub(crate) struct IteratorGuard<'a> {
    raw: MutexGuard<'a, *mut c_void>,
    _engine: MutexGuard<'a, *mut c_void>,
}

impl Deref for IteratorGuard<'_> {
    type Target = *mut c_void;

    fn deref(&self) -> &*mut c_void {
        &self.raw
    }
}

impl<'api> NativeIterator<'api> {
    /// Wraps a freshly created native iterator.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null iterator allocation created by `api`'s engine from the
    /// recognition result identified by `epoch`, owned by nothing else, and released only by
    /// the wrapper's owner exactly once through the matching native deleter.
    pub(crate) unsafe fn new(api: &'api TesseractAPI, epoch: u64, raw: *mut c_void) -> Self {
        NativeIterator {
            api,
            epoch,
            raw: Mutex::new(raw),
        }
    }

    pub(crate) fn api(&self) -> &'api TesseractAPI {
        self.api
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Locks the iterator and the engine, failing with [`TesseractError::StaleIterator`] when the
    /// engine's recognition result changed after the iterator was created.
    pub(crate) fn lock(&self) -> Result<IteratorGuard<'_>> {
        let raw = self.raw.lock().map_err(|_| TesseractError::MutexLockError)?;
        let engine = self.api.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        if self.api.results_epoch() != self.epoch {
            return Err(TesseractError::StaleIterator);
        }
        Ok(IteratorGuard { raw, _engine: engine })
    }

    /// Locks only the iterator pointer, for releasing it. Deleting an iterator reads no
    /// recognition result, so a stale epoch does not matter here.
    pub(crate) fn lock_for_release(&self) -> Option<MutexGuard<'_, *mut c_void>> {
        self.raw.lock().ok()
    }
}

impl IteratorGuard<'_> {
    /// Fails when the iterator holds no current word.
    ///
    /// Tesseract's choice iterator asserts that the current word is non-null and ends the process
    /// before a caller can reject a null result, so a caller that needs a word checks here first.
    pub(crate) fn require_current_word(&self) -> Result<()> {
        let (mut left, mut top, mut right, mut bottom) = (0, 0, 0, 0);
        // `PageIterator::Empty(RIL_WORD)` holds exactly when there is no word, and `BoundingBox`
        // reports it by returning false; the outputs are writable C ints. ~keep
        // SAFETY: the guard keeps the iterator and its recognition result current. ~keep
        let found = unsafe {
            TessPageIteratorBoundingBox(
                *self.raw,
                TessPageIteratorLevel::RIL_WORD as c_int,
                &mut left,
                &mut top,
                &mut right,
                &mut bottom,
            )
        };
        if found == 0 {
            return Err(TesseractError::InvalidParameterError);
        }
        Ok(())
    }
}

ffi_extern! {
    fn TessPageIteratorBoundingBox(
        handle: *mut c_void,
        level: c_int,
        left: *mut c_int,
        top: *mut c_int,
        right: *mut c_int,
        bottom: *mut c_int,
    ) -> c_int;
}
