use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can properly handle `#[swift_bridge(label = "...")]` attributes.
mod argument_label {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function(
                        #[swift_bridge(label = "argumentLabel1")] parameter_name1: i32,
                        #[swift_bridge(label = "argumentLabel2")] parameter_name2: u32,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            fn __swift_bridge__some_function(parameter_name1: i32, parameter_name2: u32) {
                super::some_function(parameter_name1, parameter_name2)
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public func some_function(argumentLabel1 parameter_name1: Int32, argumentLabel2 parameter_name2: UInt32) {
    __swift_bridge__$some_function(parameter_name1, parameter_name2)
}
            
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(int32_t parameter_name1, uint32_t parameter_name2);
"#,
        )
    }

    #[test]
    fn argument_label() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can properly handle a `#[swift_bridge(label = "...")]` attribute with only one argument corresponding.
mod argument_one_label {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function(
                        #[swift_bridge(label = "argumentLabel1")] parameter_name1: i32,
                        parameter_name2: u32,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            fn __swift_bridge__some_function(parameter_name1: i32, parameter_name2: u32) {
                super::some_function(parameter_name1, parameter_name2)
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public func some_function(argumentLabel1 parameter_name1: Int32, _ parameter_name2: UInt32) {
    __swift_bridge__$some_function(parameter_name1, parameter_name2)
}

"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(int32_t parameter_name1, uint32_t parameter_name2);
"#,
        )
    }

    #[test]
    fn argument_label() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that extern "Swift" functions with `#[swift_bridge(label = "_")]` generate
/// Swift wrapper code that calls the Swift function without parameter labels.
mod extern_swift_argument_label_underscore {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Swift" {
                    fn some_function(
                        #[swift_bridge(label = "_")] arg1: i32,
                        #[swift_bridge(label = "_")] arg2: u32,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub fn some_function(arg1: i32, arg2: u32) {
                unsafe { __swift_bridge__some_function(arg1, arg2) }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg1: Int32, _ arg2: UInt32) {
    some_function(arg1, arg2)
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(r#""#)
    }

    #[test]
    fn extern_swift_argument_label_underscore() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that extern "Swift" functions with custom `#[swift_bridge(label = "...")]` generate
/// Swift wrapper code that calls the Swift function with the custom labels.
mod extern_swift_argument_label_custom {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Swift" {
                    fn some_function(
                        #[swift_bridge(label = "firstArg")] arg1: i32,
                        #[swift_bridge(label = "secondArg")] arg2: u32,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub fn some_function(arg1: i32, arg2: u32) {
                unsafe { __swift_bridge__some_function(arg1, arg2) }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (firstArg arg1: Int32, secondArg arg2: UInt32) {
    some_function(firstArg: arg1, secondArg: arg2)
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(r#""#)
    }

    #[test]
    fn extern_swift_argument_label_custom() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that extern "Swift" functions with mixed labels (some custom, some underscore, some default)
/// generate correct Swift wrapper code.
mod extern_swift_argument_label_mixed {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Swift" {
                    fn some_function(
                        #[swift_bridge(label = "_")] arg1: i32,
                        #[swift_bridge(label = "customLabel")] arg2: u32,
                        arg3: i64,
                    );
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub fn some_function(arg1: i32, arg2: u32, arg3: i64) {
                unsafe { __swift_bridge__some_function(arg1, arg2, arg3) }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg1: Int32, customLabel arg2: UInt32, _ arg3: Int64) {
    some_function(arg1, customLabel: arg2, arg3: arg3)
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim(r#""#)
    }

    #[test]
    fn extern_swift_argument_label_mixed() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
