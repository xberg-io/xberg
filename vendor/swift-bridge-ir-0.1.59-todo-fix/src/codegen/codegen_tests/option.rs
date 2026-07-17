//! See also: crates/swift-integration-tests/src/option.rs

use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Test code generation for Rust function that accepts and returns an Option<T> where T is a
/// primitive.
mod extern_rust_fn_option_primitive {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    fn some_function (arg: Option<u8>) -> Option<f32>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: swift_bridge::option::OptionU8
            ) -> swift_bridge::option::OptionF32 {
                if let Some(val) = super::some_function(
                    {
                        let val = arg;
                        if val.is_some {
                            Some(val.val)
                        } else {
                            None
                        }
                    }
                ) {
                    swift_bridge::option::OptionF32 { val, is_some: true}
                } else {
                    swift_bridge::option::OptionF32 { val: 123.4, is_some: false}
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<UInt8>) -> Optional<Float> {
    __swift_bridge__$some_function(arg.intoFfiRepr()).intoSwiftRepr()
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
struct __private__OptionF32 __swift_bridge__$some_function(struct __private__OptionU8 arg);
    "#,
    );

    #[test]
    fn extern_rust_fn_option_primitive() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for a Swift function that accepts and returns an Option<T> where T is a
/// primitive.
mod extern_swift_fn_option_primitive {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function (arg: Option<u8>) -> Option<f32>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function(arg: Option<u8>) -> Option<f32> {
                    {
                        let val = unsafe {
                            __swift_bridge__some_function(
                                if let Some(val) = arg {
                                   swift_bridge::option::OptionU8 { val, is_some: true }
                                } else {
                                   swift_bridge::option::OptionU8 { val: 123, is_some: false }
                                }
                            )
                        };
                        if val.is_some {
                            Some(val.val)
                        } else {
                            None
                        }
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: swift_bridge::option::OptionU8) -> swift_bridge::option::OptionF32;
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: __private__OptionU8) -> __private__OptionF32 {
    some_function(arg: arg.intoSwiftRepr()).intoFfiRepr()
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_fn_option_primitive() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Rust function that accepts and returns an Option<Vec<T>> where T is a
/// primitive.
mod extern_rust_fn_option_vector_primitive {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    fn some_function (arg: Option<Vec<i32>>) -> Option<Vec<u32>>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: *mut Vec<i32>
            ) -> *mut Vec<u32> {
                if let Some(value) = super::some_function(
                    if arg.is_null() {
                        None
                    } else {
                        Some( unsafe { * Box::from_raw(arg) } )
                    }
                ) {
                    Box::into_raw(Box::new(value))
                } else {
                    std::ptr::null_mut()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public func some_function(_ arg: Optional<RustVec<Int32>>) -> Optional<RustVec<UInt32>> {
    { let val = __swift_bridge__$some_function({ if let val = arg { val.isOwned = false; return val.ptr } else { return nil } }()); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
void* __swift_bridge__$some_function(void* arg);
    "#,
    );

    #[test]
    fn extern_rust_fn_option_vector_primitive() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

// TODO: Split this into two test modules, one for `Option<String>` arg
//  and another for `Option<String>` return value.
//  Easier to reason about when each codegen module is focused on one type in one position.
/// Test code generation for Rust function that accepts and returns Option<String>.
mod extern_rust_fn_option_string {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    fn some_function (arg: Option<String>) -> Option<String>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: *mut swift_bridge::string::RustString
            ) -> *mut swift_bridge::string::RustString {
                if let Some(val) = super::some_function(
                    {
                        let val = arg;

                        if val.is_null() {
                            None
                        } else {
                            Some(unsafe { Box::from_raw(val).0 })
                        }
                    }
                ) {
                    swift_bridge::string::RustString(val).box_into_raw()
                } else {
                    std::ptr::null::<swift_bridge::string::RustString>() as *mut swift_bridge::string::RustString
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function<GenericIntoRustString: IntoRustString>(_ arg: Optional<GenericIntoRustString>) -> Optional<RustString> {
    { let val = __swift_bridge__$some_function({ if let rustString = optionalStringIntoRustString(arg) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()); if val != nil { return RustString(ptr: val!) } else { return nil } }()
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
void* __swift_bridge__$some_function(void* arg);
    "#,
    );

    #[test]
    fn extern_rust_fn_return_option_string() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Rust function that returns Option<&str>
mod extern_rust_fn_return_option_str {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    fn some_function () -> Option<&str>;
                    fn another_function () -> Option<&'static str>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        let no_lifetime = quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> swift_bridge::string::RustStr {
                if let Some(val) = super::some_function() {
                    swift_bridge::string::RustStr::from_str(val)
                } else {
                    swift_bridge::string::RustStr { start: std::ptr::null::<u8>(), len: 0}
                }
            }
        };
        let lifetime = quote! {
            #[export_name = "__swift_bridge__$another_function"]
            pub extern "C" fn __swift_bridge__another_function() -> swift_bridge::string::RustStr {
                if let Some(val) = super::another_function() {
                    swift_bridge::string::RustStr::from_str(val)
                } else {
                    swift_bridge::string::RustStr { start: std::ptr::null::<u8>(), len: 0}
                }
            }
        };

        ExpectedRustTokens::ContainsMany(vec![no_lifetime, lifetime])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        let no_lifetime = r#"
func some_function() -> Optional<RustStr> {
    { let val = __swift_bridge__$some_function(); if val.start != nil { return val; } else { return nil; } }()
}
"#;
        let lifetime = r#"
func another_function() -> Optional<RustStr> {
    { let val = __swift_bridge__$another_function(); if val.start != nil { return val; } else { return nil; } }()
}
        "#;

        ExpectedSwiftCode::ContainsManyAfterTrim(vec![no_lifetime, lifetime])
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
struct RustStr __swift_bridge__$some_function(void);
struct RustStr __swift_bridge__$another_function(void);
    "#,
    );

    #[test]
    fn extern_rust_fn_return_option_str() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<&str> arg
mod extern_rust_fn_arg_option_str {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    fn some_function (arg: Option<&str>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
               arg: swift_bridge::string::RustStr
            ) {
                super::some_function(
                    {
                        let val = arg;

                        if val.start.is_null() {
                            None
                        } else {
                            Some(val.to_str())
                        }
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function<GenericToRustStr: ToRustStr>(_ arg: Optional<GenericToRustStr>) {
    optionalRustStrToRustStr(arg, { argAsRustStr in
        __swift_bridge__$some_function(argAsRustStr)
    })
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
void __swift_bridge__$some_function(struct RustStr arg);
    "#,
    );

    #[test]
    fn extern_rust_fn_arg_option_str() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Swift function that accepts an Option<String> argument.
mod extern_swift_func_option_string_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function (arg: Option<String>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
              pub fn some_function(arg: Option<String>) {
                  unsafe {
                      __swift_bridge__some_function(
                          if let Some(val) = arg {
                              swift_bridge::string::RustString(val).box_into_raw()
                          } else {
                              std::ptr::null::<swift_bridge::string::RustString>() as *mut swift_bridge::string::RustString
                          }
                      )
                  }
              }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: *mut swift_bridge::string::RustString);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: UnsafeMutableRawPointer?) {
    some_function(arg: { let val = arg; if val != nil { return RustString(ptr: val!) } else { return nil } }())
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_func_option_string_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Swift function that returns an owned optional String argument.
mod extern_swift_func_returns_optional_string {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                extern "Swift" {
                    fn some_function () -> Option<String>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function () -> Option<String> {
                    {
                        let val = unsafe { __swift_bridge__some_function() };

                        if val.is_null() {
                            None
                        } else {
                            Some( unsafe { Box::from_raw(val).0 })
                        }
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function() -> *mut swift_bridge::string::RustString ;
            },
        ])
    }

    const EXPECTED_SWIFT_CODE: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function () -> UnsafeMutableRawPointer? {
    { if let rustString = optionalStringIntoRustString(some_function()) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()
}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_func_returns_optional_string() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT_CODE,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Swift function that accepts an Option<&str> argument.
mod extern_swift_func_option_str_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    fn some_function (arg: Option<&str>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
              pub fn some_function(arg: Option<&str>) {
                  unsafe {
                      __swift_bridge__some_function(
                          if let Some(val) = arg {
                              swift_bridge::string::RustStr::from_str(val)
                          } else {
                              swift_bridge::string::RustStr { start: std::ptr::null::<u8>(), len: 0}
                          }
                      )
                  }
              }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: swift_bridge::string::RustStr);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: RustStr) {
    some_function(arg: { let val = arg; if val.start != nil { return val; } else { return nil; } }())
}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_func_option_str_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<OpaqueRustType>
mod extern_rust_fn_return_option_opaque_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn some_function () -> Option<SomeType>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> *mut super::SomeType {
                if let Some(val) = super::some_function() {
                    Box::into_raw(Box::new(val))
                } else {
                    std::ptr::null_mut()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeType> {
    { let val = __swift_bridge__$some_function(); if val != nil { return SomeType(ptr: val!) } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<OpaqueSwiftType>
mod extern_rust_fn_return_option_opaque_swift_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeSwiftType;
                }

                extern "Rust" {
                    fn some_function() -> Option<SomeSwiftType>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                #[export_name = "__swift_bridge__$some_function"]
                pub extern "C" fn __swift_bridge__some_function() -> *mut super::SomeSwiftType {
                    if let Some(val) = super::some_function() {
                        let val = std::mem::ManuallyDrop::new(val);
                        val.0 as *mut super::SomeSwiftType
                    } else {
                        std::ptr::null_mut()
                    }
                }
            },
            quote! {
                #[repr(C)]
                pub struct SomeSwiftType(*mut std::ffi::c_void);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeSwiftType> {
    { if let val = __swift_bridge__$some_function() { return Unmanaged<SomeSwiftType>.fromOpaque(val).takeRetainedValue() } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function(void);
            "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_opaque_swift_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<&OpaqueRustType>
mod extern_rust_fn_return_option_ref_opaque_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn some_function () -> Option<&SomeType>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> *const super::SomeType {
                if let Some(val) = super::some_function() {
                    val as *const super::SomeType
                } else {
                    std::ptr::null()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeTypeRef> {
    { let val = __swift_bridge__$some_function(); if val != nil { return SomeTypeRef(ptr: val!) } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<&OpaqueRustType> argument
mod extern_rust_fn_arg_option_ref_opaque_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn some_function (arg: Option<&SomeType>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(arg: *const super::SomeType) {
                super::some_function(
                    if arg.is_null() {
                        None
                    } else {
                        Some( unsafe { & * arg })
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeTypeRef>) {
    __swift_bridge__$some_function({ if let val = arg { return val.ptr } else { return nil } }())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(void* arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<OpaqueRustType> argument.
mod extern_rust_fn_with_option_opaque_rust_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn some_function (arg: Option<SomeType>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: *mut super::SomeType
            ) {
                super::some_function(
                    if arg.is_null() {
                        None
                    } else {
                        Some( unsafe { * Box::from_raw(arg) } )
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeType>) {
    __swift_bridge__$some_function({ if let val = arg { val.isOwned = false; return val.ptr } else { return nil } }())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(void* arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_with_option_opaque_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<OpaqueSwiftType> argument.
mod extern_rust_fn_with_option_opaque_swift_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeSwiftType;
                }

                extern "Rust" {
                    fn some_function(arg: Option<SomeSwiftType>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                #[export_name = "__swift_bridge__$some_function"]
                pub extern "C" fn __swift_bridge__some_function(
                    arg: *mut super::SomeSwiftType
                ) {
                    super::some_function({
                        let val = arg;
                        if val.is_null() {
                            None
                        } else {
                            Some(SomeSwiftType(val as *mut std::ffi::c_void))
                        }
                    })
                }
            },
            quote! {
                #[repr(C)]
                pub struct SomeSwiftType(*mut std::ffi::c_void);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeSwiftType>) {
    __swift_bridge__$some_function({ if let val = arg { return Unmanaged.passRetained(val).toOpaque() } else { return nil } }())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(void* arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_with_option_opaque_swift_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<OpaqueRustType<T>>
mod extern_rust_fn_return_option_generic_opaque_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                    fn some_function () -> Option<SomeType<u32>>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> *mut super::SomeType<u32> {
                if let Some(val) = super::some_function() {
                    Box::into_raw(Box::new(val))
                } else {
                    std::ptr::null_mut()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeType<UInt32>> {
    { let val = __swift_bridge__$some_function(); if val != nil { return SomeType(ptr: val!) } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_generic_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<OpaqueRustType<T>> argument.
mod extern_rust_fn_with_option_generic_opaque_rust_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                    fn some_function (arg: Option<SomeType<u32>>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: *mut super::SomeType<u32>
            ) {
                super::some_function(
                    if arg.is_null() {
                        None
                    } else {
                        Some( unsafe { * Box::from_raw(arg) } )
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeType<UInt32>>) {
    __swift_bridge__$some_function({ if let val = arg { val.isOwned = false; return val.ptr } else { return nil } }())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(void* arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_with_option_generic_opaque_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<OpaqueCopyRustType>
mod extern_rust_fn_return_option_opaque_copy_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(4))]
                    type SomeType;
                    fn some_function () -> Option<SomeType>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> __swift_bridge__Option_SomeType {
                if let Some(val) = super::some_function() {
                    __swift_bridge__Option_SomeType {
                        is_some: true,
                        val: std::mem::MaybeUninit::new(__swift_bridge__SomeType::from_rust_repr(val))
                    }
                } else {
                    __swift_bridge__Option_SomeType {
                        is_some: false,
                        val: std::mem::MaybeUninit::uninit()
                    }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeType> {
    { let val = __swift_bridge__$some_function(); if val.is_some { return SomeType(bytes: val.val) } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
__swift_bridge__$Option$SomeType __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_opaque_copy_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<OpaqueCopyRustType> argument.
mod extern_rust_fn_with_option_opaque_copy_rust_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(4))]
                    type SomeType;
                    fn some_function (arg: Option<SomeType>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: __swift_bridge__Option_SomeType
            ) {
                super::some_function(
                    if arg.is_some {
                        Some( unsafe { arg.val.assume_init() }.into_rust_repr() )
                    } else {
                        None
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeType>) {
    __swift_bridge__$some_function(__swift_bridge__$Option$SomeType(is_some: arg != nil, val: { if let val = arg { return val.intoFfiRepr() } else { return __swift_bridge__$SomeType() } }() ))
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(__swift_bridge__$Option$SomeType arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_with_option_opaque_copy_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that returns an Option<OpaqueCopyRustType<T>>
mod extern_rust_fn_return_option_generic_opaque_copy_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    #[swift_bridge(Copy(4))]
                    type SomeType<u32>;

                    fn some_function () -> Option<SomeType<u32>>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function() -> __swift_bridge__Option_SomeType_u32 {
                if let Some(val) = super::some_function() {
                    __swift_bridge__Option_SomeType_u32 {
                        is_some: true,
                        val: std::mem::MaybeUninit::new(__swift_bridge__SomeType_u32::from_rust_repr(val))
                    }
                } else {
                    __swift_bridge__Option_SomeType_u32 {
                        is_some: false,
                        val: std::mem::MaybeUninit::uninit()
                    }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> Optional<SomeType<UInt32>> {
    { let val = __swift_bridge__$some_function(); if val.is_some { return SomeType(bytes: val.val) } else { return nil } }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
__swift_bridge__$Option$SomeType$u32 __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_return_option_generic_opaque_copy_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for Rust function that takes an Option<OpaqueCopyRustType<T>> argument.
mod extern_rust_fn_with_option_generic_opaque_copy_rust_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    #[swift_bridge(Copy(4))]
                    type SomeType<u32>;

                    fn some_function (arg: Option<SomeType<u32>>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function(
                arg: __swift_bridge__Option_SomeType_u32
            ) {
                super::some_function(
                    if arg.is_some {
                        Some( unsafe { arg.val.assume_init() }.into_rust_repr() )
                    } else {
                        None
                    }
                )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: Optional<SomeType<UInt32>>) {
    __swift_bridge__$some_function(__swift_bridge__$Option$SomeType$u32(is_some: arg != nil, val: { if let val = arg { return val.intoFfiRepr() } else { return __swift_bridge__$SomeType$u32() } }() ))
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(__swift_bridge__$Option$SomeType$u32 arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_with_option_generic_opaque_copy_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test conversion to and from the FFI representation of a struct that contains Option<F> fields.
mod shared_struct_with_option_field_ffi_repr {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                struct SomeStruct {
                    field: Option<u8>
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub struct __swift_bridge__SomeStruct {
                    field: swift_bridge::option::OptionU8
                }
            },
            quote! {
                impl SomeStruct {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_ffi_repr(self) -> __swift_bridge__SomeStruct {
                        {
                            let val = self;
                            __swift_bridge__SomeStruct {
                                field: if let Some(val) = val.field {
                                    swift_bridge::option::OptionU8 {
                                        val,
                                        is_some: true
                                    }
                                } else {
                                    swift_bridge::option::OptionU8 {
                                        val: 123,
                                        is_some: false
                                    }
                                }
                            }
                        }
                    }
                }
            },
            quote! {
                impl __swift_bridge__SomeStruct {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_rust_repr(self) -> SomeStruct {
                         {
                            let val = self;
                            SomeStruct {
                                field: {
                                    let val = val.field;
                                    if val.is_some {
                                        Some(val.val)
                                    } else {
                                        None
                                    }
                                }
                            }
                        }
                    }
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public struct SomeStruct {
    public var field: Optional<UInt8>

    public init(field: Optional<UInt8>) {
        self.field = field
    }

    @inline(__always)
    func intoFfiRepr() -> __swift_bridge__$SomeStruct {
        { let val = self; return __swift_bridge__$SomeStruct(field: val.field.intoFfiRepr()); }()
    }
}
extension __swift_bridge__$SomeStruct {
    @inline(__always)
    func intoSwiftRepr() -> SomeStruct {
        { let val = self; return SomeStruct(field: val.field.intoSwiftRepr()); }()
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
typedef struct __swift_bridge__$SomeStruct { struct __private__OptionU8 field; } __swift_bridge__$SomeStruct;
    "#,
        )
    }

    #[test]
    fn shared_struct_with_option_field_ffi_repr() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test conversion to and from the FFI representation of a struct that contains Option<String> fields.
mod shared_struct_with_option_string_field_ffi_repr {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                struct SomeStruct {
                    field: Option<String>
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub struct __swift_bridge__SomeStruct {
                    field: *mut swift_bridge::string::RustString
                }
            },
            quote! {
                impl SomeStruct {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_ffi_repr(self) -> __swift_bridge__SomeStruct {
                        {
                            let val = self;
                            __swift_bridge__SomeStruct {
                                field: if let Some(val) = val.field {
                                    swift_bridge::string::RustString(val).box_into_raw()
                                } else {
                                    std::ptr::null::<swift_bridge::string::RustString>() as *mut swift_bridge::string::RustString
                                }
                            }
                        }
                    }
                }
            },
            quote! {
                impl __swift_bridge__SomeStruct {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_rust_repr(self) -> SomeStruct {
                         {
                            let val = self;
                            SomeStruct {
                                field: {
                                    let val = val.field;

                                    if val.is_null() {
                                        None
                                    } else {
                                        Some( unsafe { Box::from_raw(val).0 } )
                                    }
                                }
                            }
                        }
                    }
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public struct SomeStruct {
    public var field: Optional<RustString>

    public init(field: Optional<RustString>) {
        self.field = field
    }

    @inline(__always)
    func intoFfiRepr() -> __swift_bridge__$SomeStruct {
        { let val = self; return __swift_bridge__$SomeStruct(field: { if let rustString = optionalStringIntoRustString(val.field) { rustString.isOwned = false; return rustString.ptr } else { return nil } }()); }()
    }
}
extension __swift_bridge__$SomeStruct {
    @inline(__always)
    func intoSwiftRepr() -> SomeStruct {
        { let val = self; return SomeStruct(field: { let val = val.field; if val != nil { return RustString(ptr: val!) } else { return nil } }()); }()
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
typedef struct __swift_bridge__$SomeStruct { void* field; } __swift_bridge__$SomeStruct;
    "#,
        )
    }

    #[test]
    fn shared_struct_with_option_string_field_ffi_repr() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
