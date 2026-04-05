use crate::api::TessDeleteText;
use crate::enums::TessPageIteratorLevel;
use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::sync::{Arc, Mutex};

/// Font attributes detected by Tesseract for a word.
#[derive(Debug, Clone)]
pub struct FontAttributes {
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub is_monospace: bool,
    pub is_serif: bool,
    pub is_smallcaps: bool,
    pub pointsize: i32,
    pub font_id: i32,
}

/// Complete word data extracted in a single mutex lock.
#[derive(Debug, Clone)]
pub struct WordData {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub confidence: f32,
    pub font_attrs: Option<FontAttributes>,
}

pub struct ResultIterator {
    pub handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for ResultIterator {}
unsafe impl Sync for ResultIterator {}

impl ResultIterator {
    /// Creates a new instance of the ResultIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the ResultIterator.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the ResultIterator.
    pub fn new(handle: *mut c_void) -> Self {
        ResultIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the UTF-8 text of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the text.
    ///
    /// # Returns
    ///
    /// Returns the UTF-8 text as a `String` if successful, otherwise returns an error.
    pub fn get_utf8_text(&self, level: TessPageIteratorLevel) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorGetUTF8Text() allocates and returns a pointer to a C string.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator (mutex-guarded)
        // 2. level is a valid TessPageIteratorLevel enum converted to c_int (in valid range)
        // 3. The returned pointer is either null (error) or a valid null-terminated C string
        //    allocated on Tesseract's heap (must be freed with TessDeleteText)
        let text_ptr = unsafe { TessResultIteratorGetUTF8Text(*handle, level as c_int) };
        if text_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: We've verified text_ptr is non-null. The allocation/deallocation pattern is:
        // 1. text_ptr was allocated by TessResultIteratorGetUTF8Text() on the FFI boundary
        // 2. CStr::from_ptr(text_ptr) is safe: pointer is non-null and points to valid C string
        // 3. We read from the string (to_str() creates temporary immutable borrow)
        // 4. We immediately copy all data to owned String before deallocation
        // 5. The string data remains valid until TessDeleteText is called
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() deallocates memory allocated by TessResultIteratorGetUTF8Text():
        // 1. text_ptr must be non-null (verified above)
        // 2. text_ptr came from the Tesseract API (trusted source, correct allocation)
        // 3. TessDeleteText() is the correct deallocation function for this allocation
        // 4. Must be called exactly once per allocation to avoid double-free (we ensure this)
        // 5. After this call, text_ptr is invalid; all uses must be via owned result String
        unsafe { TessDeleteText(text_ptr as *mut c_char) };
        Ok(result)
    }

    /// Gets the confidence of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the confidence.
    ///
    /// # Returns
    ///
    /// Returns the confidence as a `f32`.
    pub fn confidence(&self, level: TessPageIteratorLevel) -> Result<f32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorConfidence() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. level is a valid TessPageIteratorLevel enum converted to c_int
        // 3. The function only reads state and returns an f32 value (copyable)
        // 4. No pointer operations or memory access is needed
        Ok(unsafe { TessResultIteratorConfidence(*handle, level as c_int) })
    }

    /// Gets the recognition language of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the recognition language as a `String` if successful, otherwise returns an error.
    pub fn word_recognition_language(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorWordRecognitionLanguage() returns a pointer to a C string
        // in the iterator's memory. This is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The returned pointer is either null or a valid null-terminated C string
        let lang_ptr = unsafe { TessResultIteratorWordRecognitionLanguage(*handle) };
        if lang_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: We've verified lang_ptr is non-null. CStr::from_ptr() is safe because:
        // 1. lang_ptr points to a valid null-terminated C string managed by Tesseract
        // 2. We only read from it (to_str() creates temporary borrow)
        let c_str = unsafe { CStr::from_ptr(lang_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the font attributes of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the font attributes as a tuple if successful, otherwise returns an error.
    pub fn word_font_attributes(&self) -> Result<(bool, bool, bool, bool, bool, bool, i32, i32)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut pointsize = 0;
        let mut font_id = 0;

        // SAFETY: TessResultIteratorWordFontAttributes() takes output parameter pointers
        // and fills them with font attribute values. This is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator (mutex-guarded)
        // 2. All mutable references (&mut ...) are valid local stack variables
        // 3. Each reference has a distinct memory location (no aliasing)
        // 4. The references outlive the FFI call (defined on stack, used immediately after)
        // 5. The function writes output i32 values (0/1 for bools, integers for size/id)
        // 6. Each reference has exclusive mutable access (Rust borrow checker enforces this)
        // 7. The output parameters are independent (function cannot cause data races)
        let result = unsafe {
            TessResultIteratorWordFontAttributes(
                *handle,
                &mut is_bold,
                &mut is_italic,
                &mut is_underlined,
                &mut is_monospace,
                &mut is_serif,
                &mut is_smallcaps,
                &mut pointsize,
                &mut font_id,
            )
        };

        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((
                is_bold != 0,
                is_italic != 0,
                is_underlined != 0,
                is_monospace != 0,
                is_serif != 0,
                is_smallcaps != 0,
                pointsize,
                font_id,
            ))
        }
    }

    /// Checks if the current iterator is from the dictionary.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is from the dictionary, otherwise returns `false`.
    pub fn word_is_from_dictionary(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorWordIsFromDictionary() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The function only reads state and returns an i32 value (0 or non-zero)
        // 3. No pointer operations or memory modifications are needed
        Ok(unsafe { TessResultIteratorWordIsFromDictionary(*handle) != 0 })
    }

    /// Checks if the current iterator is numeric.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is numeric, otherwise returns `false`.
    pub fn word_is_numeric(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorWordIsNumeric() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations or state modifications needed
        Ok(unsafe { TessResultIteratorWordIsNumeric(*handle) != 0 })
    }

    /// Checks if the current iterator is superscript.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is superscript, otherwise returns `false`.
    pub fn symbol_is_superscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorSymbolIsSuperscript() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations or state modifications needed
        Ok(unsafe { TessResultIteratorSymbolIsSuperscript(*handle) != 0 })
    }

    /// Checks if the current iterator is subscript.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is subscript, otherwise returns `false`.
    pub fn symbol_is_subscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorSymbolIsSubscript() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations or state modifications needed
        Ok(unsafe { TessResultIteratorSymbolIsSubscript(*handle) != 0 })
    }

    /// Checks if the current iterator is dropcap.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is dropcap, otherwise returns `false`.
    pub fn symbol_is_dropcap(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorSymbolIsDropcap() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations or state modifications needed
        Ok(unsafe { TessResultIteratorSymbolIsDropcap(*handle) != 0 })
    }

    /// Moves to the next iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the next iterator.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next iterator exists, otherwise returns `false`.
    pub fn next(&self, level: TessPageIteratorLevel) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessResultIteratorNext() is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator
        // 2. level is a valid TessPageIteratorLevel enum converted to c_int
        // 3. The function modifies iterator state (advances position) and returns i32 result
        // 4. The mutex ensures exclusive access during state modification
        Ok(unsafe { TessResultIteratorNext(*handle, level as c_int) != 0 })
    }

    /// Gets the current word from the iterator with its bounding box and confidence.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (text, left, top, right, bottom, confidence) if successful
    pub fn get_word_with_bounds(&self) -> Result<(String, i32, i32, i32, i32, f32)> {
        let text = self.get_utf8_text(TessPageIteratorLevel::RIL_WORD)?;
        let (left, top, right, bottom) = self.get_bounding_box(TessPageIteratorLevel::RIL_WORD)?;
        let confidence = self.confidence(TessPageIteratorLevel::RIL_WORD)?;

        Ok((text, left, top, right, bottom, confidence))
    }

    /// Advances the iterator to the next word.
    ///
    /// # Returns
    ///
    /// Returns true if successful, false if there are no more words
    pub fn next_word(&self) -> Result<bool> {
        self.next(TessPageIteratorLevel::RIL_WORD)
    }

    /// Gets the word information for the current position in the iterator.
    /// Should be called before next() to ensure valid data.
    ///
    /// # Returns
    /// Returns a tuple of (text, left, top, right, bottom, confidence) if successful
    pub fn get_current_word(&self) -> Result<(String, i32, i32, i32, i32, f32)> {
        let text = self.get_utf8_text(TessPageIteratorLevel::RIL_WORD)?;
        let (left, top, right, bottom) = self.get_bounding_box(TessPageIteratorLevel::RIL_WORD)?;
        let confidence = self.confidence(TessPageIteratorLevel::RIL_WORD)?;

        Ok((text, left, top, right, bottom, confidence))
    }

    /// Gets the bounding box for the current element.
    pub fn get_bounding_box(&self, level: TessPageIteratorLevel) -> Result<(i32, i32, i32, i32)> {
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;

        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        // SAFETY: TessPageIteratorBoundingBox() queries iterator state and returns coordinates
        // via output parameters. This is safe because:
        // 1. *handle is a valid pointer to an initialized ResultIterator or PageIterator (mutex-guarded)
        // 2. level is a valid TessPageIteratorLevel enum converted to c_int (in valid range)
        // 3. All mutable references (&mut left, &mut top, &mut right, &mut bottom)
        //    are valid local stack variables with distinct memory locations
        // 4. Each reference is exclusively borrowed (Rust enforces no aliasing)
        // 5. The references outlive the FFI call (defined on stack, used immediately after)
        // 6. The function writes four i32 coordinate values into these references
        // 7. No pointer escaping: the function only writes to these parameters, doesn't store them
        // 8. Return value indicates success/failure (checked below)
        let result = unsafe {
            TessPageIteratorBoundingBox(*handle, level as c_int, &mut left, &mut top, &mut right, &mut bottom)
        };

        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((left, top, right, bottom))
        }
    }

    /// Extracts all word data from the iterator in a single mutex lock.
    ///
    /// Acquires the mutex once and iterates all words, collecting text, bounding box,
    /// confidence, and font attributes for each word. This is more efficient than
    /// calling individual methods in a loop since it avoids repeated mutex acquisitions.
    ///
    /// The iterator is always reset to the beginning before traversal so that partial
    /// prior consumption does not cause words to be missed.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<WordData>` containing data for every word, or an error if the
    /// mutex cannot be acquired.
    pub fn extract_all_words(&self) -> Result<Vec<WordData>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let raw = *handle;
        let mut words = Vec::new();

        // Reset to the first element before traversal.  ResultIterator inherits from
        // PageIterator in C++, so TessPageIteratorBegin operates on the same handle.
        // SAFETY: raw is a valid mutex-guarded ResultIterator pointer; TessPageIteratorBegin
        // simply resets the internal position and does not allocate or free memory.
        unsafe { TessPageIteratorBegin(raw) };

        loop {
            // SAFETY: raw is the mutex-guarded *mut c_void handle. All calls within this
            // loop are performed while holding the mutex lock, ensuring exclusive access.
            // We pass raw directly to the unlocked helper to avoid re-locking.
            match extract_word_data_unlocked(raw) {
                Ok(word) => words.push(word),
                // NullPointerError means the text pointer was null; skip this position.
                // InvalidParameterError means bounding box failed; skip this position.
                // Utf8Error means the text was not valid UTF-8; skip this word rather than
                // aborting, so the remaining words in the iterator are not lost.
                Err(TesseractError::NullPointerError)
                | Err(TesseractError::InvalidParameterError)
                | Err(TesseractError::Utf8Error(_)) => {}
                Err(e) => return Err(e),
            }

            // SAFETY: TessResultIteratorNext() advances the iterator state and returns
            // non-zero if a next element exists. This is safe because:
            // 1. raw is a valid pointer to an initialized ResultIterator (mutex-guarded)
            // 2. RIL_WORD is a valid TessPageIteratorLevel enum value
            // 3. The mutex is held for the duration of this call (exclusive access)
            // 4. The function modifies iterator position and returns an i32 result
            let has_next = unsafe { TessResultIteratorNext(raw, TessPageIteratorLevel::RIL_WORD as c_int) != 0 };
            if !has_next {
                break;
            }
        }

        Ok(words)
    }

    /// Extracts the current word's data in a single mutex lock.
    ///
    /// Acquires the mutex once and calls all FFI functions (text, bounding box,
    /// confidence, font attributes) within that lock scope. More efficient than
    /// calling the individual methods separately when all fields are needed.
    ///
    /// # Returns
    ///
    /// Returns a [`WordData`] struct if successful, otherwise returns an error.
    pub fn extract_word_data(&self) -> Result<WordData> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        extract_word_data_unlocked(*handle)
    }
}

/// Extracts word data from a raw iterator handle without acquiring the mutex.
///
/// The caller MUST hold the mutex lock for the `ResultIterator` this handle belongs to
/// before calling this function. Passing a handle that is not mutex-guarded, or calling
/// this function concurrently on the same handle, is undefined behaviour.
fn extract_word_data_unlocked(raw: *mut c_void) -> Result<WordData> {
    // SAFETY: TessResultIteratorGetUTF8Text() allocates and returns a pointer to a C string.
    // This is safe because:
    // 1. raw is a valid pointer to an initialized ResultIterator (caller holds mutex lock)
    // 2. RIL_WORD is a valid TessPageIteratorLevel enum value converted to c_int
    // 3. The returned pointer is either null (error) or a valid null-terminated C string
    //    allocated on Tesseract's heap (must be freed with TessDeleteText)
    let text_ptr = unsafe { TessResultIteratorGetUTF8Text(raw, TessPageIteratorLevel::RIL_WORD as c_int) };
    if text_ptr.is_null() {
        return Err(TesseractError::NullPointerError);
    }
    // SAFETY: We've verified text_ptr is non-null. The allocation/deallocation pattern is:
    // 1. text_ptr was allocated by TessResultIteratorGetUTF8Text() on the FFI boundary
    // 2. CStr::from_ptr(text_ptr) is safe: pointer is non-null and points to valid C string
    // 3. We immediately copy all data to an owned String before deallocation
    // 4. The string data remains valid until TessDeleteText is called
    let text = {
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let owned = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() deallocates memory allocated by TessResultIteratorGetUTF8Text():
        // 1. text_ptr is non-null (verified above)
        // 2. text_ptr came from the Tesseract API (correct allocation type)
        // 3. TessDeleteText() is the correct deallocation function for this allocation
        // 4. Called exactly once per allocation to avoid double-free
        // 5. owned String was already populated; text_ptr is no longer accessed after this call
        unsafe { TessDeleteText(text_ptr as *mut c_char) };
        owned
    };

    let mut left = 0;
    let mut top = 0;
    let mut right = 0;
    let mut bottom = 0;
    // SAFETY: TessPageIteratorBoundingBox() queries iterator state and fills output parameters.
    // This is safe because:
    // 1. raw is a valid pointer to an initialized ResultIterator (caller holds mutex lock)
    // 2. RIL_WORD is a valid TessPageIteratorLevel enum value converted to c_int
    // 3. All mutable references are valid local stack variables with distinct memory locations
    // 4. Each reference is exclusively borrowed (Rust enforces no aliasing)
    // 5. The references outlive the FFI call (defined on stack, used immediately after)
    // 6. Return value indicates success/failure (checked below)
    let bbox_result = unsafe {
        TessPageIteratorBoundingBox(
            raw,
            TessPageIteratorLevel::RIL_WORD as c_int,
            &mut left,
            &mut top,
            &mut right,
            &mut bottom,
        )
    };
    if bbox_result == 0 {
        return Err(TesseractError::InvalidParameterError);
    }

    // SAFETY: TessResultIteratorConfidence() reads iterator state and returns an f32 value.
    // This is safe because:
    // 1. raw is a valid pointer to an initialized ResultIterator (caller holds mutex lock)
    // 2. RIL_WORD is a valid TessPageIteratorLevel enum value converted to c_int
    // 3. The function only reads state and returns a copy (no pointer operations)
    let confidence = unsafe { TessResultIteratorConfidence(raw, TessPageIteratorLevel::RIL_WORD as c_int) };

    // Collect font attributes; treat any failure as absent rather than propagating the error.
    let font_attrs = {
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut pointsize = 0;
        let mut font_id = 0;
        // SAFETY: TessResultIteratorWordFontAttributes() fills output parameters with font info.
        // This is safe because:
        // 1. raw is a valid pointer to an initialized ResultIterator (caller holds mutex lock)
        // 2. All mutable references are valid local stack variables with distinct memory locations
        // 3. Each reference is exclusively borrowed (no aliasing)
        // 4. The references outlive the FFI call
        // 5. Return value is non-zero on success, zero on failure (checked below)
        let result = unsafe {
            TessResultIteratorWordFontAttributes(
                raw,
                &mut is_bold,
                &mut is_italic,
                &mut is_underlined,
                &mut is_monospace,
                &mut is_serif,
                &mut is_smallcaps,
                &mut pointsize,
                &mut font_id,
            )
        };
        if result != 0 {
            Some(FontAttributes {
                is_bold: is_bold != 0,
                is_italic: is_italic != 0,
                is_underlined: is_underlined != 0,
                is_monospace: is_monospace != 0,
                is_serif: is_serif != 0,
                is_smallcaps: is_smallcaps != 0,
                pointsize,
                font_id,
            })
        } else {
            None
        }
    };

    Ok(WordData {
        text,
        left,
        top,
        right,
        bottom,
        confidence,
        font_attrs,
    })
}

impl Drop for ResultIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            // SAFETY: TessResultIteratorDelete() frees the ResultIterator handle allocated by Tesseract:
            // 1. We use .ok() pattern to handle poisoned mutex gracefully (no panic in Drop)
            // 2. *handle is a valid opaque pointer allocated by TessBaseAPIGetIterator()
            //    or TessBaseAPIGetMutableIterator() - Tesseract owns this memory
            // 3. TessResultIteratorDelete() is the single correct way to deallocate this type
            // 4. The function must be called exactly once per allocation to avoid double-free
            // 5. After calling delete, the pointer is invalid; future use would cause use-after-free
            // 6. Drop impl never panics (we use .ok() guard), ensuring cleanup always executes
            // 7. If mutex is poisoned, handle cleanup is skipped (OS will reclaim process memory)
            unsafe { TessResultIteratorDelete(*handle) };
        }
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
unsafe extern "C-unwind" {
    pub fn TessResultIteratorDelete(handle: *mut c_void);
    pub fn TessPageIteratorBegin(handle: *mut c_void);
    pub fn TessResultIteratorGetUTF8Text(handle: *mut c_void, level: c_int) -> *mut c_char;
    pub fn TessResultIteratorConfidence(handle: *mut c_void, level: c_int) -> c_float;
    pub fn TessResultIteratorWordRecognitionLanguage(handle: *mut c_void) -> *const c_char;
    pub fn TessResultIteratorWordFontAttributes(
        handle: *mut c_void,
        is_bold: *mut c_int,
        is_italic: *mut c_int,
        is_underlined: *mut c_int,
        is_monospace: *mut c_int,
        is_serif: *mut c_int,
        is_smallcaps: *mut c_int,
        pointsize: *mut c_int,
        font_id: *mut c_int,
    ) -> c_int;
    pub fn TessResultIteratorWordIsFromDictionary(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorWordIsNumeric(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsSuperscript(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsSubscript(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsDropcap(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorNext(handle: *mut c_void, level: c_int) -> c_int;
    pub fn TessPageIteratorBoundingBox(
        handle: *mut c_void,
        level: c_int,
        left: *mut c_int,
        top: *mut c_int,
        right: *mut c_int,
        bottom: *mut c_int,
    ) -> c_int;
}
