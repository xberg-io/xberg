use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we generate the proper code for extern "Rust" methods that take owned
/// opaque Rust arguments.
mod test_extern_rust_function_owned_opaque_rust_type_argument {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    fn some_function(arg: SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: *mut super::SomeType
            ) {
                super::some_function(unsafe { * Box::from_raw(arg) })
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeType) {
    __swift_bridge__$some_function({arg.isOwned = false; return arg.ptr;}())
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
    fn extern_rust_fn_opaque_type_argument() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Rust" methods that take owned
/// opaque Rust arguments.
mod test_extern_rust_function_ref_opaque_rust_type_argument {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    fn some_function(arg: &SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: *const super::SomeType
            ) {
                super::some_function(unsafe { & * arg } )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeTypeRef) {
    __swift_bridge__$some_function(arg.ptr)
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
    fn extern_rust_fn_ref_opaque_type_argument() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Rust" methods that take owned
/// opaque Rust arguments.
mod test_extern_rust_function_ref_mut_opaque_rust_type_argument {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    fn some_function(arg: &mut SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: *mut super::SomeType
            ) {
                super::some_function(unsafe { &mut * arg } )
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeTypeRefMut) {
    __swift_bridge__$some_function(arg.ptr)
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
    fn extern_rust_fn_ref_mut_opaque_type_argument() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Rust" methods that take
/// opaque Rust arguments that implement Copy.
mod test_extern_rust_function_copy_opaque_rust_type_argument {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(4))]
                    type SomeType;

                    fn some_function(arg: SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: __swift_bridge__SomeType
            ) {
                super::some_function(arg.into_rust_repr())
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeType) {
    __swift_bridge__$some_function(arg.intoFfiRepr())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(struct __swift_bridge__$SomeType arg);
            "#,
        )
    }

    #[test]
    fn test_extern_rust_function_copy_opaque_rust_type_argument() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Rust" methods that takes a reference to an
/// opaque Rust argument that implement Copy.
mod test_extern_rust_function_copy_opaque_rust_ref_type_argument {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(4))]
                    type SomeType;

                    fn some_function(arg: &SomeType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: __swift_bridge__SomeType
            ) {
                super::some_function(&arg.into_rust_repr())
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeType) {
    __swift_bridge__$some_function(arg.intoFfiRepr())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(struct __swift_bridge__$SomeType arg);
            "#,
        )
    }

    #[test]
    fn test_extern_rust_function_copy_opaque_rust_ref_type_argument() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
