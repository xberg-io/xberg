use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can return into an already_declared enum.
mod return_into_already_declared_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                #[swift_bridge(already_declared)]
                enum SomeEnum { }

                extern "Rust" {
                    #[swift_bridge(return_into)]
                    fn some_function() -> SomeEnum;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            fn __swift_bridge__some_function() -> <super::SomeEnum as swift_bridge::SharedEnum>::FfiRepr {
                {let val: super::SomeEnum = super::some_function().into(); val}.into_ffi_repr()
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::SkipTest
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
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
