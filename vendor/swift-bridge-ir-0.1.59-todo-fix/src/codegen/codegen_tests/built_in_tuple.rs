use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can use a (primitive type, primitive type) as Rust function arg and return type.
mod extern_rust_tuple_primitives {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function(arg: (i32, u8)) -> (i32, u8);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__some_function (arg: __swift_bridge__tuple_I32U8) -> __swift_bridge__tuple_I32U8 {
                    { let val = super::some_function({let val = arg; (val.0, val.1)}); __swift_bridge__tuple_I32U8(val.0, val.1) }
                }
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_I32U8(i32, u8);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func some_function(_ arg: (Int32, UInt8)) -> (Int32, UInt8) {
    { let val = __swift_bridge__$some_function(__swift_bridge__$tuple$I32U8(_0: arg.0, _1: arg.1)); return (val._0, val._1); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdint.h>
"#,
            r#"
typedef struct __swift_bridge__$tuple$I32U8 { int32_t _0; uint8_t _1; } __swift_bridge__$tuple$I32U8;
"#,
            r#"
struct __swift_bridge__$tuple$I32U8 __swift_bridge__$some_function(struct __swift_bridge__$tuple$I32U8 arg);
"#,
        ])
    }

    #[test]
    fn extern_rust_tuple_primitives() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (String, primitive type) as Rust function arg and return type.
mod extern_rust_tuple_string_primitive {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function(arg1: (String, u32)) -> (String, u32);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__some_function (arg1: __swift_bridge__tuple_StringU32) -> __swift_bridge__tuple_StringU32 {
                    { let val = super::some_function({ let val = arg1; (unsafe { Box::from_raw(val.0).0 }, val.1)}); __swift_bridge__tuple_StringU32(swift_bridge::string::RustString(val.0).box_into_raw(), val.1) }
                }
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_StringU32(*mut swift_bridge::string::RustString, u32);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func some_function<GenericIntoRustString: IntoRustString>(_ arg1: (GenericIntoRustString, UInt32)) -> (RustString, UInt32) {
    { let val = __swift_bridge__$some_function(__swift_bridge__$tuple$StringU32(_0: { let rustString = arg1.0.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), _1: arg1.1)); return (RustString(ptr: val._0), val._1); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdint.h>
"#,
            r#"
typedef struct __swift_bridge__$tuple$StringU32 { void* _0; uint32_t _1; } __swift_bridge__$tuple$StringU32;
"#,
            r#"
struct __swift_bridge__$tuple$StringU32 __swift_bridge__$some_function(struct __swift_bridge__$tuple$StringU32 arg1);
"#,
        ])
    }

    #[test]
    fn extern_rust_tuple_string_primitive() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (OpaqueRustType, primitive type) as Rust function arg and return type.
mod extern_rust_tuple_opaque_rust_primitive {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    fn some_function(arg1: (SomeType, u32)) -> (SomeType, u32);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__some_function (arg1: __swift_bridge__tuple_SomeTypeU32) -> __swift_bridge__tuple_SomeTypeU32 {
                    { let val = super::some_function({let val = arg1; (unsafe { * Box::from_raw(val.0) }, val.1)});
                    __swift_bridge__tuple_SomeTypeU32(Box::into_raw(Box::new({
                        let val: super::SomeType = val.0;
                        val
                    })) as *mut super::SomeType, val.1) }
                }
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_SomeTypeU32(*mut super::SomeType, u32);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func some_function(_ arg1: (SomeType, UInt32)) -> (SomeType, UInt32) {
    { let val = __swift_bridge__$some_function(__swift_bridge__$tuple$SomeTypeU32(_0: {arg1.0.isOwned = false; return arg1.0.ptr;}(), _1: arg1.1)); return (SomeType(ptr: val._0), val._1); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdint.h>
"#,
            r#"
typedef struct __swift_bridge__$tuple$SomeTypeU32 { void* _0; uint32_t _1; } __swift_bridge__$tuple$SomeTypeU32;
"#,
            r#"
struct __swift_bridge__$tuple$SomeTypeU32 __swift_bridge__$some_function(struct __swift_bridge__$tuple$SomeTypeU32 arg1);
"#,
        ])
    }

    #[test]
    fn extern_rust_tuple_opaque_rust_primitive() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (f32, isize, bool) as Rust function arg and return type.
mod extern_rust_tuple_f32_isize_bool {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function(arg1: (f32, isize, bool)) -> (f32, isize, bool);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__some_function (arg1: __swift_bridge__tuple_F32IntBool) -> __swift_bridge__tuple_F32IntBool {
                    { let val = super::some_function({let val = arg1; (val.0, val.1, val.2)});
                    __swift_bridge__tuple_F32IntBool(val.0, val.1, val.2) }
                }
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_F32IntBool(f32, isize, bool);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func some_function(_ arg1: (Float, Int, Bool)) -> (Float, Int, Bool) {
    { let val = __swift_bridge__$some_function(__swift_bridge__$tuple$F32IntBool(_0: arg1.0, _1: arg1.1, _2: arg1.2)); return (val._0, val._1, val._2); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdint.h>
"#,
            r#"
#include <stdbool.h>
"#,
            r#"
typedef struct __swift_bridge__$tuple$F32IntBool { float _0; intptr_t _1; bool _2; } __swift_bridge__$tuple$F32IntBool;
"#,
            r#"
struct __swift_bridge__$tuple$F32IntBool __swift_bridge__$some_function(struct __swift_bridge__$tuple$F32IntBool arg1);
"#,
        ])
    }

    #[test]
    fn extern_rust_tuple_f32_isize_bool() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (primitive type, primitive type) as Swift function arg and return type.
mod extern_swift_tuple_primitives {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Swift" {
                    fn some_function(arg: (i32, u8)) -> (i32, u8);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (arg: (i32, u8)) -> (i32, u8) {
                    {
                        let val = unsafe { __swift_bridge__some_function ({ let val = arg ; __swift_bridge__tuple_I32U8 (val . 0 , val . 1) }) };
                        (val.0, val.1)
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: __swift_bridge__tuple_I32U8) -> __swift_bridge__tuple_I32U8;
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_I32U8(i32, u8);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: __swift_bridge__$tuple$I32U8) -> __swift_bridge__$tuple$I32U8 {
    { let val = some_function(arg: { let val = arg; return (val._0, val._1); }()); return __swift_bridge__$tuple$I32U8(_0: val.0, _1: val.1); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"typedef struct __swift_bridge__$tuple$I32U8 { int32_t _0; uint8_t _1; } __swift_bridge__$tuple$I32U8;
"#,
        ])
    }

    #[test]
    fn extern_swift_tuple_primitives() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (opaque type, String) as Swift function arg and return type.
mod extern_swift_tuple_opaque_and_string {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                }
                extern "Swift" {
                    fn some_function(arg: (SomeType, String)) -> (SomeType, String);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (arg: (super::SomeType, String)) -> (super::SomeType, String) {
                    {
                        let val = unsafe { __swift_bridge__some_function ({ let val = arg ; __swift_bridge__tuple_SomeTypeString (Box::into_raw(Box::new({
                            let val: super::SomeType = val.0;
                            val
                        })) as *mut super::SomeType , swift_bridge::string::RustString(val.1).box_into_raw()) }) };
                        (unsafe { * Box::from_raw(val.0) }, unsafe { Box::from_raw(val.1).0 })
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: __swift_bridge__tuple_SomeTypeString) -> __swift_bridge__tuple_SomeTypeString;
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_SomeTypeString(*mut super::SomeType, *mut swift_bridge::string::RustString);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: __swift_bridge__$tuple$SomeTypeString) -> __swift_bridge__$tuple$SomeTypeString {
    { let val = some_function(arg: { let val = arg; return (SomeType(ptr: val._0), RustString(ptr: val._1)); }()); return __swift_bridge__$tuple$SomeTypeString(_0: {val.0.isOwned = false; return val.0.ptr;}(), _1: { let rustString = val.1.intoRustString(); rustString.isOwned = false; return rustString.ptr }()); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"typedef struct __swift_bridge__$tuple$SomeTypeString { void* _0; void* _1; } __swift_bridge__$tuple$SomeTypeString;
"#,
        ])
    }

    #[test]
    fn extern_swift_tuple_opaque_and_string() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a (transparent struct type, transparent enum type) as Swift function arg and return type.
mod extern_swift_tuple_transparent_struct_and_transparent_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant1,
                    Variant2,
                }
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }
                extern "Swift" {
                    fn some_function(arg: (SomeStruct, SomeEnum)) -> (SomeStruct, SomeEnum);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function (arg: (SomeStruct, SomeEnum)) -> (SomeStruct, SomeEnum) {
                    {
                        let val = unsafe { __swift_bridge__some_function ({ let val = arg ; __swift_bridge__tuple_SomeStructSomeEnum (val.0.into_ffi_repr(), val.1.into_ffi_repr()) }) };
                        (val.0.into_rust_repr(), val.1.into_rust_repr())
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function(arg: __swift_bridge__tuple_SomeStructSomeEnum) -> __swift_bridge__tuple_SomeStructSomeEnum;
            },
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__tuple_SomeStructSomeEnum(__swift_bridge__SomeStruct, __swift_bridge__SomeEnum);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: __swift_bridge__$tuple$SomeStructSomeEnum) -> __swift_bridge__$tuple$SomeStructSomeEnum {
    { let val = some_function(arg: { let val = arg; return (val._0.intoSwiftRepr(), val._1.intoSwiftRepr()); }()); return __swift_bridge__$tuple$SomeStructSomeEnum(_0: val.0.intoFfiRepr(), _1: val.1.intoFfiRepr()); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"typedef struct __swift_bridge__$tuple$SomeStructSomeEnum { struct __swift_bridge__$SomeStruct _0; struct __swift_bridge__$SomeEnum _1; } __swift_bridge__$tuple$SomeStructSomeEnum;
"#,
        ])
    }

    #[test]
    fn extern_swift_tuple_transparent_struct_and_transparent_enum() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
