use super::{ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can add `#[swift_bridge(Sendable)]` to an extern Rust type.
mod extern_rust_sendable_attribute {
    use super::*;
    use crate::codegen::codegen_tests::CodegenTest;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Sendable)]
                    type SomeType;
                }
            }
        }
    }

    /// Verify that we generate a function that frees the memory behind an opaque pointer to a Rust
    /// type.
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            const fn __swift_bridge__assert_send_sync<T: Send + Sync>() {}
            const _: () = { __swift_bridge__assert_send_sync::<super::SomeType>() };
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
extension SomeType: @unchecked Sendable {}
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ContainsAfterTrim(
        r#"
typedef struct SomeType SomeType;
void __swift_bridge__$SomeType$_free(void* self);
    "#,
    );

    #[test]
    fn extern_rust_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Verify that we can add `#[swift_bridge(Sendable)]` to an extern Swift type.
mod extern_swift_sendable_attribute {
    use super::*;
    use crate::codegen::codegen_tests::CodegenTest;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                extern "Swift" {
                    #[swift_bridge(Sendable)]
                    type SomeSwiftType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[repr(C)]
            pub struct SomeSwiftType(*mut std::ffi::c_void);

            impl Drop for SomeSwiftType {
                fn drop (&mut self) {
                    unsafe { __swift_bridge__SomeSwiftType__free(self.0) }
                }
            }

            unsafe impl Send for SomeSwiftType {}
            unsafe impl Sync for SomeSwiftType {}

        })
    }

    const EXPECTED_SWIFT_CODE: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
protocol __swift_bridge__IsSendable: Sendable {}
extension SomeSwiftType: __swift_bridge__IsSendable {}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_type_derive_sendable() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT_CODE,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}
