use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::Arc;
use std::sync::Mutex;

use crate::result_iterator::{
    TessResultIteratorConfidence, TessResultIteratorGetUTF8Text, TessResultIteratorNext,
    TessResultIteratorSymbolIsDropcap, TessResultIteratorSymbolIsSubscript, TessResultIteratorSymbolIsSuperscript,
    TessResultIteratorWordFontAttributes, TessResultIteratorWordIsFromDictionary, TessResultIteratorWordIsNumeric,
    TessResultIteratorWordRecognitionLanguage, copy_and_delete_tess_text,
};

/// Owns a native iterator allocation for result-iterator operations.
///
/// # Safety
///
/// Queries require a live, uniquely owned handle compatible with `TessResultIterator` and
/// its original `PAGE_RES`. That data belongs to the originating `TessBaseAPI`, which
/// must remain live and must not run Init, SetImage, Recognize, Clear, End, DetectOS
/// or any operation that replaces or invalidates those results while the iterator is in use.
///
/// The raw constructor does not validate ownership, type or backing lifetime.
/// The mutex only serializes calls through this wrapper. These requirements are not enforced
/// by the API. See [#3228](https://github.com/xberg-io/xberg/issues/3228).
pub struct MutableIterator {
    handle: Arc<Mutex<*mut c_void>>,
}

// SAFETY: Moving the wrapper transfers the handle and its deletion obligation without accessing native data.
// Subsequent use or deletion still depends on the ownership and coordination in MutableIterator's Safety section (#3228). ~keep
unsafe impl Send for MutableIterator {}
// SAFETY: Shared methods serialize native access through this wrapper's mutex. Access through
// other wrappers or the API must be externally coordinated as in MutableIterator's Safety section (#3228). ~keep
unsafe impl Sync for MutableIterator {}

impl MutableIterator {
    /// Creates a new instance of the MutableIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the MutableIterator.
    pub fn new(handle: *mut c_void) -> Self {
        MutableIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the UTF-8 text for the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the text.
    pub fn get_utf8_text(&self, level: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The iterator is locked. A non-null text result is owned.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        let text_ptr = unsafe { TessResultIteratorGetUTF8Text(*handle, level) };
        // SAFETY: Tesseract returns an owned NUL-terminated allocation or null. ~keep
        unsafe { copy_and_delete_tess_text(text_ptr, |pointer| TessDeleteText(pointer)) }
    }

    /// Gets the confidence of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the confidence.
    pub fn confidence(&self, level: i32) -> Result<f32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The iterator is locked. The confidence result is a C float.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorConfidence(*handle, level) })
    }

    /// Gets the recognition language of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the recognition language as a `String` if successful, otherwise returns an error.
    pub fn word_recognition_language(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The iterator is locked while requesting the language string borrowed from Tesseract.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        let lang_ptr = unsafe { TessResultIteratorWordRecognitionLanguage(*handle) };
        if lang_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: The non-null native language string is NUL-terminated and copied under the iterator lock. ~keep
        let c_str = unsafe { CStr::from_ptr(lang_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the font attributes of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the font attributes as a tuple if successful, otherwise returns an error.
    pub fn word_font_attributes(&self) -> Result<(bool, bool, bool, bool, bool, bool, i32, i32)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut pointsize = 0;
        let mut font_id = 0;

        // SAFETY: The iterator is locked and the local C int outputs match BOOL and int slots.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
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

        if result.is_null() {
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

    /// Checks if the current word is from the dictionary.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current word is from the dictionary, otherwise returns `Ok(false)`.
    pub fn word_is_from_dictionary(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes the native dictionary query on the current word.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorWordIsFromDictionary(*handle) != 0 })
    }

    /// Checks if the current word is numeric.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current word is numeric, otherwise returns `Ok(false)`.
    pub fn word_is_numeric(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes the native numeric query on the current word.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorWordIsNumeric(*handle) != 0 })
    }

    /// Checks if the current symbol is superscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is superscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_superscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorSymbolIsSuperscript(*handle) != 0 })
    }

    /// Checks if the current symbol is subscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is subscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_subscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorSymbolIsSubscript(*handle) != 0 })
    }

    /// Checks if the current symbol is dropcap.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is dropcap, otherwise returns `Ok(false)`.
    pub fn symbol_is_dropcap(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorSymbolIsDropcap(*handle) != 0 })
    }

    /// Gets the next iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the iterator.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next iterator is successful, otherwise returns `false`.
    pub fn next(&self, level: i32) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        // SAFETY: The guard serializes advancement. The raw level is passed as a C int.
        // Requires the live handle and backing data documented on `MutableIterator`. ~keep
        Ok(unsafe { TessResultIteratorNext(*handle, level) != 0 })
    }
}

impl Drop for MutableIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            // SAFETY: The raw handle must be an owned pointer usable as a ResultIterator base.
            // TessResultIteratorDelete deletes through that base's virtual destructor. ~keep
            unsafe { TessResultIteratorDelete(*handle) };
        }
    }
}

ffi_extern! {
    pub fn TessResultIteratorDelete(handle: *mut c_void);
    pub fn TessDeleteText(text: *mut c_char);
}
