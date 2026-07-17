use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we generate debugDescription in Swift and Debug function in Rust when using #\[derive(Debug)]
mod derive_debug_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[derive(Debug)]
                enum SomeEnum {
                    Variant1
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                #[derive(Copy, Clone, ::std::fmt::Debug)]
                pub enum SomeEnum {
                    Variant1
                }
            },
            quote! {
                #[export_name = "__swift_bridge__$SomeEnum$Debug"]
                pub extern "C" fn __swift_bridge__SomeEnum_Debug(this: __swift_bridge__SomeEnum) -> *mut swift_bridge::string::RustString {
                    swift_bridge::string::RustString(format!("{:?}", this.into_rust_repr())).box_into_raw()
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#" 
extension SomeEnum: CustomDebugStringConvertible {
    public var debugDescription: String {
        RustString(ptr: __swift_bridge__$SomeEnum$Debug(self.intoFfiRepr())).toString()
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#" 
void* __swift_bridge__$SomeEnum$Debug(__swift_bridge__$SomeEnum this);
"#,
        )
    }

    #[test]
    fn generates_enum_to_and_from_ffi_conversions_no_data() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
