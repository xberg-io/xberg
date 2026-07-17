use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use quote::quote;

/// Verify that we properly handle a `#[cfg(feature = "foo")]` for a bridge module when the
/// feature is enabled.
mod cfg_feature_bridge_module_feature_enabled {
    use super::*;
    use crate::codegen::codegen_tests::BridgeModule;

    fn bridge_module() -> BridgeModule {
        let tokens = quote! {
            #[swift_bridge::bridge]
            #[cfg(feature = "some-feature")]
            mod ffi {
                extern "Rust" {
                    fn some_function();
                }
            }
        };
        BridgeModule {
            tokens,
            enabled_crate_features: vec!["some-feature"],
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[cfg(feature = "some-feature")]
            mod ffi {
                #[export_name = "__swift_bridge__$some_function"]
                pub extern "C" fn __swift_bridge__some_function() {
                    super::some_function()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function()
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(
        r#"
void __swift_bridge__$some_function(void);
    "#,
    );

    #[test]
    fn cfg_feature_bridge_module_feature_enabled() {
        CodegenTest {
            bridge_module: bridge_module(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Verify that we properly handle add `#[cfg(feature = "foo")]` for a bridge module when the
/// feature is enabled.
mod cfg_feature_bridge_module_feature_disabled {
    use super::*;
    use crate::codegen::codegen_tests::BridgeModule;

    fn bridge_module() -> BridgeModule {
        let tokens = quote! {
            #[swift_bridge::bridge]
            #[cfg(feature = "some-feature")]
            mod ffi {
                extern "Rust" {
                    fn some_function();
                }
            }
        };
        BridgeModule {
            tokens,
            enabled_crate_features: vec![],
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[cfg(feature = "some-feature")]
            mod ffi {
                #[export_name = "__swift_bridge__$some_function"]
                pub extern "C" fn __swift_bridge__some_function() {
                    super::some_function()
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim(
            r#"
func some_function
"#,
        )
    }

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::DoesNotContainAfterTrim(
        r#"
some_function
    "#,
    );

    #[test]
    fn cfg_feature_bridge_module_feature_disabled() {
        CodegenTest {
            bridge_module: bridge_module(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}
