use crate::TesseractError;
use crate::enums::{
    TessOrientation, TessPageIteratorLevel, TessParagraphJustification, TessPolyBlockType, TessTextlineOrder,
    TessWritingDirection,
};
use crate::error::Result;
use std::os::raw::{c_float, c_int, c_void};
use std::sync::Arc;
use std::sync::Mutex;

/// Block-level layout information from Tesseract.
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub block_type: TessPolyBlockType,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Paragraph-level information from Tesseract.
#[derive(Debug, Clone)]
pub struct ParaInfo {
    pub justification: TessParagraphJustification,
    pub is_list_item: bool,
    pub is_crown: bool,
    pub first_line_indent: i32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub struct PageIterator {
    pub handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for PageIterator {}
unsafe impl Sync for PageIterator {}

impl PageIterator {
    /// Creates a new instance of the PageIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the PageIterator.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the PageIterator.
    pub fn new(handle: *mut c_void) -> Self {
        PageIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Begins the iteration.
    pub fn begin(&self) -> Result<()> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        unsafe { TessPageIteratorBegin(*handle) };
        Ok(())
    }

    /// Gets the next iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the iterator.
    ///
    /// # Returns
    ///
    /// Returns `Result<bool>` - `Ok(true)` if the next iterator is successful, `Ok(false)` otherwise.
    pub fn next(&self, level: TessPageIteratorLevel) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessPageIteratorNext(*handle, level as c_int) != 0 })
    }

    /// Checks if the current iterator is at the beginning of the specified level.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the iterator.
    ///
    /// # Returns
    ///
    /// Returns `Result<bool>` - `Ok(true)` if at the beginning, `Ok(false)` otherwise.
    pub fn is_at_beginning_of(&self, level: TessPageIteratorLevel) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessPageIteratorIsAtBeginningOf(*handle, level as c_int) != 0 })
    }

    /// Checks if the current iterator is at the final element of the specified level.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the iterator.
    /// * `element` - Element of the iterator.
    ///
    /// # Returns
    ///
    /// Returns `Result<bool>` - `Ok(true)` if at the final element, `Ok(false)` otherwise.
    pub fn is_at_final_element(&self, level: TessPageIteratorLevel, element: TessPageIteratorLevel) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessPageIteratorIsAtFinalElement(*handle, level as c_int, element as c_int) != 0 })
    }

    /// Gets the bounding box of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the bounding box.
    ///
    /// # Returns
    ///
    /// Returns the bounding box as a tuple if successful, otherwise returns an error.
    pub fn bounding_box(&self, level: TessPageIteratorLevel) -> Result<(i32, i32, i32, i32)> {
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let result = unsafe {
            TessPageIteratorBoundingBox(*handle, level as c_int, &mut left, &mut top, &mut right, &mut bottom)
        };
        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((left, top, right, bottom))
        }
    }

    /// Gets the block type of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the block type as a `TessPolyBlockType`.
    pub fn block_type(&self) -> Result<TessPolyBlockType> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let block_type = unsafe { TessPageIteratorBlockType(*handle) };
        Ok(TessPolyBlockType::from_int(block_type))
    }

    /// Gets the baseline of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the baseline.
    ///
    /// # Returns
    ///
    /// Returns the baseline as a tuple if successful, otherwise returns an error.
    pub fn baseline(&self, level: i32) -> Result<(i32, i32, i32, i32)> {
        let mut x1 = 0;
        let mut y1 = 0;
        let mut x2 = 0;
        let mut y2 = 0;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let result = unsafe { TessPageIteratorBaseline(*handle, level, &mut x1, &mut y1, &mut x2, &mut y2) };
        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((x1, y1, x2, y2))
        }
    }

    /// Gets the orientation of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the orientation as a tuple if successful, otherwise returns an error.
    pub fn orientation(&self) -> Result<(TessOrientation, TessWritingDirection, TessTextlineOrder, f32)> {
        let mut orientation = 0;
        let mut writing_direction = 0;
        let mut textline_order = 0;
        let mut deskew_angle = 0.0;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let result = unsafe {
            TessPageIteratorOrientation(
                *handle,
                &mut orientation,
                &mut writing_direction,
                &mut textline_order,
                &mut deskew_angle,
            )
        };
        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((
                TessOrientation::from_int(orientation),
                TessWritingDirection::from_int(writing_direction),
                TessTextlineOrder::from_int(textline_order),
                deskew_angle,
            ))
        }
    }

    /// Extracts all blocks from the page in a single mutex-locked pass.
    ///
    /// Resets the iterator to the beginning, then iterates at `RIL_BLOCK` level,
    /// collecting block type and bounding box for each block found.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<BlockInfo>)` with one entry per block, or an error if the
    /// mutex cannot be acquired.
    pub fn extract_all_blocks(&self) -> Result<Vec<BlockInfo>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let level = TessPageIteratorLevel::RIL_BLOCK as c_int;
        let mut blocks = Vec::new();

        // SAFETY: `*handle` is a valid non-null TessPageIterator pointer owned by this struct.
        // `TessPageIteratorBegin` resets the iterator to the first element and takes only
        // the pointer — no aliasing occurs because we hold the mutex for the duration.
        unsafe { TessPageIteratorBegin(*handle) };

        loop {
            let block_type = unsafe {
                // SAFETY: `*handle` is valid; TessPageIteratorBlockType reads the current
                // iterator position and returns an integer enum value without taking ownership.
                TessPageIteratorBlockType(*handle)
            };

            let mut left: c_int = 0;
            let mut top: c_int = 0;
            let mut right: c_int = 0;
            let mut bottom: c_int = 0;

            let bbox_ok = unsafe {
                // SAFETY: `*handle` is valid; the four `*mut c_int` pointers point to local
                // stack variables whose lifetimes exceed this call.
                TessPageIteratorBoundingBox(*handle, level, &mut left, &mut top, &mut right, &mut bottom)
            };

            if bbox_ok != 0 {
                blocks.push(BlockInfo {
                    block_type: TessPolyBlockType::from_int(block_type),
                    left,
                    top,
                    right,
                    bottom,
                });
            }

            let has_next = unsafe {
                // SAFETY: `*handle` is valid; TessPageIteratorNext advances the iterator
                // in-place and returns 0 when there are no more elements at this level.
                TessPageIteratorNext(*handle, level)
            };
            if has_next == 0 {
                break;
            }
        }

        Ok(blocks)
    }

    /// Extracts all paragraphs from the page in a single mutex-locked pass.
    ///
    /// Resets the iterator to the beginning, then iterates at `RIL_PARA` level,
    /// collecting paragraph metadata and bounding box for each paragraph found.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<ParaInfo>)` with one entry per paragraph, or an error if the
    /// mutex cannot be acquired.
    pub fn extract_all_paragraphs(&self) -> Result<Vec<ParaInfo>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let level = TessPageIteratorLevel::RIL_PARA as c_int;
        let mut paragraphs = Vec::new();

        // SAFETY: `*handle` is a valid non-null TessPageIterator pointer owned by this struct.
        // `TessPageIteratorBegin` resets the iterator to the first element; the mutex ensures
        // exclusive access for the entire loop.
        unsafe { TessPageIteratorBegin(*handle) };

        loop {
            let mut justification: c_int = 0;
            // SAFETY: TessPageIteratorParagraphInfo expects BOOL* (int*) for is_list_item and
            // is_crown. Rust bool is 1 byte while C int is 4 bytes, so we use c_int temporaries
            // to avoid undefined behaviour (stack corruption) and convert afterwards.
            let mut is_list_item_raw: c_int = 0;
            let mut is_crown_raw: c_int = 0;
            let mut first_line_indent: c_int = 0;

            let para_ok = unsafe {
                // SAFETY: `*handle` is valid; all output pointers reference stack variables
                // whose lifetimes exceed this call. TessPageIteratorParagraphInfo writes
                // through these pointers without retaining them.
                TessPageIteratorParagraphInfo(
                    *handle,
                    &mut justification,
                    &mut is_list_item_raw,
                    &mut is_crown_raw,
                    &mut first_line_indent,
                )
            };

            let is_list_item = is_list_item_raw != 0;
            let is_crown = is_crown_raw != 0;

            let mut left: c_int = 0;
            let mut top: c_int = 0;
            let mut right: c_int = 0;
            let mut bottom: c_int = 0;

            let bbox_ok = unsafe {
                // SAFETY: `*handle` is valid; the four `*mut c_int` pointers reference local
                // stack variables. TessPageIteratorBoundingBox does not retain these pointers.
                TessPageIteratorBoundingBox(*handle, level, &mut left, &mut top, &mut right, &mut bottom)
            };

            if para_ok != 0 && bbox_ok != 0 {
                paragraphs.push(ParaInfo {
                    justification: TessParagraphJustification::from_int(justification),
                    is_list_item,
                    is_crown,
                    first_line_indent,
                    left,
                    top,
                    right,
                    bottom,
                });
            }

            let has_next = unsafe {
                // SAFETY: `*handle` is valid; TessPageIteratorNext advances the iterator
                // in-place and returns 0 when there are no more elements at this level.
                TessPageIteratorNext(*handle, level)
            };
            if has_next == 0 {
                break;
            }
        }

        Ok(paragraphs)
    }

    /// Gets the paragraph information of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the paragraph information as a tuple if successful, otherwise returns an error.
    pub fn paragraph_info(&self) -> Result<(TessParagraphJustification, bool, bool, i32)> {
        let mut justification = 0;
        // SAFETY: TessPageIteratorParagraphInfo expects BOOL* (int*) for is_list_item and
        // is_crown. Rust bool is 1 byte while C int is 4 bytes, so we use c_int temporaries
        // to avoid undefined behaviour (stack corruption) and convert afterwards.
        let mut is_list_item_raw: c_int = 0;
        let mut is_crown_raw: c_int = 0;
        let mut first_line_indent = 0;
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let result = unsafe {
            TessPageIteratorParagraphInfo(
                *handle,
                &mut justification,
                &mut is_list_item_raw,
                &mut is_crown_raw,
                &mut first_line_indent,
            )
        };
        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((
                TessParagraphJustification::from_int(justification),
                is_list_item_raw != 0,
                is_crown_raw != 0,
                first_line_indent,
            ))
        }
    }
}

impl Drop for PageIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            unsafe { TessPageIteratorDelete(*handle) };
        }
    }
}

unsafe extern "C-unwind" {
    pub fn TessPageIteratorDelete(handle: *mut c_void);
    pub fn TessPageIteratorBegin(handle: *mut c_void);
    pub fn TessPageIteratorNext(handle: *mut c_void, level: c_int) -> c_int;
    pub fn TessPageIteratorIsAtBeginningOf(handle: *mut c_void, level: c_int) -> c_int;
    pub fn TessPageIteratorIsAtFinalElement(handle: *mut c_void, level: c_int, element: c_int) -> c_int;
    pub fn TessPageIteratorBoundingBox(
        handle: *mut c_void,
        level: c_int,
        left: *mut c_int,
        top: *mut c_int,
        right: *mut c_int,
        bottom: *mut c_int,
    ) -> c_int;
    pub fn TessPageIteratorBlockType(handle: *mut c_void) -> c_int;
    pub fn TessPageIteratorBaseline(
        handle: *mut c_void,
        level: c_int,
        x1: *mut c_int,
        y1: *mut c_int,
        x2: *mut c_int,
        y2: *mut c_int,
    ) -> c_int;
    pub fn TessPageIteratorOrientation(
        handle: *mut c_void,
        orientation: *mut c_int,
        writing_direction: *mut c_int,
        textline_order: *mut c_int,
        deskew_angle: *mut c_float,
    ) -> c_int;
    pub fn TessBaseAPIGetIterator(handle: *mut c_void) -> *mut c_void;
    pub fn TessPageIteratorParagraphInfo(
        handle: *mut c_void,
        justification: *mut c_int,
        is_list_item: *mut c_int,
        is_crown: *mut c_int,
        first_line_indent: *mut c_int,
    ) -> c_int;
}
