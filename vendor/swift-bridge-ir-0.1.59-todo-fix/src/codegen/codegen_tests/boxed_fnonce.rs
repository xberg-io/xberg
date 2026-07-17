//! Tests for passing Box<dyn FnOnce(A, B) -> C> between languages.
//!
//! `*mut dyn FnOnce(A, B) -> C` is not FFI safe, so we pass `*mut Box dyn FnOnce(A, B) -> C`
//! over FFI instead.
//!
//! This means that passing a boxed fn from Rust -> Swift involves a tiny allocation where we
//! call a `Box::new` on a `Box<dyn FnOnce>`.
//!
//! Ideally we'd just pass the `*mut dyn FnOnce` pointer from Rust -> Swift in order to avoid this
//! tiny allocation entirely, but we ran into issues when trying that.
//! We're assuming that it's because `*mut dyn FnOnce` is not FFI safe, but we should research and
//! confirm this.
//! Given that Swift does nothing with the pointer other than eventually pass it back to Rust,
//! there may be a way to simply pass the `Box::into_raw(Box<dyn FnOnce>))` pointer transmuted into
//! some FFI safe type, as opposed to needing to do a `Box::into_raw(Box::new(Box<dyn FnOnce>))`
//! as we do now.

use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can pass a callback that has no args or return value from Rust to Swift.
mod test_swift_takes_no_args_no_return_callback {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce() -> ()>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce() -> ()>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce() -> ()>
                        )
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce() ->()>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallbackNoArgsNoRet(ptr: callback); let _ = some_function(callback: { cb0.call() }) }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim("")
    }

    #[test]
    fn test_swift_takes_no_args_no_return_callback() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with primitive arg from Rust to Swift.
mod test_swift_takes_callback_one_primitive_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce(u8) -> ()>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce(u8) -> ()>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce(u8) -> ()>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce(u8) -> ()>, arg0: u8) {
                    unsafe { Box::from_raw(some_function_callback)(arg0) }
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce(u8) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce(u8) -> ()>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call(_ arg0: UInt8) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param0(ptr, arg0)
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { arg0 in cb0.call(arg0) }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function$param0(void* some_function_callback, uint8_t arg0);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_one_primitive_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with a primitive return value from Rust to Swift.
mod test_swift_takes_callback_primitive_return {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce() -> u8>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce() -> u8>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce() -> u8>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce() -> u8>) -> u8 {
                    unsafe { Box::from_raw(some_function_callback)() }
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce() -> u8>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce() -> u8>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call() -> UInt8 {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param0(ptr)
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { cb0.call() }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
uint8_t __swift_bridge__$some_function$param0(void* some_function_callback);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_primitive_return() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with an opaque Rust arg from Rust to Swift.
mod test_swift_takes_callback_one_opaque_rust_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce(ARustType) -> ()>);
                }

                extern "Rust" {
                    type ARustType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce(super::ARustType) -> ()>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce(super::ARustType) -> ()>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce(super::ARustType) -> ()>, arg0: *mut super::ARustType) {
                    unsafe { Box::from_raw(some_function_callback)(unsafe { *Box::from_raw(arg0) }) }
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce(super::ARustType) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce(super::ARustType) -> ()>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call(_ arg0: ARustType) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param0(ptr, {arg0.isOwned = false; return arg0.ptr;}())
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { arg0 in cb0.call(arg0) }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function$param0(void* some_function_callback, void* arg0);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_one_opaque_rust_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with an opaque Rust arg from Rust to Swift.
mod test_swift_takes_callback_return_opaque_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce() -> ARustType>);
                }

                extern "Rust" {
                    type ARustType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce() -> super::ARustType>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce() -> super::ARustType>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce() -> super::ARustType>) -> *mut super::ARustType {
                    Box::into_raw(Box::new({
                        let val: super::ARustType = unsafe { Box::from_raw(some_function_callback)() };
                        val
                    })) as *mut super::ARustType
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce() -> super::ARustType>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce() -> super::ARustType>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call() -> ARustType {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return ARustType(ptr: __swift_bridge__$some_function$param0(ptr))
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { cb0.call() }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function$param0(void* some_function_callback);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_return_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with an opaque Rust arg from Rust to Swift.
mod test_swift_takes_callback_with_result_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce(Result<ARustType, ARustType>) -> ()>);
                }

                extern "Rust" {
                    type ARustType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce(Result<super::ARustType, super::ARustType>) -> ()>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce(Result<super::ARustType, super::ARustType>) -> ()>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce(Result<super::ARustType, super::ARustType>) -> ()>, arg0: swift_bridge::result::ResultPtrAndPtr) {
                    unsafe { Box::from_raw(some_function_callback)(
                        if arg0.is_ok {
                            std::result::Result::Ok(unsafe { *Box::from_raw(arg0.ok_or_err as *mut super::ARustType) })
                        } else {
                            std::result::Result::Err(unsafe { *Box::from_raw(arg0.ok_or_err as *mut super::ARustType) })
                        }
                    )}
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce(Result<super::ARustType, super::ARustType>) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce(Result<super::ARustType, super::ARustType>) -> ()>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call(_ arg0: RustResult<ARustType, ARustType>) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param0(ptr, { switch arg0 { case .Ok(let ok): return __private__ResultPtrAndPtr(is_ok: true, ok_or_err: {ok.isOwned = false; return ok.ptr;}()) case .Err(let err): return __private__ResultPtrAndPtr(is_ok: false, ok_or_err: {err.isOwned = false; return err.ptr;}()) } }())
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { arg0 in cb0.call(arg0) }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function$param0(void* some_function_callback, struct __private__ResultPtrAndPtr arg0);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_with_result_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass two callbacks from Rust to Swift.
///
/// We put a callback that takes arguments in the second position to ensure that our codegen
/// factors in the callbacks position when generating the FFI glue.
///
/// We also put a third callback with no arguments or return values. Callbacks with no args or
/// return value are handled slightly differently
/// (we don't generate new types to handle them and instead use a pre-defined type for this special
///  case), so we confirm that we generate the correct code.
///
/// This all helps ensure that if a function has multiple callbacks our generated code calls
/// the right one.
mod test_swift_takes_multiple_callbacks {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(
                        arg0: Box<dyn FnOnce() -> ()>,
                        arg1: Box<dyn FnOnce(u8) -> ()>,
                        arg2: Box<dyn FnOnce() -> ()>,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (
                    arg0: Box<dyn FnOnce() -> ()>,
                    arg1: Box<dyn FnOnce(u8) -> ()>,
                    arg2: Box<dyn FnOnce() -> ()>
                ) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(arg0)) as *mut Box<dyn FnOnce() -> ()>,
                            Box::into_raw(Box::new(arg1)) as *mut Box<dyn FnOnce(u8) -> ()>,
                            Box::into_raw(Box::new(arg2)) as *mut Box<dyn FnOnce() -> ()>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param1"]
                pub extern "C" fn some_function_param1(some_function_arg1: *mut Box<dyn FnOnce(u8) -> ()>, arg0: u8) {
                    unsafe { Box::from_raw(some_function_arg1)(arg0) }
                }

                #[export_name = "__swift_bridge__$some_function$_free$param1"]
                pub extern "C" fn free_some_function_param1(some_function_arg1: *mut Box<dyn FnOnce(u8) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_function_arg1) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(
                    arg0: *mut Box<dyn FnOnce() -> ()>,
                    arg1: *mut Box<dyn FnOnce(u8) -> ()>,
                    arg2: *mut Box<dyn FnOnce() -> ()>
                );
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param1 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param1(ptr)
        }
    }

    func call(_ arg0: UInt8) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param1(ptr, arg0)
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg0: UnsafeMutableRawPointer, _ arg1: UnsafeMutableRawPointer, _ arg2: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallbackNoArgsNoRet(ptr: arg0); let cb1 = __private__RustFnOnceCallback$some_function$param1(ptr: arg1); let cb2 = __private__RustFnOnceCallbackNoArgsNoRet(ptr: arg2); let _ = some_function(arg0: { cb0.call() }, arg1: { arg0 in cb1.call(arg0) }, arg2: { cb2.call() }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function$param1(void* some_function_arg1, uint8_t arg0);
void __swift_bridge__$some_function$_free$param1(void* some_function_arg1);
"#,
        )
    }

    #[test]
    fn test_swift_takes_multiple_callbacks() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback that takes multiple arguments from Rust to Swift.
mod test_swift_takes_callback_multiple_args {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function(callback: Box<dyn FnOnce(ARustType, u32) -> ()>);
                }

                extern "Rust" {
                    type ARustType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (callback: Box<dyn FnOnce(super::ARustType, u32) -> ()>) {
                    unsafe {
                        __swift_bridge__some_function(
                            Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce(super::ARustType, u32) -> ()>
                        )
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$some_function$param0"]
                pub extern "C" fn some_function_param0(some_function_callback: *mut Box<dyn FnOnce(super::ARustType, u32) -> ()>, arg0: *mut super::ARustType, arg1: u32) {
                    unsafe { Box::from_raw(some_function_callback)(unsafe { *Box::from_raw(arg0) }, arg1) }
                }

                #[export_name = "__swift_bridge__$some_function$_free$param0"]
                pub extern "C" fn free_some_function_param0(some_function_callback: *mut Box<dyn FnOnce(super::ARustType, u32) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_function_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(callback: *mut Box<dyn FnOnce(super::ARustType, u32) -> ()>);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$some_function$param0 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$some_function$_free$param0(ptr)
        }
    }

    func call(_ arg0: ARustType, _ arg1: UInt32) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$some_function$param0(ptr, {arg0.isOwned = false; return arg0.ptr;}(), arg1)
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ callback: UnsafeMutableRawPointer) {
    { let cb0 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let _ = some_function(callback: { arg0, arg1 in cb0.call(arg0, arg1) }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function$param0(void* some_function_callback, void* arg0, uint32_t arg1);
void __swift_bridge__$some_function$_free$param0(void* some_function_callback);
"#,
        )
    }

    #[test]
    fn test_swift_takes_callback_multiple_args() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback that has no args or return value from Rust to Swift.
mod test_swift_method_takes_no_args_no_return_callback {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeType;

                    fn some_method(&self, callback: Box<dyn FnOnce() -> ()>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                impl SomeType {
                    pub fn some_method (&self, callback: Box<dyn FnOnce() -> ()>) {
                        unsafe {
                            __swift_bridge__SomeType_some_method(
                                swift_bridge::PointerToSwiftType(self.0),
                                Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce() -> ()>
                            )
                        }
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$SomeType$some_method"]
                fn __swift_bridge__SomeType_some_method(
                    this: swift_bridge::PointerToSwiftType,
                    callback: *mut Box<dyn FnOnce() ->()>
                );
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$SomeType$some_method")
func __swift_bridge__SomeType_some_method (_ this: UnsafeMutableRawPointer, _ callback: UnsafeMutableRawPointer) {
    { let cb1 = __private__RustFnOnceCallbackNoArgsNoRet(ptr: callback); let _ = Unmanaged<SomeType>.fromOpaque(this).takeUnretainedValue().some_method(callback: { cb1.call() }) }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim("")
    }

    #[test]
    fn test_swift_method_takes_no_args_no_return_callback() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can pass a callback with primitive arg from Rust to Swift.
mod test_swift_method_takes_callback_one_primitive_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeType;

                    fn some_method(&self, callback: Box<dyn FnOnce(u8) -> ()>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                impl SomeType {
                    pub fn some_method (&self, callback: Box<dyn FnOnce(u8) -> ()>) {
                        unsafe {
                            __swift_bridge__SomeType_some_method(
                                swift_bridge::PointerToSwiftType(self.0),
                                Box::into_raw(Box::new(callback)) as *mut Box<dyn FnOnce(u8) -> ()>
                            )
                        }
                    }
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$SomeType$some_method$param1"]
                pub extern "C" fn SomeType_some_method_param1(some_method_callback: *mut Box<dyn FnOnce(u8) -> ()>, arg0: u8) {
                    unsafe { Box::from_raw(some_method_callback)(arg0) }
                }

                #[export_name = "__swift_bridge__$SomeType$some_method$_free$param1"]
                pub extern "C" fn free_SomeType_some_method_param1(some_method_callback: *mut Box<dyn FnOnce(u8) -> ()>) {
                    let _ = unsafe { Box::from_raw(some_method_callback) };
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$SomeType$some_method"]
                fn __swift_bridge__SomeType_some_method(
                    this: swift_bridge::PointerToSwiftType,
                    callback: *mut Box<dyn FnOnce(u8) ->()>
                );
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
class __private__RustFnOnceCallback$SomeType$some_method$param1 {
    var ptr: UnsafeMutableRawPointer
    var called = false

    init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }

    deinit {
        if !called {
            __swift_bridge__$SomeType$some_method$_free$param1(ptr)
        }
    }

    func call(_ arg0: UInt8) {
        if called {
            fatalError("Cannot call a Rust FnOnce function twice")
        }
        called = true
        return __swift_bridge__$SomeType$some_method$param1(ptr, arg0)
    }
}
            "#,
            r#"
@_cdecl("__swift_bridge__$SomeType$some_method")
func __swift_bridge__SomeType_some_method (_ this: UnsafeMutableRawPointer, _ callback: UnsafeMutableRawPointer) {
    { let cb1 = __private__RustFnOnceCallback$SomeType$some_method$param1(ptr: callback); let _ = Unmanaged<SomeType>.fromOpaque(this).takeUnretainedValue().some_method(callback: { arg0 in cb1.call(arg0) }) }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$SomeType$some_method$param1(void* some_method_callback, uint8_t arg0);
void __swift_bridge__$SomeType$some_method$_free$param1(void* some_method_callback);
"#,
        )
    }

    #[test]
    fn test_swift_method_takes_callback_one_primitive_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
