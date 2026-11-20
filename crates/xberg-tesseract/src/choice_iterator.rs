use crate::api::TesseractAPI;
use crate::error::{Result, TesseractError};
use crate::native_iterator::NativeIterator;
use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_int, c_void};

/// Owns a native choice iterator allocation, created by [`ResultIterator::choices`](crate::ResultIterator::choices).
///
/// It borrows the [`TesseractAPI`](crate::TesseractAPI) that produced the source iterator and
/// reads the same recognition result: once the engine replaces that result, every query returns
/// [`TesseractError::StaleIterator`]. Choice text is copied before the engine lock is released.
pub struct ChoiceIterator<'api> {
    inner: NativeIterator<'api>,
}

impl<'api> ChoiceIterator<'api> {
    /// Wraps a choice iterator created by `api` from the recognition result at `epoch`.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null `TessChoiceIterator` that `api`'s engine created from the result
    /// at `epoch`, owned by nothing else.
    pub(crate) unsafe fn from_raw(api: &'api TesseractAPI, epoch: u64, raw: *mut c_void) -> Self {
        // SAFETY: forwarded from this function's contract. ~keep
        ChoiceIterator {
            inner: unsafe { NativeIterator::new(api, epoch, raw) },
        }
    }

    /// Gets the next choice.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next choice is successful, otherwise returns `false`.
    pub fn next(&self) -> Result<bool> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes advancement on a live choice iterator whose source word is still valid. ~keep
        Ok(unsafe { TessChoiceIteratorNext(*handle) != 0 })
    }

    /// Gets the UTF-8 text for the current choice.
    ///
    /// # Returns
    ///
    /// Returns the UTF-8 text as a `String` if successful, otherwise returns an error.
    pub fn get_utf8_text(&self) -> Result<String> {
        let handle = self.inner.lock()?;
        // The lock guard proves the handle is live and its recognition result current. ~keep
        // SAFETY: The guard prevents advancement through this wrapper. The source word must remain valid.
        // The returned string borrows Tesseract's internal storage and requires no deallocation. ~keep
        let text_ptr = unsafe { TessChoiceIteratorGetUTF8Text(*handle) };
        if text_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: The non-null native string is NUL-terminated and copied before advancing or unlocking. ~keep
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the confidence of the current choice.
    ///
    /// # Returns
    ///
    /// Returns the confidence as a `f32`.
    pub fn confidence(&self) -> Result<f32> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the read of the current choice confidence, returned as a C float.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessChoiceIteratorConfidence(*handle) })
    }
}

impl Drop for ChoiceIterator<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.inner.lock_for_release() {
            // SAFETY: The wrapper must own the choice iterator allocation. This native deleter frees it. ~keep
            unsafe { TessChoiceIteratorDelete(*handle) };
        }
    }
}

ffi_extern! {
    fn TessChoiceIteratorDelete(handle: *mut c_void);
    fn TessChoiceIteratorNext(handle: *mut c_void) -> c_int;
    fn TessChoiceIteratorGetUTF8Text(handle: *mut c_void) -> *const c_char;
    fn TessChoiceIteratorConfidence(handle: *mut c_void) -> c_float;
}

#[cfg(test)]
mod native_signature_contract_tests {
    use super::*;

    #[test]
    fn choice_text_is_a_borrowed_const_pointer() {
        #[cfg(not(target_arch = "wasm32"))]
        type NativeFunction = unsafe extern "C-unwind" fn(*mut c_void) -> *const c_char;
        #[cfg(target_arch = "wasm32")]
        type NativeFunction = unsafe extern "C" fn(*mut c_void) -> *const c_char;

        let _: NativeFunction = super::TessChoiceIteratorGetUTF8Text;
    }
}
