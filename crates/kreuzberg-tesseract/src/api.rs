use crate::enums::{TessPageIteratorLevel, TessPageSegMode};
use crate::error::{Result, TesseractError};
use crate::page_iterator::{TessBaseAPIGetIterator, TessPageIteratorDelete};
use crate::result_iterator::TessResultIteratorDelete;
use crate::{PageIterator, ResultIterator};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_float, c_int, c_void};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Batch bounding box results from Tesseract layout analysis.
///
/// Holds all bounding boxes returned in a single FFI call, along with optional
/// block and paragraph IDs when available (e.g., from `get_textlines`).
///
/// Each box is represented as `(x, y, width, height)` in image coordinates.
///
/// # Examples
///
/// ```rust,no_run
/// # use kreuzberg_tesseract::TesseractAPI;
/// # let api = TesseractAPI::new().unwrap();
/// # api.init("/tessdata", "eng").unwrap();
/// # api.set_image(&[], 1, 1, 1, 1).unwrap();
/// let words = api.get_words().unwrap();
/// for i in 0..words.len() {
///     if let Some((x, y, w, h)) = words.get(i) {
///         println!("Word at ({x},{y}) size {w}x{h}");
///     }
/// }
/// ```
pub struct BoundingBoxArray {
    boxes: Vec<(i32, i32, i32, i32)>,
    block_ids: Option<Vec<i32>>,
    para_ids: Option<Vec<i32>>,
}

impl BoundingBoxArray {
    /// Returns the number of bounding boxes in the array.
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    /// Returns `true` if the array contains no bounding boxes.
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    /// Returns the bounding box at `index` as `(x, y, width, height)`, or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<(i32, i32, i32, i32)> {
        self.boxes.get(index).copied()
    }

    /// Returns the block ID for the box at `index`, if block IDs were captured.
    pub fn block_id(&self, index: usize) -> Option<i32> {
        self.block_ids.as_ref()?.get(index).copied()
    }

    /// Returns the paragraph ID for the box at `index`, if paragraph IDs were captured.
    pub fn para_id(&self, index: usize) -> Option<i32> {
        self.para_ids.as_ref()?.get(index).copied()
    }

    /// Returns an iterator over all `(x, y, width, height)` tuples.
    pub fn iter(&self) -> impl Iterator<Item = &(i32, i32, i32, i32)> {
        self.boxes.iter()
    }
}

#[derive(Clone)]
pub struct TesseractConfiguration {
    datapath: String,
    language: String,
    variables: HashMap<String, String>,
}

/// Main interface to the Tesseract OCR engine.
#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
pub struct TesseractAPI {
    /// Handle to the Tesseract engine.
    pub handle: Arc<Mutex<*mut c_void>>,
    config: Arc<Mutex<TesseractConfiguration>>,
}

unsafe impl Send for TesseractAPI {}
unsafe impl Sync for TesseractAPI {}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl TesseractAPI {
    /// Creates a new instance of the Tesseract API.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TesseractAPI)` on success, or `Err(TesseractError::NullPointerError)`
    /// if the underlying C library fails to allocate the engine handle.
    ///
    /// # Errors
    ///
    /// Returns [`TesseractError::NullPointerError`] when `TessBaseAPICreate` returns a
    /// null pointer, which indicates an allocation failure in the Tesseract C library.
    pub fn new() -> Result<Self> {
        // SAFETY: TessBaseAPICreate() is a C FFI function that allocates and initializes
        // a new Tesseract engine handle. It returns a valid opaque pointer on success or
        // null on allocation failure. The returned handle is owned exclusively by this
        // Rust struct and will be freed in Drop.
        let handle = unsafe { TessBaseAPICreate() };
        if handle.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Ok(TesseractAPI {
            handle: Arc::new(Mutex::new(handle)),
            config: Arc::new(Mutex::new(TesseractConfiguration {
                datapath: String::new(),
                language: String::new(),
                variables: HashMap::new(),
            })),
        })
    }

    /// Gets the version of the Tesseract engine.
    ///
    /// # Returns
    ///
    /// Returns the version of the Tesseract engine as a string.
    pub fn version() -> String {
        // SAFETY: TessVersion() returns a pointer to a valid, null-terminated C string that is
        // stored in static memory by the Tesseract library. The pointer is valid for the entire
        // program lifetime and never freed by Tesseract. CStr::from_ptr() is safe because:
        // 1. The returned pointer is guaranteed to be non-null by TessVersion()
        // 2. The string is valid and properly null-terminated
        // 3. We only read from it (to_string_lossy), not modify it
        let version = unsafe { TessVersion() };
        unsafe { CStr::from_ptr(version) }.to_string_lossy().into_owned()
    }

    /// Initializes the Tesseract engine with the specified datapath and language.
    ///
    /// # Arguments
    ///
    /// * `datapath` - Path to the directory containing Tesseract data files.
    /// * `language` - Language code (e.g., "eng" for English).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization is successful, otherwise returns an error.
    pub fn init<P: AsRef<Path>>(&self, datapath: P, language: &str) -> Result<()> {
        let datapath_str = datapath
            .as_ref()
            .to_str()
            .ok_or(TesseractError::InvalidParameterError)?
            .to_owned();
        let language_str = language.to_owned();

        {
            let mut config = self.config.lock().map_err(|_| TesseractError::MutexLockError)?;
            config.datapath = datapath_str.clone();
            config.language = language_str.clone();
        }

        let datapath = CString::new(datapath_str).map_err(|_| TesseractError::NullByteInString)?;
        let language = CString::new(language_str).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInit3() is a C FFI function that initializes the Tesseract engine.
        // It is safe to call because:
        // 1. *handle is a valid non-null pointer created by TessBaseAPICreate()
        // 2. The mutex guard ensures exclusive access to the handle during this call
        // 3. datapath.as_ptr() and language.as_ptr() are valid, properly null-terminated C strings
        //    from CString, which ensures no interior null bytes
        // 4. TessBaseAPIInit3() only reads from these pointers and doesn't take ownership
        let result = unsafe { TessBaseAPIInit3(*handle, datapath.as_ptr(), language.as_ptr()) };
        if result != 0 {
            Err(TesseractError::InitError)
        } else {
            Ok(())
        }
    }

    /// Gets the confidence values for all recognized words.
    ///
    /// # Returns
    ///
    /// Returns a vector of confidence values (0-100) for each recognized word.
    pub fn get_word_confidences(&self) -> Result<Vec<i32>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        // SAFETY: TessBaseAPIAllWordConfidences() returns a pointer to a C array of i32 values
        // terminated by -1. This is safe because:
        // 1. *handle is a valid pointer from TessBaseAPICreate()
        // 2. The function is called on a properly initialized Tesseract engine
        // 3. The returned pointer must be freed with TessDeleteIntArray after use
        let confidences_ptr = unsafe { TessBaseAPIAllWordConfidences(*handle) };
        if confidences_ptr.is_null() {
            return Ok(Vec::new());
        }
        let mut confidences = Vec::new();
        let mut i = 0;
        // SAFETY: We iterate through the array using pointer arithmetic (offset()).
        // This is safe because:
        // 1. confidences_ptr is a valid array pointer returned by TessBaseAPIAllWordConfidences()
        // 2. The array is terminated by -1 sentinel value (invariant maintained by Tesseract)
        // 3. We read each element before checking termination condition (safe dereference)
        // 4. We only read from the array (no mutable access or aliasing)
        // 5. The array remains valid until TessDeleteIntArray is called (after loop)
        // 6. Integer i never overflows: we read i32 values, so at most 2^31 iterations
        // 7. Offset arithmetic is valid: array bounds guaranteed by -1 terminator
        while unsafe { *confidences_ptr.offset(i) } != -1 {
            confidences.push(unsafe { *confidences_ptr.offset(i) });
            i += 1;
        }
        // SAFETY: TessDeleteIntArray() deallocates the array returned by TessBaseAPIAllWordConfidences():
        // 1. confidences_ptr is non-null (checked above)
        // 2. confidences_ptr came from the Tesseract API (trusted source)
        // 3. All array data has been copied into `confidences` before this call
        // 4. Called exactly once per allocation to avoid double-free
        unsafe { TessDeleteIntArray(confidences_ptr) };
        Ok(confidences)
    }

    /// Gets the mean text confidence.
    ///
    /// # Returns
    ///
    /// Returns the mean text confidence as an integer.
    pub fn mean_text_conf(&self) -> Result<i32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIMeanTextConf() is a simple FFI call that returns a computed i32 value.
        // It is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function only reads engine state and returns a value (no pointer dereference needed)
        // 3. The mutex ensures exclusive access to the handle during the call
        Ok(unsafe { TessBaseAPIMeanTextConf(*handle) })
    }

    /// Sets a Tesseract variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    /// * `value` - Value to set.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the variable is successful, otherwise returns an error.
    pub fn set_variable(&self, name: &str, value: &str) -> Result<()> {
        {
            let mut config = self.config.lock().map_err(|_| TesseractError::MutexLockError)?;
            config.variables.insert(name.to_owned(), value.to_owned());
        }

        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let value = CString::new(value).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetVariable() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() and value.as_ptr() are valid, properly null-terminated C strings
        //    (CString guarantees no interior null bytes)
        // 3. The function only modifies engine state and doesn't take ownership of the pointers
        // 4. The mutex ensures exclusive access during modification
        let result = unsafe { TessBaseAPISetVariable(*handle, name.as_ptr(), value.as_ptr()) };
        if result != 1 {
            Err(TesseractError::SetVariableError)
        } else {
            Ok(())
        }
    }

    /// Gets a string variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    ///
    /// # Returns
    ///
    /// Returns the value of the variable as a string.
    pub fn get_string_variable(&self, name: &str) -> Result<String> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetStringVariable() returns a pointer to a C string stored in
        // Tesseract's internal state. This is safe because:
        // 1. name.as_ptr() is a valid null-terminated C string from CString
        // 2. *handle is a valid pointer to an initialized Tesseract engine
        // 3. The returned value_ptr is either null or a valid pointer to a null-terminated C string
        let value_ptr = unsafe { TessBaseAPIGetStringVariable(*handle, name.as_ptr()) };
        if value_ptr.is_null() {
            return Err(TesseractError::GetVariableError);
        }
        // SAFETY: We've verified value_ptr is non-null. CStr::from_ptr() is safe because:
        // 1. The pointer was returned from TessBaseAPIGetStringVariable() (non-null check above)
        // 2. It points to a valid null-terminated C string in Tesseract's memory
        // 3. We only read from it (to_str() creates a temporary borrow)
        let c_str = unsafe { CStr::from_ptr(value_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets an integer variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    ///
    /// # Returns
    ///
    /// Returns the value of the variable as an integer.
    pub fn get_int_variable(&self, name: &str) -> Result<i32> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetIntVariable() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only reads state and returns an i32 value
        // 4. The mutex ensures exclusive access during the call
        Ok(unsafe { TessBaseAPIGetIntVariable(*handle, name.as_ptr()) })
    }

    /// Gets a boolean variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    ///
    /// # Returns
    ///
    /// Returns the value of the variable as a boolean.
    pub fn get_bool_variable(&self, name: &str) -> Result<bool> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetBoolVariable() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only reads state and returns an i32 value (0 or non-zero)
        // 4. No pointer dereference is needed on the Rust side
        Ok(unsafe { TessBaseAPIGetBoolVariable(*handle, name.as_ptr()) } != 0)
    }

    /// Gets a double variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    ///
    /// # Returns
    ///
    /// Returns the value of the variable as a double.
    pub fn get_double_variable(&self, name: &str) -> Result<f64> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetDoubleVariable() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only reads state and returns an f64 value (copyable)
        // 4. No pointer manipulation occurs on the Rust side
        Ok(unsafe { TessBaseAPIGetDoubleVariable(*handle, name.as_ptr()) })
    }

    /// Sets the page segmentation mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Page segmentation mode.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the page segmentation mode is successful, otherwise returns an error.
    pub fn set_page_seg_mode(&self, mode: TessPageSegMode) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetPageSegMode() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. mode is a valid TessPageSegMode enum converted to c_int (no pointer operations)
        // 3. The function modifies only engine state via the handle
        // 4. The mutex ensures exclusive access during modification
        unsafe { TessBaseAPISetPageSegMode(*handle, mode as c_int) };
        Ok(())
    }

    /// Gets the page segmentation mode.
    ///
    /// # Returns
    ///
    /// Returns the page segmentation mode.
    pub fn get_page_seg_mode(&self) -> Result<TessPageSegMode> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetPageSegMode() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer dereference or memory access is needed
        let mode = unsafe { TessBaseAPIGetPageSegMode(*handle) };
        TessPageSegMode::try_from_int(mode).ok_or(TesseractError::InvalidEnumValue(mode))
    }

    /// Recognizes the text in the current image.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if recognition is successful, otherwise returns an error.
    pub fn recognize(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIRecognize() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. std::ptr::null_mut() is passed as the monitor parameter, which Tesseract accepts
        //    to indicate no progress monitoring is needed
        // 3. The mutex ensures exclusive access during the potentially long recognition process
        let result = unsafe { TessBaseAPIRecognize(*handle, std::ptr::null_mut()) };
        if result != 0 {
            Err(TesseractError::OcrError)
        } else {
            Ok(())
        }
    }

    /// Gets the HOCR text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the HOCR text for the specified page as a string.
    pub fn get_hocr_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetHOCRText() returns a pointer to an allocated C string.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. page is a valid page index
        // 3. The returned pointer is either null or points to a null-terminated C string
        //    allocated by Tesseract (must be freed with TessDeleteText)
        let text_ptr = unsafe { TessBaseAPIGetHOCRText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: We've verified text_ptr is non-null. CStr::from_ptr() is safe because:
        // 1. text_ptr points to a valid null-terminated C string allocated by Tesseract
        // 2. We only read from it (to_str() creates temporary borrow)
        // 3. We then immediately free it with TessDeleteText
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() correctly frees the memory allocated by TessBaseAPIGetHOCRText().
        // This must be called exactly once for each allocated pointer to avoid memory leaks or double-frees.
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the ALTO text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the ALTO text for the specified page as a string.
    pub fn get_alto_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetAltoText() returns a pointer to an allocated C string.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. page is a valid page index
        // 3. The returned pointer is either null or points to a valid null-terminated C string
        let text_ptr = unsafe { TessBaseAPIGetAltoText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: We've verified text_ptr is non-null. CStr::from_ptr() and TessDeleteText()
        // follow the same safety invariants as get_hocr_text()
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() must be called to free the allocated string
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the TSV text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the TSV text for the specified page as a string.
    pub fn get_tsv_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetTsvText() returns a pointer to an allocated C string.
        // This follows the same safety model as get_hocr_text() and get_alto_text()
        let text_ptr = unsafe { TessBaseAPIGetTsvText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: CStr::from_ptr() is safe because text_ptr is non-null and points to
        // a valid null-terminated C string allocated by Tesseract
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() frees the memory allocated by TessBaseAPIGetTsvText()
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Sets the input name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the input.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the input name is successful, otherwise returns an error.
    pub fn set_input_name(&self, name: &str) -> Result<()> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetInputName() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only stores the name and doesn't take ownership
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPISetInputName(*handle, name.as_ptr()) };
        Ok(())
    }

    /// Gets the input name.
    ///
    /// # Returns
    ///
    /// Returns the input name as a string.
    pub fn get_input_name(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetInputName() returns a pointer to a C string in Tesseract's memory.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or points to a valid null-terminated C string
        let name_ptr = unsafe { TessBaseAPIGetInputName(*handle) };
        if name_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: We've verified name_ptr is non-null. CStr::from_ptr() is safe because:
        // 1. name_ptr points to a valid null-terminated C string managed by Tesseract
        // 2. We only read from it (to_str() creates temporary borrow, no modification)
        let c_str = unsafe { CStr::from_ptr(name_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the data path.
    ///
    /// # Returns
    ///
    /// Returns the data path as a string.
    pub fn get_datapath(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetDatapath() returns a pointer to a C string in Tesseract's memory.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or points to a valid null-terminated C string
        let path_ptr = unsafe { TessBaseAPIGetDatapath(*handle) };
        if path_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: CStr::from_ptr() is safe because we've verified path_ptr is non-null
        // and it points to a valid null-terminated C string managed by Tesseract
        let c_str = unsafe { CStr::from_ptr(path_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the source Y resolution.
    ///
    /// # Returns
    ///
    /// Returns the source Y resolution as an integer.
    pub fn get_source_y_resolution(&self) -> Result<i32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetSourceYResolution() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations are needed
        Ok(unsafe { TessBaseAPIGetSourceYResolution(*handle) })
    }

    /// Gets the thresholded image.
    ///
    /// # Returns
    ///
    /// Returns a pointer to the thresholded image.
    pub fn get_thresholded_image(&self) -> Result<*mut c_void> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetThresholdedImage() returns a pointer to a Pix structure.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. The returned pointer is either null or a valid Pix pointer managed by Tesseract
        // 3. The caller must NOT free this pointer (it's managed by Tesseract)
        let pix = unsafe { TessBaseAPIGetThresholdedImage(*handle) };
        if pix.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(pix)
        }
    }

    /// Gets the box text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the box text for the specified page as a string.
    pub fn get_box_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetBoxText() returns a pointer to an allocated C string.
        // This follows the same pattern as get_hocr_text(), get_alto_text(), etc.
        let text_ptr = unsafe { TessBaseAPIGetBoxText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: CStr::from_ptr() is safe because text_ptr is non-null and points to
        // a valid null-terminated C string allocated by Tesseract
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() frees the memory allocated by TessBaseAPIGetBoxText()
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the LSTM box text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the LSTM box text for the specified page as a string.
    pub fn get_lstm_box_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetLSTMBoxText() returns a pointer to an allocated C string.
        // This follows the standard pattern for Tesseract text allocation/deallocation.
        let text_ptr = unsafe { TessBaseAPIGetLSTMBoxText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: CStr::from_ptr() is safe because text_ptr is non-null and points to
        // a valid null-terminated C string allocated by Tesseract
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() frees the memory allocated by TessBaseAPIGetLSTMBoxText()
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the word str box text for the specified page.
    ///
    /// # Arguments
    ///
    /// * `page` - Page number.
    ///
    /// # Returns
    ///
    /// Returns the word str box text for the specified page as a string.
    pub fn get_word_str_box_text(&self, page: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetWordStrBoxText() returns a pointer to an allocated C string.
        // This follows the standard pattern for Tesseract text allocation/deallocation.
        let text_ptr = unsafe { TessBaseAPIGetWordStrBoxText(*handle, page) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: CStr::from_ptr() is safe because text_ptr is non-null and points to
        // a valid null-terminated C string allocated by Tesseract
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() frees the memory allocated by TessBaseAPIGetWordStrBoxText()
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the UNLV text.
    ///
    /// # Returns
    ///
    /// Returns the UNLV text as a string.
    pub fn get_unlv_text(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetUNLVText() returns a pointer to an allocated C string.
        // This follows the standard pattern for Tesseract text allocation/deallocation.
        let text_ptr = unsafe { TessBaseAPIGetUNLVText(*handle) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        // SAFETY: CStr::from_ptr() is safe because text_ptr is non-null and points to
        // a valid null-terminated C string allocated by Tesseract
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        // SAFETY: TessDeleteText() frees the memory allocated by TessBaseAPIGetUNLVText()
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets all word confidences.
    ///
    /// # Returns
    ///
    /// Returns a vector of all word confidences.
    pub fn all_word_confidences(&self) -> Result<Vec<i32>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIAllWordConfidences() returns a pointer to a C array of i32 values
        // terminated by -1. The returned pointer must be freed with TessDeleteIntArray.
        let confidences_ptr = unsafe { TessBaseAPIAllWordConfidences(*handle) };
        if confidences_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }
        let mut confidences = Vec::new();
        let mut i = 0;
        // SAFETY: We iterate through the array using pointer arithmetic (offset()).
        // This is safe because:
        // 1. confidences_ptr is a valid heap-allocated array pointer from TessBaseAPIAllWordConfidences()
        // 2. The array is terminated by -1 sentinel value (API contract from Tesseract)
        // 3. We dereference and read each element before checking termination
        // 4. No mutable access or aliasing occurs (read-only access)
        // 5. The array remains valid until TessDeleteIntArray is called (after loop)
        // 6. Offset arithmetic never overflows: bounded by -1 terminator
        // 7. We later free the array exactly once with TessDeleteIntArray (no double-free)
        while unsafe { *confidences_ptr.offset(i) } != -1 {
            confidences.push(unsafe { *confidences_ptr.offset(i) });
            i += 1;
        }
        // SAFETY: TessDeleteIntArray() deallocates the array returned by TessBaseAPIAllWordConfidences():
        // 1. confidences_ptr is non-null (verified above)
        // 2. confidences_ptr comes from the Tesseract API (trusted source)
        // 3. TessDeleteIntArray() must be called exactly once per allocation to avoid double-free
        // 4. We ensure single call: array data is fully consumed and copied before deletion
        // 5. Accessing the array after this call would cause use-after-free
        unsafe { TessDeleteIntArray(confidences_ptr) };
        Ok(confidences)
    }

    /// Adapts to the word string.
    ///
    /// # Arguments
    ///
    /// * `mode` - Mode.
    /// * `wordstr` - Word string.
    ///
    /// # Returns
    ///
    /// Returns `true` if adaptation is successful, otherwise returns `false`.
    pub fn adapt_to_word_str(&self, mode: i32, wordstr: &str) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let wordstr = CString::new(wordstr).map_err(|_| TesseractError::NullByteInString)?;
        // SAFETY: TessBaseAPIAdaptToWordStr() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. wordstr.as_ptr() is a valid null-terminated C string from CString
        // 3. mode is a user-provided i32 parameter
        // 4. The function modifies internal state but doesn't take ownership
        let result = unsafe { TessBaseAPIAdaptToWordStr(*handle, mode, wordstr.as_ptr()) };
        Ok(result != 0)
    }

    /// Detects the orientation and script.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the orientation in degrees, the orientation confidence, the script name, and the script confidence.
    pub fn detect_os(&self) -> Result<(i32, f32, String, f32)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut orient_deg = 0;
        let mut orient_conf = 0.0;
        let mut script_name_ptr = std::ptr::null_mut();
        let mut script_conf = 0.0;
        // SAFETY: TessBaseAPIDetectOrientationScript() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. All mutable references (&mut ...) are valid local stack variables that outlive this call
        // 3. The function writes output values into these mutable references
        // 4. script_name_ptr may be returned as null (meaning no script detected) or as a valid pointer
        let result = unsafe {
            TessBaseAPIDetectOrientationScript(
                *handle,
                &mut orient_deg,
                &mut orient_conf,
                &mut script_name_ptr,
                &mut script_conf,
            )
        };
        if result == 0 {
            return Err(TesseractError::OcrError);
        }
        let script_name = if !script_name_ptr.is_null() {
            // SAFETY: script_name_ptr is non-null and points to a valid null-terminated C string
            // allocated by Tesseract. We read it and then free it with TessDeleteText.
            let c_str = unsafe { CStr::from_ptr(script_name_ptr) };
            let result = c_str.to_str()?.to_owned();
            // SAFETY: TessDeleteText() must be called exactly once to free the string allocated
            // by TessBaseAPIDetectOrientationScript()
            unsafe { TessDeleteText(script_name_ptr) };
            result
        } else {
            String::new()
        };
        Ok((orient_deg, orient_conf, script_name, script_conf))
    }

    /// Sets the minimum orientation margin.
    ///
    /// # Arguments
    ///
    /// * `margin` - Minimum orientation margin.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the minimum orientation margin is successful, otherwise returns an error.
    pub fn set_min_orientation_margin(&self, margin: f64) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetMinOrientationMargin() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. margin is a user-provided f64 value (copyable, no pointer operations)
        // 3. The function modifies internal state via the handle
        // 4. The mutex ensures exclusive access during modification
        unsafe { TessBaseAPISetMinOrientationMargin(*handle, margin) };
        Ok(())
    }

    /// Gets the page iterator.
    ///
    /// # Returns
    ///
    /// Returns a `PageIterator` object.
    pub fn get_page_iterator(&self) -> Result<PageIterator> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetIterator() returns a pointer to a PageIterator structure.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid PageIterator pointer
        // 3. The PageIterator wrapper will manage the lifetime and free it in Drop
        let iterator = unsafe { TessBaseAPIGetIterator(*handle) };
        if iterator.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Ok(PageIterator::new(iterator))
    }

    /// Sets the input image.
    ///
    /// # Arguments
    ///
    /// * `pix` - Pointer to the input image.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the input image is successful, otherwise returns an error.
    pub fn set_input_image(&self, pix: *mut c_void) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetInputImage() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. pix is a pointer parameter provided by the caller (trusted to be valid)
        // 3. The caller is responsible for ensuring pix points to a valid Pix structure
        // 4. Tesseract does not take ownership; the caller retains responsibility
        unsafe { TessBaseAPISetInputImage(*handle, pix) };
        Ok(())
    }

    /// Gets the input image.
    ///
    /// # Returns
    ///
    /// Returns a pointer to the input image.
    pub fn get_input_image(&self) -> Result<*mut c_void> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetInputImage() returns a pointer to the input Pix.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid Pix pointer managed by Tesseract
        // 3. The caller must NOT free this pointer (it's managed by Tesseract)
        let pix = unsafe { TessBaseAPIGetInputImage(*handle) };
        if pix.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(pix)
        }
    }

    /// Sets the output name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the output name is successful, otherwise returns an error.
    pub fn set_output_name(&self, name: &str) -> Result<()> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetOutputName() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only stores the name and doesn't take ownership
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPISetOutputName(*handle, name.as_ptr()) };
        Ok(())
    }

    /// Sets the debug variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the variable.
    /// * `value` - Value of the variable.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the debug variable is successful, otherwise returns an error.
    pub fn set_debug_variable(&self, name: &str, value: &str) -> Result<()> {
        let name = CString::new(name).map_err(|_| TesseractError::NullByteInString)?;
        let value = CString::new(value).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetDebugVariable() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. name.as_ptr() and value.as_ptr() are valid null-terminated C strings from CString
        // 3. The function modifies only engine state and doesn't take ownership
        // 4. The mutex ensures exclusive access during modification
        let result = unsafe { TessBaseAPISetDebugVariable(*handle, name.as_ptr(), value.as_ptr()) };
        if result != 1 {
            Err(TesseractError::SetVariableError)
        } else {
            Ok(())
        }
    }

    /// Prints the variables to a file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to print the variables to.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if printing the variables to the file is successful, otherwise returns an error.
    pub fn print_variables_to_file(&self, filename: &str) -> Result<()> {
        let filename = CString::new(filename).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIPrintVariablesToFile() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. filename.as_ptr() is a valid null-terminated C string from CString
        // 3. The function reads engine state and writes to a file
        // 4. The mutex ensures exclusive access during this operation
        let result = unsafe { TessBaseAPIPrintVariablesToFile(*handle, filename.as_ptr()) };
        if result != 0 {
            Err(TesseractError::IoError)
        } else {
            Ok(())
        }
    }

    /// Initializes for analysing a page.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization is successful, otherwise returns an error.
    pub fn init_for_analyse_page(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInitForAnalysePage() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function initializes internal state for page analysis
        // 3. No pointer parameters are passed
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPIInitForAnalysePage(*handle) };
        Ok(())
    }
    /// Reads the configuration file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the configuration file.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if reading the configuration file is successful, otherwise returns an error.
    pub fn read_config_file(&self, filename: &str) -> Result<()> {
        let filename = CString::new(filename).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIReadConfigFile() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. filename.as_ptr() is a valid null-terminated C string from CString
        // 3. The function reads a file and updates engine state
        // 4. The mutex ensures exclusive access during configuration
        unsafe { TessBaseAPIReadConfigFile(*handle, filename.as_ptr()) };
        Ok(())
    }

    /// Reads the debug configuration file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the debug configuration file.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if reading the debug configuration file is successful, otherwise returns an error.
    pub fn read_debug_config_file(&self, filename: &str) -> Result<()> {
        let filename = CString::new(filename).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIReadDebugConfigFile() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. filename.as_ptr() is a valid null-terminated C string from CString
        // 3. The function reads a debug configuration file and updates state
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPIReadDebugConfigFile(*handle, filename.as_ptr()) };
        Ok(())
    }

    /// Gets the thresholded image scale factor.
    ///
    /// # Returns
    ///
    /// Returns the thresholded image scale factor as an integer.
    pub fn get_thresholded_image_scale_factor(&self) -> Result<i32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetThresholdedImageScaleFactor() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function only reads state and returns an i32 value
        // 3. No pointer operations or memory access is needed
        Ok(unsafe { TessBaseAPIGetThresholdedImageScaleFactor(*handle) })
    }

    /// Processes the pages.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name of the file to process.
    /// * `retry_config` - Retry configuration.
    /// * `timeout_millisec` - Timeout in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns the processed text as a string.
    pub fn process_pages(&self, filename: &str, retry_config: Option<&str>, timeout_millisec: i32) -> Result<String> {
        let filename = CString::new(filename).map_err(|_| TesseractError::NullByteInString)?;
        let retry_config_cstr = retry_config
            .map(|s| CString::new(s).map_err(|_| TesseractError::NullByteInString))
            .transpose()?;
        // Extract the pointer before the FFI call. retry_config_cstr must remain alive
        // until TessBaseAPIProcessPages returns to prevent a dangling pointer.
        let retry_ptr = retry_config_cstr.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIProcessPages() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. filename.as_ptr() is a valid null-terminated C string from CString
        // 3. retry_ptr is either null or a valid C string pointer from retry_config_cstr,
        //    which is kept alive for the duration of this call
        // 4. timeout_millisec is a user-provided i32 value
        // 5. std::ptr::null_mut() is a valid null renderer pointer (no rendering)
        // 6. The returned pointer is either null or points to an allocated C string
        let result = unsafe {
            TessBaseAPIProcessPages(
                *handle,
                filename.as_ptr(),
                retry_ptr,
                timeout_millisec,
                std::ptr::null_mut(),
            )
        };
        if result.is_null() {
            Err(TesseractError::ProcessPagesError)
        } else {
            // SAFETY: We've verified result is non-null. CStr::from_ptr() is safe because:
            // 1. result points to a valid null-terminated C string allocated by Tesseract
            // 2. We only read from it (to_str() creates temporary borrow)
            // 3. We then immediately free it with TessDeleteText
            let c_str = unsafe { CStr::from_ptr(result) };
            let output = c_str.to_str()?.to_owned();
            // SAFETY: TessDeleteText() must be called exactly once to free the string
            unsafe { TessDeleteText(result) };
            Ok(output)
        }
    }

    /// Gets the initial languages as a string.
    ///
    /// # Returns
    ///
    /// Returns the initial languages as a string.
    pub fn get_init_languages_as_string(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetInitLanguagesAsString() returns a pointer to a C string
        // in Tesseract's memory. This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid null-terminated C string
        let result = unsafe { TessBaseAPIGetInitLanguagesAsString(*handle) };
        if result.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            // SAFETY: We've verified result is non-null. CStr::from_ptr() is safe because:
            // 1. result points to a valid null-terminated C string managed by Tesseract
            // 2. We only read from it (to_str() creates temporary borrow)
            let c_str = unsafe { CStr::from_ptr(result) };
            Ok(c_str.to_str()?.to_owned())
        }
    }

    /// Gets the loaded languages as a vector of strings.
    ///
    /// # Returns
    ///
    /// Returns a vector of loaded languages.
    pub fn get_loaded_languages(&self) -> Result<Vec<String>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let vec_ptr = unsafe { TessBaseAPIGetLoadedLanguagesAsVector(*handle) };
        self.string_vec_to_rust(vec_ptr)
    }

    /// Gets the available languages as a vector of strings.
    ///
    /// # Returns
    ///
    /// Returns a vector of available languages.
    pub fn get_available_languages(&self) -> Result<Vec<String>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let vec_ptr = unsafe { TessBaseAPIGetAvailableLanguagesAsVector(*handle) };
        self.string_vec_to_rust(vec_ptr)
    }

    /// Converts a vector of C strings to a Rust vector of strings.
    ///
    /// # Arguments
    ///
    /// * `vec_ptr` - Pointer to the vector of C strings.
    ///
    /// # Returns
    ///
    /// Returns a vector of strings.
    fn string_vec_to_rust(&self, vec_ptr: *mut *mut c_char) -> Result<Vec<String>> {
        if vec_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let mut result = Vec::new();
        let mut i = 0;
        loop {
            // SAFETY: We dereference vec_ptr at offset(i) to get a C string pointer.
            // This is safe because:
            // 1. vec_ptr is non-null (checked above)
            // 2. vec_ptr is a valid array pointer from Tesseract, allocated on heap
            // 3. The array is null-terminated (invariant maintained by Tesseract)
            // 4. We iterate until we find a null element, preventing out-of-bounds read
            // 5. Each element is either null (terminator) or a valid *mut c_char pointer to a string
            // 6. Offset arithmetic is safe: we check for null before each dereference
            // 7. No integer overflow: i increments monotonically from 0
            let str_ptr = unsafe { *vec_ptr.offset(i) };
            if str_ptr.is_null() {
                break;
            }
            // SAFETY: str_ptr is non-null (checked above) and points to a valid null-terminated
            // C string stored in the array. This is safe because:
            // 1. str_ptr came from the Tesseract-allocated array (trusted source)
            // 2. C strings are guaranteed null-terminated by Tesseract's API contract
            // 3. CStr::from_ptr() doesn't modify the string, only reads it
            // 4. The borrowed CStr is immediately converted to owned String
            // 5. The string remains valid until TessDeleteTextArray (called after loop)
            let c_str = unsafe { CStr::from_ptr(str_ptr) };
            result.push(c_str.to_str()?.to_owned());
            i += 1;
        }
        // SAFETY: TessDeleteTextArray() deallocates both the array and all contained strings:
        // 1. vec_ptr must be non-null (verified above)
        // 2. vec_ptr must come from Tesseract (TessBaseAPIGetLoadedLanguagesAsVector, etc.)
        // 3. TessDeleteTextArray() must be called exactly once per allocation (no double-free)
        // 4. The function is called after all strings are copied to Rust (owned String objects)
        // 5. Calling it twice would cause use-after-free; we ensure single call by consuming data
        unsafe { TessDeleteTextArray(vec_ptr) };
        Ok(result)
    }

    /// Clears the adaptive classifier.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if clearing the adaptive classifier is successful, otherwise returns an error.
    pub fn clear_adaptive_classifier(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIClearAdaptiveClassifier() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function modifies internal state via the handle
        // 3. No pointer parameters are passed
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPIClearAdaptiveClassifier(*handle) };
        Ok(())
    }

    /// Clears the OCR engine.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if clearing the OCR engine is successful, otherwise returns an error.
    pub fn clear(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIClear() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function resets internal state
        // 3. No pointer parameters are passed
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPIClear(*handle) };
        Ok(())
    }

    /// Ends the OCR engine.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if ending the OCR engine is successful, otherwise returns an error.
    pub fn end(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIEnd() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The function finalizes the engine but does NOT free the handle
        //    (the handle is freed separately in Drop via TessBaseAPIDelete)
        // 3. No pointer parameters are passed
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPIEnd(*handle) };
        Ok(())
    }

    /// Checks if a word is valid.
    ///
    /// # Arguments
    ///
    /// * `word` - Word to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the word is valid, otherwise returns `false`.
    pub fn is_valid_word(&self, word: &str) -> Result<i32> {
        let word = CString::new(word).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIIsValidWord() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. word.as_ptr() is a valid null-terminated C string from CString
        // 3. The function only reads state and returns an i32 value
        // 4. No modification of engine state occurs
        Ok(unsafe { TessBaseAPIIsValidWord(*handle, word.as_ptr()) })
    }

    /// Gets the text direction.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the degrees and confidence.
    pub fn get_text_direction(&self) -> Result<(i32, f32)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut out_degrees = 0;
        let mut out_confidence = 0.0;
        // SAFETY: TessBaseAPIGetTextDirection() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. &mut out_degrees and &mut out_confidence are valid mutable references to stack variables
        // 3. These references outlive the function call
        // 4. The function only writes output values into these mutable references
        // 5. No pointer aliasing occurs (exclusive access via mutable references)
        unsafe {
            TessBaseAPIGetTextDirection(*handle, &mut out_degrees, &mut out_confidence);
        }
        Ok((out_degrees, out_confidence))
    }

    /// Initializes the OCR engine.
    ///
    /// # Arguments
    ///
    /// * `datapath` - Path to the data directory.
    /// * `language` - Language to use.
    /// * `oem` - OCR engine mode.
    /// * `configs` - Configuration strings.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initializing the OCR engine is successful, otherwise returns an error.
    pub fn init_1(&self, datapath: &str, language: &str, oem: i32, configs: &[&str]) -> Result<()> {
        let datapath = CString::new(datapath).map_err(|_| TesseractError::NullByteInString)?;
        let language = CString::new(language).map_err(|_| TesseractError::NullByteInString)?;
        let config_ptrs: Vec<_> = configs
            .iter()
            .map(|&s| CString::new(s).map_err(|_| TesseractError::NullByteInString))
            .collect::<Result<_>>()?;
        let config_ptr_ptrs: Vec<_> = config_ptrs.iter().map(|cs| cs.as_ptr()).collect();
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInit1() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. datapath.as_ptr() and language.as_ptr() are valid null-terminated C strings
        // 3. config_ptr_ptrs.as_ptr() is a valid array of C string pointers; each pointer
        //    comes from CString::as_ptr() which guarantees valid null-terminated strings
        // 4. config_ptrs.len() is the correct count of configuration strings
        // 5. The strings in config_ptrs outlive the FFI call (stored in config_ptr_ptrs vector)
        // 6. oem is a user-provided integer parameter
        let result = unsafe {
            TessBaseAPIInit1(
                *handle,
                datapath.as_ptr(),
                language.as_ptr(),
                oem,
                config_ptr_ptrs.as_ptr(),
                config_ptrs.len() as c_int,
            )
        };
        if result != 0 {
            Err(TesseractError::InitError)
        } else {
            Ok(())
        }
    }

    /// Initializes the OCR engine.
    ///
    /// # Arguments
    ///
    /// * `datapath` - Path to the data directory.
    /// * `language` - Language to use.
    /// * `oem` - OCR engine mode.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initializing the OCR engine is successful, otherwise returns an error.
    pub fn init_2(&self, datapath: &str, language: &str, oem: i32) -> Result<()> {
        let datapath = CString::new(datapath).map_err(|_| TesseractError::NullByteInString)?;
        let language = CString::new(language).map_err(|_| TesseractError::NullByteInString)?;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInit2() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. datapath.as_ptr() and language.as_ptr() are valid null-terminated C strings
        // 3. oem is a user-provided integer parameter
        // 4. The CStrings outlive the FFI call (held in local variables)
        let result = unsafe { TessBaseAPIInit2(*handle, datapath.as_ptr(), language.as_ptr(), oem) };
        if result != 0 {
            Err(TesseractError::InitError)
        } else {
            Ok(())
        }
    }

    /// Initializes the OCR engine.
    ///
    /// # Arguments
    ///
    /// * `datapath` - Path to the data directory.
    /// * `language` - Language to use.
    /// * `oem` - OCR engine mode.
    /// * `configs` - Configuration strings.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initializing the OCR engine is successful, otherwise returns an error.
    pub fn init_4(&self, datapath: &str, language: &str, oem: i32, configs: &[&str]) -> Result<()> {
        let datapath = CString::new(datapath).map_err(|_| TesseractError::NullByteInString)?;
        let language = CString::new(language).map_err(|_| TesseractError::NullByteInString)?;
        let config_ptrs: Vec<_> = configs
            .iter()
            .map(|&s| CString::new(s).map_err(|_| TesseractError::NullByteInString))
            .collect::<Result<_>>()?;
        let config_ptr_ptrs: Vec<_> = config_ptrs.iter().map(|cs| cs.as_ptr()).collect();
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInit4() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. datapath.as_ptr() and language.as_ptr() are valid null-terminated C strings
        // 3. config_ptr_ptrs.as_ptr() is a valid array of C string pointers from config_ptrs
        // 4. All strings in config_ptrs outlive the FFI call
        // 5. config_ptrs.len() is the correct count
        // 6. oem is a user-provided integer
        // 7. null pointers for vars_vec/vars_values with size 0 is valid (no variable overrides)
        // 8. set_only_non_debug_params=0 (FALSE) means apply all params
        let result = unsafe {
            TessBaseAPIInit4(
                *handle,
                datapath.as_ptr(),
                language.as_ptr(),
                oem,
                config_ptr_ptrs.as_ptr(),
                config_ptrs.len() as c_int,
                std::ptr::null(),
                std::ptr::null(),
                0,
                0,
            )
        };
        if result != 0 {
            Err(TesseractError::InitError)
        } else {
            Ok(())
        }
    }

    /// Initializes the OCR engine.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw data.
    /// * `data_size` - Size of the data.
    /// * `language` - Language to use.
    /// * `oem` - OCR engine mode.
    /// * `configs` - Configuration strings.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initializing the OCR engine is successful, otherwise returns an error.
    pub fn init_5(&self, data: &[u8], data_size: i32, language: &str, oem: i32, configs: &[&str]) -> Result<()> {
        let language = CString::new(language).map_err(|_| TesseractError::NullByteInString)?;
        let config_ptrs: Vec<_> = configs
            .iter()
            .map(|&s| CString::new(s).map_err(|_| TesseractError::NullByteInString))
            .collect::<Result<_>>()?;
        let config_ptr_ptrs: Vec<_> = config_ptrs.iter().map(|cs| cs.as_ptr()).collect();
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIInit5() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. data.as_ptr() is a valid pointer to the data slice (non-null if slice is non-empty)
        // 3. language.as_ptr() is a valid null-terminated C string
        // 4. config_ptr_ptrs.as_ptr() is a valid array of C string pointers
        // 5. All pointers (data, language, configs) outlive the FFI call
        // 6. data_size and oem are user-provided integers
        // 7. The caller is responsible for ensuring data_size matches the actual data length
        // 8. null pointers for vars_vec/vars_values with size 0 is valid (no variable overrides)
        // 9. set_only_non_debug_params=0 (FALSE) means apply all params
        let result = unsafe {
            TessBaseAPIInit5(
                *handle,
                data.as_ptr(),
                data_size,
                language.as_ptr(),
                oem,
                config_ptr_ptrs.as_ptr(),
                config_ptrs.len() as c_int,
                std::ptr::null(),
                std::ptr::null(),
                0,
                0,
            )
        };
        if result != 0 {
            Err(TesseractError::InitError)
        } else {
            Ok(())
        }
    }

    /// Sets the image for OCR processing.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Raw image data.
    /// * `width` - Width of the image.
    /// * `height` - Height of the image.
    /// * `bytes_per_pixel` - Number of bytes per pixel (e.g., 3 for RGB, 1 for grayscale).
    /// * `bytes_per_line` - Number of bytes per line (usually width * bytes_per_pixel, but might be padded).
    pub fn set_image(
        &self,
        image_data: &[u8],
        width: i32,
        height: i32,
        bytes_per_pixel: i32,
        bytes_per_line: i32,
    ) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Err(TesseractError::InvalidDimensions);
        }

        if bytes_per_pixel <= 0 {
            return Err(TesseractError::InvalidBytesPerPixel);
        }

        if bytes_per_line < width * bytes_per_pixel {
            return Err(TesseractError::InvalidBytesPerLine);
        }

        let expected_size = (height * bytes_per_line) as usize;
        if image_data.len() < expected_size {
            return Err(TesseractError::InvalidImageData);
        }

        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        // SAFETY: TessBaseAPISetImage() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. image_data.as_ptr() is a valid pointer to the image data slice
        // 3. All dimension parameters (width, height, bytes_per_pixel, bytes_per_line) have been
        //    validated above to ensure consistency
        // 4. The image data buffer size is verified to be sufficient for the given dimensions
        // 5. The function only reads from the image data (no modifications)
        // 6. The image data outlives the FFI call (borrowed from caller)
        // 7. The mutex ensures exclusive access
        unsafe {
            TessBaseAPISetImage(
                *handle,
                image_data.as_ptr(),
                width,
                height,
                bytes_per_pixel,
                bytes_per_line,
            );
        }
        Ok(())
    }

    /// Sets the image for OCR processing.
    ///
    /// # Arguments
    ///
    /// * `pix` - Pointer to the image data.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the image is successful, otherwise returns an error.
    pub fn set_image_2(&self, pix: *mut c_void) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetImage2() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. pix is a pointer parameter provided by the caller (trusted to be valid)
        // 3. The caller is responsible for ensuring pix points to a valid Pix structure
        // 4. Tesseract does not take ownership; the caller retains responsibility
        // 5. The mutex ensures exclusive access
        unsafe { TessBaseAPISetImage2(*handle, pix) };
        Ok(())
    }

    /// Sets the source resolution for the image.
    ///
    /// # Arguments
    ///
    /// * `ppi` - PPI of the image.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the source resolution is successful, otherwise returns an error.
    pub fn set_source_resolution(&self, ppi: i32) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetSourceResolution() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. ppi is a user-provided integer parameter
        // 3. The function modifies only engine state
        // 4. The mutex ensures exclusive access
        unsafe { TessBaseAPISetSourceResolution(*handle, ppi) };
        Ok(())
    }

    /// Sets the rectangle for OCR processing.
    ///
    /// # Arguments
    ///
    /// * `left` - Left coordinate.
    /// * `top` - Top coordinate.
    /// * `width` - Width.
    /// * `height` - Height.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if setting the rectangle is successful, otherwise returns an error.
    pub fn set_rectangle(&self, left: i32, top: i32, width: i32, height: i32) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPISetRectangle() is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. All parameters (left, top, width, height) are user-provided integers
        // 3. The function modifies only engine state (region of interest)
        // 4. The caller is responsible for ensuring valid rectangle coordinates
        // 5. The mutex ensures exclusive access
        unsafe { TessBaseAPISetRectangle(*handle, left, top, width, height) };
        Ok(())
    }

    /// Performs OCR on the set image and returns the recognized text.
    ///
    /// # Returns
    ///
    /// Returns the recognized text as a String if successful, otherwise returns an error.
    pub fn get_utf8_text(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        if *handle == std::ptr::null_mut() {
            return Err(TesseractError::UninitializedError);
        }

        // SAFETY: TessBaseAPIGetUTF8Text() returns a pointer to an allocated C string.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid null-terminated C string
        let text_ptr = unsafe { TessBaseAPIGetUTF8Text(*handle) };
        if text_ptr.is_null() {
            return Err(TesseractError::OcrError);
        }

        // SAFETY: We've verified text_ptr is non-null. CStr::from_ptr() and TessDeleteText()
        // follow the same safety model:
        // 1. text_ptr points to a valid null-terminated C string allocated by Tesseract
        // 2. We read from it (to_str()), convert to String, then immediately free it
        // 3. TessDeleteText() must be called exactly once to avoid memory leaks
        let result = unsafe {
            let c_str = CStr::from_ptr(text_ptr);
            let result = c_str.to_str()?.to_owned();
            TessDeleteText(text_ptr);
            result
        };

        Ok(result)
    }

    /// Gets the iterator for the OCR results.
    ///
    /// # Returns
    ///
    /// Returns the iterator for the OCR results as a `ResultIterator` if successful, otherwise returns an error.
    pub fn get_iterator(&self) -> Result<ResultIterator> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetIterator() returns a pointer to a ResultIterator structure.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid ResultIterator pointer
        // 3. The ResultIterator wrapper will manage the lifetime and free it in Drop
        let iterator = unsafe { TessBaseAPIGetIterator(*handle) };
        if iterator.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(ResultIterator::new(iterator))
        }
    }

    /// Gets the mutable iterator for the OCR results.
    ///
    /// # Returns
    ///
    /// Returns the mutable iterator for the OCR results as a `ResultIterator` if successful, otherwise returns an error.
    pub fn get_mutable_iterator(&self) -> Result<ResultIterator> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetMutableIterator() returns a pointer to a mutable ResultIterator.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid ResultIterator pointer
        // 3. The ResultIterator wrapper will manage the lifetime and free it in Drop
        let iterator = unsafe { TessBaseAPIGetMutableIterator(*handle) };
        if iterator.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(ResultIterator::new(iterator))
        }
    }

    /// Analyzes the layout of the image.
    ///
    /// # Returns
    ///
    /// Returns the layout of the image as a `PageIterator` if successful, otherwise returns an error.
    pub fn analyse_layout(&self) -> Result<PageIterator> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIAnalyseLayout() returns a pointer to a PageIterator structure.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid PageIterator pointer
        // 3. The PageIterator wrapper will manage the lifetime and free it in Drop
        let iterator = unsafe { TessBaseAPIAnalyseLayout(*handle) };
        if iterator.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(PageIterator::new(iterator))
        }
    }

    /// Gets the Unicode character for a given ID.
    ///
    /// # Arguments
    ///
    /// * `unichar_id` - ID of the Unicode character.
    ///
    /// # Returns
    ///
    /// Returns the Unicode character as a String if successful, otherwise returns an error.
    pub fn get_unichar(&self, unichar_id: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetUnichar() returns a pointer to a C string in Tesseract's memory.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. unichar_id is a user-provided integer index
        // 3. The returned pointer is either null or a valid null-terminated C string
        let char_ptr = unsafe { TessBaseAPIGetUnichar(*handle, unichar_id) };
        if char_ptr.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            // SAFETY: We've verified char_ptr is non-null. CStr::from_ptr() is safe because:
            // 1. char_ptr points to a valid null-terminated C string managed by Tesseract
            // 2. We only read from it (to_str() creates temporary borrow)
            let c_str = unsafe { CStr::from_ptr(char_ptr) };
            Ok(c_str.to_str()?.to_owned())
        }
    }

    /// Gets a page iterator for analyzing layout and getting bounding boxes
    pub fn analyze_layout(&self) -> Result<PageIterator> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIAnalyseLayout() returns a pointer to a PageIterator structure.
        // This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine
        // 2. The returned pointer is either null or a valid PageIterator pointer
        // 3. The PageIterator wrapper will manage the lifetime and free it in Drop
        let iterator = unsafe { TessBaseAPIAnalyseLayout(*handle) };
        if iterator.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        Ok(PageIterator::new(iterator))
    }

    /// Get all word bounding boxes in a single FFI call.
    ///
    /// Calls `TessBaseAPIGetWords` and returns every word bounding box as a
    /// [`BoundingBoxArray`].  This is more efficient than iterating via
    /// [`get_iterator`](Self::get_iterator) when only bounding boxes are needed.
    ///
    /// # Returns
    ///
    /// Returns a [`BoundingBoxArray`] containing `(x, y, width, height)` for every
    /// word detected on the current page, or an error if the engine returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::TesseractAPI;
    /// # let api = TesseractAPI::new().unwrap();
    /// # api.init("/tessdata", "eng").unwrap();
    /// # api.set_image(&[], 1, 1, 1, 1).unwrap();
    /// let words = api.get_words().unwrap();
    /// println!("{} words found", words.len());
    /// ```
    pub fn get_words(&self) -> Result<BoundingBoxArray> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetWords() returns a newly-allocated BOXA* (Leptonica bounding-box
        // array) containing one BOX per detected word.  This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. We pass null for the PIXA** parameter — Tesseract accepts null and simply skips
        //    image extraction, returning only the bounding boxes
        // 3. The returned pointer is either null (no words found is treated as an error) or a
        //    valid BOXA* that we own and must free with boxaDestroy
        // 4. The mutex ensures exclusive access during the call
        let boxa = unsafe { TessBaseAPIGetWords(*handle, std::ptr::null_mut()) };
        if boxa.is_null() {
            // Null boxa means no words found — return empty result rather than an error.
            return Ok(BoundingBoxArray {
                boxes: Vec::new(),
                block_ids: None,
                para_ids: None,
            });
        }
        // SAFETY: boxa is a valid non-null BOXA* returned by TessBaseAPIGetWords.
        // boxaGetCount reads the count field and returns an i32 — no allocation, safe to call.
        let count = unsafe { boxaGetCount(boxa) };
        let mut boxes = Vec::with_capacity(count as usize);
        for i in 0..count {
            let mut x = 0_i32;
            let mut y = 0_i32;
            let mut w = 0_i32;
            let mut h = 0_i32;
            // SAFETY: boxaGetBox with L_NOCOPY (0) returns a borrowed pointer into the BOXA
            // that is valid for the lifetime of boxa.  We immediately call boxGetGeometry to
            // extract the geometry into local variables and never store the BOX pointer beyond
            // this iteration.  boxGetGeometry returns 1 on success and 0 on failure.
            // Both calls are safe because:
            // 1. boxa is a valid non-null BOXA* (checked above)
            // 2. i is in [0, count) — within bounds of the array
            // 3. All four *mut i32 are valid, properly aligned stack locals
            // 4. The BOX* returned by boxaGetBox with L_NOCOPY is an interior pointer into boxa
            //    and remains valid until boxaDestroy is called below
            let bx = unsafe { boxaGetBox(boxa, i, 0) };
            if !bx.is_null() {
                let ok = unsafe { boxGetGeometry(bx, &mut x, &mut y, &mut w, &mut h) };
                if ok != 0 {
                    boxes.push((x, y, w, h));
                }
                // Skip this box if boxGetGeometry failed (would have pushed zero-values otherwise).
            }
        }
        // SAFETY: boxaDestroy takes a *mut *mut BOXA, sets the pointer to null, and frees the
        // array together with all contained BOX objects.  This must be called exactly once — we
        // have transferred all data into `boxes` above, so using boxa after this point would be
        // use-after-free.  We pass &mut boxa (a mutable reference to the local variable) which
        // satisfies the *mut *mut BOXA parameter.
        let mut boxa_mut = boxa;
        unsafe { boxaDestroy(&mut boxa_mut) };
        Ok(BoundingBoxArray {
            boxes,
            block_ids: None,
            para_ids: None,
        })
    }

    /// Get all region bounding boxes in a single FFI call.
    ///
    /// Calls `TessBaseAPIGetRegions` and returns every layout region as a
    /// [`BoundingBoxArray`].
    ///
    /// # Returns
    ///
    /// Returns a [`BoundingBoxArray`] containing `(x, y, width, height)` for every
    /// region on the current page, or an error if the engine returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::TesseractAPI;
    /// # let api = TesseractAPI::new().unwrap();
    /// # api.init("/tessdata", "eng").unwrap();
    /// # api.set_image(&[], 1, 1, 1, 1).unwrap();
    /// let regions = api.get_regions().unwrap();
    /// println!("{} regions found", regions.len());
    /// ```
    pub fn get_regions(&self) -> Result<BoundingBoxArray> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        // SAFETY: TessBaseAPIGetRegions() returns a newly-allocated BOXA* containing one BOX per
        // layout region.  Safety invariants are identical to get_words():
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. null is passed for the PIXA** — Tesseract skips image extraction
        // 3. We own the returned BOXA* and must free it with boxaDestroy
        // 4. The mutex ensures exclusive access
        let boxa = unsafe { TessBaseAPIGetRegions(*handle, std::ptr::null_mut()) };
        if boxa.is_null() {
            // Null boxa means no regions found — return empty result rather than an error.
            return Ok(BoundingBoxArray {
                boxes: Vec::new(),
                block_ids: None,
                para_ids: None,
            });
        }
        // SAFETY: See get_words() for full explanation — same pattern applies here.
        let count = unsafe { boxaGetCount(boxa) };
        let mut boxes = Vec::with_capacity(count as usize);
        for i in 0..count {
            let mut x = 0_i32;
            let mut y = 0_i32;
            let mut w = 0_i32;
            let mut h = 0_i32;
            let bx = unsafe { boxaGetBox(boxa, i, 0) };
            if !bx.is_null() {
                // Only push if boxGetGeometry succeeds (returns 1); skip on failure.
                let ok = unsafe { boxGetGeometry(bx, &mut x, &mut y, &mut w, &mut h) };
                if ok != 0 {
                    boxes.push((x, y, w, h));
                }
            }
        }
        let mut boxa_mut = boxa;
        // SAFETY: boxaDestroy sets boxa_mut to null after freeing — called exactly once.
        unsafe { boxaDestroy(&mut boxa_mut) };
        Ok(BoundingBoxArray {
            boxes,
            block_ids: None,
            para_ids: None,
        })
    }

    /// Get all textline bounding boxes with block and paragraph IDs.
    ///
    /// Calls `TessBaseAPIGetTextlines1` with `raw_image=FALSE` and `raw_padding=0`,
    /// capturing both the `blockids` and `paraids` arrays alongside the bounding boxes.
    ///
    /// # Returns
    ///
    /// Returns a [`BoundingBoxArray`] where [`BoundingBoxArray::block_id`] and
    /// [`BoundingBoxArray::para_id`] return the corresponding IDs for each textline.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::TesseractAPI;
    /// # let api = TesseractAPI::new().unwrap();
    /// # api.init("/tessdata", "eng").unwrap();
    /// # api.set_image(&[], 1, 1, 1, 1).unwrap();
    /// let lines = api.get_textlines().unwrap();
    /// for i in 0..lines.len() {
    ///     let (x, y, w, h) = lines.get(i).unwrap();
    ///     let block = lines.block_id(i).unwrap_or(-1);
    ///     println!("Line {i}: ({x},{y},{w},{h}) block={block}");
    /// }
    /// ```
    pub fn get_textlines(&self) -> Result<BoundingBoxArray> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut blockids_ptr: *mut c_int = std::ptr::null_mut();
        let mut paraids_ptr: *mut c_int = std::ptr::null_mut();
        // SAFETY: TessBaseAPIGetTextlines1() returns a newly-allocated BOXA* and optionally
        // heap-allocates int arrays for blockids/paraids.  This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. raw_image=0 (FALSE) and raw_padding=0 are valid inputs
        // 3. null is passed for the PIXA** — Tesseract skips image extraction
        // 4. &mut blockids_ptr and &mut paraids_ptr are valid mutable references to null-initialized
        //    local variables; Tesseract will write heap-allocated int* values into them if the
        //    call succeeds
        // 5. We own all returned allocations (BOXA* + two int* arrays) and free them below
        // 6. The mutex ensures exclusive access
        let boxa = unsafe {
            TessBaseAPIGetTextlines1(
                *handle,
                0,                    // raw_image = FALSE
                0,                    // raw_padding = 0
                std::ptr::null_mut(), // pixa** — not needed
                &mut blockids_ptr,
                &mut paraids_ptr,
            )
        };
        if boxa.is_null() {
            // Null boxa means no textlines found — return empty result rather than an error.
            return Ok(BoundingBoxArray {
                boxes: Vec::new(),
                block_ids: None,
                para_ids: None,
            });
        }
        // SAFETY: See get_words() — same Leptonica traversal pattern.
        let count = unsafe { boxaGetCount(boxa) };
        let n = count as usize;
        let mut boxes = Vec::with_capacity(n);
        // Collect block_ids and para_ids in lock-step with boxes: only push an ID
        // when the corresponding box is successfully pushed (boxGetGeometry succeeded).
        // Collecting all `count` IDs unconditionally would leave the ID vecs longer
        // than the boxes vec whenever boxGetGeometry fails for some indices.
        let mut block_ids_vec: Option<Vec<i32>> = if blockids_ptr.is_null() {
            None
        } else {
            Some(Vec::with_capacity(n))
        };
        let mut para_ids_vec: Option<Vec<i32>> = if paraids_ptr.is_null() {
            None
        } else {
            Some(Vec::with_capacity(n))
        };
        for i in 0..count {
            let mut x = 0_i32;
            let mut y = 0_i32;
            let mut w = 0_i32;
            let mut h = 0_i32;
            let bx = unsafe { boxaGetBox(boxa, i, 0) };
            if !bx.is_null() {
                // Only push if boxGetGeometry succeeds (returns 1); skip on failure.
                // Push IDs in the same branch so boxes and ID arrays stay in sync.
                let ok = unsafe { boxGetGeometry(bx, &mut x, &mut y, &mut w, &mut h) };
                if ok != 0 {
                    boxes.push((x, y, w, h));
                    // SAFETY: blockids_ptr/paraids_ptr are valid int arrays of `count`
                    // elements (Tesseract-allocated); index i is within [0, count).
                    if let Some(ref mut ids) = block_ids_vec {
                        ids.push(unsafe { *blockids_ptr.offset(i as isize) });
                    }
                    if let Some(ref mut ids) = para_ids_vec {
                        ids.push(unsafe { *paraids_ptr.offset(i as isize) });
                    }
                }
            }
        }
        // Free the Tesseract-allocated int arrays now that we have copied the needed values.
        // SAFETY: TessDeleteIntArray frees the Tesseract-allocated int array exactly once.
        if !blockids_ptr.is_null() {
            unsafe { TessDeleteIntArray(blockids_ptr) };
        }
        if !paraids_ptr.is_null() {
            unsafe { TessDeleteIntArray(paraids_ptr) };
        }
        let mut boxa_mut = boxa;
        // SAFETY: boxaDestroy sets boxa_mut to null after freeing — called exactly once.
        unsafe { boxaDestroy(&mut boxa_mut) };
        Ok(BoundingBoxArray {
            boxes,
            block_ids: block_ids_vec,
            para_ids: para_ids_vec,
        })
    }

    /// Get all component bounding boxes at the specified iterator level.
    ///
    /// Calls `TessBaseAPIGetComponentImages` and returns every matching component as a
    /// [`BoundingBoxArray`].
    ///
    /// # Arguments
    ///
    /// * `level` - The [`TessPageIteratorLevel`] granularity (block, paragraph, line, word, symbol).
    /// * `text_only` - If `true`, only text components are returned; if `false`, all components.
    ///
    /// # Returns
    ///
    /// Returns a [`BoundingBoxArray`] containing `(x, y, width, height)` for every
    /// matching component, or an error if the engine returns null.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use kreuzberg_tesseract::{TesseractAPI, TessPageIteratorLevel};
    /// # let api = TesseractAPI::new().unwrap();
    /// # api.init("/tessdata", "eng").unwrap();
    /// # api.set_image(&[], 1, 1, 1, 1).unwrap();
    /// let components = api.get_component_images(TessPageIteratorLevel::RIL_WORD, true).unwrap();
    /// println!("{} text components at word level", components.len());
    /// ```
    pub fn get_component_images(&self, level: TessPageIteratorLevel, text_only: bool) -> Result<BoundingBoxArray> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut blockids_ptr: *mut c_int = std::ptr::null_mut();
        // SAFETY: TessBaseAPIGetComponentImages() returns a newly-allocated BOXA* and optionally
        // heap-allocates an int array for blockids.  This is safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with an image set
        // 2. level as c_int is a valid TessPageIteratorLevel enum value
        // 3. text_only as c_int (0 or 1) is a valid BOOL parameter
        // 4. null is passed for the PIXA** — Tesseract skips image extraction
        // 5. &mut blockids_ptr is a valid mutable reference to a null-initialized local; Tesseract
        //    writes a heap-allocated int* into it if the call succeeds
        // 6. We own all returned allocations and free them below
        // 7. The mutex ensures exclusive access
        let boxa = unsafe {
            TessBaseAPIGetComponentImages(
                *handle,
                level as c_int,
                text_only as c_int,
                std::ptr::null_mut(), // pixa** — not needed
                &mut blockids_ptr,
            )
        };
        if boxa.is_null() {
            // Null boxa means no components found — return empty result rather than an error.
            return Ok(BoundingBoxArray {
                boxes: Vec::new(),
                block_ids: None,
                para_ids: None,
            });
        }
        // SAFETY: See get_words() — same Leptonica traversal pattern.
        let count = unsafe { boxaGetCount(boxa) };
        let n = count as usize;
        let mut boxes = Vec::with_capacity(n);
        // Collect block_ids in lock-step with boxes so the two arrays stay aligned.
        // Pushing IDs unconditionally for all `count` indices would leave block_ids
        // longer than boxes when boxGetGeometry fails for some entries.
        let mut block_ids_vec: Option<Vec<i32>> = if blockids_ptr.is_null() {
            None
        } else {
            Some(Vec::with_capacity(n))
        };
        for i in 0..count {
            let mut x = 0_i32;
            let mut y = 0_i32;
            let mut w = 0_i32;
            let mut h = 0_i32;
            let bx = unsafe { boxaGetBox(boxa, i, 0) };
            if !bx.is_null() {
                // Only push if boxGetGeometry succeeds (returns 1); skip on failure.
                // Push the block ID in the same branch so boxes and block_ids stay in sync.
                let ok = unsafe { boxGetGeometry(bx, &mut x, &mut y, &mut w, &mut h) };
                if ok != 0 {
                    boxes.push((x, y, w, h));
                    // SAFETY: blockids_ptr is a valid int array of `count` elements; i < count.
                    if let Some(ref mut ids) = block_ids_vec {
                        ids.push(unsafe { *blockids_ptr.offset(i as isize) });
                    }
                }
            }
        }
        // Free the Tesseract-allocated block IDs array now that we have copied the values.
        // SAFETY: TessDeleteIntArray frees the Tesseract-allocated int array exactly once.
        if !blockids_ptr.is_null() {
            unsafe { TessDeleteIntArray(blockids_ptr) };
        }
        let mut boxa_mut = boxa;
        // SAFETY: boxaDestroy sets boxa_mut to null after freeing — called exactly once.
        unsafe { boxaDestroy(&mut boxa_mut) };
        Ok(BoundingBoxArray {
            boxes,
            block_ids: block_ids_vec,
            para_ids: None,
        })
    }

    /// Gets both page and result iterators for full text analysis
    pub fn get_iterators(&self) -> Result<(PageIterator, ResultIterator)> {
        self.recognize()?;

        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        // SAFETY: TessBaseAPIAnalyseLayout() and TessBaseAPIGetIterator() both return pointers
        // to iterator structures. These are safe because:
        // 1. *handle is a valid pointer to an initialized Tesseract engine with recognized text
        // 2. Both functions return either null or valid iterator pointers
        // 3. Each iterator is wrapped in its respective wrapper type which manages cleanup
        let page_iter = unsafe { TessBaseAPIAnalyseLayout(*handle) };
        let result_iter = unsafe { TessBaseAPIGetIterator(*handle) };

        if page_iter.is_null() || result_iter.is_null() {
            // SAFETY: If either iterator is null, we manually clean up the non-null one.
            // This is safe because:
            // 1. We verify each iterator is non-null before calling delete
            // 2. TessPageIteratorDelete and TessResultIteratorDelete handle their respective types
            // 3. Each delete is called exactly once on valid pointers
            if !page_iter.is_null() {
                unsafe { TessPageIteratorDelete(page_iter) };
            }
            if !result_iter.is_null() {
                unsafe { TessResultIteratorDelete(result_iter) };
            }
            return Err(TesseractError::NullPointerError);
        }

        Ok((PageIterator::new(page_iter), ResultIterator::new(result_iter)))
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Drop for TesseractAPI {
    /// Drops the TesseractAPI instance.
    ///
    /// SAFETY: Drop must never panic, so we use `.ok()` to handle potential mutex poisoning.
    fn drop(&mut self) {
        // Use .ok() to avoid panic on poisoned mutex during cleanup
        if let Ok(handle) = self.handle.lock() {
            // SAFETY: We clean up the Tesseract handle by calling FFI functions in the correct order:
            // 1. Verify *handle is non-null before calling delete functions to avoid undefined behavior
            // 2. TessBaseAPIEnd() finalizes engine state (may release resources, but doesn't deallocate)
            // 3. TessBaseAPIDelete() deallocates the opaque handle allocated by TessBaseAPICreate()
            // 4. Calling TessBaseAPIDelete() on null is undefined behavior; we guard with is_null() check
            // 5. Each function is called exactly once per drop (no double-free)
            // 6. Drop impl never panics (we use .ok() on mutex lock), ensuring cleanup always executes
            // 7. If mutex is poisoned, handle cleanup is skipped but OS will clean up process memory
            unsafe {
                if !(*handle).is_null() {
                    TessBaseAPIEnd(*handle);
                    TessBaseAPIDelete(*handle);
                }
            }
        }
        // If mutex is poisoned, we silently ignore and let the OS clean up
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
impl Clone for TesseractAPI {
    /// Clones the TesseractAPI instance, attempting to clone its configuration and state.
    ///
    /// If the mutex is poisoned, defaults to empty configuration.
    /// Initialization errors during cloning are silently ignored to prevent panics
    /// in Clone::clone() (which returns Self, not Result).
    fn clone(&self) -> Self {
        // Get config, using default if mutex is poisoned
        let config = self
            .config
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| TesseractConfiguration {
                datapath: String::new(),
                language: String::new(),
                variables: HashMap::new(),
            });

        let new_handle = unsafe { TessBaseAPICreate() };
        let new_api = TesseractAPI {
            handle: Arc::new(Mutex::new(new_handle)),
            config: Arc::new(Mutex::new(config.clone())),
        };

        // Attempt to initialize, but don't panic if it fails
        // The cloned instance will be in an uninitialized state which will
        // return errors on subsequent operations
        if !config.datapath.is_empty() && new_api.init(&config.datapath, &config.language).is_ok() {
            for (name, value) in &config.variables {
                // Ignore variable setting errors during clone
                let _ = new_api.set_variable(name, value);
            }
        }

        new_api
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
unsafe extern "C-unwind" {
    fn TessBaseAPIMeanTextConf(handle: *mut c_void) -> c_int;
    fn TessBaseAPISetVariable(handle: *mut c_void, name: *const c_char, value: *const c_char) -> c_int;
    fn TessBaseAPIGetStringVariable(handle: *mut c_void, name: *const c_char) -> *const c_char;
    fn TessBaseAPIGetIntVariable(handle: *mut c_void, name: *const c_char) -> c_int;
    fn TessBaseAPIGetBoolVariable(handle: *mut c_void, name: *const c_char) -> c_int;
    fn TessBaseAPIGetDoubleVariable(handle: *mut c_void, name: *const c_char) -> c_double;
    fn TessBaseAPISetPageSegMode(handle: *mut c_void, mode: c_int);
    fn TessBaseAPIGetPageSegMode(handle: *mut c_void) -> c_int;
    fn TessBaseAPIRecognize(handle: *mut c_void, monitor: *mut c_void) -> c_int;
    fn TessBaseAPIGetHOCRText(handle: *mut c_void, page: c_int) -> *mut c_char;

    fn TessBaseAPIGetAltoText(handle: *mut c_void, page: c_int) -> *mut c_char;
    fn TessBaseAPIGetTsvText(handle: *mut c_void, page: c_int) -> *mut c_char;
    fn TessBaseAPIGetBoxText(handle: *mut c_void, page: c_int) -> *mut c_char;
    fn TessBaseAPIGetLSTMBoxText(handle: *mut c_void, page: c_int) -> *mut c_char;
    fn TessBaseAPIGetWordStrBoxText(handle: *mut c_void, page: c_int) -> *mut c_char;
    fn TessBaseAPIGetUNLVText(handle: *mut c_void) -> *mut c_char;
    fn TessBaseAPIAllWordConfidences(handle: *mut c_void) -> *const c_int;
    fn TessBaseAPIAdaptToWordStr(handle: *mut c_void, mode: c_int, wordstr: *const c_char) -> c_int;
    fn TessBaseAPIDetectOrientationScript(
        handle: *mut c_void,
        orient_deg: *mut c_int,
        orient_conf: *mut c_float,
        script_name: *mut *mut c_char,
        script_conf: *mut c_float,
    ) -> c_int;
    fn TessBaseAPISetMinOrientationMargin(handle: *mut c_void, margin: c_double);
    fn TessBaseAPIGetMutableIterator(handle: *mut c_void) -> *mut c_void;
    fn TessDeleteIntArray(arr: *const c_int);
    fn TessBaseAPISetInputImage(handle: *mut c_void, pix: *mut c_void);
    fn TessBaseAPIGetInputImage(handle: *mut c_void) -> *mut c_void;
    fn TessBaseAPISetOutputName(handle: *mut c_void, name: *const c_char);
    fn TessBaseAPISetDebugVariable(handle: *mut c_void, name: *const c_char, value: *const c_char) -> c_int;
    fn TessBaseAPIPrintVariablesToFile(handle: *mut c_void, filename: *const c_char) -> c_int;
    fn TessBaseAPIInitForAnalysePage(handle: *mut c_void);
    fn TessBaseAPIReadConfigFile(handle: *mut c_void, filename: *const c_char);
    fn TessBaseAPIReadDebugConfigFile(handle: *mut c_void, filename: *const c_char);
    fn TessBaseAPIGetThresholdedImageScaleFactor(handle: *mut c_void) -> c_int;
    fn TessBaseAPIAnalyseLayout(handle: *mut c_void) -> *mut c_void;
    fn TessBaseAPIGetInitLanguagesAsString(handle: *mut c_void) -> *const c_char;
    fn TessBaseAPIGetLoadedLanguagesAsVector(handle: *mut c_void) -> *mut *mut c_char;
    fn TessBaseAPIGetAvailableLanguagesAsVector(handle: *mut c_void) -> *mut *mut c_char;
    fn TessBaseAPIClearAdaptiveClassifier(handle: *mut c_void);
    fn TessDeleteTextArray(arr: *mut *mut c_char);

    fn TessVersion() -> *const c_char;
    fn TessBaseAPICreate() -> *mut c_void;
    fn TessBaseAPIDelete(handle: *mut c_void);
    fn TessBaseAPIInit3(handle: *mut c_void, datapath: *const c_char, language: *const c_char) -> c_int;
    fn TessBaseAPIInit1(
        handle: *mut c_void,
        datapath: *const c_char,
        language: *const c_char,
        oem: c_int,
        configs: *const *const c_char,
        configs_size: c_int,
    ) -> c_int;
    fn TessBaseAPIInit2(handle: *mut c_void, datapath: *const c_char, language: *const c_char, oem: c_int) -> c_int;
    fn TessBaseAPIInit4(
        handle: *mut c_void,
        datapath: *const c_char,
        language: *const c_char,
        oem: c_int,
        configs: *const *const c_char,
        configs_size: c_int,
        vars_vec: *const *const c_char,
        vars_values: *const *const c_char,
        vars_vec_size: usize,
        set_only_non_debug_params: c_int,
    ) -> c_int;
    fn TessBaseAPIInit5(
        handle: *mut c_void,
        data: *const u8,
        data_size: c_int,
        language: *const c_char,
        oem: c_int,
        configs: *const *const c_char,
        configs_size: c_int,
        vars_vec: *const *const c_char,
        vars_values: *const *const c_char,
        vars_vec_size: usize,
        set_only_non_debug_params: c_int,
    ) -> c_int;
    fn TessBaseAPISetImage(
        handle: *mut c_void,
        imagedata: *const u8,
        width: c_int,
        height: c_int,
        bytes_per_pixel: c_int,
        bytes_per_line: c_int,
    );
    fn TessBaseAPISetImage2(handle: *mut c_void, pix: *mut c_void);
    fn TessBaseAPISetSourceResolution(handle: *mut c_void, ppi: c_int);
    fn TessBaseAPISetRectangle(handle: *mut c_void, left: c_int, top: c_int, width: c_int, height: c_int);
    fn TessBaseAPIGetUTF8Text(handle: *mut c_void) -> *mut c_char;
    fn TessBaseAPIClear(handle: *mut c_void);
    fn TessBaseAPIEnd(handle: *mut c_void);
    fn TessBaseAPIIsValidWord(handle: *mut c_void, word: *const c_char) -> c_int;
    fn TessBaseAPIGetTextDirection(handle: *mut c_void, out_degrees: *mut c_int, out_confidence: *mut c_float);
    pub fn TessDeleteText(text: *mut c_char);

    fn TessBaseAPIGetUnichar(handle: *mut c_void, unichar_id: c_int) -> *const c_char;

    fn TessBaseAPIProcessPages(
        handle: *mut c_void,
        filename: *const c_char,
        retry_config: *const c_char,
        timeout_millisec: c_int,
        renderer: *mut c_void,
    ) -> *mut c_char;

    fn TessBaseAPIGetInputName(handle: *mut c_void) -> *const c_char;
    fn TessBaseAPISetInputName(handle: *mut c_void, name: *const c_char);
    fn TessBaseAPIGetSourceYResolution(handle: *mut c_void) -> c_int;
    fn TessBaseAPIGetDatapath(handle: *mut c_void) -> *const c_char;
    fn TessBaseAPIGetThresholdedImage(handle: *mut c_void) -> *mut c_void;

    // Batch layout-analysis functions returning Leptonica BOXA* arrays.
    // BOXA* and PIXA* are opaque Leptonica types represented here as *mut c_void.
    fn TessBaseAPIGetWords(handle: *mut c_void, pixa: *mut *mut c_void) -> *mut c_void;
    fn TessBaseAPIGetRegions(handle: *mut c_void, pixa: *mut *mut c_void) -> *mut c_void;
    fn TessBaseAPIGetTextlines1(
        handle: *mut c_void,
        raw_image: c_int,
        raw_padding: c_int,
        pixa: *mut *mut c_void,
        blockids: *mut *mut c_int,
        paraids: *mut *mut c_int,
    ) -> *mut c_void;
    fn TessBaseAPIGetComponentImages(
        handle: *mut c_void,
        level: c_int,
        text_only: c_int,
        pixa: *mut *mut c_void,
        blockids: *mut *mut c_int,
    ) -> *mut c_void;

    // Leptonica BOXA traversal and destruction.
    fn boxaGetCount(boxa: *mut c_void) -> c_int;
    // accessflag: L_NOCOPY=0 returns borrowed interior pointer (valid until boxaDestroy).
    fn boxaGetBox(boxa: *mut c_void, index: c_int, accessflag: c_int) -> *mut c_void;
    fn boxGetGeometry(bx: *mut c_void, px: *mut c_int, py: *mut c_int, pw: *mut c_int, ph: *mut c_int) -> c_int;
    // boxaDestroy sets *pboxa to null after freeing, so we pass *mut *mut c_void.
    fn boxaDestroy(pboxa: *mut *mut c_void);

}
