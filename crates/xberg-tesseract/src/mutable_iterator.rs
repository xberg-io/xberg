use crate::api::TesseractAPI;
use crate::error::{Result, TesseractError};
use crate::native_iterator::NativeIterator;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

use crate::result_iterator::{
    TessResultIteratorConfidence, TessResultIteratorGetUTF8Text, TessResultIteratorNext,
    TessResultIteratorSymbolIsDropcap, TessResultIteratorSymbolIsSubscript, TessResultIteratorSymbolIsSuperscript,
    TessResultIteratorWordFontAttributes, TessResultIteratorWordIsFromDictionary, TessResultIteratorWordIsNumeric,
    TessResultIteratorWordRecognitionLanguage, copy_and_delete_tess_text,
};

/// Owns a native mutable result iterator allocation, from [`TesseractAPI::get_mutable_iterator`].
///
/// It borrows the engine that produced it and reads one recognition result: once the engine
/// replaces that result, every query returns [`TesseractError::StaleIterator`].
pub struct MutableIterator<'api> {
    inner: NativeIterator<'api>,
}

impl<'api> MutableIterator<'api> {
    /// Wraps a mutable iterator created by `api` from the recognition result at `epoch`.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null `TessResultIterator` that `api`'s engine created from the result
    /// at `epoch`, owned by nothing else.
    pub(crate) unsafe fn from_raw(api: &'api TesseractAPI, epoch: u64, raw: *mut c_void) -> Self {
        // SAFETY: forwarded from this function's contract. ~keep
        MutableIterator {
            inner: unsafe { NativeIterator::new(api, epoch, raw) },
        }
    }

    /// Gets the UTF-8 text for the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the text.
    pub fn get_utf8_text(&self, level: i32) -> Result<String> {
        let handle = self.inner.lock()?;
        // SAFETY: The iterator is locked. A non-null text result is owned.
        // The lock guard proves the handle is live and its recognition result current. ~keep
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
        let handle = self.inner.lock()?;
        // SAFETY: The iterator is locked. The confidence result is a C float.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorConfidence(*handle, level) })
    }

    /// Gets the recognition language of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the recognition language as a `String` if successful, otherwise returns an error.
    pub fn word_recognition_language(&self) -> Result<String> {
        let handle = self.inner.lock()?;
        // SAFETY: The iterator is locked while requesting the language string borrowed from Tesseract.
        // The lock guard proves the handle is live and its recognition result current. ~keep
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
        let handle = self.inner.lock()?;
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut pointsize = 0;
        let mut font_id = 0;

        // SAFETY: The iterator is locked and the local C int outputs match BOOL and int slots.
        // The lock guard proves the handle is live and its recognition result current. ~keep
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
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the native dictionary query on the current word.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorWordIsFromDictionary(*handle) != 0 })
    }

    /// Checks if the current word is numeric.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current word is numeric, otherwise returns `Ok(false)`.
    pub fn word_is_numeric(&self) -> Result<bool> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the native numeric query on the current word.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorWordIsNumeric(*handle) != 0 })
    }

    /// Checks if the current symbol is superscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is superscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_superscript(&self) -> Result<bool> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorSymbolIsSuperscript(*handle) != 0 })
    }

    /// Checks if the current symbol is subscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is subscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_subscript(&self) -> Result<bool> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorSymbolIsSubscript(*handle) != 0 })
    }

    /// Checks if the current symbol is dropcap.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is dropcap, otherwise returns `Ok(false)`.
    pub fn symbol_is_dropcap(&self) -> Result<bool> {
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes the native query at the current symbol.
        // The lock guard proves the handle is live and its recognition result current. ~keep
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
        let handle = self.inner.lock()?;
        // SAFETY: The guard serializes advancement. The raw level is passed as a C int.
        // The lock guard proves the handle is live and its recognition result current. ~keep
        Ok(unsafe { TessResultIteratorNext(*handle, level) != 0 })
    }
}

impl Drop for MutableIterator<'_> {
    fn drop(&mut self) {
        if let Some(handle) = self.inner.lock_for_release() {
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
