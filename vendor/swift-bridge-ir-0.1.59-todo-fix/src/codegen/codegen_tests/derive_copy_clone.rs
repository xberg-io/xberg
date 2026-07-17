use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can derive the `Copy` trait on a transparent struct.
mod derive_copy_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                #[derive(Copy)]
                struct SomeStruct {
                    field: u8,
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![quote! {
            #[derive(Copy)]
            pub struct SomeStruct {
                pub field: u8
            }
        }])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::SkipTest
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn generates_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can derive the `Clone` trait on a transparent struct.
mod derive_clone_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                #[derive(Clone)]
                struct SomeStruct {
                    field: u8,
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![quote! {
            #[derive(Clone)]
            pub struct SomeStruct {
                pub field: u8
            }
        }])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::SkipTest
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn generates_struct() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
