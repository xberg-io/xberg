use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we generate the functions to convert a shared struct that does not have fields
/// to and from its ffi representation.
mod generates_struct_to_and_from_ffi_conversions_no_fields {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct();
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub struct SomeStruct();

            #[repr(C)]
            #[doc(hidden)]
            pub struct __swift_bridge__SomeStruct {
                _private: u8
            }

            impl swift_bridge::SharedStruct for SomeStruct {
                type FfiRepr = __swift_bridge__SomeStruct;
            }

            impl SomeStruct {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> __swift_bridge__SomeStruct {
                    __swift_bridge__SomeStruct {
                        _private: 123
                    }
                }
            }

            impl __swift_bridge__SomeStruct {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> SomeStruct {
                    SomeStruct()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public struct SomeStruct {
    public init() {}

    @inline(__always)
    func intoFfiRepr() -> __swift_bridge__$SomeStruct {
        __swift_bridge__$SomeStruct(_private: 123)
    }
}
extension __swift_bridge__$SomeStruct {
    @inline(__always)
    func intoSwiftRepr() -> SomeStruct {
        SomeStruct()
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(
            r#"
#include <stdint.h>
#include <stdbool.h>
typedef struct __swift_bridge__$SomeStruct { uint8_t _private; } __swift_bridge__$SomeStruct;
typedef struct __swift_bridge__$Option$SomeStruct { bool is_some; __swift_bridge__$SomeStruct val; } __swift_bridge__$Option$SomeStruct;
    "#,
        )
    }

    #[test]
    fn generates_struct_to_and_from_ffi_conversions_with_fields() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the functions to convert a shared struct that has fields to and from
/// its ffi representation.
mod generates_struct_to_and_from_ffi_conversions_with_fields {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub struct SomeStruct {
                pub field: u8
            }

            #[repr(C)]
            #[doc(hidden)]
            pub struct __swift_bridge__SomeStruct {
                field: u8
            }

            impl swift_bridge::SharedStruct for SomeStruct {
                type FfiRepr = __swift_bridge__SomeStruct;
            }

            impl SomeStruct {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> __swift_bridge__SomeStruct {
                    { let val = self; __swift_bridge__SomeStruct{ field: val.field } }
                }
            }

            impl __swift_bridge__SomeStruct {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> SomeStruct {
                    { let val = self; SomeStruct { field: val.field } }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
struct SomeStruct {
    public var field: UInt8

    public init(field: UInt8) {
        self.field = field
    }

    @inline(__always)
    func intoFfiRepr() -> __swift_bridge__$SomeStruct {
        { let val = self; return __swift_bridge__$SomeStruct(field: val.field); }()
    }
}
extension __swift_bridge__$SomeStruct {
    @inline(__always)
    func intoSwiftRepr() -> SomeStruct {
        { let val = self; return SomeStruct(field: val.field); }()
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
typedef struct __swift_bridge__$SomeStruct { uint8_t field; } __swift_bridge__$SomeStruct;
    "#,
        )
    }

    #[test]
    fn generates_struct_to_and_from_ffi_conversions_with_fields() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for a struct that has a primitive field and swift_repr = "struct".
mod struct_with_primitive_field {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }

                #[swift_bridge(swift_repr = "struct")]
                pub struct AnotherStruct(u8);
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub struct SomeStruct {
                    pub field: u8
                }

                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__SomeStruct {
                    field: u8
                }
            },
            quote! {
                pub struct AnotherStruct(pub u8);

                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__AnotherStruct(u8);
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
struct SomeStruct {
    public var field: UInt8

    public init(field: UInt8) {
        self.field = field
    }
"#,
            r#"
struct AnotherStruct {
    public var _0: UInt8

    public init(_0: UInt8) {
        self._0 = _0
    }
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
typedef struct __swift_bridge__$SomeStruct { uint8_t field; } __swift_bridge__$SomeStruct;
    "#,
            r#"
typedef struct __swift_bridge__$AnotherStruct { uint8_t _0; } __swift_bridge__$AnotherStruct;
    "#,
        ])
    }

    #[test]
    fn struct_with_primitive_field() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for passing a `swift_repr = "struct"` as an argument to a
/// extern "Rust" fn.
mod extern_rust_fn_arg_swift_repr_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                pub struct SomeStruct {
                    pub field: u8
                }

                extern "Rust" {
                    fn some_function(arg: SomeStruct);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function (arg: __swift_bridge__SomeStruct) {
                super::some_function(arg.into_rust_repr())
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeStruct) {
    __swift_bridge__$some_function(arg.intoFfiRepr())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(struct __swift_bridge__$SomeStruct arg);
    "#,
        )
    }

    #[test]
    fn extern_rust_fn_arg_swift_repr_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for passing a `swift_repr = "struct"` as an argument to a
/// extern "Swift" fn.
mod extern_swift_fn_arg_swift_repr_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                pub struct SomeStruct {
                    pub field: u8
                }

                extern "Swift" {
                    fn some_function(arg: SomeStruct);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
             extern "C" {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function (arg : __swift_bridge__SomeStruct);
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: __swift_bridge__$SomeStruct) {
    some_function(arg: arg.intoSwiftRepr())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn extern_swift_fn_arg_swift_repr_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for passing a `swift_repr = "struct"` from extern "Rust".
mod extern_rust_return_swift_repr_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }

                extern "Rust" {
                    fn some_function() -> SomeStruct;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function () -> __swift_bridge__SomeStruct {
                super::some_function().into_ffi_repr()
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> SomeStruct {
    __swift_bridge__$some_function().intoSwiftRepr()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
typedef struct __swift_bridge__$SomeStruct { uint8_t field; } __swift_bridge__$SomeStruct;
    "#,
            r#"
struct __swift_bridge__$SomeStruct __swift_bridge__$some_function(void);
    "#,
        ])
    }

    #[test]
    fn extern_rust_return_swift_repr_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for passing a `swift_repr = "struct"` from extern "Rust".
mod extern_swift_return_swift_repr_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }

                extern "Swift" {
                    fn some_function() -> SomeStruct;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
             extern "C" {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function () -> __swift_bridge__SomeStruct;
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function () -> __swift_bridge__$SomeStruct {
    some_function().intoFfiRepr()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
typedef struct __swift_bridge__$SomeStruct { uint8_t field; } __swift_bridge__$SomeStruct;
    "#,
        )
    }

    #[test]
    fn extern_swift_return_swift_repr_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that the original name of the struct is not present in any of the generated Swift
/// code when we use the `swift_name` attribute..
/// Related: crates/swift-integration-tests/src/struct_attributes/swift_name.rs
mod shared_struct_swift_name_attribute {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "StructRename1")]
                struct StructName1;

                #[swift_bridge(swift_name = "StructRename2", swift_repr = "struct")]
                struct StructName2 {
                    field: u8,
                }

                #[swift_bridge(swift_name = "StructRename3", swift_repr = "struct")]
                struct StructName3(u8);

                extern "Rust" {
                    fn extern_rust_struct_rename_1(arg: StructName1) -> StructName1;
                    fn extern_rust_struct_rename_2(arg: StructName2) -> StructName2;
                    fn extern_rust_struct_rename_3(arg: StructName3) -> StructName3;
                }

                extern "Rust" {
                    fn extern_swift_struct_rename_1(arg: StructName1) -> StructName1;
                    fn extern_swift_struct_rename_2(arg: StructName2) -> StructName2;
                    fn extern_swift_struct_rename_3(arg: StructName3) -> StructName3;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::SkipTest
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainManyAfterTrim(vec![
            "StructName1",
            "StructName2",
            "StructName3",
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::DoesNotContainManyAfterTrim(vec![
            "StructName1",
            "StructName2",
            "StructName3",
        ])
    }

    #[test]
    fn shared_struct_swift_name_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use `Option<Struct>` as Rust function arg and return type.
mod extern_rust_option_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct SomeStruct {
                    field: u8
                }

                extern "Rust" {
                    fn some_function(arg: Option<SomeStruct>) -> Option<SomeStruct>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__Option_SomeStruct {
                    is_some: bool,
                    val: std::mem::MaybeUninit<__swift_bridge__SomeStruct>,
                }

                impl __swift_bridge__Option_SomeStruct {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_rust_repr(self) -> Option<SomeStruct> {
                        if self.is_some {
                            Some(unsafe { self.val.assume_init().into_rust_repr() })
                        } else {
                            None
                        }
                    }

                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn from_rust_repr(val: Option<SomeStruct>) -> __swift_bridge__Option_SomeStruct {
                        if let Some(val) = val {
                            __swift_bridge__Option_SomeStruct {
                                is_some: true,
                                val: std::mem::MaybeUninit::new(val.into_ffi_repr())
                            }
                        } else {
                            __swift_bridge__Option_SomeStruct {
                                is_some: false,
                                val: std::mem::MaybeUninit::uninit()
                            }
                        }
                    }
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__some_function(arg: __swift_bridge__Option_SomeStruct) -> __swift_bridge__Option_SomeStruct {
                    __swift_bridge__Option_SomeStruct::from_rust_repr(super::some_function(arg.into_rust_repr()))
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
extension __swift_bridge__$Option$SomeStruct {
    @inline(__always)
    func intoSwiftRepr() -> Optional<SomeStruct> {
        if self.is_some {
            return self.val.intoSwiftRepr()
        } else {
            return nil
        }
    }

    @inline(__always)
    static func fromSwiftRepr(_ val: Optional<SomeStruct>) -> __swift_bridge__$Option$SomeStruct {
        if let v = val {
            return __swift_bridge__$Option$SomeStruct(is_some: true, val: v.intoFfiRepr())
        } else {
            return __swift_bridge__$Option$SomeStruct(is_some: false, val: __swift_bridge__$SomeStruct())
        }
    }
}
"#,
            r#"
func some_function(_ arg: Optional<SomeStruct>) -> Optional<SomeStruct> {
    __swift_bridge__$some_function(__swift_bridge__$Option$SomeStruct.fromSwiftRepr(arg)).intoSwiftRepr()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(
            r#"
#include <stdint.h>
#include <stdbool.h>
typedef struct __swift_bridge__$SomeStruct { uint8_t field; } __swift_bridge__$SomeStruct;
typedef struct __swift_bridge__$Option$SomeStruct { bool is_some; __swift_bridge__$SomeStruct val; } __swift_bridge__$Option$SomeStruct;
struct __swift_bridge__$Option$SomeStruct __swift_bridge__$some_function(struct __swift_bridge__$Option$SomeStruct arg);
    "#,
        )
    }

    #[test]
    fn option_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
