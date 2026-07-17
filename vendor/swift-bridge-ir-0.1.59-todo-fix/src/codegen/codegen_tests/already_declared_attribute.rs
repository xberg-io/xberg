use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we properly handle a `#[swift_bridge(already_declared)]` attribute on an opaque
/// Rust type.
mod extern_rust_already_declared_type_attribute {
    use super::*;

    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(already_declared)]
                    type SomeType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::DoesNotContain(quote! {
            SomeType
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim(
            r#"
SomeType
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::DoesNotContainAfterTrim(
            r#"
SomeType
    "#,
        )
    }

    #[test]
    fn extern_rust_already_declared_type_attribute() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate associated functions and methods for already declared types.
mod extern_rust_already_declared_type_still_generates_methods {
    use super::*;

    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(already_declared)]
                    type SomeType;

                    fn some_function(self);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            fn __swift_bridge__SomeType_some_function
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(r#"func some_function"#)
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"void __swift_bridge__$SomeType$some_function(void* self);"#,
        )
    }

    #[test]
    fn extern_rust_already_declared_type_still_generates_methods() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we do not re-declare an already defined struct.
mod already_declared_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                #[swift_bridge(already_declared, swift_repr = "struct")]
                struct FfiSomeType;

                extern "Rust" {
                    fn rust_some_function(arg: FfiSomeType) -> FfiSomeType;
                }

                extern "Swift" {
                    fn swift_some_function(arg: FfiSomeType) -> FfiSomeType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsManyAndDoesNotContainMany {
            contains: vec![
                quote! {
                    pub extern "C" fn __swift_bridge__rust_some_function(arg: <super::FfiSomeType as swift_bridge::SharedStruct>::FfiRepr) -> <super::FfiSomeType as swift_bridge::SharedStruct>::FfiRepr {
                        super::rust_some_function(arg.into_rust_repr()).into_ffi_repr()
                    }
                },
                quote! {
                    extern "C" {
                        #[link_name = "__swift_bridge__$swift_some_function"]
                        fn __swift_bridge__swift_some_function(arg: <super::FfiSomeType as swift_bridge::SharedStruct>::FfiRepr) -> <super::FfiSomeType as swift_bridge::SharedStruct>::FfiRepr;
                    }
                },
            ],
            does_not_contain: vec![quote! {
                struct FfiSomeType
            }],
        }
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim("struct FfiSomeType")
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(
            r#"
struct __swift_bridge__$FfiSomeType __swift_bridge__$rust_some_function(struct __swift_bridge__$FfiSomeType arg);
"#,
        )
    }

    #[test]
    fn already_declared_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we do not re-declare an already defined enum.
mod already_declared_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                #[swift_bridge(already_declared)]
                enum FfiSomeEnum {}

                extern "Rust" {
                    fn rust_some_function(arg: FfiSomeEnum) -> FfiSomeEnum;
                }

                extern "Swift" {
                    fn swift_some_function(arg: FfiSomeEnum) -> FfiSomeEnum;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsManyAndDoesNotContainMany {
            contains: vec![
                quote! {
                    pub extern "C" fn __swift_bridge__rust_some_function(arg: <super::FfiSomeEnum as swift_bridge::SharedEnum>::FfiRepr) -> <super::FfiSomeEnum as swift_bridge::SharedEnum>::FfiRepr {
                        super::rust_some_function(arg.into_rust_repr()).into_ffi_repr()
                    }
                },
                quote! {
                    extern "C" {
                        #[link_name = "__swift_bridge__$swift_some_function"]
                        fn __swift_bridge__swift_some_function(arg: <super::FfiSomeEnum as swift_bridge::SharedEnum>::FfiRepr) -> <super::FfiSomeEnum as swift_bridge::SharedEnum>::FfiRepr;
                    }
                },
            ],
            does_not_contain: vec![quote! {
                enum FfiSomeEnum
            }],
        }
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim("enum FfiSomeEnum")
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(
            r#"
struct __swift_bridge__$FfiSomeEnum __swift_bridge__$rust_some_function(struct __swift_bridge__$FfiSomeEnum arg);
"#,
        )
    }

    #[test]
    fn already_declared_enum() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we do not re-declare the Swift struct of an already defined Rust Copy type.
/// We still generate the Rust #\[repr(C)] struct since that is private to this module.
mod already_declared_rust_copy_type_does_not_redeclare_swift {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(already_declared, Copy(4))]
                    type SomeType;
                }
            }
        }
    }

    // Make sure we still generate our repr(C) struct representation
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(
            quote! { #[repr(C)] #[doc(hidden)] pub struct __swift_bridge__SomeType },
        )
    }

    // Make sure that we don't regenerate the Swift struct.
    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim("SomeType")
    }

    // Make sure that we don't re-generate our C header for the type.
    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::DoesNotContainAfterTrim("SomeType")
    }

    #[test]
    fn already_declared_rust_copy_type_does_not_redeclare_swift() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate methods for an already declared opaque Rust Copy type.
mod already_declared_rust_copy_type_methods {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(already_declared, Copy(4))]
                    type SomeType;

                    fn some_method(&self);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            fn __swift_bridge__SomeType_some_method

        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(r#"func some_method"#)
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"void __swift_bridge__$SomeType$some_method(struct __swift_bridge__$SomeType this);"#,
        )
    }

    #[test]
    fn already_declared_rust_copy_type_methods() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
