//! Defines the [Pdfium] struct, a high-level idiomatic Rust wrapper around Pdfium.

use crate::bindings::PdfiumLibraryBindings;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::font_provider::FontDescriptor;
use crate::pdf::document::{PdfDocument, PdfDocumentVersion};
use once_cell::sync::OnceCell;
use std::ffi::CString;
use std::fmt::{Debug, Formatter};

#[cfg(all(not(target_arch = "wasm32"), not(pdfium_use_static)))]
use {
    crate::bindings::dynamic_bindings::DynamicPdfiumBindings, libloading::Library, std::ffi::OsString,
    std::path::PathBuf,
};

#[cfg(all(not(target_arch = "wasm32"), pdfium_use_static))]
use crate::bindings::static_bindings::StaticPdfiumBindings;

#[cfg(not(target_arch = "wasm32"))]
use {
    crate::utils::files::get_pdfium_file_accessor_from_reader,
    std::fs::File,
    std::io::{Read, Seek},
    std::path::Path,
};

#[cfg(target_arch = "wasm32")]
use {
    crate::bindings::wasm_bindings::{PdfiumRenderWasmState, WasmPdfiumBindings},
    js_sys::{ArrayBuffer, Uint8Array},
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::JsFuture,
    web_sys::{Blob, Response, window},
};

// The following dummy declaration is used only when running cargo doc.
// It allows documentation of WASM-specific functionality to be included
// in documentation generated on non-WASM targets.

#[cfg(doc)]
struct Blob;

// The first instantiation of a Pdfium object will promote a concrete PdfiumLibraryBindings
// trait implementation into a global static OnceCell. This allows for thread-safe,
// lifetime-free access to that PdfiumLibraryBindings instance from any object that
// implements the PdfiumLibraryBindingsAccessor trait.

static BINDINGS: OnceCell<Box<dyn PdfiumLibraryBindings>> = OnceCell::new();

#[cfg(feature = "thread_safe")]
pub(crate) trait PdfiumLibraryBindingsAccessor: Send + Sync {
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        BINDINGS.wait().as_ref()
    }
}

#[cfg(not(feature = "thread_safe"))]
pub(crate) trait PdfiumLibraryBindingsAccessor {
    fn bindings(&self) -> &dyn PdfiumLibraryBindings {
        BINDINGS.get().unwrap().as_ref()
    }
}

/// Configuration options for initializing the Pdfium library.
#[derive(Debug, Default, Clone)]
pub struct PdfiumConfig {
    user_font_paths: Option<Vec<String>>,
    font_provider: Option<Vec<FontDescriptor>>,
}

impl PdfiumConfig {
    /// Creates a new [PdfiumConfig] with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the paths to scan for fonts in addition to the default system paths.
    ///
    /// This is useful when you want to use custom fonts that are not installed on the system.
    pub fn set_user_font_paths(mut self, paths: Vec<String>) -> Self {
        self.user_font_paths = Some(paths);
        self
    }

    /// Sets a custom font provider with pre-loaded font data.
    ///
    /// This bypasses all filesystem scanning and serves fonts directly from memory.
    /// Fonts are matched by family name, weight, italic style, and charset.
    ///
    /// # Example
    /// ```rust,no_run
    /// use pdfium_render::prelude::*;
    /// use std::sync::Arc;
    ///
    /// let fonts = vec![
    ///     FontDescriptor {
    ///         family: "Arial".to_string(),
    ///         weight: 400,
    ///         is_italic: false,
    ///         charset: 0,
    ///         data: Arc::from(std::fs::read("/fonts/Arial.ttf")?),
    ///     },
    /// ];
    ///
    /// let config = PdfiumConfig::new()
    ///     .set_font_provider(fonts);
    /// # Ok::<(), pdfium_render::error::PdfiumError>(())
    /// ```
    pub fn set_font_provider(mut self, fonts: Vec<FontDescriptor>) -> Self {
        self.font_provider = Some(fonts);
        self
    }
}

/// A high-level idiomatic Rust wrapper around Pdfium, the C++ PDF library used by
/// the Google Chromium project.
#[derive(Clone)]
pub struct Pdfium;

impl Pdfium {
    /// Binds to a Pdfium library that was statically linked into the currently running
    /// executable, returning a new [PdfiumLibraryBindings] object that contains bindings to the
    /// functions exposed by the library. The application will immediately crash if Pdfium
    /// was not correctly statically linked into the executable at compile time.
    ///
    /// This function is only available when this crate's `static` feature is enabled.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(any(doc, pdfium_use_static))]
    #[inline]
    pub fn bind_to_statically_linked_library() -> Result<Box<dyn PdfiumLibraryBindings>, PdfiumError> {
        if BINDINGS.get().is_none() {
            let bindings = StaticPdfiumBindings::new();

            Ok(Box::new(bindings))
        } else {
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized)
        }
    }

    /// Initializes the external Pdfium library, loading it from the system libraries.
    /// Returns a new [PdfiumLibraryBindings] object that contains bindings to the functions exposed
    /// by the library, or an error if the library could not be loaded.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(pdfium_use_static))]
    #[inline]
    pub fn bind_to_system_library() -> Result<Box<dyn PdfiumLibraryBindings>, PdfiumError> {
        if BINDINGS.get().is_none() {
            let bindings = DynamicPdfiumBindings::new(
                unsafe { Library::new(Self::pdfium_platform_library_name()) }.map_err(PdfiumError::LoadLibraryError)?,
            )?;

            Ok(Box::new(bindings))
        } else {
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized)
        }
    }

    /// Initializes the external Pdfium library, binding to an external WASM module.
    /// Returns a new [PdfiumLibraryBindings] object that contains bindings to the functions exposed
    /// by the library, or an error if the library is not available.
    ///
    /// It is essential that the exported `initialize_pdfium_render()` function be called
    /// from Javascript _before_ calling this function from within your Rust code. For an example, see:
    /// <https://github.com/ajrcarey/pdfium-render/blob/master/examples/index.html>
    #[cfg(target_arch = "wasm32")]
    #[inline]
    pub fn bind_to_system_library() -> Result<Box<dyn PdfiumLibraryBindings>, PdfiumError> {
        if PdfiumRenderWasmState::lock().is_ready() {
            let bindings = WasmPdfiumBindings::new();

            Ok(Box::new(bindings))
        } else {
            Err(PdfiumError::PdfiumWasmModuleNotInitialized)
        }
    }

    /// Initializes the external pdfium library, loading it from the given path.
    /// Returns a new [PdfiumLibraryBindings] object that contains bindings to the functions
    /// exposed by the library, or an error if the library could not be loaded.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(pdfium_use_static))]
    #[inline]
    pub fn bind_to_library(path: impl AsRef<Path>) -> Result<Box<dyn PdfiumLibraryBindings>, PdfiumError> {
        if BINDINGS.get().is_none() {
            let bindings = DynamicPdfiumBindings::new(
                unsafe { Library::new(path.as_ref().as_os_str()) }.map_err(PdfiumError::LoadLibraryError)?,
            )?;

            Ok(Box::new(bindings))
        } else {
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized)
        }
    }

    /// Returns the name of the external Pdfium library on the currently running platform.
    /// On Linux and Android, this will be `libpdfium.so` or similar; on Windows, this will
    /// be `pdfium.dll` or similar; on MacOS, this will be `libpdfium.dylib` or similar.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(pdfium_use_static))]
    #[inline]
    pub fn pdfium_platform_library_name() -> OsString {
        libloading::library_filename("pdfium")
    }

    /// Returns the name of the external Pdfium library on the currently running platform,
    /// prefixed with the given path string.
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(pdfium_use_static))]
    #[inline]
    pub fn pdfium_platform_library_name_at_path(path: &(impl AsRef<Path> + ?Sized)) -> PathBuf {
        path.as_ref().join(Pdfium::pdfium_platform_library_name())
    }

    /// Creates a new [Pdfium] instance from the given external Pdfium library bindings.
    #[inline]
    pub fn new(bindings: Box<dyn PdfiumLibraryBindings>) -> Self {
        Pdfium::new_with_config(bindings, &PdfiumConfig::default())
    }

    /// Creates a new [Pdfium] instance from the given external Pdfium library bindings and configuration.
    ///
    /// # Performance Note
    ///
    /// When no configuration is provided (default `PdfiumConfig`), this method uses the simple
    /// `FPDF_InitLibrary()` function to avoid potential font enumeration overhead that can occur
    /// with `FPDF_InitLibraryWithConfig()`. This optimization eliminates ~52ms overhead on
    /// documents with many fonts (e.g., academic PDFs with 18+ custom Type 1 fonts).
    ///
    /// Configuration features (`user_font_paths` and `font_provider`) are only initialized when
    /// explicitly set, ensuring zero overhead when not in use.
    #[inline]
    pub fn new_with_config(bindings: Box<dyn PdfiumLibraryBindings>, config: &PdfiumConfig) -> Self {
        assert!(BINDINGS.get().is_none());

        // Fast path: if no config is provided, use simple FPDF_InitLibrary() to avoid
        // potential font enumeration overhead in FPDF_InitLibraryWithConfig()
        let has_user_font_paths =
            config.user_font_paths.is_some() && !config.user_font_paths.as_ref().unwrap().is_empty();
        let has_font_provider = config.font_provider.is_some() && !config.font_provider.as_ref().unwrap().is_empty();

        if !has_user_font_paths && !has_font_provider {
            // No configuration needed - use simple initialization
            bindings.FPDF_InitLibrary();
        } else {
            // Configuration needed - build config structure
            let mut c_strings = Vec::new();
            let mut c_ptrs = Vec::new();

            if let Some(paths) = &config.user_font_paths {
                for path in paths {
                    // We ignore paths that contain null bytes, as they cannot be passed to C APIs.
                    if let Ok(c_str) = CString::new(path.as_str()) {
                        c_ptrs.push(c_str.as_ptr());
                        c_strings.push(c_str);
                    }
                }
                c_ptrs.push(std::ptr::null());
            }

            let font_paths_ptr = if c_ptrs.is_empty() {
                std::ptr::null_mut()
            } else {
                // Leak vectors to ensure font path pointers remain valid for library lifetime
                Box::leak(c_strings.into_boxed_slice());

                let leaked_ptrs = Box::leak(c_ptrs.into_boxed_slice());
                leaked_ptrs.as_mut_ptr()
            };

            let library_config = crate::bindgen::FPDF_LIBRARY_CONFIG_ {
                version: 2,
                m_pUserFontPaths: font_paths_ptr,
                m_pIsolate: std::ptr::null_mut(),
                m_v8EmbedderSlot: 0,
                m_pPlatform: std::ptr::null_mut(),
                m_RendererType: 0,
            };

            bindings
                .FPDF_InitLibraryWithConfig(&library_config as *const _ as *const crate::bindgen::FPDF_LIBRARY_CONFIG);

            // Set up custom font provider if configured
            if let Some(font_descriptors) = &config.font_provider
                && !font_descriptors.is_empty()
            {
                use crate::font_provider::MemoryFontProvider;

                // Create provider and box it immediately
                let provider = MemoryFontProvider::new(font_descriptors.clone());
                let mut boxed_provider = Box::new(provider);

                // Get pointer from the boxed provider
                let provider_ptr = boxed_provider.as_mut_ptr();

                // Leak the provider to ensure static lifetime
                let _leaked_provider = Box::leak(boxed_provider);

                // Register with Pdfium
                bindings.FPDF_SetSystemFontInfo(provider_ptr);
            }
        }

        assert!(BINDINGS.set(bindings).is_ok());

        Self {}
    }

    /// Attempts to open a [PdfDocument] from the given static byte buffer.
    ///
    /// If the document is password protected, the given password will be used to unlock it.
    pub fn load_pdf_from_byte_slice<'a>(
        &'a self,
        bytes: &'a [u8],
        password: Option<&str>,
    ) -> Result<PdfDocument<'a>, PdfiumError> {
        Self::pdfium_document_handle_to_result(self.bindings().FPDF_LoadMemDocument64(bytes, password), self.bindings())
    }

    /// Attempts to open a [PdfDocument] from the given owned byte buffer.
    ///
    /// If the document is password protected, the given password will be used to unlock it.
    ///
    /// `pdfium-render` will take ownership of the given byte buffer, ensuring its lifetime lasts
    /// as long as the [PdfDocument] opened from it.
    pub fn load_pdf_from_byte_vec(
        &self,
        bytes: Vec<u8>,
        password: Option<&str>,
    ) -> Result<PdfDocument<'_>, PdfiumError> {
        Self::pdfium_document_handle_to_result(
            self.bindings().FPDF_LoadMemDocument64(bytes.as_slice(), password),
            self.bindings(),
        )
        .map(|mut document| {
            // Give the newly-created document ownership of the byte buffer, so that Pdfium can continue
            // to read from it on an as-needed basis throughout the lifetime of the document.

            document.set_source_byte_buffer(bytes);

            document
        })
    }

    /// Attempts to open a [PdfDocument] from the given file path.
    ///
    /// If the document is password protected, the given password will be used
    /// to unlock it.
    ///
    /// This function is not available when compiling to WASM. You have several options for
    /// loading your PDF document data in WASM:
    /// * Use the [Pdfium::load_pdf_from_fetch()] function to download document data from a
    ///   URL using the browser's built-in `fetch` API. This function is only available when
    ///   compiling to WASM.
    /// * Use the [Pdfium::load_pdf_from_blob()] function to load document data from a
    ///   Javascript `File` or `Blob` object (such as a `File` object returned from an HTML
    ///   `<input type="file">` element). This function is only available when compiling to WASM.
    /// * Use another method to retrieve the bytes of the target document over the network,
    ///   then load those bytes into Pdfium using either the [Pdfium::load_pdf_from_byte_slice()]
    ///   function or the [Pdfium::load_pdf_from_byte_vec()] function.
    /// * Embed the bytes of the target document directly into the compiled WASM module
    ///   using the `include_bytes!` macro.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_pdf_from_file<'a>(
        &'a self,
        path: &(impl AsRef<Path> + ?Sized),
        password: Option<&'a str>,
    ) -> Result<PdfDocument<'a>, PdfiumError> {
        self.load_pdf_from_reader(File::open(path).map_err(PdfiumError::IoError)?, password)
    }

    /// Attempts to open a [PdfDocument] from the given reader.
    ///
    /// Pdfium will only load the portions of the document it actually needs into memory.
    /// This is more efficient than loading the entire document into memory, especially when
    /// working with large documents, and allows for working with documents larger than the
    /// amount of available memory.
    ///
    /// Because Pdfium must know the total content length in advance prior to loading
    /// any portion of it, the given reader must implement the [Seek] trait as well as
    /// the [Read] trait.
    ///
    /// If the document is password protected, the given password will be used
    /// to unlock it.
    ///
    /// This function is not available when compiling to WASM. You have several options for
    /// loading your PDF document data in WASM:
    /// * Use the [Pdfium::load_pdf_from_fetch()] function to download document data from a
    ///   URL using the browser's built-in `fetch` API. This function is only available when
    ///   compiling to WASM.
    /// * Use the [Pdfium::load_pdf_from_blob()] function to load document data from a
    ///   Javascript `File` or `Blob` object (such as a `File` object returned from an HTML
    ///   `<input type="file">` element). This function is only available when compiling to WASM.
    /// * Use another method to retrieve the bytes of the target document over the network,
    ///   then load those bytes into Pdfium using either the [Pdfium::load_pdf_from_byte_slice()]
    ///   function or the [Pdfium::load_pdf_from_byte_vec()] function.
    /// * Embed the bytes of the target document directly into the compiled WASM module
    ///   using the `include_bytes!` macro.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_pdf_from_reader<'a, R: Read + Seek + 'a>(
        &'a self,
        reader: R,
        password: Option<&'a str>,
    ) -> Result<PdfDocument<'a>, PdfiumError> {
        let mut reader = get_pdfium_file_accessor_from_reader(reader);

        Pdfium::pdfium_document_handle_to_result(
            self.bindings()
                .FPDF_LoadCustomDocument(reader.as_fpdf_file_access_mut_ptr(), password),
            self.bindings(),
        )
        .map(|mut document| {
            // Give the newly-created document ownership of the reader, so that Pdfium can continue
            // to read from it on an as-needed basis throughout the lifetime of the document.

            document.set_file_access_reader(reader);

            document
        })
    }

    /// Attempts to open a [PdfDocument] by loading document data from the given URL.
    /// The Javascript `fetch` API is used to download data over the network.
    ///
    /// If the document is password protected, the given password will be used to unlock it.
    ///
    /// This function is only available when compiling to WASM.
    #[cfg(any(doc, target_arch = "wasm32"))]
    pub async fn load_pdf_from_fetch<'a>(
        &'a self,
        url: impl ToString,
        password: Option<&str>,
    ) -> Result<PdfDocument<'a>, PdfiumError> {
        if let Some(window) = window() {
            let fetch_result = JsFuture::from(window.fetch_with_str(url.to_string().as_str()))
                .await
                .map_err(PdfiumError::WebSysFetchError)?;

            debug_assert!(fetch_result.is_instance_of::<Response>());

            let response: Response = fetch_result
                .dyn_into()
                .map_err(|_| PdfiumError::WebSysInvalidResponseError)?;

            let blob: Blob = JsFuture::from(response.blob().map_err(PdfiumError::WebSysFetchError)?)
                .await
                .map_err(PdfiumError::WebSysFetchError)?
                .into();

            self.load_pdf_from_blob(blob, password).await
        } else {
            Err(PdfiumError::WebSysWindowObjectNotAvailable)
        }
    }

    /// Attempts to open a [PdfDocument] by loading document data from the given `Blob`.
    /// A `File` object returned from a `FileList` is a suitable `Blob`:
    ///
    /// ```text
    /// <input id="filePicker" type="file">
    ///
    /// const file = document.getElementById('filePicker').files[0];
    /// ```
    ///
    /// If the document is password protected, the given password will be used to unlock it.
    ///
    /// This function is only available when compiling to WASM.
    #[cfg(any(doc, target_arch = "wasm32"))]
    pub async fn load_pdf_from_blob<'a>(
        &'a self,
        blob: Blob,
        password: Option<&str>,
    ) -> Result<PdfDocument<'a>, PdfiumError> {
        let array_buffer: ArrayBuffer = JsFuture::from(blob.array_buffer())
            .await
            .map_err(PdfiumError::WebSysFetchError)?
            .into();

        let u8_array: Uint8Array = Uint8Array::new(&array_buffer);

        let bytes: Vec<u8> = u8_array.to_vec();

        self.load_pdf_from_byte_vec(bytes, password)
    }

    /// Creates a new, empty [PdfDocument] in memory.
    pub fn create_new_pdf(&self) -> Result<PdfDocument<'_>, PdfiumError> {
        Self::pdfium_document_handle_to_result(self.bindings().FPDF_CreateNewDocument(), self.bindings()).map(
            |mut document| {
                document.set_version(PdfDocumentVersion::DEFAULT_VERSION);

                document
            },
        )
    }

    /// Returns a [PdfDocument] from the given `FPDF_DOCUMENT` handle, if possible.
    pub(crate) fn pdfium_document_handle_to_result(
        handle: crate::bindgen::FPDF_DOCUMENT,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<PdfDocument<'_>, PdfiumError> {
        if handle.is_null() {
            // Retrieve the error code of the last error recorded by Pdfium.

            #[allow(clippy::unnecessary_cast)]
            if let Some(error) = match bindings.FPDF_GetLastError() as u32 {
                crate::bindgen::FPDF_ERR_SUCCESS => None,
                crate::bindgen::FPDF_ERR_UNKNOWN => Some(PdfiumInternalError::Unknown),
                crate::bindgen::FPDF_ERR_FILE => Some(PdfiumInternalError::FileError),
                crate::bindgen::FPDF_ERR_FORMAT => Some(PdfiumInternalError::FormatError),
                crate::bindgen::FPDF_ERR_PASSWORD => Some(PdfiumInternalError::PasswordError),
                crate::bindgen::FPDF_ERR_SECURITY => Some(PdfiumInternalError::SecurityError),
                crate::bindgen::FPDF_ERR_PAGE => Some(PdfiumInternalError::PageError),
                // The Pdfium documentation says "... if the previous SDK call succeeded, [then] the
                // return value of this function is not defined". On Linux, at least, a return value
                // of FPDF_ERR_SUCCESS seems to be consistently returned; on Windows, however, the
                // return values are indeed unpredictable. See https://github.com/ajrcarey/pdfium-render/issues/24.
                // Therefore, if the return value does not match one of the FPDF_ERR_* constants, we must
                // assume success.
                _ => None,
            } {
                Err(PdfiumError::PdfiumLibraryInternalError(error))
            } else {
                // This would be an unusual situation; a null handle indicating failure,
                // yet Pdfium's error code indicates success.

                Err(PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::Unknown))
            }
        } else {
            Ok(PdfDocument::from_pdfium(handle, bindings))
        }
    }
}

impl PdfiumLibraryBindingsAccessor for Pdfium {}

impl Default for Pdfium {
    /// Binds to a Pdfium library that was statically linked into the currently running
    /// executable by calling [Pdfium::bind_to_statically_linked_library]. This function
    /// will panic if no statically linked Pdfium functions can be located.
    #[cfg(pdfium_use_static)]
    #[inline]
    fn default() -> Self {
        Pdfium::new(Pdfium::bind_to_statically_linked_library().unwrap())
    }

    /// Binds to an external Pdfium library by first attempting to bind to a Pdfium library
    /// in the current working directory; if that fails, then a system-provided library
    /// will be used as a fall back.
    ///
    /// This function will panic if no suitable Pdfium library can be loaded.
    #[cfg(not(pdfium_use_static))]
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    fn default() -> Self {
        // Attempt to bind to a Pdfium library in the current working directory.

        match Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")) {
            Ok(bindings) => Pdfium::new(bindings), // Create new bindings
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium {}, // Re-use the existing bindings
            Err(PdfiumError::LoadLibraryError(err)) => {
                match err {
                    libloading::Error::DlOpen { .. } => {
                        // For DlOpen errors specifically, indicating the Pdfium library in the
                        // current working directory does not exist or is corrupted, we attempt
                        // to fall back to a system-provided library.

                        Pdfium::new(Pdfium::bind_to_system_library().unwrap())
                    }
                    _ => panic!("Failed to load Pdfium library: {:?}", err),
                }
            }
            Err(err) => panic!("Failed to initialize Pdfium: {:?}", err),
        }
    }

    /// Binds to an external Pdfium library by attempting to a system-provided library.
    ///
    /// This function will panic if no suitable Pdfium library can be loaded.
    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        Pdfium::new(Pdfium::bind_to_system_library().unwrap())
    }
}

impl Debug for Pdfium {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pdfium").finish()
    }
}

#[cfg(feature = "thread_safe")]
unsafe impl Sync for Pdfium {}

#[cfg(feature = "thread_safe")]
unsafe impl Send for Pdfium {}
