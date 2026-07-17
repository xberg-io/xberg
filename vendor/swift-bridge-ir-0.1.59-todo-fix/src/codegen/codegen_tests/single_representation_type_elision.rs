//! Tests that verify that when generating functions we elide types that have exactly one
//! representation.
//!
//! See crates/swift-integration-tests/src/single_representation_type_elision.rs

use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we elide null type arguments when generating the FFI glue for functions.
mod elides_null_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn rust_function(arg1: (), arg2: ()) -> ();
                }

                extern "Swift" {
                    fn swift_function(arg1: (), arg2: ()) -> ();
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__rust_function() {
                    super::rust_function((), ())
                }
            },
            //
            quote! {
                fn swift_function(_arg1: (), _arg2: ()) -> () {
                    unsafe { __swift_bridge__swift_function() }
                }
            },
            quote! {
                fn __swift_bridge__swift_function();
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func rust_function(_ arg1: (), _ arg2: ()) -> () {
    __swift_bridge__$rust_function()
}
"#,
            r#"
@_cdecl("__swift_bridge__$swift_function")
func __swift_bridge__swift_function () {
    swift_function(arg1: (), arg2: ())
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$rust_function(void);
"#,
        )
    }

    #[test]
    fn elides_null_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we elide unit struct arguments when generating the FFI glue for functions.
mod elides_unit_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                struct UnitStruct1;
                struct UnitStruct2 {}
                struct UnitStruct3();

                extern "Rust" {
                    fn rust_function(
                        arg1: UnitStruct1,
                        arg2: UnitStruct2,
                        arg3: UnitStruct3,
                    ) -> UnitStruct1;
                }

                extern "Swift" {
                     fn swift_function(
                        arg1: UnitStruct1,
                        arg2: UnitStruct2,
                        arg3: UnitStruct3,
                    ) -> UnitStruct1;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                fn __swift_bridge__rust_function() {
                    {super::rust_function(UnitStruct1, UnitStruct2 {}, UnitStruct3());}
                }
            },
            //
            quote! {
                fn swift_function(_arg1: UnitStruct1, _arg2: UnitStruct2, _arg3: UnitStruct3) -> UnitStruct1 {
                    {unsafe { __swift_bridge__swift_function() }; UnitStruct1}
                }
            },
            quote! {
                fn __swift_bridge__swift_function();
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public func rust_function(_ arg1: UnitStruct1, _ arg2: UnitStruct2, _ arg3: UnitStruct3) -> UnitStruct1 {
    { let _ = __swift_bridge__$rust_function(); return UnitStruct1() }()
}
"#,
            r#"
@_cdecl("__swift_bridge__$swift_function")
func __swift_bridge__swift_function () {
    { let _ = swift_function(arg1: UnitStruct1(), arg2: UnitStruct2(), arg3: UnitStruct3()); }()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$rust_function(void);
"#,
        )
    }

    #[test]
    fn elides_unit_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
