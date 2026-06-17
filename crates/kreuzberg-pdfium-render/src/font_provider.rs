//! Memory-based font provider for Pdfium.
//!
//! This module provides an in-memory font provider implementation that allows
//! Pdfium to access pre-loaded font data without requiring system font installation.
//!
//! The provider implements the FPDF_SYSFONTINFO interface callbacks, enabling
//! efficient font management through Rust's memory management and Arc for zero-copy
//! sharing of font data.

use crate::bindgen::*;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::sync::Arc;

/// A font descriptor containing pre-loaded font data.
///
/// Use `Arc<[u8]>` for zero-copy sharing when caching fonts in memory.
#[derive(Clone, Debug)]
pub struct FontDescriptor {
    /// Font family name (e.g., "Arial", "Roboto")
    pub family: String,

    /// Font weight (400 = normal, 700 = bold)
    pub weight: i32,

    /// Whether the font is italic
    pub is_italic: bool,

    /// Character set for the font.
    /// Common values:
    /// - 0 = ANSI charset (Western)
    /// - 128 = Shift-JIS charset (Japanese)
    /// - 134 = GB2312 charset (Simplified Chinese)
    /// - 136 = Hangeul charset (Korean)
    pub charset: i32,

    /// Raw font file bytes (TrueType or OpenType).
    /// Use Arc for zero-copy sharing across multiple font provider instances.
    pub data: Arc<[u8]>,
}

/// Font key for HashMap lookups with case-insensitive family name matching.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct FontKey {
    family: String, // Lowercase for case-insensitive matching
    weight: i32,
    is_italic: bool,
    charset: i32,
}

/// Font handle returned to Pdfium by the font provider callbacks.
struct FontHandle {
    key: FontKey,
}

/// Memory-based font provider for Pdfium.
///
/// This struct manages a collection of pre-loaded fonts and provides callbacks
/// for Pdfium's FPDF_SYSFONTINFO interface.
pub(crate) struct MemoryFontProvider {
    sys_font_info: FPDF_SYSFONTINFO,
    fonts: HashMap<FontKey, Arc<[u8]>>,
}

impl MemoryFontProvider {
    /// Create a new memory font provider from a list of font descriptors.
    pub(crate) fn new(descriptors: Vec<FontDescriptor>) -> Self {
        // Build HashMap from descriptors with lowercase family names for case-insensitive matching
        let mut fonts = HashMap::new();
        for descriptor in descriptors {
            let key = FontKey {
                family: descriptor.family.to_lowercase(),
                weight: descriptor.weight,
                is_italic: descriptor.is_italic,
                charset: descriptor.charset,
            };
            fonts.insert(key, descriptor.data);
        }

        // Initialize FPDF_SYSFONTINFO with version 2 (per-request behavior)
        // and set callbacks
        let sys_font_info = FPDF_SYSFONTINFO {
            version: 2,
            Release: Some(release_callback),
            EnumFonts: None, // Version 2 doesn't use EnumFonts
            MapFont: Some(map_font_callback),
            GetFont: Some(get_font_callback),
            GetFontData: Some(get_font_data_callback),
            GetFaceName: None,
            GetFontCharset: None,
            DeleteFont: Some(delete_font_callback),
        };

        MemoryFontProvider { sys_font_info, fonts }
    }

    /// Get a mutable pointer to the FPDF_SYSFONTINFO structure for Pdfium.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut FPDF_SYSFONTINFO {
        &mut self.sys_font_info as *mut FPDF_SYSFONTINFO
    }

    /// Reconstruct a mutable reference to the MemoryFontProvider from pThis pointer.
    ///
    /// # Safety
    ///
    /// This function assumes that pThis is a valid pointer to a MemoryFontProvider
    /// instance that was stored via Box::leak().
    unsafe fn from_pthis<'a>(pthis: *mut FPDF_SYSFONTINFO) -> &'a mut Self {
        unsafe { &mut *(pthis as *mut MemoryFontProvider) }
    }
}

/// Release callback - no-op since we leak via Box::leak.
///
/// Called when Pdfium no longer needs the font info interface.
/// We don't actually free the memory here because we've leaked it via Box::leak
/// to ensure it lives for the duration of the Pdfium library.
unsafe extern "C" fn release_callback(_pthis: *mut FPDF_SYSFONTINFO) {
    // No-op: we leak the provider via Box::leak to ensure it persists
    // for the lifetime of the Pdfium library
}

/// MapFont callback - match font with 3-tier fallback strategy.
///
/// Called by Pdfium to map a font request to an available font handle.
/// Uses a 3-tier fallback strategy:
/// 1. Exact match (family, weight, italic, charset)
/// 2. Fallback to weight=400 (normal weight) if exact match fails
/// 3. Fallback to any font in the family if weight-specific match fails
unsafe extern "C" fn map_font_callback(
    pthis: *mut FPDF_SYSFONTINFO,
    weight: c_int,
    bitalic: FPDF_BOOL,
    charset: c_int,
    _pitch_family: c_int,
    face: *const c_char,
    bexact: *mut FPDF_BOOL,
) -> *mut c_void {
    unsafe {
        // Safely handle null pointers
        if pthis.is_null() || face.is_null() {
            return ptr::null_mut();
        }

        // Convert C string to Rust string
        let face_name = match CStr::from_ptr(face).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        let provider = MemoryFontProvider::from_pthis(pthis);
        let face_lower = face_name.to_lowercase();
        let is_italic = bitalic != 0;

        // Tier 1: Try exact match (family, weight, italic, charset)
        let exact_key = FontKey {
            family: face_lower.clone(),
            weight,
            is_italic,
            charset,
        };

        if provider.fonts.contains_key(&exact_key) {
            if !bexact.is_null() {
                *bexact = 1;
            }
            let handle = Box::new(FontHandle { key: exact_key });
            return Box::into_raw(handle) as *mut c_void;
        }

        // Tier 2: Fallback to weight=400 (normal weight) if exact match fails
        let normal_weight_key = FontKey {
            family: face_lower.clone(),
            weight: 400,
            is_italic,
            charset,
        };

        if provider.fonts.contains_key(&normal_weight_key) {
            if !bexact.is_null() {
                *bexact = 0;
            }
            let handle = Box::new(FontHandle { key: normal_weight_key });
            return Box::into_raw(handle) as *mut c_void;
        }

        // Tier 3: Fallback to any font in the family (ignoring weight and charset)
        for key in provider.fonts.keys() {
            if key.family == face_lower && key.is_italic == is_italic {
                if !bexact.is_null() {
                    *bexact = 0;
                }
                let handle = Box::new(FontHandle { key: key.clone() });
                return Box::into_raw(handle) as *mut c_void;
            }
        }

        // No match found
        ptr::null_mut()
    }
}

/// GetFont callback - get first font matching family name.
///
/// Called by Pdfium to retrieve a font handle by family name.
unsafe extern "C" fn get_font_callback(pthis: *mut FPDF_SYSFONTINFO, face: *const c_char) -> *mut c_void {
    unsafe {
        // Safely handle null pointers
        if pthis.is_null() || face.is_null() {
            return ptr::null_mut();
        }

        // Convert C string to Rust string
        let face_name = match CStr::from_ptr(face).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };

        let provider = MemoryFontProvider::from_pthis(pthis);
        let face_lower = face_name.to_lowercase();

        // Find the first font matching the family name
        for key in provider.fonts.keys() {
            if key.family == face_lower {
                let handle = Box::new(FontHandle { key: key.clone() });
                return Box::into_raw(handle) as *mut c_void;
            }
        }

        // No font found
        ptr::null_mut()
    }
}

/// GetFontData callback - return font bytes.
///
/// Called by Pdfium to retrieve font data. Supports table=0 for full font file only.
/// Returns 0 for specific table requests as we only support full file access.
unsafe extern "C" fn get_font_data_callback(
    pthis: *mut FPDF_SYSFONTINFO,
    hfont: *mut c_void,
    table: c_uint,
    buffer: *mut c_uchar,
    buf_size: c_ulong,
) -> c_ulong {
    unsafe {
        // Safely handle null pointers
        if pthis.is_null() || hfont.is_null() {
            return 0;
        }

        // Only support table=0 (full font file), return 0 for specific tables
        if table != 0 {
            return 0;
        }

        let provider = MemoryFontProvider::from_pthis(pthis);
        let handle = &*(hfont as *const FontHandle);

        // Get font data from provider
        let font_data = match provider.fonts.get(&handle.key) {
            Some(data) => data,
            None => return 0,
        };

        let font_size = font_data.len() as c_ulong;

        // If buffer is null, return the size of the font data
        if buffer.is_null() {
            return font_size;
        }

        // If buffer provided, copy the font data (up to buf_size bytes)
        let copy_size = std::cmp::min(buf_size, font_size) as usize;
        if copy_size > 0 {
            ptr::copy_nonoverlapping(font_data.as_ptr(), buffer, copy_size);
        }

        copy_size as c_ulong
    }
}

/// DeleteFont callback - drop the font handle.
///
/// Called by Pdfium when it no longer needs a font handle.
unsafe extern "C" fn delete_font_callback(_pthis: *mut FPDF_SYSFONTINFO, hfont: *mut c_void) {
    unsafe {
        // Safely handle null pointers
        if hfont.is_null() {
            return;
        }

        // Reconstruct and drop the FontHandle, freeing its memory
        let _handle = Box::from_raw(hfont as *mut FontHandle);
        // FontHandle is dropped here
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_key_case_insensitive() {
        let key1 = FontKey {
            family: "Arial".to_lowercase(),
            weight: 400,
            is_italic: false,
            charset: 0,
        };

        let key2 = FontKey {
            family: "arial".to_lowercase(),
            weight: 400,
            is_italic: false,
            charset: 0,
        };

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_font_descriptor_creation() {
        let data: Arc<[u8]> = Arc::from(vec![0u8, 1u8, 2u8].into_boxed_slice());
        let descriptor = FontDescriptor {
            family: "TestFont".to_string(),
            weight: 400,
            is_italic: false,
            charset: 0,
            data,
        };

        assert_eq!(descriptor.family, "TestFont");
        assert_eq!(descriptor.weight, 400);
        assert!(!descriptor.is_italic);
        assert_eq!(descriptor.charset, 0);
    }

    #[test]
    fn test_memory_font_provider_empty() {
        let provider = MemoryFontProvider::new(vec![]);
        assert_eq!(provider.fonts.len(), 0);
    }

    #[test]
    fn test_memory_font_provider_with_fonts() {
        let data: Arc<[u8]> = Arc::from(vec![0u8; 100].into_boxed_slice());
        let descriptors = vec![
            FontDescriptor {
                family: "Arial".to_string(),
                weight: 400,
                is_italic: false,
                charset: 0,
                data: data.clone(),
            },
            FontDescriptor {
                family: "Arial".to_string(),
                weight: 700,
                is_italic: false,
                charset: 0,
                data: data.clone(),
            },
        ];

        let provider = MemoryFontProvider::new(descriptors);
        assert_eq!(provider.fonts.len(), 2);
    }

    #[test]
    fn test_memory_font_provider_sys_font_info() {
        let provider = MemoryFontProvider::new(vec![]);
        assert_eq!(provider.sys_font_info.version, 2);
        assert!(provider.sys_font_info.Release.is_some());
        assert!(provider.sys_font_info.MapFont.is_some());
        assert!(provider.sys_font_info.GetFont.is_some());
        assert!(provider.sys_font_info.GetFontData.is_some());
        assert!(provider.sys_font_info.DeleteFont.is_some());
        assert!(provider.sys_font_info.EnumFonts.is_none());
    }
}
