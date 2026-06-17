use crate::bindgen::{
    FPDF_ACTION, FPDF_ANNOT_APPEARANCEMODE, FPDF_ANNOTATION, FPDF_ANNOTATION_SUBTYPE, FPDF_ATTACHMENT, FPDF_AVAIL,
    FPDF_BITMAP, FPDF_BOOKMARK, FPDF_BOOL, FPDF_CLIPPATH, FPDF_COLORSCHEME, FPDF_CharsetFontMap, FPDF_DEST,
    FPDF_DOCUMENT, FPDF_DUPLEXTYPE, FPDF_DWORD, FPDF_FILEACCESS, FPDF_FILEIDTYPE, FPDF_FILEWRITE, FPDF_FONT,
    FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_GLYPHPATH, FPDF_IMAGEOBJ_METADATA, FPDF_JAVASCRIPT_ACTION,
    FPDF_LIBRARY_CONFIG, FPDF_LINK, FPDF_OBJECT_TYPE, FPDF_PAGE, FPDF_PAGELINK, FPDF_PAGEOBJECT, FPDF_PAGEOBJECTMARK,
    FPDF_PAGERANGE, FPDF_PATHSEGMENT, FPDF_SCHHANDLE, FPDF_SIGNATURE, FPDF_STRUCTELEMENT, FPDF_STRUCTELEMENT_ATTR,
    FPDF_STRUCTTREE, FPDF_SYSFONTINFO, FPDF_TEXT_RENDERMODE, FPDF_TEXTPAGE, FPDF_WCHAR, FPDF_WIDESTRING, FPDF_XOBJECT,
    FPDFANNOT_COLORTYPE, FS_FLOAT, FS_MATRIX, FS_POINTF, FS_QUADPOINTSF, FS_RECTF, FS_SIZEF, FX_DOWNLOADHINTS,
    FX_FILEAVAIL, IFSDK_PAUSE, size_t,
};

use crate::bindgen::FPDF_STRUCTELEMENT_ATTR_VALUE;

use crate::bindings::PdfiumLibraryBindings;
use crate::error::PdfiumError;
use crate::utils::files::{FpdfFileAccessExt, FpdfFileWriteExt, read_block_from_callback, write_block_from_callback};
use crate::utils::mem::create_byte_buffer;
use js_sys::{Array, Function, Object, Reflect, Uint8Array, WebAssembly};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::CString;
use std::mem::size_of;
use std::os::raw::{c_char, c_double, c_float, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use wasm_bindgen::intern;
use wasm_bindgen::prelude::*;

static PDFIUM_RENDER_WASM_STATE: Lazy<RwLock<PdfiumRenderWasmState>> =
    Lazy::new(|| RwLock::new(PdfiumRenderWasmState::default()));

#[derive(Debug, Copy, Clone)]
enum JsFunctionArgumentType {
    Void,
    Number,
    String,
    Pointer,
}

/// A global singleton used to store bindings to an external Pdfium WASM module.
///
/// This singleton is constructed when the WASM module containing pdfium-render is first loaded
/// into the browser. Binding to an external Pdfium WASM module is established by calling
/// the exported [initialize_pdfium_render()] function from Javascript. It is essential that this
/// function is called _before_ initializing `pdfium-render` from within Rust code. For an example, see:
/// <https://github.com/ajrcarey/pdfium-render/blob/master/examples/index.html>
#[derive(Debug, Default)]
pub(crate) struct PdfiumRenderWasmState {
    pdfium_wasm_module: Option<Object>,
    local_wasm_module: Option<Object>,
    wasm_table: Option<WebAssembly::Table>,
    malloc_js_fn: Option<Function>,
    free_js_fn: Option<Function>,
    call_js_fn: Option<Function>,
    debug: bool,
    file_access_callback_function_table_entry: usize,
    file_write_callback_function_table_entry: usize,
    state: HashMap<String, JsValue>,
}

impl PdfiumRenderWasmState {
    const BYTES_PER_PIXEL: i32 = 4;

    /// Returns shared read-only access to the global [PdfiumRenderWasmState] singleton.
    #[inline]
    pub fn lock() -> RwLockReadGuard<'static, PdfiumRenderWasmState> {
        match PDFIUM_RENDER_WASM_STATE.try_read() {
            Ok(lock) => lock,
            Err(err) => {
                log::error!("PdfiumRenderWasmState::lock(): unable to acquire read lock: {:#?}", err);
                log::error!(
                    "This may indicate a programming error in pdfium-render. Please file an issue: https://github.com/ajrcarey/pdfium-render/issues"
                );

                panic!()
            }
        }
    }

    /// Returns exclusive read-write access to the global [PdfiumRenderWasmState] singleton.
    #[inline]
    pub fn lock_mut() -> RwLockWriteGuard<'static, PdfiumRenderWasmState> {
        match PDFIUM_RENDER_WASM_STATE.try_write() {
            Ok(lock) => lock,
            Err(err) => {
                log::error!(
                    "PdfiumRenderWasmState::lock_mut(): unable to acquire write lock: {:#?}",
                    err
                );
                log::error!(
                    "This may indicate a programming error in pdfium-render. Please file an issue: https://github.com/ajrcarey/pdfium-render/issues"
                );

                panic!()
            }
        }
    }

    /// Returns `true` if this [PdfiumRenderWasmState] has been successfully bound to a valid
    /// external Pdfium WASM module.
    pub fn is_ready(&self) -> bool {
        self.pdfium_wasm_module.is_some()
    }

    /// Configures this [PdfiumRenderWasmState] with bindings to Emscripten-exposed Pdfium functions
    /// in the given Javascript Object.
    fn bind_to_pdfium(
        &mut self,
        pdfium_wasm_module: Object,
        local_wasm_module: Object,
        debug: bool,
    ) -> Result<(), &str> {
        self.pdfium_wasm_module = Some(pdfium_wasm_module);
        self.local_wasm_module = Some(local_wasm_module);
        self.debug = debug;

        self.malloc_js_fn = Some(Function::from(
            self.get_value_from_pdfium_wasm_module("_malloc")
                .or_else(|_| {
                    // The _malloc function is not defined. Try the wasmExports.malloc function
                    // in the Pdfium WASM module instead. For more information, see:
                    // https://github.com/ajrcarey/pdfium-render/issues/128

                    log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): _malloc function export not defined, falling back to Module['wasmExports'].malloc");

                    self.get_value_from_pdfium_wasm_module("wasmExports")
                        .and_then(|wasm_exports| self.get_value_from_browser_object(&wasm_exports, "malloc"))
                        .map_err(|_| "Module['wasmExports'] not defined when looking for malloc()")
                })
                .or_else(|_| {
                    // The _malloc function is not defined. Try the asm.malloc function in the
                    // Pdfium WASM module instead. For more information, see:
                    // https://github.com/ajrcarey/pdfium-render/issues/95

                    log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): _malloc function export not defined, falling back to Module['asm'].malloc");

                    self.get_value_from_pdfium_wasm_module("asm")
                        .and_then(|asm| self.get_value_from_browser_object(&asm, "malloc"))
                        .map_err(|_| "Module['asm'] not defined when looking for malloc()")
                })?));

        self.free_js_fn = Some(Function::from(
            self.get_value_from_pdfium_wasm_module("_free")
                .or_else(|_| {
                    // The _free function is not defined. Try the wasmExports.free function in the
                    // Pdfium WASM module instead. For more information, see:
                    // https://github.com/ajrcarey/pdfium-render/issues/128

                    log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): _free function export not defined, falling back to Module['wasmExports'].free");

                    self.get_value_from_pdfium_wasm_module("wasmExports")
                        .and_then(|wasm_exports| self.get_value_from_browser_object(&wasm_exports, "free"))
                        .map_err(|_| "Module['wasmExports'] not defined when looking for free()")
                })
                .or_else(|_| {
                    // The _free function is not defined. Try the asm.free function in the
                    // Pdfium WASM module instead. For more information, see:
                    // https://github.com/ajrcarey/pdfium-render/issues/95

                    log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): _free function export not defined, falling back to Module['asm'].free");

                    self.get_value_from_pdfium_wasm_module("asm")
                        .and_then(|asm| self.get_value_from_browser_object(&asm, "free"))
                        .map_err(|_| "Module['asm'] not defined when looking for free()")
                })?));

        self.call_js_fn = Some(Function::from(
            self.get_value_from_pdfium_wasm_module("ccall")
                .map_err(|_| "Module.ccall() not defined")?,
        ));

        // We don't define a fixed binding to it, but check now that the Module.HEAPU8 accessor works.

        if self.get_value_from_pdfium_wasm_module("HEAPU8").is_err() {
            return Err("Module.HEAPU8[] not defined");
        }

        // Look up two functions exported from Pdfium that can be used to host the
        // read_block_from_callback_wasm() and write_block_from_callback_wasm() functions.
        // The functions exported from Pdfium that we replace need to have the same
        // function signatures. For more information, see:
        // https://github.com/ajrcarey/pdfium-render/issues/8.

        // For builds of Pdfium downloaded from https://github.com/paulocoutinhox/pdfium-lib/releases
        // _before_ V5407, Pdfium's function table is exported by Emscripten directly into
        // the wasmTable global variable available to each web worker running in the browser.
        // For builds of Pdfium downloaded from https://github.com/paulocoutinhox/pdfium-lib/releases
        // _including or after_ V5407, Pdfium's function table is available in the Emscripten-wrapped
        // Pdfium WASM module, but is not exported into a global variable.

        // Retrieve the function table by looking for the wasmTable global variable first.

        let table: WebAssembly::Table = self.get_value_from_browser_globals("wasmTable")
            .or_else(|_| {
                // The wasmTable global variable is not defined. Try the
                // wasmExports.__indirect_function_table variable in the Pdfium WASM module instead.
                // For more information, see: https://github.com/ajrcarey/pdfium-render/issues/134

                log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): global wasmTable variable not defined, falling back to Module['wasmExports'].__indirect_function_table");

                self.get_value_from_pdfium_wasm_module("wasmExports")
                    .and_then(|wasm_exports| self.get_value_from_browser_object(&wasm_exports, "__indirect_function_table"))
                    .map_err(|_| "Module['wasmExports'] not defined when looking for wasmTable")
            })
            .or_else(|_| {
                // The wasmTable global variable is not defined. Try the
                // asm.__indirect_function_table property in the Pdfium WASM module instead.
                // For more information, see: https://github.com/ajrcarey/pdfium-render/issues/128

                log::debug!("pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): global wasmTable variable not defined, falling back to Module['asm'].__indirect_function_table");

                self.get_value_from_pdfium_wasm_module("asm")
                    .and_then(|asm| self.get_value_from_browser_object(&asm, "__indirect_function_table"))
                    .map_err(|_| "Module['asm'] not defined when looking for wasmTable")
            })?.into();

        // Once we have the function table, we scan it for function signatures that take 4 arguments.

        for index in 1..table.length() {
            if let Ok(function) = table.get(index)
                && function.length() == 4
            {
                // We've found a viable patch function candidate.

                self.file_access_callback_function_table_entry = self.file_write_callback_function_table_entry;
                self.file_write_callback_function_table_entry = index as usize;
            }
        }

        log::debug!(
            "pdfium_render::PdfiumRenderWasmState::bind_to_pdfium(): found candidate patch function indices {}, {}",
            self.file_access_callback_function_table_entry,
            self.file_write_callback_function_table_entry,
        );

        self.wasm_table = Some(table);

        Ok(())
    }

    /// Looks up the given key in the Emscripten-wrapped Pdfium WASM module and returns
    /// the Javascript value associated with that key, if any.
    fn get_value_from_pdfium_wasm_module(&self, key: &str) -> Result<JsValue, PdfiumError> {
        if let Some(pdfium_wasm_module) = self.pdfium_wasm_module.as_ref() {
            self.get_value_from_browser_object(pdfium_wasm_module, key)
        } else {
            Err(PdfiumError::JsSysErrorRetrievingFunction(JsValue::UNDEFINED))
        }
    }

    /// Looks up the given key in the given Javascript object and returns
    /// the Javascript value associated with that key, if any.
    fn get_value_from_browser_object(&self, source: &JsValue, key: &str) -> Result<JsValue, PdfiumError> {
        // When looking up functions in a Javascript object, treat a return value of
        // JsValue::UNDEFINED as if it were an error.

        Reflect::get(source, &JsValue::from(key))
            .map_err(|_| PdfiumError::JsValueUndefined)
            .and_then(|value: JsValue| {
                if value == JsValue::UNDEFINED {
                    Err(PdfiumError::JsValueUndefined)
                } else {
                    Ok(value)
                }
            })
    }

    /// Looks up the given key in the browser's global Window object and returns
    /// the Javascript value associated with that key, if any.
    #[inline]
    fn get_value_from_browser_globals(&self, key: &str) -> Result<JsValue, PdfiumError> {
        self.get_value_from_browser_object(&js_sys::global(), key)
    }

    /// Allocates the given number of bytes in Pdfium's WASM memory heap, returning a pointer
    /// to the allocation address.
    fn malloc(&self, len: usize) -> usize {
        match self
            .malloc_js_fn
            .as_ref()
            .unwrap()
            .call1(&JsValue::null(), &JsValue::from_f64(len as f64))
        {
            Ok(result) => match result.as_f64() {
                Some(result) => result as usize,
                None => {
                    log::error!(
                        "pdfium-render::PdfiumRenderWasmState::malloc(): return value from Module._malloc() is not a Javascript number"
                    );

                    panic!();
                }
            },
            Err(err) => {
                log::error!(
                    "pdfium-render::PdfiumRenderWasmState::malloc(): call to Module._malloc() failed for allocation length {}: {:#?}",
                    len,
                    err
                );

                panic!();
            }
        }
    }

    /// Frees a previously-made memory allocation in Pdfium's WASM memory heap.
    fn free(&self, ptr: usize) {
        if ptr > 0 {
            let result = self
                .free_js_fn
                .as_ref()
                .unwrap()
                .call1(&JsValue::null(), &JsValue::from_f64(ptr as f64));

            if let Some(err) = result.err() {
                log::error!(
                    "pdfium-render::PdfiumRenderWasmState::free(): call to Module._free() failed: {:#?}",
                    err
                );

                panic!()
            }
        }
    }

    /// Calls an emscripten-wrapped Pdfium function.
    fn call(
        &self,
        fn_name: &str,
        return_type: JsFunctionArgumentType,
        arg_types: Option<Vec<JsFunctionArgumentType>>,
        args: Option<&JsValue>,
    ) -> JsValue {
        fn js_value_from_argument_type(arg_type: JsFunctionArgumentType) -> JsValue {
            match arg_type {
                JsFunctionArgumentType::Void => JsValue::undefined(),
                JsFunctionArgumentType::Number | JsFunctionArgumentType::Pointer => JsValue::from(intern("number")),
                JsFunctionArgumentType::String => JsValue::from(intern("string")),
            }
        }

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::call(): performing call to function: {:#?}",
            fn_name
        );

        let js_fn_name = JsValue::from(intern(fn_name));

        let js_return_type = js_value_from_argument_type(return_type);

        let js_arg_types = match arg_types {
            Some(arg_types) => {
                if self.debug {
                    // Type-check arguments.

                    if let Some(args) = args {
                        if !Array::is_array(args) {
                            log::warn!(
                                "pdfium-render::PdfiumRenderWasmState::call(): argument type list given, but arguments is not an Array"
                            );
                        }

                        let args = Array::from(args);

                        if arg_types.len() != args.length() as usize {
                            log::warn!(
                                "pdfium-render::PdfiumRenderWasmState::call(): length of argument type and argument lists does not match"
                            );
                        }

                        for (index, (arg_type, arg)) in
                            arg_types.iter().zip(args.iter().collect::<Vec<_>>()).enumerate()
                        {
                            if !match arg_type {
                                JsFunctionArgumentType::Void => arg.is_undefined(),
                                JsFunctionArgumentType::Number | JsFunctionArgumentType::Pointer => {
                                    arg.as_f64().is_some()
                                }
                                JsFunctionArgumentType::String => arg.as_string().is_some(),
                            } {
                                log::warn!(
                                    "pdfium-render::PdfiumRenderWasmState::call(): type-checking of argument {} failed: expected {:#?}, received {:#?}",
                                    index,
                                    arg_type,
                                    arg
                                );
                            }
                        }
                    }
                }

                JsValue::from(
                    arg_types
                        .into_iter()
                        .map(js_value_from_argument_type)
                        .collect::<Array>(),
                )
            }
            None => JsValue::undefined(),
        };

        log::debug!("pdfium-render::PdfiumRenderWasmState::call(): arguments: {:#?}", args);

        match self.call_js_fn.as_ref().unwrap().apply(
            &JsValue::null(),
            &Array::of4(
                &js_fn_name,
                &js_return_type,
                &js_arg_types,
                args.unwrap_or(&JsValue::undefined()),
            ),
        ) {
            Ok(result) => {
                log::debug!(
                    "pdfium-render::PdfiumRenderWasmState::call(): call returned result: {:#?}",
                    result
                );

                if self.debug {
                    // Type-check result.

                    if !match return_type {
                        JsFunctionArgumentType::Void => result.is_undefined(),
                        JsFunctionArgumentType::Number | JsFunctionArgumentType::Pointer => result.as_f64().is_some(),
                        JsFunctionArgumentType::String => result.as_string().is_some(),
                    } {
                        log::warn!(
                            "pdfium-render::PdfiumRenderWasmState::call(): result data type does not match expected return type {:#?}",
                            return_type
                        );
                    }
                }

                result
            }
            Err(err) => {
                log::error!(
                    "pdfium-render::PdfiumRenderWasmState::call(): call to {:#?} failed: {:#?}",
                    fn_name,
                    err,
                );

                panic!();
            }
        }
    }

    /// Returns a live view of Pdfium's WASM memory heap.
    fn heap_u8(&self) -> Uint8Array {
        match Reflect::get(self.pdfium_wasm_module.as_ref().unwrap(), &JsValue::from("HEAPU8")) {
            Ok(result) => Uint8Array::from(result),
            Err(err) => {
                log::error!(
                    "pdfium-render::PdfiumRenderWasmState::heap_u8(): Module.HEAPU8[] not defined: {:#?}",
                    err
                );

                panic!();
            }
        }
    }

    /// Copies the given bytes into Pdfium's WASM memory heap at the given destination location.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    fn copy_bytes_to_pdfium_address(&self, bytes: &[u8], remote_ptr: usize) {
        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium_address(): entering");

        // We must NOT use Uint8Array::view(bytes) here. That creates a zero-copy view into
        // kreuzberg's WASM linear memory, which becomes detached if memory.grow() is triggered
        // by any subsequent JS interop call (including heap_u8()'s Reflect::get below).
        // Instead, copy bytes into a JS-heap-allocated Uint8Array that is immune to detachment.
        let safe_copy = Uint8Array::new_with_length(bytes.len() as u32);
        safe_copy.copy_from(bytes);
        self.heap_u8().set(&safe_copy, remote_ptr as u32);

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium_address(): copied {} bytes into WASM heap at address {}",
            bytes.len(),
            remote_ptr
        );

        if self.debug {
            // Compare memory after copying to ensure it is the same.

            let mut differences_count = 0;

            let pdfium_heap = self.heap_u8();

            for (index, byte) in bytes.iter().enumerate() {
                let dest = pdfium_heap.get_index((remote_ptr + index) as u32);

                if *byte != dest {
                    differences_count += 1;

                    log::warn!(
                        "pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium_address(): byte index {} differs between source and destination: source data = {}, destination data = {}",
                        index,
                        byte,
                        dest,
                    );
                }
            }

            log::debug!(
                "pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium_address(): {} bytes differ between source and destination byte buffers",
                differences_count
            );
        }

        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium_address(): leaving");
    }

    /// Copies the given bytes into Pdfium's WASM memory heap, returning a pointer to the
    /// destination location.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    fn copy_bytes_to_pdfium(&self, bytes: &[u8]) -> usize {
        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium(): entering");

        let remote_ptr = self.malloc(bytes.len());

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium(): got pointer offset back from Module._malloc(): {}",
            remote_ptr
        );

        self.copy_bytes_to_pdfium_address(bytes, remote_ptr);

        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_to_pdfium(): leaving");

        remote_ptr
    }

    /// Copies a given number of bytes starting at the given pointer into Pdfium's WASM memory heap,
    /// returning a pointer to the copied bytes at the destination location.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    #[inline]
    fn copy_ptr_with_len_to_pdfium<T>(&self, ptr: *const T, len: usize) -> usize {
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        self.copy_bytes_to_pdfium(bytes)
    }

    /// Copies the raw bytes at the given pointer into Pdfium's WASM memory heap, returning a
    /// pointer to the copied struct at the destination location.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    #[inline]
    fn copy_struct_to_pdfium<T>(&self, ptr: *const T) -> usize {
        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_struct_to_pdfium(): entering");

        let len = size_of::<T>();

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_struct_to_pdfium(): struct is at heap offset {}, data length {} bytes",
            ptr as usize as u32,
            len
        );

        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        let result = self.copy_bytes_to_pdfium(bytes);

        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_struct_to_pdfium(): leaving");

        result
    }

    /// Copies the raw bytes at the given pointer into Pdfium's WASM memory heap, returning a
    /// pointer to the copied struct at the destination location.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    #[inline]
    fn copy_struct_to_pdfium_mut<T>(&self, ptr: *mut T) -> usize {
        self.copy_struct_to_pdfium(ptr as *const T)
    }

    /// Copies the raw bytes of the given `FPDF_WIDESTRING` into Pdfium's WASM memory heap,
    /// returning a pointer to the copied string at the destination location.
    ///
    /// The source `FPDF_WIDESTRING` must be terminated by two null bytes.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from our own memory heap across into Pdfium's memory heap, and vice versa.
    #[inline]
    fn copy_string_to_pdfium(&self, str: FPDF_WIDESTRING) -> usize {
        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_string_to_pdfium(): entering");

        // Copying the FPDF_WIDESTRING using copy_struct_to_pdfium() will only copy the
        // two-byte pointer, not the string data itself. We must scan the source memory
        // location for two null bytes to find the correct data length.

        // Many characters, such as standard ASCII characters, can be represented with
        // a single byte. Since all characters in UTF-16 consume two bytes, ASCII characters
        // will receive a second byte of padding in UTF-16. This second byte will be a null.
        // When scanning the FPDF_WIDESTRING for two null bytes, we must be careful not to
        // misinterpret a null padding byte as part of a string termination sequence.
        // The easiest way is to iterate over the string in two-byte pairs rather than
        // byte-by-byte. See: https://github.com/ajrcarey/pdfium-render/issues/171

        let mut len = 0;

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_string_to_pdfium(): FPDF_WIDESTRING is at heap offset {}",
            str as usize as u32,
        );

        loop {
            // Read bytes directly from the pointer instead of creating unsafe JS Uint8Array
            // views. Uint8Array::view_mut_raw creates a view into WASM linear memory that can
            // become detached if memory.grow() occurs.
            let byte0 = unsafe { *((str as *const u8).add(len)) };
            let byte1 = unsafe { *((str as *const u8).add(len + 1)) };

            len += 2;

            if byte0 == 0 && byte1 == 0 {
                // This is the end of the string.

                break;
            }
        }

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_string_to_pdfium(): FPDF_WIDESTRING has data length {} bytes",
            len,
        );

        let result = self.copy_ptr_with_len_to_pdfium(str, len);

        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_string_to_pdfium(): leaving");

        result
    }

    /// Copies bytes from the given pointer address in Pdfium's memory heap into our memory
    /// heap, returning an address to the location in our memory heap where the first copied
    /// byte was placed.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from Pdfium's memory heap across into our own, and vice versa.
    fn copy_bytes_from_pdfium(&self, ptr: usize, len: usize) -> Vec<u8> {
        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_from_pdfium(): entering");

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::copy_bytes_from_pdfium(): copying {} bytes from pointer offset {}",
            len,
            ptr
        );

        let copy = self.heap_u8().slice(ptr as u32, (ptr + len) as u32).to_vec();

        if self.debug {
            let mut differences_count = 0;

            let pdfium_heap = self.heap_u8();

            for (index, byte) in copy.iter().enumerate() {
                let src = pdfium_heap.get_index((ptr + index) as u32);

                if *byte != src {
                    differences_count += 1;

                    log::warn!(
                        "pdfium-render::PdfiumRenderWasmState::copy_bytes_from_pdfium(): byte index {} differs between source and destination: source data = {}, destination data = {}",
                        index,
                        src,
                        byte,
                    );
                }
            }

            log::debug!(
                "pdfium-render::PdfiumRenderWasmState::copy_bytes_from_pdfium(): {} bytes differ between source and destination byte buffers",
                differences_count
            );
        }

        log::debug!("pdfium-render::PdfiumRenderWasmState::copy_bytes_from_pdfium(): leaving");

        copy
    }

    /// Copies bytes from the given pointer address in Pdfium's memory heap into our memory
    /// heap, using the given address to a pointer in our memory heap.
    ///
    /// WASM modules are isolated from one another and cannot directly share memory. We must
    /// therefore copy buffers from Pdfium's memory heap across into our own, and vice versa.
    fn copy_struct_from_pdfium<T>(
        &self,
        pdfium_buffer_ptr: usize,
        pdfium_buffer_len_bytes: usize,
        local_buffer_ptr: *mut T,
    ) {
        if pdfium_buffer_len_bytes > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::copy_struct_from_pdfium(): copying {} bytes from Pdfium's WASM heap into local buffer at offset {}",
                pdfium_buffer_len_bytes,
                local_buffer_ptr as usize
            );

            unsafe {
                local_buffer_ptr.copy_from(
                    self.copy_bytes_from_pdfium(pdfium_buffer_ptr, pdfium_buffer_len_bytes)
                        .as_ptr() as *mut T,
                    // Pdfium's buffer length is expressed in bytes, but buffer.copy_from()
                    // expects the _count_ of pointer-sized objects to copy, i.e. the number of Ts,
                    // which may or may not be the same as the number of bytes depending on
                    // the size_of::<T>().
                    pdfium_buffer_len_bytes / size_of::<T>(),
                );
            }
        }
    }

    /// Copies the give `FPDF_FILEACCESS` struct into Pdfium's WASM heap, returning a
    /// pointer to the copied struct at the destination location.
    ///
    /// A wrapper function is used to transfer blocks of data from our local WASM memory heap
    /// to Pdfium's WASM memory heap.
    fn copy_file_access_to_pdfium(&self, file_access: *mut FPDF_FILEACCESS) -> usize {
        // The approach we take here is fairly insane. WASM memory heaps contain only data and
        // are non-executable by design; while we can copy the compiled bytes for a function into
        // a WASM memory heap, we can never execute it, even if we pass the correct function pointer
        // to the function's address in memory. As a security measure, WASM only executes functions
        // that are pre-defined in a function table specific to the compiled module. To support
        // C-style callback functions, WASM provides a call_indirect() virtual CPU instruction that
        // looks up a function from the function table, verifies it has the expected number of
        // arguments, and passes control to that function. But function tables are unique to each
        // compiled WASM module, so even if we pass Pdfium the correct index entry to a function
        // in _our_ WASM module, Pdfium will try to call the function with the corresponding
        // index entry in _its_ WASM module. This obviously will not work.

        // WASM does support the idea of dynamically "growing" the function table for a WASM module,
        // allowing for the addition of new functions into a function table at run time.
        // If the function table in one WASM module is growable, then an index entry from a different
        // function table in a different WASM module can be copied across into it, providing access
        // to the function in the different WASM module. This perfectly fits our use case!
        // Unfortunately, Pdfium's WASM builds are compiled with non-growable function tables.

        // This is where the insanity comes in. We find an existing function in Pdfium's WASM
        // function table with the same function signature as the callback we want to provide,
        // then replace its entry with the index entry corresponding to a wrapper callback
        // we define in this file, read_block_from_callback_wasm(). When Pdfium runs the
        // call_indirect() instruction that passes control to the adjusted function index entry,
        // our callback function is called.

        // The identification of a suitable function in Pdfium's WASM function table takes place
        // during the call to initialize_pdfium_render().

        let file_access_with_callback = unsafe {
            FPDF_FILEACCESS {
                m_FileLen: (*file_access).m_FileLen,
                m_GetBlock: Some(
                    // Replace our callback pointer with a function index entry in Pdfium's
                    // WASM function table. This function index entry will invoke the wrapper
                    // callback function read_block_from_callback_wasm(), which will in turn
                    // pass control to the actual callback inside our WASM module.
                    std::mem::transmute::<
                        usize,
                        unsafe extern "C" fn(
                            param: *mut c_void,
                            position: c_ulong,
                            buf: *mut c_uchar,
                            size: c_ulong,
                        ) -> c_int,
                    >(self.file_access_callback_function_table_entry),
                ),
                m_Param: (*file_access).m_Param,
            }
        };

        self.copy_struct_to_pdfium(&file_access_with_callback)
    }

    /// Copies the give `FPDF_FILEWRITE` struct into Pdfium's WASM heap, returning a
    /// pointer to the copied struct at the destination location.
    ///
    /// A wrapper function is used to transfer blocks of data from Pdfium's WASM memory heap
    /// to our local WASM memory heap for writing.
    fn copy_file_write_to_pdfium(&mut self, file_write: *mut FPDF_FILEWRITE) -> usize {
        // See comments in copy_file_access_to_pdfium() for an explanation of the approach used
        // here. It is arguably slightly less insane when writing, because Pdfium's save operations
        // occur immediately and synchronously; for that reason, we could in theory arrange things so
        // that our adjustment of Pdfium's function table lasts only as long as the save operation.
        // This differs from copy_file_access_to_pdfium() where the lifetime of the reader (and
        // therefore the duration of the adjustment of Pdfium's function table) can potentially
        // last as long as the lifetime of the document.

        // The identification of a suitable function in Pdfium's WASM function table takes place
        // during the call to initialize_pdfium_render().

        let file_write_with_callback = unsafe {
            FPDF_FILEWRITE {
                version: 1,
                WriteBlock: Some(
                    // Replace our callback pointer with a function index entry in Pdfium's
                    // WASM function table. This function index entry will invoke the wrapper
                    // callback function write_block_from_callback_wasm(), which will in turn
                    // pass control to the actual callback inside our WASM module.
                    std::mem::transmute::<
                        usize,
                        unsafe extern "C" fn(param: *mut FPDF_FILEWRITE, buf: *const c_void, size: c_ulong) -> c_int,
                    >(self.file_write_callback_function_table_entry),
                ),
            }
        };

        let file_write_ptr = self.copy_struct_to_pdfium(&file_write_with_callback);

        // The callback function will receive a pointer to the location of the _copied_
        // FPDF_FILEWRITE, in Pdfium's memory heap. This doesn't help us very much, since we need
        // to know the location of the _original_ struct in our own memory heap. We save the
        // memory location of the original so that our callback can look it up as required.

        self.set(
            format!("file_write_{}", file_write_ptr).as_str(),
            JsValue::from_f64(file_write as usize as f64),
        );

        file_write_ptr
    }

    /// Patches the function table in Pdfium's WASM module, replacing the pdfium_function_index entry
    /// with the local_function_index entry. This enables Pdfium to invoke a callback inside our local
    /// WASM module's function table. This is necessary to enable certain file handling functionality.
    /// For more information, see https://github.com/ajrcarey/pdfium-render/issues/8.
    pub fn patch_pdfium_function_table(
        &mut self,
        pdfium_function_index: usize,
        local_function_name: &str,
    ) -> Result<(), PdfiumError> {
        log::debug!("pdfium-render::PdfiumRenderWasmState::patch_pdfium_function_table(): entering");

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::patch_pdfium_function_table(): patching Pdfium function index {} with local function {}",
            pdfium_function_index,
            local_function_name
        );

        let local_module = self.local_wasm_module.as_ref().unwrap();

        let local_function = Function::from(
            self.get_value_from_browser_object(local_module, local_function_name)
                .map_err(|_| PdfiumError::JsSysErrorRetrievingFunction(JsValue::UNDEFINED))?,
        );

        // Save the current entry in the function table, so we can restore it later.

        if let Ok(function) = self.wasm_table.as_ref().unwrap().get(pdfium_function_index as u32) {
            self.set(format!("function_{}", pdfium_function_index).as_str(), function.into());
        }

        self.wasm_table
            .as_mut()
            .unwrap()
            .set(pdfium_function_index as u32, &local_function)
            .map_err(PdfiumError::JsSysErrorPatchingFunctionTable)?;

        log::debug!("pdfium-render::PdfiumRenderWasmState::patch_pdfium_function_table(): patch complete, leaving");

        Ok(())
    }

    /// Restores an entry in Pdfium's function table from a previously cached function,
    /// undoing a previous call to patch_pdfium_function_table().
    pub fn unpatch_pdfium_function_table(&mut self, pdfium_function_index: usize) -> Result<(), PdfiumError> {
        log::debug!("pdfium-render::PdfiumRenderWasmState::unpatch_pdfium_function_table(): entering");

        if let Some(value) = self.take(format!("function_{}", pdfium_function_index).as_str()) {
            let original_function = Function::from(value);

            self.wasm_table
                .as_mut()
                .unwrap()
                .set(pdfium_function_index as u32, &original_function)
                .map_err(PdfiumError::JsSysErrorPatchingFunctionTable)?;

            log::debug!(
                "pdfium-render::PdfiumRenderWasmState::unpatch_pdfium_function_table(): function restoration complete, leaving"
            );

            Ok(())
        } else {
            // No previously cached function is available.

            log::error!(
                "pdfium-render::PdfiumRenderWasmState::unpatch_pdfium_function_table(): cannot retrieve cached function for index entry {}",
                pdfium_function_index
            );

            Err(PdfiumError::NoPreviouslyCachedFunctionSet)
        }
    }

    /// Calls FPDF_GetLastError(), returning the result.
    #[inline]
    fn get_last_error(&self) -> c_ulong {
        self.call("FPDF_GetLastError", JsFunctionArgumentType::Number, None, None)
            .as_f64()
            .unwrap() as c_ulong
    }

    /// Stores the given key / value pair.
    #[inline]
    fn set(&mut self, key: &str, value: JsValue) {
        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::set(): setting key: {}, value: {:#?}",
            key,
            value
        );

        self.state.insert(String::from(key), value);
    }

    /// Retrieves the value associated with the given key, if any.
    #[inline]
    fn get(&self, key: &str) -> Option<&JsValue> {
        let value = self.state.get(key);

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::get(): getting value for key: {}, value: {:#?}",
            key,
            value
        );

        value
    }

    /// Retrieves the value associated with the given key, if any, removing the key and value.
    #[inline]
    fn take(&mut self, key: &str) -> Option<JsValue> {
        let value = self.state.remove(key);

        log::debug!(
            "pdfium-render::PdfiumRenderWasmState::take(): getting value for key: {}, value: {:#?}",
            key,
            value
        );

        value
    }
}

unsafe impl Send for PdfiumRenderWasmState {}

unsafe impl Sync for PdfiumRenderWasmState {}

/// Establishes a binding between an external Pdfium WASM module and `pdfium-render`'s WASM module.
/// This function should be called from Javascript once the external Pdfium WASM module has been loaded
/// into the browser. It is essential that this function is called _before_ initializing
/// `pdfium-render` from within Rust code. For an example, see:
/// <https://github.com/ajrcarey/pdfium-render/blob/master/examples/index.html>
#[wasm_bindgen]
pub fn initialize_pdfium_render(pdfium_wasm_module: JsValue, local_wasm_module: JsValue, debug: bool) -> bool {
    if console_log::init_with_level(if debug { log::Level::Trace } else { log::Level::Info }).is_err() {
        log::error!("pdfium-render::initialize_pdfium_render(): Error initializing console-based logging");
    }

    if debug {
        // Output full Rust stack traces to Javascript console.

        console_error_panic_hook::set_once();
    }

    if pdfium_wasm_module.is_object() && local_wasm_module.is_object() {
        match PdfiumRenderWasmState::lock_mut().bind_to_pdfium(
            Object::from(pdfium_wasm_module),
            Object::from(local_wasm_module),
            debug,
        ) {
            Ok(()) => true,
            Err(msg) => {
                log::error!("pdfium-render::initialize_pdfium_render(): {}", msg);

                false
            }
        }
    } else {
        log::error!(
            "pdfium-render::initialize_pdfium_render(): one or more provided modules are not a valid Javascript Objects"
        );

        false
    }
}

/// A callback function that can be invoked by Pdfium's `FPDF_LoadCustomDocument()` function,
/// wrapping around `crate::utils::files::read_block_from_callback()` to shuffle data buffers
/// from our WASM memory heap to Pdfium's WASM memory heap as they are loaded.
#[wasm_bindgen]
pub fn read_block_from_callback_wasm(
    param: *mut c_void,
    position: c_ulong,
    #[allow(non_snake_case)] pBuf: *mut c_uchar,
    size: c_ulong,
) -> c_int {
    log::debug!(
        "pdfium-render::read_block_from_callback_wasm(): entering with param = {:?}, position = {:?}, pBuf = {:?}, size = {:?}",
        param,
        position,
        pBuf,
        size
    );

    // Create a buffer of the same size as Pdfium provided...

    log::debug!(
        "pdfium-render::read_block_from_callback_wasm(): creating read buffer, length = {}",
        size
    );

    let mut buffer = create_byte_buffer(size as usize);

    // ... read data into it...

    log::debug!(
        "pdfium-render::read_block_from_callback_wasm(): reading up to {} bytes into buffer",
        size
    );

    let result = read_block_from_callback(param as *mut FpdfFileAccessExt, position, buffer.as_mut_ptr(), size);

    log::debug!(
        "pdfium-render::read_block_from_callback_wasm(): read complete, {} bytes loaded into buffer",
        result
    );

    // ... and copy the read data back into Pdfium's WASM heap.

    PdfiumRenderWasmState::lock().copy_bytes_to_pdfium_address(buffer.as_slice(), pBuf as usize);

    log::debug!("pdfium-render::read_block_from_callback_wasm(): leaving");

    result
}

/// A callback function that can be invoked by Pdfium's `FPDF_SaveAsCopy()` and `FPDF_SaveWithVersion()`
/// functions, wrapping around `crate::utils::files::write_block_from_callback()` to shuffle data buffers
/// from Pdfium's WASM memory heap to our WASM memory heap as they are written.
#[wasm_bindgen]
pub fn write_block_from_callback_wasm(param: *mut c_void, buf: *const c_void, size: c_ulong) -> c_int {
    log::debug!(
        "pdfium-render::write_block_from_callback_wasm(): entering with param = {:?}, buf = {:?}, size = {:?}",
        param,
        buf,
        size
    );

    let state = PdfiumRenderWasmState::lock();

    // Look up the memory location of the underlying FpdfFileWriteExt struct in our local
    // memory heap.

    if let Some(ptr) = state
        .get(format!("file_write_{}", param as usize).as_str())
        .and_then(|value| value.as_f64())
    {
        // Copy Pdfium's buffer into our local memory heap...

        let buffer = state.copy_bytes_from_pdfium(buf as usize, size as usize);

        // ... and write the buffer out.

        log::debug!("pdfium-render::write_block_from_callback_wasm(): writing local buffer");

        let result = write_block_from_callback(
            ptr as usize as *mut FpdfFileWriteExt,
            buffer.as_ptr() as *const c_void,
            size,
        );

        log::debug!("pdfium-render::write_block_from_callback_wasm(): leaving");

        result
    } else {
        // No saved memory location of the underlying FpdfFileWriteExt is available.

        log::error!(
            "pdfium-render::write_block_from_callback_wasm(): cannot retrieve callback pointer for Pdfium WASM heap address {}",
            param as usize
        );

        0
    }
}

pub(crate) struct WasmPdfiumBindings;

impl WasmPdfiumBindings {
    // Pdfium cannot access a pointer location in our own WASM heap. When calling FPDF_* functions
    // that take pointers to caller-provided buffers, we must copy those buffers
    // from our WASM heap into Pdfium's WASM heap. When calling FPDF_* functions that fill a
    // buffer, we must create an empty buffer in Pdfium's WASM heap, then copy it back into
    // our WASM heap once Pdfium has populated it with data. These patterns are used throughout
    // the function implementations in this module.

    #[inline]
    pub(crate) fn new() -> Self {
        WasmPdfiumBindings {}
    }

    /// Converts a pointer to an `FPDF_DOCUMENT` struct to a [JsValue].
    #[inline]
    fn js_value_from_document(document: FPDF_DOCUMENT) -> JsValue {
        Self::js_value_from_offset(document as usize)
    }

    /// Converts a pointer to an `FPDF_PAGE` struct to a [JsValue].
    #[inline]
    fn js_value_from_page(page: FPDF_PAGE) -> JsValue {
        Self::js_value_from_offset(page as usize)
    }

    /// Converts a mutable pointer to an `FPDF_PAGE` struct to a [JsValue].
    #[inline]
    fn js_value_from_page_mut(page: *mut FPDF_PAGE) -> JsValue {
        Self::js_value_from_offset(page as usize)
    }

    /// Converts a pointer to an `FPDF_TEXTPAGE` struct to a [JsValue].
    #[inline]
    fn js_value_from_text_page(text_page: FPDF_TEXTPAGE) -> JsValue {
        Self::js_value_from_offset(text_page as usize)
    }

    /// Converts a pointer to an `FPDF_FORMHANDLE` struct to a [JsValue].
    #[inline]
    fn js_value_from_form(form: FPDF_FORMHANDLE) -> JsValue {
        Self::js_value_from_offset(form as usize)
    }

    /// Converts a pointer to an `FPDF_BITMAP` struct to a [JsValue].
    #[inline]
    fn js_value_from_bitmap(bitmap: FPDF_BITMAP) -> JsValue {
        Self::js_value_from_offset(bitmap as usize)
    }

    /// Converts a pointer to an `FPDF_ACTION` struct to a [JsValue].
    #[inline]
    fn js_value_from_action(action: FPDF_ACTION) -> JsValue {
        Self::js_value_from_offset(action as usize)
    }

    /// Converts a pointer to an `FPDF_DEST` struct to a [JsValue].
    #[inline]
    fn js_value_from_destination(dest: FPDF_DEST) -> JsValue {
        Self::js_value_from_offset(dest as usize)
    }

    /// Converts a pointer to an `FPDF_LINK` struct to a [JsValue].
    #[inline]
    fn js_value_from_link(link: FPDF_LINK) -> JsValue {
        Self::js_value_from_offset(link as usize)
    }

    /// Converts a pointer to an `FPDF_PAGEOBJECT` struct to a [JsValue].
    #[inline]
    fn js_value_from_object(object: FPDF_PAGEOBJECT) -> JsValue {
        Self::js_value_from_offset(object as usize)
    }

    /// Converts a pointer to an `FPDF_FONT` struct to a [JsValue].
    #[inline]
    fn js_value_from_font(font: FPDF_FONT) -> JsValue {
        Self::js_value_from_offset(font as usize)
    }

    /// Converts a pointer to an `FPDF_BOOKMARK` struct to a [JsValue].
    #[inline]
    fn js_value_from_bookmark(bookmark: FPDF_BOOKMARK) -> JsValue {
        Self::js_value_from_offset(bookmark as usize)
    }

    /// Converts a pointer to an `FPDF_PAGEOBJECTMARK` struct to a [JsValue].
    #[inline]
    fn js_value_from_mark(mark: FPDF_PAGEOBJECTMARK) -> JsValue {
        Self::js_value_from_offset(mark as usize)
    }

    /// Converts a pointer to an `FPDF_ANNOTATION` struct to a [JsValue].
    #[inline]
    fn js_value_from_annotation(annotation: FPDF_ANNOTATION) -> JsValue {
        Self::js_value_from_offset(annotation as usize)
    }

    /// Converts a pointer to an `FPDF_GLYPHPATH` struct to a [JsValue].
    #[inline]
    fn js_value_from_glyph_path(glyph_path: FPDF_GLYPHPATH) -> JsValue {
        Self::js_value_from_offset(glyph_path as usize)
    }

    /// Converts a pointer to an `FPDF_PAGERANGE` struct to a [JsValue].
    #[inline]
    fn js_value_from_page_range(page_range: FPDF_PAGERANGE) -> JsValue {
        Self::js_value_from_offset(page_range as usize)
    }

    /// Converts a pointer to an `FPDF_STRUCTTREE` struct to a [JsValue].
    #[inline]
    fn js_value_from_struct_tree(struct_tree: FPDF_STRUCTTREE) -> JsValue {
        Self::js_value_from_offset(struct_tree as usize)
    }

    /// Converts a pointer to an `FPDF_STRUCTELEMENT` struct to a [JsValue].
    #[inline]
    fn js_value_from_struct_element(struct_element: FPDF_STRUCTELEMENT) -> JsValue {
        Self::js_value_from_offset(struct_element as usize)
    }

    /// Converts a pointer to an `FPDF_STRUCTELEMENT_ATTR` struct to a [JsValue].
    #[inline]
    fn js_value_from_struct_attribute(struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> JsValue {
        Self::js_value_from_offset(struct_attribute as usize)
    }

    /// Converts a pointer to an `FPDF_STRUCTELEMENT_ATTR_VALUE` struct to a [JsValue].
    #[inline]
    fn js_value_from_struct_attribute_value(value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> JsValue {
        Self::js_value_from_offset(value as usize)
    }

    /// Converts a pointer to an `FPDF_SIGNATURE` struct to a [JsValue].
    #[inline]
    fn js_value_from_signature(signature: FPDF_SIGNATURE) -> JsValue {
        Self::js_value_from_offset(signature as usize)
    }

    /// Converts a pointer to an `FPDF_ATTACHMENT` struct to a [JsValue].
    #[inline]
    fn js_value_from_attachment(attachment: FPDF_ATTACHMENT) -> JsValue {
        Self::js_value_from_offset(attachment as usize)
    }

    /// Converts a pointer to an `FPDF_SCHHANDLE` struct to a [JsValue].
    #[inline]
    fn js_value_from_search(search: FPDF_SCHHANDLE) -> JsValue {
        Self::js_value_from_offset(search as usize)
    }

    /// Converts a pointer to an `FPDF_PAGELINK` struct to a [JsValue].
    #[inline]
    fn js_value_from_page_link(page_link: FPDF_PAGELINK) -> JsValue {
        Self::js_value_from_offset(page_link as usize)
    }

    /// Converts a pointer to an `FPDF_PATHSEGMENT` struct to a [JsValue].
    #[inline]
    fn js_value_from_segment(segment: FPDF_PATHSEGMENT) -> JsValue {
        Self::js_value_from_offset(segment as usize)
    }

    /// Converts a pointer to an `FPDF_CLIPPATH` struct to a [JsValue].
    #[inline]
    fn js_value_from_clip_path(clip_path: FPDF_CLIPPATH) -> JsValue {
        Self::js_value_from_offset(clip_path as usize)
    }

    /// Converts a pointer to an `FPDF_AVAIL` struct to a [JsValue].
    #[inline]
    fn js_value_from_avail(avail: FPDF_AVAIL) -> JsValue {
        Self::js_value_from_offset(avail as usize)
    }

    /// Converts a pointer to an `FPDF_XOBJECT` struct to a [JsValue].
    #[inline]
    fn js_value_from_xobject(xobject: FPDF_XOBJECT) -> JsValue {
        Self::js_value_from_offset(xobject as usize)
    }

    /// Converts a pointer to an `FPDF_JAVASCRIPT_ACTION` struct to a [JsValue].
    #[inline]
    fn js_value_from_javascript(javascript: FPDF_JAVASCRIPT_ACTION) -> JsValue {
        Self::js_value_from_offset(javascript as usize)
    }

    /// Converts a WASM memory heap offset to a [JsValue].
    #[inline]
    fn js_value_from_offset(offset: usize) -> JsValue {
        JsValue::from_f64(offset as f64)
    }

    /// Converts a `Vec<JsValue>` to a Javascript [Array].
    #[inline]
    fn js_array_from_vec(vec: Vec<JsValue>) -> Array {
        let array = Array::new_with_length(vec.len() as u32);

        for (index, value) in vec.into_iter().enumerate() {
            array.set(index as u32, value);
        }

        array
    }

    /// Calls an `FPDF_Get*Box()` function. Since all of these functions share the same
    /// signature, we abstract out the function call into this separate function so it can
    /// be re-used.
    #[inline]
    fn call_pdfium_get_page_box_fn(
        &self,
        fn_name: &str,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        // Generic function used to retrieve bounding box co-ordinates for the
        // FPDFPage_Get*Box() functions.

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_left = state.malloc(len);

        let ptr_bottom = state.malloc(len);

        let ptr_right = state.malloc(len);

        let ptr_top = state.malloc(len);

        let result = state
            .call(
                fn_name,
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(ptr_left),
                    &Self::js_value_from_offset(ptr_bottom),
                    &Self::js_value_from_offset(ptr_right),
                    &Self::js_value_from_offset(ptr_top),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *left = state
                    .copy_bytes_from_pdfium(ptr_left, len)
                    .try_into()
                    .map(f32::from_ne_bytes)
                    .unwrap_or(0_f32);

                *bottom = state
                    .copy_bytes_from_pdfium(ptr_bottom, len)
                    .try_into()
                    .map(f32::from_ne_bytes)
                    .unwrap_or(0_f32);

                *right = state
                    .copy_bytes_from_pdfium(ptr_right, len)
                    .try_into()
                    .map(f32::from_ne_bytes)
                    .unwrap_or(0_f32);

                *top = state
                    .copy_bytes_from_pdfium(ptr_top, len)
                    .try_into()
                    .map(f32::from_ne_bytes)
                    .unwrap_or(0_f32);
            }
        }

        state.free(ptr_left);
        state.free(ptr_bottom);
        state.free(ptr_right);
        state.free(ptr_top);

        result
    }

    /// Calls an FPDF_Set*Box() function. Since all of these functions share the same
    /// signature, we extract out the function call into this separate function so it can
    /// be re-used.
    #[inline]
    fn call_pdfium_set_page_box_fn(
        &self,
        fn_name: &str,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    ) {
        PdfiumRenderWasmState::lock().call(
            fn_name,
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Array::of5(
                &Self::js_value_from_page(page),
                &JsValue::from(left),
                &JsValue::from(bottom),
                &JsValue::from(right),
                &JsValue::from(top),
            ))),
        );
    }

    /// Calls a FORM_On*() event function. Since all of these functions share the same
    /// signature, we extract out the function call into this separate function so it can
    /// be re-used.
    #[inline]
    fn call_pdfium_form_mouse_fn(
        &self,
        fn_name: &str,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        PdfiumRenderWasmState::lock()
            .call(
                fn_name,
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(modifier as f64),
                    &JsValue::from_f64(page_x),
                    &JsValue::from_f64(page_y),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }
}

impl Default for WasmPdfiumBindings {
    #[inline]
    fn default() -> Self {
        WasmPdfiumBindings::new()
    }
}

impl PdfiumLibraryBindings for WasmPdfiumBindings {
    #[allow(non_snake_case)]
    fn FPDF_InitLibraryWithConfig(&self, config: *const FPDF_LIBRARY_CONFIG) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_InitLibraryWithConfig()");

        let state = PdfiumRenderWasmState::lock();

        let config_ptr = state.copy_ptr_with_len_to_pdfium(config, size_of::<FPDF_LIBRARY_CONFIG>());

        state.call(
            "FPDF_InitLibraryWithConfig",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_offset(config_ptr)))),
        );

        state.free(config_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDF_InitLibrary(&self) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_InitLibrary()");

        // Different Pdfium WASM builds have different ways of initializing the library.

        let state = PdfiumRenderWasmState::lock();

        let init = if state.get_value_from_pdfium_wasm_module("FPDF_InitLibrary").is_ok() {
            "PDF_InitLibrary"
        } else {
            "PDFium_Init"
        };

        state.call(init, JsFunctionArgumentType::Void, None, None);
    }

    #[allow(non_snake_case)]
    fn FPDF_SetSandBoxPolicy(&self, policy: FPDF_DWORD, enable: FPDF_BOOL) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SetSandBoxPolicy()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_SetSandBoxPolicy",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Number, JsFunctionArgumentType::Number]),
            Some(&JsValue::from(Array::of2(
                &JsValue::from(policy),
                &JsValue::from(enable as f64),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_DestroyLibrary(&self) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_DestroyLibrary()");

        PdfiumRenderWasmState::lock().call("FPDF_DestroyLibrary", JsFunctionArgumentType::Void, None, None);
    }

    #[allow(non_snake_case)]
    fn FPDF_GetLastError(&self) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetLastError()");

        PdfiumRenderWasmState::lock().get_last_error()
    }

    #[allow(non_snake_case)]
    fn FPDF_CreateNewDocument(&self) -> FPDF_DOCUMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CreateNewDocument()");

        PdfiumRenderWasmState::lock()
            .call("FPDF_CreateNewDocument", JsFunctionArgumentType::Pointer, None, None)
            .as_f64()
            .unwrap() as usize as FPDF_DOCUMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument64(&self, data_buf: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_LoadMemDocument64(): entering");

        let mut state = PdfiumRenderWasmState::lock_mut();

        let ptr = state.copy_bytes_to_pdfium(data_buf);

        let result = state
            .call(
                "FPDF_LoadMemDocument64",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::String,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_offset(ptr),
                    &Self::js_value_from_offset(data_buf.len()),
                    &JsValue::from(password.unwrap_or("")),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DOCUMENT;

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_LoadMemDocument64(): FPDF_DOCUMENT = {:#?}",
            result
        );

        state.set(
            format!("document_ptr_{:#?}", result).as_str(),
            JsValue::from_f64(ptr as f64),
        );

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_LoadMemDocument64(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_LoadCustomDocument(&self, pFileAccess: *mut FPDF_FILEACCESS, password: Option<&str>) -> FPDF_DOCUMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_LoadCustomDocument()");

        let state = PdfiumRenderWasmState::lock();

        let file_access_ptr = state.copy_file_access_to_pdfium(pFileAccess);

        state
            .call(
                "FPDF_LoadCustomDocument",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::String]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_offset(file_access_ptr),
                    &JsValue::from(password.unwrap_or("")),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DOCUMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_SaveAsCopy(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SaveAsCopy()");

        let file_write_ptr = {
            // Patch in the callback function we want Pdfium to invoke into Pdfium's function table.

            let mut state = PdfiumRenderWasmState::lock_mut();

            let entry = state.file_write_callback_function_table_entry;

            match state.patch_pdfium_function_table(entry, "write_block_from_callback_wasm") {
                Ok(_) => {}
                Err(err) => {
                    log::error!(
                        "pdfium-render::PdfiumLibraryBindings::FPDF_SaveAsCopy(): aborting with error {:#?}",
                        err
                    );

                    return self.FALSE();
                }
            };

            state.copy_file_write_to_pdfium(pFileWrite)

            // Mutable lock on state will be dropped at the end of this scope...
        };

        // ... meaning we can now have multiple shared immutable locks.

        let result = PdfiumRenderWasmState::lock()
            .call(
                "FPDF_SaveAsCopy",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(file_write_ptr),
                    &JsValue::from_f64(flags as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL;

        {
            // Un-patch our callback function, restoring the function we temporarily replaced.

            let mut state = PdfiumRenderWasmState::lock_mut();

            let entry = state.file_write_callback_function_table_entry;

            match state.unpatch_pdfium_function_table(entry) {
                Ok(_) => {}
                Err(err) => {
                    log::error!(
                        "pdfium-render::PdfiumLibraryBindings::FPDF_SaveAsCopy(): aborting with error {:#?}",
                        err
                    );

                    return self.FALSE();
                }
            };
        }

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_SaveWithVersion(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
        fileVersion: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SaveWithVersion()");

        let file_write_ptr = {
            // Patch in the callback function we want Pdfium to invoke into Pdfium's function table.

            let mut state = PdfiumRenderWasmState::lock_mut();

            let entry = state.file_write_callback_function_table_entry;

            match state.patch_pdfium_function_table(entry, "write_block_from_callback_wasm") {
                Ok(_) => {}
                Err(err) => {
                    log::error!(
                        "pdfium-render::PdfiumLibraryBindings::FPDF_SaveWithVersion(): aborting with error {:#?}",
                        err
                    );

                    return self.FALSE();
                }
            };

            state.copy_file_write_to_pdfium(pFileWrite)

            // Mutable lock on state will be dropped at the end of this scope...
        };

        // ... meaning we can now have multiple shared immutable locks.

        let result = PdfiumRenderWasmState::lock()
            .call(
                "FPDF_SaveWithVersion",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(file_write_ptr),
                    &JsValue::from_f64(flags as f64),
                    &JsValue::from_f64(fileVersion as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL;

        {
            // Un-patch our callback function, restoring the function we temporarily replaced.

            let mut state = PdfiumRenderWasmState::lock_mut();

            let entry = state.file_write_callback_function_table_entry;

            match state.unpatch_pdfium_function_table(entry) {
                Ok(_) => {}
                Err(err) => {
                    log::error!(
                        "pdfium-render::PdfiumLibraryBindings::FPDF_SaveWithVersion(): aborting with error {:#?}",
                        err
                    );

                    return self.FALSE();
                }
            };
        }

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Create(&self, file_avail: *mut FX_FILEAVAIL, file: *mut FPDF_FILEACCESS) -> FPDF_AVAIL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_Create()");

        let state = PdfiumRenderWasmState::lock();

        let out_file_avail_length = size_of::<FX_FILEAVAIL>();

        let out_file_avail_ptr = state.copy_struct_to_pdfium(file_avail);

        let out_file_length = size_of::<FPDF_FILEACCESS>();

        let out_file_ptr = state.copy_struct_to_pdfium(file);

        let result = state
            .call(
                "FPDFAvail_Create",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_offset(out_file_avail_ptr),
                    &Self::js_value_from_offset(out_file_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_AVAIL;

        state.copy_struct_from_pdfium(out_file_avail_ptr, out_file_avail_length, file_avail);
        state.copy_struct_from_pdfium(out_file_ptr, out_file_length, file);

        state.free(out_file_avail_ptr);
        state.free(out_file_ptr);

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_Destroy(&self, avail: FPDF_AVAIL) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_Destroy()");

        PdfiumRenderWasmState::lock().call(
            "FPDFAvail_Destroy",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_avail(avail)))),
        );
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsDocAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_IsDocAvail()");

        let state = PdfiumRenderWasmState::lock();

        let out_hints_length = size_of::<FX_DOWNLOADHINTS>();

        let out_hints_ptr = state.copy_struct_to_pdfium(hints);

        let result = state
            .call(
                "FPDFAvail_IsDocAvail",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_avail(avail),
                    &Self::js_value_from_offset(out_hints_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as c_int;

        state.copy_struct_from_pdfium(out_hints_ptr, out_hints_length, hints);

        state.free(out_hints_ptr);

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetDocument(&self, avail: FPDF_AVAIL, password: Option<&str>) -> FPDF_DOCUMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_GetDocument(): entering");

        let result = PdfiumRenderWasmState::lock()
            .call(
                "FPDFAvail_GetDocument",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::String]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_avail(avail),
                    &JsValue::from(password.unwrap_or("")),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DOCUMENT;

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAvail_GetDocument(): FPDF_DOCUMENT = {:#?}",
            result
        );

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_GetDocument(): leaving");

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_GetFirstPageNum(&self, doc: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_GetFirstPageNum()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAvail_GetFirstPageNum",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(doc)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsPageAvail(&self, avail: FPDF_AVAIL, page_index: c_int, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_IsPageAvail()");

        let state = PdfiumRenderWasmState::lock();

        let out_hints_length = size_of::<FX_DOWNLOADHINTS>();

        let out_hints_ptr = state.copy_struct_to_pdfium(hints);

        let result = state
            .call(
                "FPDFAvail_IsPageAvail",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_avail(avail),
                    &JsValue::from(page_index),
                    &Self::js_value_from_offset(out_hints_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as c_int;

        state.copy_struct_from_pdfium(out_hints_ptr, out_hints_length, hints);

        state.free(out_hints_ptr);

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsFormAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_IsFormAvail()");

        let state = PdfiumRenderWasmState::lock();

        let out_hints_length = size_of::<FX_DOWNLOADHINTS>();

        let out_hints_ptr = state.copy_struct_to_pdfium(hints);

        let result = state
            .call(
                "FPDFAvail_IsFormAvail",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_avail(avail),
                    &Self::js_value_from_offset(out_hints_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as c_int;

        state.copy_struct_from_pdfium(out_hints_ptr, out_hints_length, hints);

        state.free(out_hints_ptr);

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAvail_IsLinearized(&self, avail: FPDF_AVAIL) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAvail_IsLinearized()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAvail_IsLinearized",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_avail(avail)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CloseDocument(): entering");

        let state = PdfiumRenderWasmState::lock();

        state.call(
            "FPDF_CloseDocument",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
        );

        // The bytes representing the document were previously copied across to a separate
        // buffer in Pdfium's WASM module using state.copy_bytes_to_pdfium().
        // Free that buffer now.

        let key = format!("document_ptr_{:#?}", document);

        if let Some(ptr) = state.get(key.as_str()) {
            state.free(ptr.as_f64().unwrap() as usize);
        }

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CloseDocument(): leaving");
    }

    #[allow(non_snake_case)]
    fn FPDF_DeviceToPage(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_DeviceToPage()");

        let state = PdfiumRenderWasmState::lock();

        let page_x_length = size_of::<c_double>();
        let page_x_ptr = state.malloc(page_x_length);

        let page_y_length = size_of::<c_double>();
        let page_y_ptr = state.malloc(page_y_length);

        let result = state
            .call(
                "FPDF_DeviceToPage",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_page(page),
                    JsValue::from(start_x),
                    JsValue::from(start_y),
                    JsValue::from(size_x),
                    JsValue::from(size_y),
                    JsValue::from(rotate),
                    JsValue::from(device_x),
                    JsValue::from(device_y),
                    Self::js_value_from_offset(page_x_ptr),
                    Self::js_value_from_offset(page_y_ptr),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(page_x_ptr, page_x_length, page_x);
            state.copy_struct_from_pdfium(page_y_ptr, page_y_length, page_y);
        }

        state.free(page_x_ptr);
        state.free(page_y_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_PageToDevice(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_PageToDevice()");

        let state = PdfiumRenderWasmState::lock();

        let device_x_length = size_of::<c_int>();
        let device_x_ptr = state.malloc(device_x_length);

        let device_y_length = size_of::<c_int>();
        let device_y_ptr = state.malloc(device_y_length);

        let result = state
            .call(
                "FPDF_PageToDevice",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_page(page),
                    JsValue::from(start_x),
                    JsValue::from(start_y),
                    JsValue::from(size_x),
                    JsValue::from(size_y),
                    JsValue::from(rotate),
                    JsValue::from(page_x),
                    JsValue::from(page_y),
                    Self::js_value_from_offset(device_x_ptr),
                    Self::js_value_from_offset(device_y_ptr),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(device_x_ptr, device_x_length, device_x);
            state.copy_struct_from_pdfium(device_y_ptr, device_y_length, device_y);
        }

        state.free(device_x_ptr);
        state.free(device_y_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_GetFileVersion(&self, doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetFileVersion()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<c_int>();

        let buffer_ptr = state.malloc(buffer_length);

        let result = state
            .call(
                "FPDF_GetFileVersion",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(doc),
                    &Self::js_value_from_offset(buffer_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(buffer_ptr, buffer_length, fileVersion);
        }

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_DocumentHasValidCrossReferenceTable(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_DocumentHasValidCrossReferenceTable()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_DocumentHasValidCrossReferenceTable",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDF_GetTrailerEnds(&self, document: FPDF_DOCUMENT, buffer: *mut c_uint, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetTrailerEnds(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_GetTrailerEnds(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetTrailerEnds(): calling FPDF_GetTrailerEnds()");

        let result = state
            .call(
                "FPDF_GetTrailerEnds",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(buffer_ptr),
                    &Self::js_value_from_offset(buffer_length),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetTrailerEnds(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetFileIdentifier(
        &self,
        document: FPDF_DOCUMENT,
        id_type: FPDF_FILEIDTYPE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetFileIdentifier(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_GetFileIdentifier(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetFileIdentifier(): calling FPDF_GetFileIdentifier()");

        let result = state
            .call(
                "FPDF_GetFileIdentifier",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(id_type as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &Self::js_value_from_offset(buffer_length),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetFileIdentifier(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetMetaText(&self, document: FPDF_DOCUMENT, tag: &str, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetMetaText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let c_tag = CString::new(tag).unwrap();

        let tag_ptr = state.copy_bytes_to_pdfium(&c_tag.into_bytes_with_nul());

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_GetMetaText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetMetaText(): calling FPDF_GetMetaText()");

        let result = state
            .call(
                "FPDF_GetMetaText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(tag_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &Self::js_value_from_offset(buffer_length),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);
        state.free(tag_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetMetaText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDocPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDocPermissions()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetDocPermissions",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDocUserPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDocUserPermissions()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetDocUserPermissions",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetSecurityHandlerRevision(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetSecurityHandlerRevision()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetSecurityHandlerRevision",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_LoadPage()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_LoadPage",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(page_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGE
    }

    #[allow(non_snake_case)]
    fn FPDF_ClosePage(&self, page: FPDF_PAGE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_ClosePage()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_ClosePage",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithColorScheme_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        color_scheme: *const FPDF_COLORSCHEME,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPageBitmapWithColorScheme_Start()");

        let state = PdfiumRenderWasmState::lock();

        // null is a valid value for the pause argument - handle this.

        let pause_length = size_of::<IFSDK_PAUSE>();

        let pause_ptr = if pause.is_null() {
            pause as usize
        } else {
            state.copy_struct_to_pdfium_mut(pause)
        };

        let color_scheme_ptr = state.copy_struct_to_pdfium(color_scheme);

        let result = state
            .call(
                "FPDF_RenderPageBitmapWithColorScheme_Start",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_bitmap(bitmap),
                    Self::js_value_from_page(page),
                    JsValue::from_f64(start_x as f64),
                    JsValue::from_f64(start_y as f64),
                    JsValue::from_f64(size_x as f64),
                    JsValue::from_f64(size_y as f64),
                    JsValue::from_f64(rotate as f64),
                    JsValue::from_f64(flags as f64),
                    Self::js_value_from_offset(color_scheme_ptr),
                    Self::js_value_from_offset(pause_ptr),
                ]))),
            )
            .as_f64()
            .unwrap() as c_int;

        if !pause.is_null() {
            state.copy_struct_from_pdfium(pause_ptr, pause_length, pause);
            state.free(pause_ptr);
        }

        state.free(color_scheme_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPageBitmap_Start()");

        let state = PdfiumRenderWasmState::lock();

        // null is a valid value for the pause argument - handle this.

        let pause_length = size_of::<IFSDK_PAUSE>();

        let pause_ptr = if pause.is_null() {
            pause as usize
        } else {
            state.copy_struct_to_pdfium_mut(pause)
        };

        let result = state
            .call(
                "FPDF_RenderPageBitmap_Start",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_bitmap(bitmap),
                    Self::js_value_from_page(page),
                    JsValue::from_f64(start_x as f64),
                    JsValue::from_f64(start_y as f64),
                    JsValue::from_f64(size_x as f64),
                    JsValue::from_f64(size_y as f64),
                    JsValue::from_f64(rotate as f64),
                    JsValue::from_f64(flags as f64),
                    Self::js_value_from_offset(pause_ptr),
                ]))),
            )
            .as_f64()
            .unwrap() as c_int;

        if !pause.is_null() {
            state.copy_struct_from_pdfium(pause_ptr, pause_length, pause);
            state.free(pause_ptr);
        }

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Continue(&self, page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPage_Continue()");

        let state = PdfiumRenderWasmState::lock();

        // null is a valid value for the pause argument - handle this.

        let pause_length = size_of::<IFSDK_PAUSE>();

        let pause_ptr = if pause.is_null() {
            pause as usize
        } else {
            state.copy_struct_to_pdfium_mut(pause)
        };

        let result = state
            .call(
                "FPDF_RenderPage_Continue",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(pause_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as c_int;

        if !pause.is_null() {
            state.copy_struct_from_pdfium(pause_ptr, pause_length, pause);
            state.free(pause_ptr);
        }

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Close(&self, page: FPDF_PAGE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPage_Close()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_RenderPage_Close",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: *const c_int,
        length: c_ulong,
        index: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_ImportPagesByIndex()");

        let state = PdfiumRenderWasmState::lock();

        let page_indices_ptr = state.copy_ptr_with_len_to_pdfium(page_indices, length as usize * size_of::<c_int>());

        let result = state
            .call(
                "FPDF_ImportPagesByIndex",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(dest_doc),
                    &Self::js_value_from_document(src_doc),
                    &Self::js_value_from_offset(page_indices_ptr),
                    &JsValue::from_f64(length as f64),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(page_indices_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_ImportPages(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        pagerange: &str,
        index: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_ImportPages()");

        let state = PdfiumRenderWasmState::lock();

        let c_page_range = CString::new(pagerange).unwrap();

        let page_range_ptr = state.copy_bytes_to_pdfium(&c_page_range.into_bytes_with_nul());

        let result = state
            .call(
                "FPDF_ImportPages",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(dest_doc),
                    &Self::js_value_from_document(src_doc),
                    &Self::js_value_from_offset(page_range_ptr),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL;

        state.free(page_range_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_ImportNPagesToOne(
        &self,
        src_doc: FPDF_DOCUMENT,
        output_width: c_float,
        output_height: c_float,
        num_pages_on_x_axis: size_t,
        num_pages_on_y_axis: size_t,
    ) -> FPDF_DOCUMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_ImportNPagesToOne()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_ImportNPagesToOne",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(src_doc),
                    &JsValue::from(output_width),
                    &JsValue::from(output_height),
                    &JsValue::from_f64(num_pages_on_x_axis as f64),
                    &JsValue::from_f64(num_pages_on_y_axis as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DOCUMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_NewXObjectFromPage(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        src_page_index: c_int,
    ) -> FPDF_XOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_NewXObjectFromPage()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_NewXObjectFromPage",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(dest_doc),
                    &Self::js_value_from_document(src_doc),
                    &JsValue::from_f64(src_page_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_XOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDF_CloseXObject(&self, xobject: FPDF_XOBJECT) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CloseXObject()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_CloseXObject",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_xobject(xobject)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_NewFormObjectFromXObject(&self, xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_NewFormObjectFromXObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_NewFormObjectFromXObject",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_xobject(xobject)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDF_CopyViewerPreferences(&self, dest_doc: FPDF_DOCUMENT, src_doc: FPDF_DOCUMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CopyViewerPreferences()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_CopyViewerPreferences",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(dest_doc),
                    &Self::js_value_from_document(src_doc),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageWidthF(&self, page: FPDF_PAGE) -> c_float {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageWidthF()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageWidthF",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as c_float
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageWidth(&self, page: FPDF_PAGE) -> f64 {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageWidth()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageWidth",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageHeightF(&self, page: FPDF_PAGE) -> c_float {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageHeightF()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageHeightF",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as c_float
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageHeight(&self, page: FPDF_PAGE) -> f64 {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageHeight()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageHeight",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageLabel(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageLabel(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_GetPageLabel(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageLabel(): calling FPDF_GetPageLabel()");

        let result = state
            .call(
                "FPDF_GetPageLabel",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &JsValue::from(page_index),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageLabel(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexFromTextIndex(&self, text_page: FPDF_TEXTPAGE, nTextIndex: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetCharIndexFromTextIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetCharIndexFromTextIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(nTextIndex as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetTextIndexFromCharIndex(&self, text_page: FPDF_TEXTPAGE, nCharIndex: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetTextIndexFromCharIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetTextIndexFromCharIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(nCharIndex as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_GetSignatureCount(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetSignatureCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetSignatureCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_GetSignatureObject(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetSignatureObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetSignatureObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_SIGNATURE
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetContents(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetContents(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetContents(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetContents(): calling FPDFSignatureObj_GetContents()"
        );

        let result = state
            .call(
                "FPDFSignatureObj_GetContents",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_signature(signature),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetContents(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetByteRange(&self, signature: FPDF_SIGNATURE, buffer: *mut c_int, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetByteRange(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetByteRange(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetByteRange(): calling FPDFSignatureObj_GetByteRange()"
        );

        let result = state
            .call(
                "FPDFSignatureObj_GetByteRange",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_signature(signature),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetByteRange(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetSubFilter(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetSubFilter(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetSubFilter(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetSubFilter(): calling FPDFSignatureObj_GetSubFilter()"
        );

        let result = state
            .call(
                "FPDFSignatureObj_GetSubFilter",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_signature(signature),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetSubFilter(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetReason(&self, signature: FPDF_SIGNATURE, buffer: *mut c_void, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetReason(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetReason(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetReason(): calling FPDFSignatureObj_GetReason()"
        );

        let result = state
            .call(
                "FPDFSignatureObj_GetReason",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_signature(signature),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetReason(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetTime(&self, signature: FPDF_SIGNATURE, buffer: *mut c_char, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetTime(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetTime(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetTime(): calling FPDFSignatureObj_GetTime()"
        );

        let result = state
            .call(
                "FPDFSignatureObj_GetTime",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_signature(signature),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetTime(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetDocMDPPermission(&self, signature: FPDF_SIGNATURE) -> c_uint {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFSignatureObj_GetDocMDPPermission()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFSignatureObj_GetDocMDPPermission",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_signature(signature)))),
            )
            .as_f64()
            .unwrap() as c_uint
    }

    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetForPage(&self, page: FPDF_PAGE) -> FPDF_STRUCTTREE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructTree_GetForPage()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructTree_GetForPage",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTTREE
    }

    #[allow(non_snake_case)]
    fn FPDF_StructTree_Close(&self, struct_tree: FPDF_STRUCTTREE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructTree_Close()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_StructTree_Close",
            JsFunctionArgumentType::Number,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_tree(
                struct_tree,
            )))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_StructTree_CountChildren(&self, struct_tree: FPDF_STRUCTTREE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructTree_CountChildren()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructTree_CountChildren",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_tree(
                    struct_tree,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetChildAtIndex(&self, struct_tree: FPDF_STRUCTTREE, index: c_int) -> FPDF_STRUCTELEMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructTree_GetChildAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructTree_GetChildAtIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_tree(struct_tree),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAltText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAltText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAltText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAltText(): calling FPDF_StructElement_GetAltText()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetAltText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAltText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetActualText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetActualText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetActualText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetActualText(): calling FPDF_StructElement_GetActualText()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetActualText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetActualText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetID(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetID(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetID(): calling FPDF_StructElement_GetID()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetID",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetID(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetLang(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetLang(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetLang(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetLang(): calling FPDF_StructElement_GetLang()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetLang",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetLang(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetStringAttribute(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        attr_name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetStringAttribute(): entering");

        let state = PdfiumRenderWasmState::lock();

        let c_attr_name = CString::new(attr_name).unwrap();

        let attr_name_ptr = state.copy_bytes_to_pdfium(&c_attr_name.into_bytes_with_nul());

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetStringAttribute(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetStringAttribute(): calling FPDF_StructElement_GetStringAttribute()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetStringAttribute",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(attr_name_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        state.free(attr_name_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetStringAttribute(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentID()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetMarkedContentID",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_element(
                    struct_element,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetType(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetType(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetType(): calling FPDF_StructElement_GetType()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetType",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetType(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetObjType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetObjType(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetObjType(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetObjType(): calling FPDF_StructElement_GetObjType()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetObjType",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetObjType(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetTitle(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetTitle(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetTitle(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetTitle(): calling FPDF_StructElement_GetTitle()"
        );

        let result = state
            .call(
                "FPDF_StructElement_GetTitle",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_struct_element(struct_element),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetTitle(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_CountChildren(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_CountChildren()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_CountChildren",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_element(
                    struct_element,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetChildAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetChildAtIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_element(struct_element),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetChildMarkedContentID()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetChildMarkedContentID",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_element(struct_element),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetParent(&self, struct_element: FPDF_STRUCTELEMENT) -> FPDF_STRUCTELEMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetParent()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetParent",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_element(
                    struct_element,
                )))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAttributeCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetAttributeCount",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_element(
                    struct_element,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetAttributeAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetAttributeAtIndex",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_element(struct_element),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT_ATTR
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetCount(&self, struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_Attr_GetCount",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_attribute(
                    struct_attribute,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetName(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetName()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_struct_attribute(struct_attribute),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_name = CString::new(name).unwrap();

        let name_ptr = state.copy_bytes_to_pdfium(&c_name.into_bytes_with_nul());

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetValue",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_attribute(struct_attribute),
                    &Self::js_value_from_offset(name_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT_ATTR_VALUE;

        state.free(name_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> FPDF_OBJECT_TYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_Attr_GetType",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_attribute_value(
                    value,
                )))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_OBJECT_TYPE
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetBooleanValue()");

        let state = PdfiumRenderWasmState::lock();

        let out_value_length = size_of::<FPDF_BOOL>();

        let out_value_ptr = state.malloc(out_value_length);

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetBooleanValue",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_attribute_value(value),
                    &Self::js_value_from_offset(out_value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_value = state
                    .copy_bytes_from_pdfium(out_value_ptr, out_value_length)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(out_value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut f32,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetNumberValue()");

        let state = PdfiumRenderWasmState::lock();

        let out_value_length = size_of::<f32>();

        let out_value_ptr = state.malloc(out_value_length);

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetNumberValue",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_attribute_value(value),
                    &Self::js_value_from_offset(out_value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_value = state
                    .copy_bytes_from_pdfium(out_value_ptr, out_value_length)
                    .try_into()
                    .map(f32::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(out_value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetStringValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetStringValue()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_struct_attribute_value(value),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBlobValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetBlobValue()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDF_StructElement_Attr_GetBlobValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_struct_attribute_value(value),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_CountChildren(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetBlobValue()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_Attr_GetBlobValue",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_attribute_value(
                    value,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetChildAtIndex(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_Attr_GetChildAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_Attr_GetChildAtIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_attribute_value(value),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_STRUCTELEMENT_ATTR_VALUE
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetMarkedContentIdCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_struct_element(
                    struct_element,
                )))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdAtIndex(&self, struct_element: FPDF_STRUCTELEMENT, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_StructElement_GetMarkedContentIdAtIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_struct_element(struct_element),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_New(&self, document: FPDF_DOCUMENT, page_index: c_int, width: c_double, height: c_double) -> FPDF_PAGE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_New()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_New",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(page_index as f64),
                    &JsValue::from(width),
                    &JsValue::from(height),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGE
    }

    #[allow(non_snake_case)]
    fn FPDFPage_Delete(&self, document: FPDF_DOCUMENT, page_index: c_int) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_Delete()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_Delete",
            JsFunctionArgumentType::Number,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_document(document),
                &JsValue::from_f64(page_index as f64),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_MovePages(
        &self,
        document: FPDF_DOCUMENT,
        page_indices: *const c_int,
        page_indices_len: c_ulong,
        dest_page_index: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_MovePages()");

        let state = PdfiumRenderWasmState::lock();

        let page_indices_ptr =
            state.copy_ptr_with_len_to_pdfium(page_indices, page_indices_len as usize * size_of::<c_int>());

        let result = state
            .call(
                "FPDF_MovePages",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(page_indices_ptr),
                    &JsValue::from_f64(page_indices_len as f64),
                    &JsValue::from_f64(dest_page_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(page_indices_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetRotation()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetRotation",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetRotation(&self, page: FPDF_PAGE, rotate: c_int) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetRotation()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_SetRotation",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_page(page),
                &JsValue::from(rotate),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageBoundingBox(&self, page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetPageBoundingBox()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<FS_RECTF>();

        let buffer_ptr = state.malloc(buffer_length);

        let result = state
            .call(
                "FPDF_GetPageBoundingBox",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(buffer_ptr, buffer_length, rect);
        }

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndexF(&self, document: FPDF_DOCUMENT, page_index: c_int, size: *mut FS_SIZEF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageSizeByIndexF()");

        let state = PdfiumRenderWasmState::lock();

        let size_length = size_of::<FS_SIZEF>();

        let size_ptr = state.malloc(size_length);

        let result = state
            .call(
                "FPDF_GetPageSizeByIndexF",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &JsValue::from(page_index),
                    &Self::js_value_from_offset(size_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(size_ptr, size_length, size);
        }

        state.free(size_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndex(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageSizeByIndex()");

        let state = PdfiumRenderWasmState::lock();

        let f64_length = size_of::<f64>();

        let width_ptr = state.malloc(f64_length);

        let height_ptr = state.malloc(f64_length);

        let result = state
            .call(
                "FPDF_GetPageSizeByIndex",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &JsValue::from(page_index),
                    &Self::js_value_from_offset(width_ptr),
                    &Self::js_value_from_offset(height_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as c_int;

        if result != 0 {
            unsafe {
                *width = state
                    .copy_bytes_from_pdfium(width_ptr, f64_length)
                    .try_into()
                    .map(f64::from_le_bytes)
                    .unwrap_or(0.0);

                *height = state
                    .copy_bytes_from_pdfium(height_ptr, f64_length)
                    .try_into()
                    .map(f64::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(width_ptr);
        state.free(height_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetMediaBox()");

        self.call_pdfium_get_page_box_fn("FPDFPage_GetMediaBox", page, left, bottom, right, top)
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetCropBox()");

        self.call_pdfium_get_page_box_fn("FPDFPage_GetCropBox", page, left, bottom, right, top)
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetBleedBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetBleedBox()");

        self.call_pdfium_get_page_box_fn("FPDFPage_GetBleedBox", page, left, bottom, right, top)
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetTrimBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetTrimBox()");

        self.call_pdfium_get_page_box_fn("FPDFPage_GetTrimBox", page, left, bottom, right, top)
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetArtBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetArtBox()");

        self.call_pdfium_get_page_box_fn("FPDFPage_GetArtBox", page, left, bottom, right, top)
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetMediaBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetMediaBox()");

        self.call_pdfium_set_page_box_fn("FPDFPage_SetMediaBox", page, left, bottom, right, top);
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetCropBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetCropBox()");

        self.call_pdfium_set_page_box_fn("FPDFPage_SetCropBox", page, left, bottom, right, top);
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetBleedBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetBleedBox()");

        self.call_pdfium_set_page_box_fn("FPDFPage_SetBleedBox", page, left, bottom, right, top);
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetTrimBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetTrimBox()");

        self.call_pdfium_set_page_box_fn("FPDFPage_SetTrimBox", page, left, bottom, right, top);
    }

    #[allow(non_snake_case)]
    fn FPDFPage_SetArtBox(&self, page: FPDF_PAGE, left: c_float, bottom: c_float, right: c_float, top: c_float) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_SetArtBox()");

        self.call_pdfium_set_page_box_fn("FPDFPage_SetArtBox", page, left, bottom, right, top);
    }

    #[allow(non_snake_case)]
    fn FPDFPage_TransFormWithClip(
        &self,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipRect: *const FS_RECTF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_TransFormWithClip()");

        let state = PdfiumRenderWasmState::lock();

        let matrix_ptr = state.copy_struct_to_pdfium(matrix);

        let clipRect_ptr = state.copy_struct_to_pdfium(clipRect);

        let result = state
            .call(
                "FPDFPage_TransFormWithClip",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(matrix_ptr),
                    &Self::js_value_from_offset(clipRect_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(matrix_ptr);

        state.free(clipRect_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformClipPath(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_TransformClipPath()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPageObj_TransformClipPath",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Self::js_array_from_vec(vec![
                Self::js_value_from_object(page_object),
                JsValue::from(a),
                JsValue::from(b),
                JsValue::from(c),
                JsValue::from(d),
                JsValue::from(e),
                JsValue::from(f),
            ]))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetClipPath(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetClipPath()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetClipPath",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_CLIPPATH
    }

    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPaths(&self, clip_path: FPDF_CLIPPATH) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFClipPath_CountPaths()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFClipPath_CountPaths",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_clip_path(clip_path)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPathSegments(&self, clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFClipPath_CountPathSegments()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFClipPath_CountPathSegments",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_clip_path(clip_path),
                    &JsValue::from_f64(path_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFClipPath_GetPathSegment(
        &self,
        clip_path: FPDF_CLIPPATH,
        path_index: c_int,
        segment_index: c_int,
    ) -> FPDF_PATHSEGMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFClipPath_GetPathSegment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFClipPath_GetPathSegment",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_clip_path(clip_path),
                    &JsValue::from_f64(path_index as f64),
                    &JsValue::from_f64(segment_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PATHSEGMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_CreateClipPath(&self, left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CreateClipPath()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_CreateClipPath",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &JsValue::from_f64(left as f64),
                    &JsValue::from_f64(bottom as f64),
                    &JsValue::from_f64(right as f64),
                    &JsValue::from_f64(top as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_CLIPPATH
    }

    #[allow(non_snake_case)]
    fn FPDF_DestroyClipPath(&self, clipPath: FPDF_CLIPPATH) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_DestroyClipPath()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_DestroyClipPath",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_clip_path(clipPath)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPage_InsertClipPath(&self, page: FPDF_PAGE, clipPath: FPDF_CLIPPATH) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_InsertClipPath()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_InsertClipPath",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_page(page),
                &Self::js_value_from_clip_path(clipPath),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPage_HasTransparency(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_HasTransparency()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_HasTransparency",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GenerateContent(&self, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GenerateContent()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GenerateContent",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_TransformAnnots(
        &self,
        page: FPDF_PAGE,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_TransformAnnots()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_TransformAnnots",
            JsFunctionArgumentType::Number,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Self::js_array_from_vec(vec![
                Self::js_value_from_page(page),
                JsValue::from(a),
                JsValue::from(b),
                JsValue::from(c),
                JsValue::from(d),
                JsValue::from(e),
                JsValue::from(f),
            ]))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_Create(&self, width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_Create()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_Create",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &JsValue::from(width),
                    &JsValue::from(height),
                    &JsValue::from(alpha),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_CreateEx(
        &self,
        width: c_int,
        height: c_int,
        format: c_int,
        first_scan: *mut c_void,
        stride: c_int,
    ) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_CreateEx()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_CreateEx",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &JsValue::from(width),
                    &JsValue::from(height),
                    &JsValue::from(format),
                    &Self::js_value_from_offset(first_scan as usize),
                    &JsValue::from(stride),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_Destroy(&self, bitmap: FPDF_BITMAP) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_Destroy()");

        PdfiumRenderWasmState::lock().call(
            "FPDFBitmap_Destroy",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
        );
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetFormat(&self, bitmap: FPDF_BITMAP) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetFormat()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_GetFormat",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_FillRect(
        &self,
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_FillRect()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_FillRect",
                JsFunctionArgumentType::Void,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_bitmap(bitmap),
                    JsValue::from(left),
                    JsValue::from(top),
                    JsValue::from(width),
                    JsValue::from(height),
                    JsValue::from(color),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    // TODO: AJRC - 27/11/24 - remove deprecated item as part of #36
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *const c_void {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetBuffer()");

        let width = self.FPDFBitmap_GetWidth(bitmap);

        let height = self.FPDFBitmap_GetHeight(bitmap);

        let buffer_len = (width * height * PdfiumRenderWasmState::BYTES_PER_PIXEL) as usize;

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = state
            .call(
                "FPDFBitmap_GetBuffer",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as usize;

        let buffer = state.copy_bytes_from_pdfium(buffer_ptr, buffer_len);

        buffer.as_ptr() as *const c_void
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_GetBuffer_as_array(&self, bitmap: FPDF_BITMAP) -> Uint8Array {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetBuffer_as_array()");

        let buffer_len = (self.FPDFBitmap_GetStride(bitmap) * self.FPDFBitmap_GetHeight(bitmap)) as u32;

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = state
            .call(
                "FPDFBitmap_GetBuffer",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as u32;

        state.heap_u8().subarray(buffer_ptr, buffer_ptr + buffer_len)
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_SetBuffer(&self, bitmap: FPDF_BITMAP, buffer: &[u8]) -> bool {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_SetBuffer()");

        let buffer_length = (self.FPDFBitmap_GetStride(bitmap) * self.FPDFBitmap_GetHeight(bitmap)) as usize;

        if buffer.len() != buffer_length {
            return false;
        }

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = state
            .call(
                "FPDFBitmap_GetBuffer",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as usize;

        state.copy_bytes_to_pdfium_address(buffer, buffer_ptr);

        true
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_GetWidth(&self, bitmap: FPDF_BITMAP) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetWidth()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_GetWidth",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_GetHeight(&self, bitmap: FPDF_BITMAP) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetHeight()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_GetHeight",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFBitmap_GetStride(&self, bitmap: FPDF_BITMAP) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBitmap_GetStride()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBitmap_GetStride",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bitmap(bitmap)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPageBitmap()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_RenderPageBitmap",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Self::js_array_from_vec(vec![
                Self::js_value_from_bitmap(bitmap),
                Self::js_value_from_page(page),
                JsValue::from(start_x),
                JsValue::from(start_y),
                JsValue::from(size_x),
                JsValue::from(size_y),
                JsValue::from(rotate),
                JsValue::from(flags),
            ]))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithMatrix(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipping: *const FS_RECTF,
        flags: c_int,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RenderPageBitmapWithMatrix()");

        let state = PdfiumRenderWasmState::lock();

        let matrix_ptr = state.copy_struct_to_pdfium(matrix);

        let clipping_ptr = state.copy_struct_to_pdfium(clipping);

        state.call(
            "FPDF_RenderPageBitmapWithMatrix",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Array::of5(
                &Self::js_value_from_bitmap(bitmap),
                &Self::js_value_from_page(page),
                &Self::js_value_from_offset(matrix_ptr),
                &Self::js_value_from_offset(clipping_ptr),
                &JsValue::from(flags),
            ))),
        );

        state.free(matrix_ptr);
        state.free(clipping_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_IsSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_IsSupportedSubtype()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_IsSupportedSubtype",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of1(&JsValue::from_f64(subtype as f64)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_CreateAnnot(&self, page: FPDF_PAGE, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_ANNOTATION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_CreateAnnot()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_CreateAnnot",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(subtype as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ANNOTATION
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotCount(&self, page: FPDF_PAGE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetAnnotCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetAnnotCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetAnnot()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetAnnot",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ANNOTATION
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotIndex(&self, page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetAnnotIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetAnnotIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_CloseAnnot(&self, annot: FPDF_ANNOTATION) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetAnnotIndex()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_CloseAnnot",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPage_RemoveAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_RemoveAnnot()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_RemoveAnnot",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetSubtype(&self, annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetSubtype()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetSubtype",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as FPDF_ANNOTATION_SUBTYPE
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_IsObjectSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetSubtype()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_IsObjectSupportedSubtype",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&JsValue::from_f64(subtype as f64)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_UpdateObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_UpdateObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_UpdateObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_object(obj),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_AddInkStroke(&self, annot: FPDF_ANNOTATION, points: *const FS_POINTF, point_count: size_t) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_AddInkStroke()");

        let state = PdfiumRenderWasmState::lock();

        let points_ptr = state.copy_ptr_with_len_to_pdfium(points, point_count * size_of::<FS_POINTF>());

        let result = state
            .call(
                "FPDFAnnot_AddInkStroke",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(points_ptr),
                    &JsValue::from(point_count),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(points_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveInkList(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_RemoveInkList()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_RemoveInkList",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_AppendObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_AppendObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_object(obj),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObjectCount(&self, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetObjectCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetObjectCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetObject",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_RemoveObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_RemoveObject",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetColor()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_SetColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_annotation(annot),
                    JsValue::from(color_type),
                    JsValue::from(R),
                    JsValue::from(G),
                    JsValue::from(B),
                    JsValue::from(A),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetColor()");

        let state = PdfiumRenderWasmState::lock();

        let c_uint_len = size_of::<c_uint>();

        let ptr_r = state.malloc(c_uint_len);

        let ptr_g = state.malloc(c_uint_len);

        let ptr_b = state.malloc(c_uint_len);

        let ptr_a = state.malloc(c_uint_len);

        let result = state
            .call(
                "FPDFAnnot_GetColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_annotation(annot),
                    JsValue::from_f64(color_type as f64),
                    Self::js_value_from_offset(ptr_r),
                    Self::js_value_from_offset(ptr_g),
                    Self::js_value_from_offset(ptr_b),
                    Self::js_value_from_offset(ptr_a),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, c_uint_len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, c_uint_len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, c_uint_len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *A = state
                    .copy_bytes_from_pdfium(ptr_a, c_uint_len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);
        state.free(ptr_a);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_HasAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_HasAttachmentPoints()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_HasAttachmentPoints",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetAttachmentPoints()");

        let state = PdfiumRenderWasmState::lock();

        let quad_points_ptr = state.copy_struct_to_pdfium(quad_points);

        let result = state
            .call(
                "FPDFAnnot_SetAttachmentPoints",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from(quad_index),
                    &Self::js_value_from_offset(quad_points_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(quad_points_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_AppendAttachmentPoints()");

        let state = PdfiumRenderWasmState::lock();

        let quad_points_ptr = state.copy_struct_to_pdfium(quad_points);

        let result = state
            .call(
                "FPDFAnnot_AppendAttachmentPoints",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(quad_points_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(quad_points_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_CountAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> size_t {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_CountAttachmentPoints()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_CountAttachmentPoints",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as size_t
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetAttachmentPoints()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<FS_QUADPOINTSF>();

        let ptr_quad_points = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetAttachmentPoints",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(quad_index as f64),
                    &Self::js_value_from_offset(ptr_quad_points),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_quad_points, len, quad_points);
        }

        state.free(ptr_quad_points);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetRect(&self, annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetRect()");

        let state = PdfiumRenderWasmState::lock();

        let rect_ptr = state.copy_struct_to_pdfium(rect);

        let result = state
            .call(
                "FPDFAnnot_SetRect",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(rect_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(rect_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetRect(&self, annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetRect()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<FS_RECTF>();

        let ptr_rect = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetRect",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(ptr_rect),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_rect, len, rect);
        }

        state.free(ptr_rect);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetVertices(&self, annot: FPDF_ANNOTATION, buffer: *mut FS_POINTF, length: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetVertices()");

        let state = PdfiumRenderWasmState::lock();

        let len = length as usize * size_of::<FS_POINTF>();

        let ptr_buffer = if len > 0 { state.malloc(len) } else { 0 };

        let result = state
            .call(
                "FPDFAnnot_GetVertices",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(ptr_buffer),
                ))),
            )
            .as_f64()
            .unwrap() as c_ulong;

        if result > 0 && result <= length {
            state.copy_struct_from_pdfium(ptr_buffer, len, buffer);
        }

        state.free(ptr_buffer);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListCount(&self, annot: FPDF_ANNOTATION) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetInkListCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetInkListCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListPath(
        &self,
        annot: FPDF_ANNOTATION,
        path_index: c_ulong,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetInkListPath()");

        let state = PdfiumRenderWasmState::lock();

        let len = length as usize * size_of::<FS_POINTF>();

        let ptr_buffer = if len > 0 { state.malloc(len) } else { 0 };

        let result = state
            .call(
                "FPDFAnnot_GetInkListPath",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(path_index as f64),
                    &Self::js_value_from_offset(ptr_buffer),
                ))),
            )
            .as_f64()
            .unwrap() as c_ulong;

        if result > 0 && result <= length {
            state.copy_struct_from_pdfium(ptr_buffer, len, buffer);
        }

        state.free(ptr_buffer);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLine(&self, annot: FPDF_ANNOTATION, start: *mut FS_POINTF, end: *mut FS_POINTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetLine()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<FS_POINTF>();

        let ptr_start = state.malloc(len);

        let ptr_end = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetLine",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(ptr_start),
                    &Self::js_value_from_offset(ptr_end),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_start, len, start);
            state.copy_struct_from_pdfium(ptr_end, len, end);
        }

        state.free(ptr_start);
        state.free(ptr_end);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: c_float,
        vertical_radius: c_float,
        border_width: c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetBorder()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_SetBorder",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from(horizontal_radius),
                    &JsValue::from(vertical_radius),
                    &JsValue::from(border_width),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: *mut c_float,
        vertical_radius: *mut c_float,
        border_width: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetBorder()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_horizontal_radius = state.malloc(len);

        let ptr_vertical_radius = state.malloc(len);

        let ptr_border_width = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetBorder",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(ptr_horizontal_radius),
                    &Self::js_value_from_offset(ptr_vertical_radius),
                    &Self::js_value_from_offset(ptr_border_width),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *horizontal_radius = state
                    .copy_bytes_from_pdfium(ptr_horizontal_radius, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *vertical_radius = state
                    .copy_bytes_from_pdfium(ptr_vertical_radius, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *border_width = state
                    .copy_bytes_from_pdfium(ptr_border_width, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_horizontal_radius);
        state.free(ptr_vertical_radius);
        state.free(ptr_border_width);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormAdditionalActionJavaScript(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        event: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormAdditionalActionJavaScript(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormAdditionalActionJavaScript(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormAdditionalActionJavaScript(): calling FPDFAnnot_GetFormAdditionalActionJavaScript()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFormAdditionalActionJavaScript",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(event as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormAdditionalActionJavaScript(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAlternateName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldAlternateName(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldAlternateName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldAlternateName(): calling FPDFAnnot_GetFormFieldAlternateName()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFormFieldAlternateName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldAlternateName(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_HasKey(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_HasKey()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_HasKey",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetValueType(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_OBJECT_TYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetValueType()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_GetValueType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_OBJECT_TYPE;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue(&self, annot: FPDF_ANNOTATION, key: &str, value: FPDF_WIDESTRING) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetStringValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let value_ptr = state.copy_string_to_pdfium(value);

        let result = state
            .call(
                "FPDFAnnot_SetStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);
        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetStringValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetStringValue(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetStringValue(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetStringValue(): calling FPDFAnnot_GetStringValue()"
        );

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_GetStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);
        state.free(key_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetStringValue(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetNumberValue(&self, annot: FPDF_ANNOTATION, key: &str, value: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetNumberValue()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let value_ptr = state.malloc(len);

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_GetNumberValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *value = state
                    .copy_bytes_from_pdfium(value_ptr, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(value_ptr);
        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetAP()");

        let state = PdfiumRenderWasmState::lock();

        let value_ptr = state.copy_string_to_pdfium(value);

        let result = state
            .call(
                "FPDFAnnot_SetAP",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(appearanceMode as f64),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetAP(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetAP(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetAP(): calling FPDFAnnot_GetAP()");

        let result = state
            .call(
                "FPDFAnnot_GetAP",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(appearanceMode as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetAP(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLinkedAnnot(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_ANNOTATION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFlags()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_GetLinkedAnnot",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ANNOTATION;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFlags(&self, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFlags()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFlags",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFlags(&self, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetFlags()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_SetFlags",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(flags as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldFlags(&self, handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldFlags()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFormFieldFlags",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(handle),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFormFieldFlags(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetFormFieldFlags()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_SetFormFieldFlags",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(flags as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        point: *const FS_POINTF,
    ) -> FPDF_ANNOTATION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldAtPoint()");

        let state = PdfiumRenderWasmState::lock();

        let point_ptr = state.copy_struct_to_pdfium(point);

        let result = state
            .call(
                "FPDFAnnot_GetFormFieldAtPoint",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(point_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ANNOTATION;

        state.free(point_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldName(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldName(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldName(): calling FPDFAnnot_GetFormFieldName()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFormFieldName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldName(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldType(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFormFieldType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldValue(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldValue(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldValue(): calling FPDFAnnot_GetFormFieldValue()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFormFieldValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldValue(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetOptionCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetOptionCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionLabel(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetOptionLabel(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetOptionLabel(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetOptionLabel(): calling FPDFAnnot_GetOptionLabel()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetOptionLabel",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetOptionLabel(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_IsOptionSelected(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_IsOptionSelected()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_IsOptionSelected",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontSize(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION, value: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFontSize()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let value_ptr = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetFontSize",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *value = state
                    .copy_bytes_from_pdfium(value_ptr, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: c_uint,
        G: c_uint,
        B: c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetFontColor()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_SetFontColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_form(form),
                    Self::js_value_from_annotation(annot),
                    JsValue::from_f64(R as f64),
                    JsValue::from_f64(G as f64),
                    JsValue::from_f64(B as f64),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontColor(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFontColor()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_r = state.malloc(len);

        let ptr_g = state.malloc(len);

        let ptr_b = state.malloc(len);

        let result = state
            .call(
                "FPDFAnnot_GetFontColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_form(form),
                    Self::js_value_from_annotation(annot),
                    Self::js_value_from_offset(ptr_r),
                    Self::js_value_from_offset(ptr_g),
                    Self::js_value_from_offset(ptr_b),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_IsChecked(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_IsChecked()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_IsChecked",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetFocusableSubtypes()");

        let state = PdfiumRenderWasmState::lock();

        let subtypes_ptr = state.copy_ptr_with_len_to_pdfium(subtypes, count * size_of::<FPDF_ANNOTATION_SUBTYPE>());

        let result = state
            .call(
                "FPDFAnnot_SetFocusableSubtypes",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_offset(subtypes_ptr),
                    &JsValue::from(count),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(subtypes_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypesCount(&self, form: FPDF_FORMHANDLE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypesCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFocusableSubtypesCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypes(
        &self,
        form: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypes(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = count * size_of::<FPDF_ANNOTATION_SUBTYPE>();

        let subtypes_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypes(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypes(): calling FPDFAnnot_GetFocusableSubtypes()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFocusableSubtypes",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_offset(subtypes_ptr),
                    &JsValue::from_f64(count as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(subtypes_ptr, buffer_length, subtypes);
        }

        state.free(subtypes_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypes(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLink(&self, annot: FPDF_ANNOTATION) -> FPDF_LINK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetLink()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetLink",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_LINK
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlCount(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormControlCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFormControlCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlIndex(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormControlIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFormControlIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldExportValue(
        &self,
        form: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldExportValue(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldExportValue(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFormFieldExportValue(): calling FPDFAnnot_GetFormFieldExportValue()"
        );

        let result = state
            .call(
                "FPDFAnnot_GetFormFieldExportValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_SetURI(&self, annot: FPDF_ANNOTATION, uri: &str) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_SetURI()");

        let state = PdfiumRenderWasmState::lock();

        let c_uri = CString::new(uri).unwrap();

        let uri_ptr = state.copy_bytes_to_pdfium(&c_uri.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAnnot_SetURI",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(uri_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(uri_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFileAttachment(&self, annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_GetFileAttachment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAnnot_GetFileAttachment",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_annotation(annot)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ATTACHMENT
    }

    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment(&self, annot: FPDF_ANNOTATION, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAnnot_AddFileAttachment()");

        let state = PdfiumRenderWasmState::lock();

        let name_ptr = state.copy_string_to_pdfium(name);

        let result = state
            .call(
                "FPDFAnnot_AddFileAttachment",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_annotation(annot),
                    &Self::js_value_from_offset(name_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ATTACHMENT;

        state.free(name_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFDOC_InitFormFillEnvironment(
        &self,
        document: FPDF_DOCUMENT,
        form_info: *mut FPDF_FORMFILLINFO,
    ) -> FPDF_FORMHANDLE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment()");

        let state = PdfiumRenderWasmState::lock();

        let form_info_ptr = state.copy_struct_to_pdfium_mut(form_info);

        state
            .call(
                "FPDFDOC_InitFormFillEnvironment",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(form_info_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_FORMHANDLE

        // Returning here without calling state.free(form_info_ptr) leaks memory, but Pdfium
        // seems to expect the struct pointer to remain valid so long as the form handle is valid.
        // TODO: AJRC - 28/2/12 - we could use PdfiumRenderWasmState() to track which form handles
        // are currently in use and drop their struct ptrs when the forms are dropped
    }

    #[allow(non_snake_case)]
    fn FPDFDOC_ExitFormFillEnvironment(&self, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDOC_ExitFormFillEnvironment()");

        PdfiumRenderWasmState::lock().call(
            "FPDFDOC_ExitFormFillEnvironment",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_OnAfterLoadPage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnAfterLoadPage()");

        PdfiumRenderWasmState::lock().call(
            "FORM_OnAfterLoadPage",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_page(page),
                &Self::js_value_from_form(form),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_OnBeforeClosePage(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnBeforeClosePage()");

        PdfiumRenderWasmState::lock().call(
            "FORM_OnBeforeClosePage",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_page(page),
                &Self::js_value_from_form(form),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_GetPageMode(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_GetPageMode()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_GetPageMode",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_Flatten(&self, page: FPDF_PAGE, nFlag: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_Flatten()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_Flatten",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(nFlag as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as c_int
    }

    #[allow(non_snake_case)]
    fn FORM_DoDocumentJSAction(&self, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_DoDocumentJSAction()");

        PdfiumRenderWasmState::lock().call(
            "FORM_DoDocumentJSAction",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_DoDocumentOpenAction(&self, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_DoDocumentOpenAction()");

        PdfiumRenderWasmState::lock().call(
            "FORM_DoDocumentOpenAction",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_DoDocumentAAction(&self, form: FPDF_FORMHANDLE, aaType: c_int) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_DoDocumentAAction()");

        PdfiumRenderWasmState::lock().call(
            "FORM_DoDocumentAAction",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_form(form),
                &JsValue::from_f64(aaType as f64),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_DoPageAAction(&self, page: FPDF_PAGE, form: FPDF_FORMHANDLE, aaType: c_int) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_DoPageAAction()");

        PdfiumRenderWasmState::lock().call(
            "FORM_DoDocumFORM_DoPageAActionentAAction",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Array::of3(
                &Self::js_value_from_page(page),
                &Self::js_value_from_form(form),
                &JsValue::from_f64(aaType as f64),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FORM_OnMouseMove(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnMouseMove()");

        self.call_pdfium_form_mouse_fn("FORM_OnMouseMove", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnMouseWheel(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_coord: *const FS_POINTF,
        delta_x: c_int,
        delta_y: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnMouseWheel()");

        let state = PdfiumRenderWasmState::lock();

        let ptr_page_coord = state.copy_struct_to_pdfium(page_coord);

        let result = state
            .call(
                "FORM_OnMouseWheel",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_form(form),
                    Self::js_value_from_page(page),
                    JsValue::from_f64(modifier as f64),
                    Self::js_value_from_offset(ptr_page_coord),
                    JsValue::from_f64(delta_x as f64),
                    JsValue::from_f64(delta_y as f64),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(ptr_page_coord);

        result
    }

    #[allow(non_snake_case)]
    fn FORM_OnFocus(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnFocus()");

        self.call_pdfium_form_mouse_fn("FORM_OnFocus", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnLButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnLButtonDown()");

        self.call_pdfium_form_mouse_fn("FORM_OnLButtonDown", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnRButtonDown(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnRButtonDown()");

        self.call_pdfium_form_mouse_fn("FORM_OnRButtonDown", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnLButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnLButtonUp()");

        self.call_pdfium_form_mouse_fn("FORM_OnLButtonUp", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnRButtonUp(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnRButtonUp()");

        self.call_pdfium_form_mouse_fn("FORM_OnRButtonUp", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnLButtonDoubleClick(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnLButtonDoubleClick()");

        self.call_pdfium_form_mouse_fn("FORM_OnLButtonDoubleClick", form, page, modifier, page_x, page_y)
    }

    #[allow(non_snake_case)]
    fn FORM_OnKeyDown(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnKeyDown()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_OnKeyDown",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(nKeyCode as f64),
                    &JsValue::from_f64(modifier as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_OnKeyUp(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nKeyCode: c_int, modifier: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnKeyUp()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_OnKeyUp",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(nKeyCode as f64),
                    &JsValue::from_f64(modifier as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_OnChar(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, nChar: c_int, modifier: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_OnChar()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_OnChar",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(nChar as f64),
                    &JsValue::from_f64(modifier as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_GetFocusedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedText(): calling FORM_GetFocusedText()");

        let result = state
            .call(
                "FORM_GetFocusedText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FORM_GetSelectedText(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetSelectedText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FORM_GetSelectedText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetSelectedText(): calling FORM_GetSelectedText()");

        let result = state
            .call(
                "FORM_GetSelectedText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetSelectedText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FORM_ReplaceAndKeepSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_ReplaceAndKeepSelection()");

        let state = PdfiumRenderWasmState::lock();

        let wsText_ptr = state.copy_string_to_pdfium(wsText);

        state.call(
            "FORM_ReplaceAndKeepSelection",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
            ]),
            Some(&JsValue::from(Array::of3(
                &Self::js_value_from_form(form),
                &Self::js_value_from_page(page),
                &Self::js_value_from_offset(wsText_ptr),
            ))),
        );

        state.free(wsText_ptr);
    }

    #[allow(non_snake_case)]
    fn FORM_ReplaceSelection(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, wsText: FPDF_WIDESTRING) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_ReplaceSelection()");

        let state = PdfiumRenderWasmState::lock();

        let wsText_ptr = state.copy_string_to_pdfium(wsText);

        state.call(
            "FORM_ReplaceSelection",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
            ]),
            Some(&JsValue::from(Array::of3(
                &Self::js_value_from_form(form),
                &Self::js_value_from_page(page),
                &Self::js_value_from_offset(wsText_ptr),
            ))),
        );

        state.free(wsText_ptr);
    }

    #[allow(non_snake_case)]
    fn FORM_SelectAllText(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_SelectAllText()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_SelectAllText",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_CanUndo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_CanUndo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_CanUndo",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_CanRedo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_CanRedo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_CanRedo",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_Undo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_Undo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_Undo",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_Redo(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_Redo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_Redo",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_ForceToKillFocus(&self, form: FPDF_FORMHANDLE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_ForceToKillFocus()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_ForceToKillFocus",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_GetFocusedAnnot(
        &self,
        form: FPDF_FORMHANDLE,
        page_index: *mut c_int,
        annot: *mut FPDF_ANNOTATION,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedAnnot(): entering");

        let state = PdfiumRenderWasmState::lock();

        let page_index_length = size_of::<c_int>();

        let page_index_ptr = state.copy_ptr_with_len_to_pdfium(page_index, page_index_length);

        let annot_length = size_of::<FPDF_ANNOTATION>();

        let annot_ptr = state.copy_ptr_with_len_to_pdfium(annot, annot_length);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedAnnot(): calling FORM_GetFocusedAnnot()");

        let result = state
            .call(
                "FORM_GetFocusedAnnot",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_offset(page_index_ptr),
                    &Self::js_value_from_offset(annot_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *page_index = state
                    .copy_bytes_from_pdfium(page_index_ptr, page_index_length)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }

            state.copy_struct_from_pdfium(annot_ptr, annot_length, annot);
        }

        state.free(page_index_ptr);
        state.free(annot_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_GetFocusedAnnot(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FORM_SetFocusedAnnot(&self, form: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_SetFocusedAnnot()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_SetFocusedAnnot",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_annotation(annot),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_HasFormFieldAtPoint(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, page_x: f64, page_y: f64) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_HasFormFieldAtPoint()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_HasFormFieldAtPoint",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(page_x),
                    &JsValue::from_f64(page_y),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_FormFieldZOrderAtPoint(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        page_x: f64,
        page_y: f64,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_FormFieldZOrderAtPoint()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_FormFieldZOrderAtPoint",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(page_x),
                    &JsValue::from_f64(page_y),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightColor(&self, form: FPDF_FORMHANDLE, field_type: c_int, color: FPDF_DWORD) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SetFormFieldHighlightColor()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_SetFormFieldHighlightColor",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Array::of3(
                &Self::js_value_from_form(form),
                &JsValue::from(field_type),
                &JsValue::from(color),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightAlpha(&self, form: FPDF_FORMHANDLE, alpha: c_uchar) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SetFormFieldHighlightAlpha()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_SetFormFieldHighlightAlpha",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_form(form),
                &JsValue::from(alpha),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_RemoveFormFieldHighlight(&self, form: FPDF_FORMHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_RemoveFormFieldHighlight()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_RemoveFormFieldHighlight",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_form(form)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_FFLDraw(
        &self,
        form: FPDF_FORMHANDLE,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_FFLDraw()");

        PdfiumRenderWasmState::lock().call(
            "FPDF_FFLDraw",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Self::js_array_from_vec(vec![
                Self::js_value_from_form(form),
                Self::js_value_from_bitmap(bitmap),
                Self::js_value_from_page(page),
                JsValue::from_f64(start_x as f64),
                JsValue::from_f64(start_y as f64),
                JsValue::from_f64(size_x as f64),
                JsValue::from_f64(size_y as f64),
                JsValue::from_f64(rotate as f64),
                JsValue::from_f64(flags as f64),
            ]))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDF_GetFormType(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetFormType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetFormType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FORM_SetIndexSelected(
        &self,
        form: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        index: c_int,
        selected: FPDF_BOOL,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_SetIndexSelected()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_SetIndexSelected",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(index as f64),
                    &JsValue::from_f64(selected as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FORM_IsIndexSelected(&self, form: FPDF_FORMHANDLE, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FORM_IsIndexSelected()");

        PdfiumRenderWasmState::lock()
            .call(
                "FORM_IsIndexSelected",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_form(form),
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDF_LoadXFA(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_LoadXFA()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_LoadXFA",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptActionCount(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_GetJavaScriptActionCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_GetJavaScriptActionCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptAction(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_JAVASCRIPT_ACTION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_GetJavaScriptAction()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_GetJavaScriptAction",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_JAVASCRIPT_ACTION
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_CloseJavaScriptAction(&self, javascript: FPDF_JAVASCRIPT_ACTION) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_CloseJavaScriptAction()");

        PdfiumRenderWasmState::lock().call(
            "FPDFDoc_CloseJavaScriptAction",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_javascript(javascript)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetName(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetName(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetName(): calling FPDFJavaScriptAction_GetName()"
        );

        let result = state
            .call(
                "FPDFJavaScriptAction_GetName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_javascript(javascript),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetName(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetScript(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetScript(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetScript(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetScript(): calling FPDFJavaScriptAction_GetScript()"
        );

        let result = state
            .call(
                "FPDFJavaScriptAction_GetScript",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_javascript(javascript),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFJavaScriptAction_GetScript(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMap(&self) -> *const FPDF_CharsetFontMap {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDefaultTTFMap()");

        let state = PdfiumRenderWasmState::lock();

        let charset_font_map_ptr = state
            .call("FPDF_GetDefaultTTFMap", JsFunctionArgumentType::Pointer, None, None)
            .as_f64()
            .unwrap() as usize;

        let result = state.copy_bytes_from_pdfium(charset_font_map_ptr, size_of::<FPDF_CharsetFontMap>());

        result.as_ptr() as *const FPDF_CharsetFontMap
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapCount(&self) -> usize {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDefaultTTFMapCount()");

        PdfiumRenderWasmState::lock()
            .call("FPDF_GetDefaultTTFMapCount", JsFunctionArgumentType::Number, None, None)
            .as_f64()
            .unwrap() as usize
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapEntry(&self, index: usize) -> *const FPDF_CharsetFontMap {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDefaultTTFMapEntry()");

        let state = PdfiumRenderWasmState::lock();

        let charset_font_map_ptr = state
            .call(
                "FPDF_GetDefaultTTFMapEntry",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of1(&JsValue::from_f64(index as f64)))),
            )
            .as_f64()
            .unwrap() as usize;

        let result = state.copy_bytes_from_pdfium(charset_font_map_ptr, size_of::<FPDF_CharsetFontMap>());

        result.as_ptr() as *const FPDF_CharsetFontMap
    }

    #[allow(non_snake_case)]
    fn FPDF_AddInstalledFont(&self, mapper: *mut c_void, face: &str, charset: c_int) {
        // TODO: AJRC - 7-Sep-2024 - this almost certainly won't work on WASM because
        // mapper is (according to the documentation) meant to be a pointer to a Foxit font mapper,
        // which doesn't exist on WASM.

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_AddInstalledFont()");

        let state = PdfiumRenderWasmState::lock();

        let c_face = CString::new(face).unwrap();

        let face_ptr = state.copy_bytes_to_pdfium(&c_face.into_bytes_with_nul());

        let mapper_ptr = state.copy_struct_to_pdfium(mapper);

        state.call(
            "FPDF_AddInstalledFont",
            JsFunctionArgumentType::Number,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Array::of3(
                &Self::js_value_from_offset(mapper_ptr),
                &Self::js_value_from_offset(face_ptr),
                &JsValue::from_f64(charset as f64),
            ))),
        );

        state.free(mapper_ptr);
        state.free(face_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDF_SetSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_SetSystemFontInfo()");

        let state = PdfiumRenderWasmState::lock();

        let p_font_info_len = size_of::<FPDF_SYSFONTINFO>();

        let p_font_info_ptr = state.copy_ptr_with_len_to_pdfium(pFontInfo, p_font_info_len);

        state.call(
            "FPDF_SetSystemFontInfo",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_offset(p_font_info_ptr)))),
        );

        state.copy_struct_from_pdfium(p_font_info_ptr, p_font_info_len, pFontInfo);

        state.free(p_font_info_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDF_GetDefaultSystemFontInfo(&self) -> *mut FPDF_SYSFONTINFO {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetDefaultSystemFontInfo()");

        let state = PdfiumRenderWasmState::lock();

        let sys_font_info_ptr = state
            .call(
                "FPDF_GetDefaultSystemFontInfo",
                JsFunctionArgumentType::Pointer,
                None,
                None,
            )
            .as_f64()
            .unwrap() as usize;

        let result = state.copy_bytes_from_pdfium(sys_font_info_ptr, size_of::<FPDF_SYSFONTINFO>());

        result.as_ptr() as *mut FPDF_SYSFONTINFO
    }

    #[allow(non_snake_case)]
    fn FPDF_FreeDefaultSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_FreeDefaultSystemFontInfo()");

        let state = PdfiumRenderWasmState::lock();

        let p_font_info_len = size_of::<FPDF_SYSFONTINFO>();

        let p_font_info_ptr = state.copy_ptr_with_len_to_pdfium(pFontInfo, p_font_info_len);

        state.call(
            "FPDF_FreeDefaultSystemFontInfo",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_offset(p_font_info_ptr)))),
        );

        state.copy_struct_from_pdfium(p_font_info_ptr, p_font_info_len, pFontInfo);

        state.free(p_font_info_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetFirstChild(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetFirstChild()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBookmark_GetFirstChild",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_bookmark(bookmark),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOKMARK
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetNextSibling(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_BOOKMARK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetNextSibling()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBookmark_GetNextSibling",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_bookmark(bookmark),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOKMARK
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetTitle(&self, bookmark: FPDF_BOOKMARK, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetTitle(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetTitle(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetTitle(): calling FPDFBookmark_GetTitle()");

        let result = state
            .call(
                "FPDFBookmark_GetTitle",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_bookmark(bookmark),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetTitle(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetCount(&self, bookmark: FPDF_BOOKMARK) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBookmark_GetCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bookmark(bookmark)))),
            )
            .as_f64()
            .unwrap() as usize as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_Find(&self, document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_Find()");

        let state = PdfiumRenderWasmState::lock();

        let title_ptr = state.copy_string_to_pdfium(title);

        let result = state
            .call(
                "FPDFBookmark_Find",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(title_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOKMARK;

        state.free(title_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetDest(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetDest()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBookmark_GetDest",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_bookmark(bookmark),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DEST
    }

    #[allow(non_snake_case)]
    fn FPDFBookmark_GetAction(&self, bookmark: FPDF_BOOKMARK) -> FPDF_ACTION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFBookmark_GetAction()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFBookmark_GetAction",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_bookmark(bookmark)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ACTION
    }

    #[allow(non_snake_case)]
    fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAction_GetType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_action(action)))),
            )
            .as_f64()
            .unwrap() as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAction_GetDest(&self, document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetDest()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFAction_GetDest",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_action(action),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DEST
    }

    #[allow(non_snake_case)]
    fn FPDFAction_GetFilePath(&self, action: FPDF_ACTION, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetFilePath(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAction_GetFilePath(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetFilePath(): calling FPDFAction_GetFilePath()");

        let result = state
            .call(
                "FPDFAction_GetFilePath",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_action(action),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetFilePath(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAction_GetURIPath(
        &self,
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetURIPath(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAction_GetURIPath(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetURIPath(): calling FPDFAction_GetURIPath()");

        let result = state
            .call(
                "FPDFAction_GetURIPath",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_action(action),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAction_GetURIPath(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFDest_GetDestPageIndex(&self, document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDest_GetDestPageIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDest_GetDestPageIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_destination(dest),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFDest_GetView(&self, dest: FPDF_DEST, pNumParams: *mut c_ulong, pParams: *mut FS_FLOAT) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDest_GetView()");

        let state = PdfiumRenderWasmState::lock();

        let len_pNumParams = size_of::<c_ulong>();

        let ptr_pNumParams = state.malloc(len_pNumParams);

        let len_pParams = size_of::<FS_FLOAT>();

        let ptr_pParams = state.malloc(len_pParams);

        let result = state
            .call(
                "FPDFDest_GetView",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_destination(dest),
                    &Self::js_value_from_offset(ptr_pNumParams),
                    &Self::js_value_from_offset(ptr_pParams),
                ))),
            )
            .as_f64()
            .unwrap() as c_ulong;

        unsafe {
            *pNumParams = state
                .copy_bytes_from_pdfium(ptr_pNumParams, len_pNumParams)
                .try_into()
                .map(c_ulong::from_le_bytes)
                .unwrap_or(0);

            *pParams = state
                .copy_bytes_from_pdfium(ptr_pParams, len_pParams)
                .try_into()
                .map(FS_FLOAT::from_le_bytes)
                .unwrap_or(0.0);
        }

        state.free(ptr_pNumParams);
        state.free(ptr_pParams);

        result
    }

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFDest_GetLocationInPage(
        &self,
        dest: FPDF_DEST,
        hasXVal: *mut FPDF_BOOL,
        hasYVal: *mut FPDF_BOOL,
        hasZoomVal: *mut FPDF_BOOL,
        x: *mut FS_FLOAT,
        y: *mut FS_FLOAT,
        zoom: *mut FS_FLOAT,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDest_GetLocationInPage()");

        let state = PdfiumRenderWasmState::lock();

        let len_hasXVal = size_of::<FPDF_BOOL>();

        let ptr_hasXVal = state.malloc(len_hasXVal);

        let len_hasYVal = size_of::<FPDF_BOOL>();

        let ptr_hasYVal = state.malloc(len_hasYVal);

        let len_hasZoomVal = size_of::<FPDF_BOOL>();

        let ptr_hasZoomVal = state.malloc(len_hasZoomVal);

        let len_x = size_of::<FS_FLOAT>();

        let ptr_x = state.malloc(len_x);

        let len_y = size_of::<FS_FLOAT>();

        let ptr_y = state.malloc(len_y);

        let len_zoom = size_of::<FS_FLOAT>();

        let ptr_zoom = state.malloc(len_zoom);

        let result = state
            .call(
                "FPDFDest_GetLocationInPage",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_destination(dest),
                    Self::js_value_from_offset(ptr_hasXVal),
                    Self::js_value_from_offset(ptr_hasYVal),
                    Self::js_value_from_offset(ptr_hasZoomVal),
                    Self::js_value_from_offset(ptr_x),
                    Self::js_value_from_offset(ptr_y),
                    Self::js_value_from_offset(ptr_zoom),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *hasXVal = state
                    .copy_bytes_from_pdfium(ptr_hasXVal, len_hasXVal)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);

                *hasYVal = state
                    .copy_bytes_from_pdfium(ptr_hasYVal, len_hasYVal)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);

                *hasZoomVal = state
                    .copy_bytes_from_pdfium(ptr_hasZoomVal, len_hasZoomVal)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);

                *x = state
                    .copy_bytes_from_pdfium(ptr_x, len_x)
                    .try_into()
                    .map(FS_FLOAT::from_le_bytes)
                    .unwrap_or(0.0);

                *y = state
                    .copy_bytes_from_pdfium(ptr_y, len_y)
                    .try_into()
                    .map(FS_FLOAT::from_le_bytes)
                    .unwrap_or(0.0);

                *zoom = state
                    .copy_bytes_from_pdfium(ptr_zoom, len_zoom)
                    .try_into()
                    .map(FS_FLOAT::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_hasXVal);
        state.free(ptr_hasYVal);
        state.free(ptr_hasZoomVal);
        state.free(ptr_x);
        state.free(ptr_y);
        state.free(ptr_zoom);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetLinkAtPoint()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_GetLinkAtPoint",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(x),
                    &JsValue::from_f64(y),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_LINK
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkZOrderAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetLinkZOrderAtPoint()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_GetLinkZOrderAtPoint",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(x),
                    &JsValue::from_f64(y),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetDest(&self, document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetDest()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_GetDest",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_link(link),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DEST
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetAction()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_GetAction",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_link(link)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ACTION
    }

    #[allow(non_snake_case)]
    fn FPDFLink_Enumerate(&self, page: FPDF_PAGE, start_pos: *mut c_int, link_annot: *mut FPDF_LINK) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_Enumerate()");

        let state = PdfiumRenderWasmState::lock();

        let len_start_pos = size_of::<c_int>();

        let ptr_start_pos = state.copy_ptr_with_len_to_pdfium(start_pos, len_start_pos);

        let len_link_annot = size_of::<FPDF_LINK>();

        let ptr_link_annot = state.malloc(len_link_annot);

        let result = state
            .call(
                "FPDFLink_Enumerate",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(ptr_start_pos),
                    &Self::js_value_from_offset(ptr_link_annot),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *start_pos = state
                    .copy_bytes_from_pdfium(ptr_start_pos, len_start_pos)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }

            state.copy_struct_from_pdfium(ptr_link_annot, len_link_annot, link_annot);
        }

        state.free(ptr_start_pos);
        state.free(ptr_link_annot);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnot(&self, page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetAnnot()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_GetAnnot",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_link(link_annot),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ANNOTATION
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetAnnotRect()");

        let state = PdfiumRenderWasmState::lock();

        let len_rect = size_of::<FS_RECTF>();

        let ptr_rect = state.malloc(len_rect);

        let result = state
            .call(
                "FPDFLink_GetAnnotRect",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_link(link_annot),
                    &Self::js_value_from_offset(ptr_rect),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_rect, len_rect, rect);
        }

        state.free(ptr_rect);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFLink_CountQuadPoints(&self, link_annot: FPDF_LINK) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_CountQuadPoints()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_CountQuadPoints",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_link(link_annot)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetQuadPoints(
        &self,
        link_annot: FPDF_LINK,
        quad_index: c_int,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetQuadPoints()");

        let state = PdfiumRenderWasmState::lock();

        let len_quad_points = size_of::<FS_QUADPOINTSF>();

        let ptr_quad_points = state.malloc(len_quad_points);

        let result = state
            .call(
                "FPDFLink_GetQuadPoints",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_link(link_annot),
                    &JsValue::from_f64(quad_index as f64),
                    &Self::js_value_from_offset(ptr_quad_points),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_quad_points, len_quad_points, quad_points);
        }

        state.free(ptr_quad_points);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_GetPageAAction(&self, page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetPageAAction()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_GetPageAAction",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(aa_type as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ACTION
    }

    #[allow(non_snake_case)]
    fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_LoadPage()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_LoadPage",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_TEXTPAGE
    }

    #[allow(non_snake_case)]
    fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_ClosePage()");

        PdfiumRenderWasmState::lock().call(
            "FPDFText_ClosePage",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_text_page(text_page)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_CountChars()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_CountChars",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_text_page(text_page)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetUnicode()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetUnicode",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_uint
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetTextObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetTextObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontSize()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetFontSize",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_double
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetFontInfo(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        flags: *mut c_int,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontInfo(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize * size_of::<c_ushort>();

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontInfo(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        let flags_len = size_of::<c_int>();

        let ptr_flags = state.malloc(flags_len);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontInfo(): calling FPDFText_GetFontInfo()");

        let result = state
            .call(
                "FPDFText_GetFontInfo",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from(index),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buflen as f64),
                    &Self::js_value_from_offset(ptr_flags),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);

            unsafe {
                *flags = state
                    .copy_bytes_from_pdfium(ptr_flags, flags_len)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(buffer_ptr);
        state.free(ptr_flags);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontInfo(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetFontWeight(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFontWeight()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetFontWeight",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetFillColor()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_r = state.malloc(len);

        let ptr_g = state.malloc(len);

        let ptr_b = state.malloc(len);

        let ptr_a = state.malloc(len);

        let result = state
            .call(
                "FPDFText_GetFillColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_text_page(text_page),
                    JsValue::from_f64(index as f64),
                    Self::js_value_from_offset(ptr_r),
                    Self::js_value_from_offset(ptr_g),
                    Self::js_value_from_offset(ptr_b),
                    Self::js_value_from_offset(ptr_a),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *A = state
                    .copy_bytes_from_pdfium(ptr_a, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);
        state.free(ptr_a);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetStrokeColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetStrokeColor()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_r = state.malloc(len);

        let ptr_g = state.malloc(len);

        let ptr_b = state.malloc(len);

        let ptr_a = state.malloc(len);

        let result = state
            .call(
                "FPDFText_GetStrokeColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_text_page(text_page),
                    JsValue::from_f64(index as f64),
                    Self::js_value_from_offset(ptr_r),
                    Self::js_value_from_offset(ptr_g),
                    Self::js_value_from_offset(ptr_b),
                    Self::js_value_from_offset(ptr_a),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *A = state
                    .copy_bytes_from_pdfium(ptr_a, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);
        state.free(ptr_a);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetCharAngle(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_float {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetCharAngle()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetCharAngle",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_float
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        left: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
        top: *mut c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetCharBox()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_double>();

        let ptr_left = state.malloc(len);

        let ptr_top = state.malloc(len);

        let ptr_right = state.malloc(len);

        let ptr_bottom = state.malloc(len);

        let result = state
            .call(
                "FPDFText_GetCharBox",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_text_page(text_page),
                    JsValue::from_f64(index as f64),
                    Self::js_value_from_offset(ptr_left),
                    Self::js_value_from_offset(ptr_top),
                    Self::js_value_from_offset(ptr_right),
                    Self::js_value_from_offset(ptr_bottom),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *left = state
                    .copy_bytes_from_pdfium(ptr_left, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *top = state
                    .copy_bytes_from_pdfium(ptr_top, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *right = state
                    .copy_bytes_from_pdfium(ptr_right, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *bottom = state
                    .copy_bytes_from_pdfium(ptr_bottom, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_left);
        state.free(ptr_top);
        state.free(ptr_right);
        state.free(ptr_bottom);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetLooseCharBox(&self, text_page: FPDF_TEXTPAGE, index: c_int, rect: *mut FS_RECTF) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetLooseCharBox()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<FS_RECTF>();

        let rect_ptr = state.copy_struct_to_pdfium_mut(rect);

        let result = state
            .call(
                "FPDFText_GetLooseCharBox",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(rect_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(rect_ptr, buffer_length, rect);
        }

        state.free(rect_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetMatrix(&self, text_page: FPDF_TEXTPAGE, index: c_int, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetMatrix()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<FS_MATRIX>();

        let matrix_ptr = state.copy_struct_to_pdfium_mut(matrix);

        let result = state
            .call(
                "FPDFText_GetMatrix",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(matrix_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(matrix_ptr, buffer_length, matrix);
        }

        state.free(matrix_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetCharOrigin()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_double>();

        let ptr_x = state.malloc(len);

        let ptr_y = state.malloc(len);

        let result = state
            .call(
                "FPDFText_GetCharOrigin",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(ptr_x),
                    &Self::js_value_from_offset(ptr_y),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *x = state
                    .copy_bytes_from_pdfium(ptr_x, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *y = state
                    .copy_bytes_from_pdfium(ptr_y, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_x);
        state.free(ptr_y);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexAtPos(
        &self,
        text_page: FPDF_TEXTPAGE,
        x: c_double,
        y: c_double,
        xTolerance: c_double,
        yTolerance: c_double,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetCharIndexAtPos()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetCharIndexAtPos",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(x),
                    &JsValue::from_f64(y),
                    &JsValue::from_f64(xTolerance),
                    &JsValue::from_f64(yTolerance),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetText(
        &self,
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetText()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_ushort>();

        let ptr_result = state.malloc(len);

        let call_result = state
            .call(
                "FPDFText_GetText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(start_index as f64),
                    &JsValue::from_f64(count as f64),
                    &Self::js_value_from_offset(ptr_result),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(call_result) {
            unsafe {
                *result = state
                    .copy_bytes_from_pdfium(ptr_result, len)
                    .try_into()
                    .map(c_ushort::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_result);

        call_result
    }

    #[allow(non_snake_case)]
    fn FPDFText_CountRects(&self, text_page: FPDF_TEXTPAGE, start_index: c_int, count: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_CountRects()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_CountRects",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(start_index as f64),
                    &JsValue::from_f64(count as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetRect(
        &self,
        text_page: FPDF_TEXTPAGE,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetRect()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_double>();

        let ptr_left = state.malloc(len);

        let ptr_top = state.malloc(len);

        let ptr_right = state.malloc(len);

        let ptr_bottom = state.malloc(len);

        let result = state
            .call(
                "FPDFText_GetRect",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_text_page(text_page),
                    JsValue::from_f64(rect_index as f64),
                    Self::js_value_from_offset(ptr_left),
                    Self::js_value_from_offset(ptr_top),
                    Self::js_value_from_offset(ptr_right),
                    Self::js_value_from_offset(ptr_bottom),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *left = state
                    .copy_bytes_from_pdfium(ptr_left, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *top = state
                    .copy_bytes_from_pdfium(ptr_top, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *right = state
                    .copy_bytes_from_pdfium(ptr_right, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *bottom = state
                    .copy_bytes_from_pdfium(ptr_bottom, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_left);
        state.free(ptr_top);
        state.free(ptr_right);
        state.free(ptr_bottom);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetBoundedText(
        &self,
        text_page: FPDF_TEXTPAGE,
        left: c_double,
        top: c_double,
        right: c_double,
        bottom: c_double,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetBoundedText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize * size_of::<c_ushort>();

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFText_GetBoundedText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFText_GetBoundedText(): calling FPDFText_GetBoundedText()"
        );

        let result = state
            .call(
                "FPDFText_GetBoundedText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_text_page(text_page),
                    JsValue::from(left),
                    JsValue::from(top),
                    JsValue::from(right),
                    JsValue::from(bottom),
                    Self::js_value_from_offset(buffer_ptr),
                    JsValue::from_f64(buflen as f64),
                ]))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            // The return result is the number of _characters_ returned, _not_ the number of bytes.

            state.copy_struct_from_pdfium(buffer_ptr, result * size_of::<c_ushort>(), buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetBoundedText(): leaving");

        result as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_FindStart(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: FPDF_WIDESTRING,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_FindStart()");

        let state = PdfiumRenderWasmState::lock();

        let findwhat_ptr = state.copy_string_to_pdfium(findwhat);

        let result = state
            .call(
                "FPDFText_FindStart",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_text_page(text_page),
                    &Self::js_value_from_offset(findwhat_ptr),
                    &JsValue::from_f64(flags as f64),
                    &JsValue::from_f64(start_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_SCHHANDLE;

        state.free(findwhat_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_FindNext(&self, form: FPDF_SCHHANDLE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_FindNext()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_FindNext",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_search(form)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFText_FindPrev(&self, form: FPDF_SCHHANDLE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_FindPrev()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_FindPrev",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_search(form)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetSchResultIndex(&self, form: FPDF_SCHHANDLE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetSchResultIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetSchResultIndex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_search(form)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_GetSchCount(&self, form: FPDF_SCHHANDLE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_GetSchCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_GetSchCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_search(form)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_FindClose(&self, form: FPDF_SCHHANDLE) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_FindClose()");

        PdfiumRenderWasmState::lock().call(
            "FPDFText_FindClose",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_search(form)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFLink_LoadWebLinks(&self, text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_LoadWebLinks()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_LoadWebLinks",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_text_page(text_page)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGELINK
    }

    #[allow(non_snake_case)]
    fn FPDFLink_CountWebLinks(&self, link_page: FPDF_PAGELINK) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_CountWebLinks()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_CountWebLinks",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page_link(link_page)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetURL(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetURL()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFLink_GetURL(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetURL(): calling FPDFLink_GetURL()");

        let result = state
            .call(
                "FPDFLink_GetURL",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_page_link(link_page),
                    &JsValue::from_f64(link_index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetURL(): leaving");

        result as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFLink_CountRects(&self, link_page: FPDF_PAGELINK, link_index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_CountRects()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFLink_CountRects",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page_link(link_page),
                    &JsValue::from_f64(link_index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFLink_GetRect(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetRect()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_double>();

        let ptr_left = state.malloc(len);

        let ptr_top = state.malloc(len);

        let ptr_right = state.malloc(len);

        let ptr_bottom = state.malloc(len);

        let result = state
            .call(
                "FPDFLink_GetRect",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_page_link(link_page),
                    JsValue::from_f64(link_index as f64),
                    JsValue::from_f64(rect_index as f64),
                    Self::js_value_from_offset(ptr_left),
                    Self::js_value_from_offset(ptr_top),
                    Self::js_value_from_offset(ptr_right),
                    Self::js_value_from_offset(ptr_bottom),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *left = state
                    .copy_bytes_from_pdfium(ptr_left, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *top = state
                    .copy_bytes_from_pdfium(ptr_top, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *right = state
                    .copy_bytes_from_pdfium(ptr_right, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);

                *bottom = state
                    .copy_bytes_from_pdfium(ptr_bottom, len)
                    .try_into()
                    .map(c_double::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_left);
        state.free(ptr_top);
        state.free(ptr_right);
        state.free(ptr_bottom);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFLink_GetTextRange(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        start_char_index: *mut c_int,
        char_count: *mut c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_GetTextRange()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_int>();

        let ptr_start_char_index = state.malloc(len);

        let ptr_char_count = state.malloc(len);

        let result = state
            .call(
                "FPDFLink_GetTextRange",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_page_link(link_page),
                    &JsValue::from_f64(link_index as f64),
                    &Self::js_value_from_offset(ptr_start_char_index),
                    &Self::js_value_from_offset(ptr_char_count),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *start_char_index = state
                    .copy_bytes_from_pdfium(ptr_start_char_index, len)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);

                *char_count = state
                    .copy_bytes_from_pdfium(ptr_char_count, len)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_start_char_index);
        state.free(ptr_char_count);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFLink_CloseWebLinks(&self, link_page: FPDF_PAGELINK) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFLink_CloseWebLinks()");

        PdfiumRenderWasmState::lock().call(
            "FPDFLink_CloseWebLinks",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_page_link(link_page)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetDecodedThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetDecodedThumbnailData()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFPage_GetDecodedThumbnailData(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFPage_GetDecodedThumbnailData(): calling FPDFPage_GetDecodedThumbnailData()"
        );

        let result = state
            .call(
                "FPDFPage_GetDecodedThumbnailData",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetDecodedThumbnailData(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetRawThumbnailData(&self, page: FPDF_PAGE, buffer: *mut c_void, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetRawThumbnailData()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFPage_GetRawThumbnailData(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFPage_GetRawThumbnailData(): calling FPDFPage_GetRawThumbnailData()"
        );

        let result = state
            .call(
                "FPDFPage_GetRawThumbnailData",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetRawThumbnailData(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetThumbnailAsBitmap(&self, page: FPDF_PAGE) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetThumbnailAsBitmap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetThumbnailAsBitmap",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFormObj_CountObjects()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFormObj_CountObjects",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(form_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFFormObj_GetObject(&self, form_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFormObj_GetObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFormObj_GetObject",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(form_object),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFFormObj_RemoveObject(&self, form_object: FPDF_PAGEOBJECT, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFormObj_RemoveObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFormObj_RemoveObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(form_object),
                    &Self::js_value_from_object(page_object),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: FPDF_FONT,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_CreateTextObj()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_CreateTextObj",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_font(font),
                    &JsValue::from(font_size),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetTextRenderMode(&self, text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetTextRenderMode()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFTextObj_GetTextRenderMode",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(text)))),
            )
            .as_f64()
            .unwrap() as FPDF_TEXT_RENDERMODE
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_SetTextRenderMode(&self, text: FPDF_PAGEOBJECT, render_mode: FPDF_TEXT_RENDERMODE) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_SetTextRenderMode()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFTextObj_SetTextRenderMode",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(text),
                    &JsValue::from_f64(render_mode as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetText(
        &self,
        text_object: FPDF_PAGEOBJECT,
        text_page: FPDF_TEXTPAGE,
        buffer: *mut FPDF_WCHAR,
        length: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetText(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetText(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetText(): calling FPDFTextObj_GetText()");

        let result = state
            .call(
                "FPDFTextObj_GetText",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_object(text_object),
                    &Self::js_value_from_text_page(text_page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetText(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        text_object: FPDF_PAGEOBJECT,
        scale: f32,
    ) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetRenderedBitmap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFTextObj_GetRenderedBitmap",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_object(text_object),
                    &JsValue::from_f64(scale as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetFont()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFTextObj_GetFont",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(text)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_FONT
    }

    #[allow(non_snake_case)]
    fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_IsGenerated()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_IsGenerated",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_IsHyphen(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_IsHyphen()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_IsHyphen",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_HasUnicodeMapError()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_HasUnicodeMapError",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_text_page(text_page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFontSize(&self, text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFTextObj_GetFontSize()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_size = state.malloc(len);

        let result = state
            .call(
                "FPDFTextObj_GetFontSize",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(text),
                    &Self::js_value_from_offset(ptr_size),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *size = state
                    .copy_bytes_from_pdfium(ptr_size, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_size);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_Close(&self, font: FPDF_FONT) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_Close()");

        PdfiumRenderWasmState::lock().call(
            "FPDFFont_Close",
            JsFunctionArgumentType::Pointer,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_font(font)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPath_MoveTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_MoveTo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_MoveTo",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(path),
                    &JsValue::from_f64(x as f64),
                    &JsValue::from_f64(y as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPath_LineTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_LineTo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_LineTo",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(path),
                    &JsValue::from_f64(x as f64),
                    &JsValue::from_f64(y as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPath_BezierTo(
        &self,
        path: FPDF_PAGEOBJECT,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        x3: c_float,
        y3: c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_BezierTo()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_BezierTo",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_object(path),
                    JsValue::from_f64(x1 as f64),
                    JsValue::from_f64(y1 as f64),
                    JsValue::from_f64(x2 as f64),
                    JsValue::from_f64(y2 as f64),
                    JsValue::from_f64(x3 as f64),
                    JsValue::from_f64(y3 as f64),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPath_Close(&self, path: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_Close()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_Close",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(path)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPath_SetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: c_int, stroke: FPDF_BOOL) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_SetDrawMode()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_SetDrawMode",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(path),
                    &JsValue::from_f64(fillmode as f64),
                    &JsValue::from_f64(stroke as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPath_GetDrawMode(&self, path: FPDF_PAGEOBJECT, fillmode: *mut c_int, stroke: *mut FPDF_BOOL) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_GetDrawMode()");

        let state = PdfiumRenderWasmState::lock();

        let fillmode_length = size_of::<c_int>();

        let fillmode_ptr = state.malloc(fillmode_length);

        let stroke_length = size_of::<FPDF_BOOL>();

        let stroke_ptr = state.malloc(stroke_length);

        let result = state
            .call(
                "FPDFPath_GetDrawMode",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(path),
                    &Self::js_value_from_offset(fillmode_ptr),
                    &Self::js_value_from_offset(stroke_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *fillmode = state
                    .copy_bytes_from_pdfium(fillmode_ptr, fillmode_length)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);

                *stroke = state
                    .copy_bytes_from_pdfium(stroke_ptr, stroke_length)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_NewTextObj(&self, document: FPDF_DOCUMENT, font: &str, font_size: c_float) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_NewTextObj()");

        let state = PdfiumRenderWasmState::lock();

        let c_font = CString::new(font).unwrap();

        let font_ptr = state.copy_bytes_to_pdfium(&c_font.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObj_NewTextObj",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(font_ptr),
                    &JsValue::from(font_size),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT;

        state.free(font_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_SetText(&self, text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_SetText()");

        let state = PdfiumRenderWasmState::lock();

        let text_ptr = state.copy_string_to_pdfium(text);

        let result = state
            .call(
                "FPDFText_SetText",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(text_object),
                    &Self::js_value_from_offset(text_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(text_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_SetCharcodes(
        &self,
        text_object: FPDF_PAGEOBJECT,
        charcodes: *const c_uint,
        count: size_t,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_SetCharcodes()");

        let state = PdfiumRenderWasmState::lock();

        let ptr_charcodes = state.copy_ptr_with_len_to_pdfium(charcodes, size_of::<c_uint>() * count);

        let result = state
            .call(
                "FPDFText_SetCharcodes",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(text_object),
                    &Self::js_value_from_offset(ptr_charcodes),
                    &JsValue::from_f64(count as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(ptr_charcodes);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_LoadFont(
        &self,
        document: FPDF_DOCUMENT,
        data: *const c_uchar,
        size: c_uint,
        font_type: c_int,
        cid: FPDF_BOOL,
    ) -> FPDF_FONT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_LoadFont()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size as usize;

        let buffer_ptr = state.copy_ptr_with_len_to_pdfium(data, buffer_length);

        let result = state
            .call(
                "FPDFText_LoadFont",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(size as f64),
                    &JsValue::from_f64(font_type as f64),
                    &JsValue::from_f64(cid as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_FONT;

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFText_LoadStandardFont(&self, document: FPDF_DOCUMENT, font: &str) -> FPDF_FONT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_LoadStandardFont()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFText_LoadStandardFont",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::String]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from(font),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_FONT
    }

    #[allow(non_snake_case)]
    fn FPDFText_LoadCidType2Font(
        &self,
        document: FPDF_DOCUMENT,
        font_data: *const u8,
        font_data_size: u32,
        to_unicode_cmap: &str,
        cid_to_gid_map_data: *const u8,
        cid_to_gid_map_data_size: u32,
    ) -> FPDF_FONT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFText_LoadCidType2Font()");

        let state = PdfiumRenderWasmState::lock();

        let c_to_unicode_cmap = CString::new(to_unicode_cmap).unwrap();

        let c_to_unicode_cmap_ptr = state.copy_bytes_to_pdfium(&c_to_unicode_cmap.into_bytes_with_nul());

        let font_data_ptr = state.copy_ptr_with_len_to_pdfium(font_data, font_data_size as usize);

        let cid_to_gid_map_data_ptr =
            state.copy_ptr_with_len_to_pdfium(cid_to_gid_map_data, cid_to_gid_map_data_size as usize);

        let result = state
            .call(
                "FPDFText_LoadCidType2Font",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_document(document),
                    Self::js_value_from_offset(font_data_ptr),
                    JsValue::from(font_data_size),
                    Self::js_value_from_offset(c_to_unicode_cmap_ptr),
                    Self::js_value_from_offset(cid_to_gid_map_data_ptr),
                    JsValue::from(cid_to_gid_map_data_size),
                ]))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_FONT;

        state.free(c_to_unicode_cmap_ptr);
        state.free(font_data_ptr);
        state.free(cid_to_gid_map_data_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPage_InsertObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_InsertObject()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPage_InsertObject",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_page(page),
                &Self::js_value_from_object(page_obj),
            ))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPage_InsertObjectAtIndex(&self, page: FPDF_PAGE, page_object: FPDF_PAGEOBJECT, index: usize) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_InsertObjectAtIndex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_InsertObjectAtIndex",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_RemoveObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_RemoveObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_RemoveObject",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_object(page_obj),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_CountObjects()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_CountObjects",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page(page)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPage_GetObject()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPage_GetObject",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page(page),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_Destroy(&self, page_obj: FPDF_PAGEOBJECT) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_Destroy()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPageObj_Destroy",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_obj)))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_HasTransparency(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_HasTransparency()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_HasTransparency",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: *mut FPDF_BOOL) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetIsActive()");

        let state = PdfiumRenderWasmState::lock();

        let active_length = size_of::<FPDF_BOOL>();

        let active_ptr = state.malloc(active_length);

        let result = state
            .call(
                "FPDFPageObj_GetIsActive",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(active_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *active = state
                    .copy_bytes_from_pdfium(active_ptr, active_length)
                    .try_into()
                    .map(FPDF_BOOL::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(active_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetIsActive(&self, page_object: FPDF_PAGEOBJECT, active: FPDF_BOOL) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetIsActive()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetIsActive",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from(active as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_Transform(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_Transform()");

        PdfiumRenderWasmState::lock().call(
            "FPDFPageObj_Transform",
            JsFunctionArgumentType::Void,
            Some(vec![
                JsFunctionArgumentType::Pointer,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
                JsFunctionArgumentType::Number,
            ]),
            Some(&JsValue::from(Self::js_array_from_vec(vec![
                Self::js_value_from_object(page_object),
                JsValue::from(a),
                JsValue::from(b),
                JsValue::from(c),
                JsValue::from(d),
                JsValue::from(e),
                JsValue::from(f),
            ]))),
        );
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformF(&self, page_object: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_TransformF()");

        let state = PdfiumRenderWasmState::lock();

        let matrix_ptr = state.copy_struct_to_pdfium(matrix);

        let result = state
            .call(
                "FPDFPageObj_TransformF",
                JsFunctionArgumentType::Void,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(matrix_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(matrix_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMatrix(&self, page_object: FPDF_PAGEOBJECT, matrix: *mut FS_MATRIX) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetMatrix()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<FS_MATRIX>();

        let matrix_ptr = state.copy_struct_to_pdfium_mut(matrix);

        let result = state
            .call(
                "FPDFPageObj_GetMatrix",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(matrix_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(matrix_ptr, buffer_length, matrix);
        }

        state.free(matrix_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetMatrix(&self, path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetMatrix()");

        let state = PdfiumRenderWasmState::lock();

        let ptr_matrix = state.copy_struct_to_pdfium(matrix);

        let result = state
            .call(
                "FPDFPageObj_SetMatrix",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(path),
                    &Self::js_value_from_offset(ptr_matrix),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(ptr_matrix);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_NewImageObj(&self, document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_NewImageObj()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_NewImageObj",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMarkedContentID(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetMarkedContentID()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetMarkedContentID",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_CountMarks(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_CountMarks()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_CountMarks",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMark(&self, page_object: FPDF_PAGEOBJECT, index: c_ulong) -> FPDF_PAGEOBJECTMARK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetMark()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetMark",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECTMARK
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_AddMark(&self, page_object: FPDF_PAGEOBJECT, name: &str) -> FPDF_PAGEOBJECTMARK {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_AddMark()");

        let state = PdfiumRenderWasmState::lock();

        let c_name = CString::new(name).unwrap();

        let name_ptr = state.copy_bytes_to_pdfium(&c_name.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObj_AddMark",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(name_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECTMARK;

        state.free(name_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_RemoveMark(&self, page_object: FPDF_PAGEOBJECT, mark: FPDF_PAGEOBJECTMARK) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_RemoveMark()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_RemoveMark",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_mark(mark),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetName(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetName()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_CountParams(&self, mark: FPDF_PAGEOBJECTMARK) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_CountParams()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObjMark_CountParams",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_mark(mark)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamKey(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        index: c_ulong,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamKey()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetParamKey",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_mark(mark),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamValueType(&self, mark: FPDF_PAGEOBJECTMARK, key: &str) -> FPDF_OBJECT_TYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamValueType()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObjMark_GetParamValueType",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_OBJECT_TYPE;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamIntValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamIntValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let out_value_length = size_of::<c_ulong>();

        let out_value_ptr = state.malloc(out_value_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetParamIntValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(out_value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_value = state
                    .copy_bytes_from_pdfium(out_value_ptr, out_value_length)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamFloatValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamFloatValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let out_value_length = size_of::<c_float>();

        let out_value_ptr = state.malloc(out_value_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetParamFloatValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(out_value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_value = state
                    .copy_bytes_from_pdfium(out_value_ptr, out_value_length)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamStringValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamStringValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetParamStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(key_ptr);
        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamBlobValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut c_uchar,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_GetParamBlobValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFPageObjMark_GetParamKey",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(key_ptr);
        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetIntParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: c_int,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_SetIntParam()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObjMark_SetIntParam",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &JsValue::from_f64(value as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);

        result
    }

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetFloatParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: f32,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_SetFloatParam()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObjMark_SetFloatParam",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &JsValue::from_f64(value as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetStringParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_SetStringParam()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let c_value = CString::new(value).unwrap();

        let value_ptr = state.copy_bytes_to_pdfium(&c_value.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObjMark_SetStringParam",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);
        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetBlobParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: *const c_uchar,
        value_len: c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_SetBlobParam()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let value_ptr = state.copy_ptr_with_len_to_pdfium(value, value_len as usize);

        let result = state
            .call(
                "FPDFPageObjMark_SetBlobParam",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_document(document),
                    Self::js_value_from_object(page_object),
                    Self::js_value_from_mark(mark),
                    Self::js_value_from_offset(key_ptr),
                    Self::js_value_from_offset(value_ptr),
                    JsValue::from_f64(value_len as f64),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);
        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_RemoveParam(
        &self,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObjMark_RemoveParam()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFPageObjMark_RemoveParam",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_mark(mark),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFile(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_LoadJpegFile()");

        let state = PdfiumRenderWasmState::lock();

        let ptr_file_access = state.copy_file_access_to_pdfium(file_access);

        state
            .call(
                "FPDFImageObj_LoadJpegFile",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_page_mut(pages),
                    &JsValue::from_f64(count as f64),
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_offset(ptr_file_access),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFileInline(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_LoadJpegFileInline()");

        let state = PdfiumRenderWasmState::lock();

        let ptr_file_access = state.copy_file_access_to_pdfium(file_access);

        state
            .call(
                "FPDFImageObj_LoadJpegFile",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_page_mut(pages),
                    &JsValue::from_f64(count as f64),
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_offset(ptr_file_access),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_SetMatrix(
        &self,
        image_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_SetMatrix()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFImageObj_SetMatrix",
                JsFunctionArgumentType::Void,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Self::js_array_from_vec(vec![
                    Self::js_value_from_object(image_object),
                    JsValue::from(a),
                    JsValue::from(b),
                    JsValue::from(c),
                    JsValue::from(d),
                    JsValue::from(e),
                    JsValue::from(f),
                ]))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_SetBitmap(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        bitmap: FPDF_BITMAP,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_SetBitmap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFImageObj_SetBitmap",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_page_mut(pages),
                    &JsValue::from_f64(count as f64),
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_bitmap(bitmap),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetBitmap(&self, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetBitmap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFImageObj_GetBitmap",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(image_object)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        image_object: FPDF_PAGEOBJECT,
    ) -> FPDF_BITMAP {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetRenderedBitmap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFImageObj_GetRenderedBitmap",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_object(image_object),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BITMAP
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataDecoded()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataDecoded(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataDecoded(): calling FPDFImageObj_GetImageDataDecoded()"
        );

        let result = state
            .call(
                "FPDFImageObj_GetImageDataDecoded",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataDecoded(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataRaw(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataRaw()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataRaw(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataRaw(): calling FPDFImageObj_GetImageDataRaw()"
        );

        let result = state
            .call(
                "FPDFImageObj_GetImageDataRaw",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageDataRaw(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilterCount(&self, image_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageFilterCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFImageObj_GetImageFilterCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(image_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilter(
        &self,
        image_object: FPDF_PAGEOBJECT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageFilter()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = state.malloc(buffer_length);

        let result = state
            .call(
                "FPDFImageObj_GetImageFilter",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_object(image_object),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageMetadata(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImageMetadata()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = size_of::<FPDF_IMAGEOBJ_METADATA>();

        let buffer_ptr = state.malloc(buffer_length);

        let result = state
            .call(
                "FPDFImageObj_GetImageMetadata",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(buffer_ptr, buffer_length, metadata);
        }

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImagePixelSize(
        &self,
        image_object: FPDF_PAGEOBJECT,
        width: *mut c_uint,
        height: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetImagePixelSize()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_width = state.malloc(len);

        let ptr_height = state.malloc(len);

        let result = state
            .call(
                "FPDFImageObj_GetImagePixelSize",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_offset(ptr_width),
                    &Self::js_value_from_offset(ptr_height),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *width = state
                    .copy_bytes_from_pdfium(ptr_width, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *height = state
                    .copy_bytes_from_pdfium(ptr_height, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_width);
        state.free(ptr_height);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFImageObj_GetIccProfileDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        buffer: *mut u8,
        buflen: usize,
        out_buflen: *mut usize,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFImageObj_GetIccProfileDataDecoded()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = if buflen > 0 { state.malloc(buflen) } else { 0 };

        let out_buflen_length = size_of::<usize>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFImageObj_GetIccProfileDataDecoded",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(image_object),
                    &Self::js_value_from_page(page),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buflen as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(usize::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewPath(&self, x: c_float, y: c_float) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_CreateNewPath()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_CreateNewPath",
                JsFunctionArgumentType::Pointer,
                Some(vec![JsFunctionArgumentType::Number, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(&JsValue::from(x), &JsValue::from(y)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewRect(&self, x: c_float, y: c_float, w: c_float, h: c_float) -> FPDF_PAGEOBJECT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_CreateNewRect()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_CreateNewRect",
                JsFunctionArgumentType::Pointer,
                Some(vec![
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &JsValue::from(x),
                    &JsValue::from(y),
                    &JsValue::from(w),
                    &JsValue::from(h),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGEOBJECT
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetBounds()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_left = state.malloc(len);

        let ptr_bottom = state.malloc(len);

        let ptr_right = state.malloc(len);

        let ptr_top = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetBounds",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_left),
                    &Self::js_value_from_offset(ptr_bottom),
                    &Self::js_value_from_offset(ptr_right),
                    &Self::js_value_from_offset(ptr_top),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *left = state
                    .copy_bytes_from_pdfium(ptr_left, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *bottom = state
                    .copy_bytes_from_pdfium(ptr_bottom, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *right = state
                    .copy_bytes_from_pdfium(ptr_right, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *top = state
                    .copy_bytes_from_pdfium(ptr_top, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_left);
        state.free(ptr_bottom);
        state.free(ptr_right);
        state.free(ptr_top);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetRotatedBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetRotatedBounds()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<FS_QUADPOINTSF>();

        let ptr_quad_points = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetRotatedBounds",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_quad_points),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(ptr_quad_points, len, quad_points);
        }

        state.free(ptr_quad_points);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetBlendMode(&self, page_object: FPDF_PAGEOBJECT, blend_mode: &str) {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetBlendMode()");

        let state = PdfiumRenderWasmState::lock();

        let c_blend_mode = CString::new(blend_mode).unwrap();

        let blend_mode_ptr = state.copy_bytes_to_pdfium(&c_blend_mode.into_bytes_with_nul());

        state.call(
            "FPDFPageObj_SetBlendMode",
            JsFunctionArgumentType::Void,
            Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
            Some(&JsValue::from(Array::of2(
                &Self::js_value_from_object(page_object),
                &Self::js_value_from_offset(blend_mode_ptr),
            ))),
        );

        state.free(blend_mode_ptr);
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetStrokeColor()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetStrokeColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(R as f64),
                    &JsValue::from_f64(G as f64),
                    &JsValue::from_f64(B as f64),
                    &JsValue::from_f64(A as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetStrokeColor()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_r = state.malloc(len);

        let ptr_g = state.malloc(len);

        let ptr_b = state.malloc(len);

        let ptr_a = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetStrokeColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_r),
                    &Self::js_value_from_offset(ptr_g),
                    &Self::js_value_from_offset(ptr_b),
                    &Self::js_value_from_offset(ptr_a),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *A = state
                    .copy_bytes_from_pdfium(ptr_a, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);
        state.free(ptr_a);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetStrokeWidth()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetStrokeWidth",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from(width),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetStrokeWidth()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_width = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetStrokeWidth",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_width),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *width = state
                    .copy_bytes_from_pdfium(ptr_width, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_width);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineJoin(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetLineJoin()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetLineJoin",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineJoin(&self, page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetLineJoin()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetLineJoin",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(line_join as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineCap(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetLineCap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetLineCap",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineCap(&self, page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetLineCap()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetLineCap",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(line_cap as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetFillColor()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetFillColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from_f64(R as f64),
                    &JsValue::from_f64(G as f64),
                    &JsValue::from_f64(B as f64),
                    &JsValue::from_f64(A as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetFillColor()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_uint>();

        let ptr_r = state.malloc(len);

        let ptr_g = state.malloc(len);

        let ptr_b = state.malloc(len);

        let ptr_a = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetFillColor",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of5(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_r),
                    &Self::js_value_from_offset(ptr_g),
                    &Self::js_value_from_offset(ptr_b),
                    &Self::js_value_from_offset(ptr_a),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *R = state
                    .copy_bytes_from_pdfium(ptr_r, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *G = state
                    .copy_bytes_from_pdfium(ptr_g, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *B = state
                    .copy_bytes_from_pdfium(ptr_b, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);

                *A = state
                    .copy_bytes_from_pdfium(ptr_a, len)
                    .try_into()
                    .map(c_uint::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_r);
        state.free(ptr_g);
        state.free(ptr_b);
        state.free(ptr_a);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashPhase()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_phase = state.malloc(len);

        let result = state
            .call(
                "FPDFPageObj_GetDashPhase",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(ptr_phase),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *phase = state
                    .copy_bytes_from_pdfium(ptr_phase, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_phase);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetDashPhase()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_SetDashPhase",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(page_object),
                    &JsValue::from(phase),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashCount(&self, page_object: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPageObj_GetDashCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(page_object)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *mut c_float,
        dash_count: size_t,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashArray()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_len = size_of::<c_float>() * dash_count;

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashArray(): allocating buffer of {} bytes in Pdfium's WASM heap",
            buffer_len
        );

        let buffer_ptr = state.malloc(buffer_len);

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashArray(): calling FPDFPageObj_GetDashArray()"
        );

        let result = state
            .call(
                "FPDFPageObj_GetDashArray",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(dash_count as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            state.copy_struct_from_pdfium(buffer_ptr, buffer_len, dash_array);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_GetDashArray(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *const c_float,
        dash_count: size_t,
        phase: c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPageObj_SetDashArray()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = state.copy_ptr_with_len_to_pdfium(dash_array, size_of::<c_float>() * dash_count);

        let result = state
            .call(
                "FPDFPageObj_SetDashArray",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_object(page_object),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(dash_count as f64),
                    &JsValue::from(phase),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(buffer_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_CountSegments()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_CountSegments",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_object(path)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPath_GetPathSegment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPath_GetPathSegment",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_object(path),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PATHSEGMENT
    }

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetPoint(&self, segment: FPDF_PATHSEGMENT, x: *mut c_float, y: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPathSegment_GetPoint()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_x = state.malloc(len);

        let ptr_y = state.malloc(len);

        let result = state
            .call(
                "FPDFPathSegment_GetPoint",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_segment(segment),
                    &Self::js_value_from_offset(ptr_x),
                    &Self::js_value_from_offset(ptr_y),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *x = state
                    .copy_bytes_from_pdfium(ptr_x, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);

                *y = state
                    .copy_bytes_from_pdfium(ptr_y, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_x);
        state.free(ptr_y);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPathSegment_GetType()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPathSegment_GetType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_segment(segment)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFPathSegment_GetClose()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFPathSegment_GetClose",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_segment(segment)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetBaseFontName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetBaseFontName()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = if length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFFont_GetBaseFontName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                length
            );

            state.malloc(length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFFont_GetBaseFontName(): calling FPDFFont_GetBaseFontName()"
        );

        let result = state
            .call(
                "FPDFFont_GetBaseFontName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_font(font),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetBaseFontName(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(&self, font: FPDF_FONT, buffer: *mut c_char, length: usize) -> usize {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFamilyName()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = if length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFamilyName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                length
            );

            state.malloc(length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFamilyName(): calling FPDFFont_GetFamilyName()");

        let result = state
            .call(
                "FPDFFont_GetFamilyName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_font(font),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFamilyName(): leaving");

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetFontData(
        &self,
        font: FPDF_FONT,
        buffer: *mut u8,
        buflen: usize,
        out_buflen: *mut usize,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFontData()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_ptr = if buflen > 0 { state.malloc(buflen) } else { 0 };

        let out_buflen_length = size_of::<usize>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFFont_GetFontData",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_font(font),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buflen as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(usize::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetIsEmbedded(&self, font: FPDF_FONT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetIsEmbedded()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFont_GetIsEmbedded",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_font(font)))),
            )
            .as_f64()
            .unwrap() as usize as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetFlags(&self, font: FPDF_FONT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetFlags()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFont_GetFlags",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_font(font)))),
            )
            .as_f64()
            .unwrap() as usize as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetWeight(&self, font: FPDF_FONT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetWeight()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFont_GetWeight",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_font(font)))),
            )
            .as_f64()
            .unwrap() as usize as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetItalicAngle(&self, font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetItalicAngle()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_int>();

        let ptr_angle = state.malloc(len);

        let result = state
            .call(
                "FPDFFont_GetItalicAngle",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_font(font),
                    &Self::js_value_from_offset(ptr_angle),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *angle = state
                    .copy_bytes_from_pdfium(ptr_angle, len)
                    .try_into()
                    .map(c_int::from_le_bytes)
                    .unwrap_or(0);
            }
        }

        state.free(ptr_angle);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetAscent(&self, font: FPDF_FONT, font_size: c_float, ascent: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetAscent()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_ascent = state.malloc(len);

        let result = state
            .call(
                "FPDFFont_GetAscent",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_font(font),
                    &JsValue::from(font_size),
                    &Self::js_value_from_offset(ptr_ascent),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *ascent = state
                    .copy_bytes_from_pdfium(ptr_ascent, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_ascent);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetDescent(&self, font: FPDF_FONT, font_size: c_float, descent: *mut c_float) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetDescent()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_descent = state.malloc(len);

        let result = state
            .call(
                "FPDFFont_GetDescent",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_font(font),
                    &JsValue::from(font_size),
                    &Self::js_value_from_offset(ptr_descent),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *descent = state
                    .copy_bytes_from_pdfium(ptr_descent, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_descent);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphWidth(
        &self,
        font: FPDF_FONT,
        glyph: c_uint,
        font_size: c_float,
        width: *mut c_float,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetGlyphWidth()");

        let state = PdfiumRenderWasmState::lock();

        let len = size_of::<c_float>();

        let ptr_width = state.malloc(len);

        let result = state
            .call(
                "FPDFFont_GetGlyphWidth",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_font(font),
                    &JsValue::from_f64(glyph as f64),
                    &JsValue::from(font_size),
                    &Self::js_value_from_offset(ptr_width),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *width = state
                    .copy_bytes_from_pdfium(ptr_width, len)
                    .try_into()
                    .map(c_float::from_le_bytes)
                    .unwrap_or(0.0);
            }
        }

        state.free(ptr_width);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphPath(&self, font: FPDF_FONT, glyph: c_uint, font_size: c_float) -> FPDF_GLYPHPATH {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFFont_GetGlyphPath()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFFont_GetGlyphPath",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_font(font),
                    &JsValue::from_f64(glyph as f64),
                    &JsValue::from(font_size),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_GLYPHPATH
    }

    #[allow(non_snake_case)]
    fn FPDFGlyphPath_CountGlyphSegments(&self, glyphpath: FPDF_GLYPHPATH) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFGlyphPath_CountGlyphSegments()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFGlyphPath_CountGlyphSegments",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_glyph_path(glyphpath)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFGlyphPath_GetGlyphPathSegment(&self, glyphpath: FPDF_GLYPHPATH, index: c_int) -> FPDF_PATHSEGMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFGlyphPath_GetGlyphPathSegment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFGlyphPath_GetGlyphPathSegment",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_glyph_path(glyphpath),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PATHSEGMENT
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintScaling(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetPrintScaling()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetPrintScaling",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetNumCopies(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetNumCopies()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetNumCopies",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRange(&self, document: FPDF_DOCUMENT) -> FPDF_PAGERANGE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetPrintPageRange()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetPrintPageRange",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_PAGERANGE
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeCount(&self, pagerange: FPDF_PAGERANGE) -> size_t {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetPrintPageRangeCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetPrintPageRangeCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_page_range(pagerange)))),
            )
            .as_f64()
            .unwrap() as size_t
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeElement(&self, pagerange: FPDF_PAGERANGE, index: size_t) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetPrintPageRangeElement()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetPrintPageRangeElement",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_page_range(pagerange),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetDuplex(&self, document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetDuplex()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_VIEWERREF_GetDuplex",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DUPLEXTYPE
    }

    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetName(
        &self,
        document: FPDF_DOCUMENT,
        key: &str,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_VIEWERREF_GetName()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = length as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDF_VIEWERREF_GetName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(key_ptr);
        state.free(buffer_ptr);

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDF_CountNamedDests(&self, document: FPDF_DOCUMENT) -> FPDF_DWORD {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_CountNamedDests()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDF_CountNamedDests",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as FPDF_DWORD
    }

    #[allow(non_snake_case)]
    fn FPDF_GetNamedDestByName(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_DEST {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetNamedDestByName()");

        let state = PdfiumRenderWasmState::lock();

        let c_name = CString::new(name).unwrap();

        let name_ptr = state.copy_bytes_to_pdfium(&c_name.into_bytes_with_nul());

        let result = state
            .call(
                "FPDF_GetNamedDestByName",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(name_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DEST;

        state.free(name_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDF_GetNamedDest(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: *mut c_long,
    ) -> FPDF_DEST {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDF_GetNamedDest()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let buflen_length = size_of::<c_ulong>();

        let buflen_ptr = state.copy_ptr_with_len_to_pdfium(buflen, buflen_length);

        let result = state
            .call(
                "FPDF_GetNamedDest",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(index as f64),
                    &Self::js_value_from_offset(buffer_ptr),
                    &Self::js_value_from_offset(buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_DEST;

        if result.is_null() {
            unsafe {
                *buflen = state
                    .copy_bytes_from_pdfium(buflen_ptr, buflen_length)
                    .try_into()
                    .map(c_long::from_le_bytes)
                    .unwrap_or(0);

                if *buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachmentCount(&self, document: FPDF_DOCUMENT) -> c_int {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_GetAttachmentCount()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_GetAttachmentCount",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as c_int
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment(&self, document: FPDF_DOCUMENT, name: FPDF_WIDESTRING) -> FPDF_ATTACHMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_AddAttachment()");

        let state = PdfiumRenderWasmState::lock();

        let name_ptr = state.copy_string_to_pdfium(name);

        let result = state
            .call(
                "FPDFDoc_AddAttachment",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(name_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ATTACHMENT;

        state.free(name_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_GetAttachment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_GetAttachment",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_ATTACHMENT
    }

    #[allow(non_snake_case)]
    fn FPDFDoc_DeleteAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFDoc_DeleteAttachment()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFDoc_DeleteAttachment",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Number]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &JsValue::from_f64(index as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_GetName(&self, attachment: FPDF_ATTACHMENT, buffer: *mut FPDF_WCHAR, buflen: c_ulong) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetName(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetName(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetName(): calling FPDFAttachment_GetName()");

        let result = state
            .call(
                "FPDFAttachment_GetName",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetName(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_HasKey(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_HasKey()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAttachment_HasKey",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_GetValueType(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_OBJECT_TYPE {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetValueType()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAttachment_GetValueType",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(key_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_OBJECT_TYPE;

        state.free(key_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_SetStringValue()");

        let state = PdfiumRenderWasmState::lock();

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let value_ptr = state.copy_string_to_pdfium(value);

        let result = state
            .call(
                "FPDFAttachment_SetStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(value_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(key_ptr);
        state.free(value_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_GetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetStringValue(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            log::debug!(
                "pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetStringValue(): allocating buffer of {} bytes in Pdfium's WASM heap",
                buffer_length
            );

            state.malloc(buffer_length)
        } else {
            0
        };

        log::debug!(
            "pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetStringValue(): calling FPDFAttachment_GetStringValue()"
        );

        let c_key = CString::new(key).unwrap();

        let key_ptr = state.copy_bytes_to_pdfium(&c_key.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFAttachment_GetStringValue",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(key_ptr),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);
        state.free(key_ptr);

        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetStringValue(): leaving");

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_SetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        document: FPDF_DOCUMENT,
        contents: *const c_void,
        len: c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_SetFile()");

        let state = PdfiumRenderWasmState::lock();

        let contents_ptr = state.copy_ptr_with_len_to_pdfium(contents, len as usize);

        let result = state
            .call(
                "FPDFAttachment_SetFile",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(contents_ptr),
                    &JsValue::from_f64(len as f64),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(contents_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_GetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetFile()");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let out_buflen_length = size_of::<c_ulong>();

        let out_buflen_ptr = state.malloc(out_buflen_length);

        let result = state
            .call(
                "FPDFAttachment_GetFile",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                    JsFunctionArgumentType::Pointer,
                ]),
                Some(&JsValue::from(Array::of4(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                    &Self::js_value_from_offset(out_buflen_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        if self.is_true(result) {
            unsafe {
                *out_buflen = state
                    .copy_bytes_from_pdfium(out_buflen_ptr, out_buflen_length)
                    .try_into()
                    .map(c_ulong::from_le_bytes)
                    .unwrap_or(0);

                if *out_buflen > 0 {
                    state.copy_struct_from_pdfium(buffer_ptr, *out_buflen as usize, buffer);
                }
            }
        }

        state.free(buffer_ptr);
        state.free(out_buflen_ptr);

        result
    }

    #[allow(non_snake_case)]
    fn FPDFAttachment_GetSubtype(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFAttachment_GetSubtype(): entering");

        let state = PdfiumRenderWasmState::lock();

        let buffer_length = buflen as usize;

        let buffer_ptr = if buffer_length > 0 {
            state.malloc(buffer_length)
        } else {
            0
        };

        let result = state
            .call(
                "FPDFAttachment_GetSubtype",
                JsFunctionArgumentType::Number,
                Some(vec![
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Pointer,
                    JsFunctionArgumentType::Number,
                ]),
                Some(&JsValue::from(Array::of3(
                    &Self::js_value_from_attachment(attachment),
                    &Self::js_value_from_offset(buffer_ptr),
                    &JsValue::from_f64(buffer_length as f64),
                ))),
            )
            .as_f64()
            .unwrap() as usize;

        if result > 0 && result <= buffer_length {
            state.copy_struct_from_pdfium(buffer_ptr, result, buffer);
        }

        state.free(buffer_ptr);

        result as c_ulong
    }

    #[allow(non_snake_case)]
    fn FPDFCatalog_IsTagged(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFCatalog_IsTagged()");

        PdfiumRenderWasmState::lock()
            .call(
                "FPDFCatalog_IsTagged",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of1(&Self::js_value_from_document(document)))),
            )
            .as_f64()
            .unwrap() as usize as FPDF_BOOL
    }

    #[allow(non_snake_case)]
    fn FPDFCatalog_SetLanguage(&self, document: FPDF_DOCUMENT, language: &str) -> FPDF_BOOL {
        log::debug!("pdfium-render::PdfiumLibraryBindings::FPDFCatalog_SetLanguage()");

        let state = PdfiumRenderWasmState::lock();

        let c_language = CString::new(language).unwrap();

        let language_ptr = state.copy_bytes_to_pdfium(&c_language.into_bytes_with_nul());

        let result = state
            .call(
                "FPDFCatalog_SetLanguage",
                JsFunctionArgumentType::Number,
                Some(vec![JsFunctionArgumentType::Pointer, JsFunctionArgumentType::Pointer]),
                Some(&JsValue::from(Array::of2(
                    &Self::js_value_from_document(document),
                    &Self::js_value_from_offset(language_ptr),
                ))),
            )
            .as_f64()
            .unwrap() as FPDF_BOOL;

        state.free(language_ptr);

        result
    }
}

impl Drop for WasmPdfiumBindings {
    fn drop(&mut self) {
        self.FPDF_DestroyLibrary();
    }
}
