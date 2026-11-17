use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::sync::{Arc, Mutex};

/// Owns a native choice iterator allocation.
///
/// # Safety
///
/// Queries require a live, uniquely owned `TessChoiceIterator` and the source word's
/// recognition data, including its `WERD_RES` in the API-owned `PAGE_RES`. Choice text
/// borrows from that native data. The originating `TessBaseAPI` must remain live and
/// must not run Init, SetImage, Recognize, Clear, End, DetectOS or any operation that
/// replaces or invalidates those results while the iterator is in use.
///
/// The raw constructor does not validate ownership, type or backing lifetime.
/// The mutex only serializes calls through this wrapper. These requirements are not enforced
/// by the API. See [#3228](https://github.com/xberg-io/xberg/issues/3228).
pub struct ChoiceIterator {
    handle: Arc<Mutex<*mut c_void>>,
}

// SAFETY: Moving the wrapper transfers the handle and its deletion obligation without accessing native data.
// Subsequent use or deletion still depends on the ownership and coordination in ChoiceIterator's Safety section (#3228). ~keep
unsafe impl Send for ChoiceIterator {}
// SAFETY: Shared methods serialize native access through this wrapper's mutex. Access through
// other wrappers or the API must be externally coordinated as in ChoiceIterator's Safety section (#3228). ~keep
unsafe impl Sync for ChoiceIterator {}

impl ChoiceIterator {
    /// Creates a new instance of the ChoiceIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the ChoiceIterator.
    pub fn new(handle: *mut c_void) -> Self {
        ChoiceIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the next choice.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next choice is successful, otherwise returns `false`.
    pub fn next(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: The guard serializes advancement on a live choice iterator whose source word is still valid. ~keep
        Ok(unsafe { TessChoiceIteratorNext(*handle) != 0 })
    }

    /// Gets the UTF-8 text for the current choice.
    ///
    /// # Returns
    ///
    /// Returns the UTF-8 text as a `String` if successful, otherwise returns an error.
    pub fn get_utf8_text(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // Requires the live handle and backing data documented on `ChoiceIterator`. ~keep
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
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: The guard serializes the read of the current choice confidence, returned as a C float.
        // Requires the live handle and backing data documented on `ChoiceIterator`. ~keep
        Ok(unsafe { TessChoiceIteratorConfidence(*handle) })
    }
}

impl Drop for ChoiceIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
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
