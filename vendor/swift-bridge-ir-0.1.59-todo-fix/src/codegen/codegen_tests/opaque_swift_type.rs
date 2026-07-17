use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Test code generation for freestanding Swift function that takes an opaque Swift type argument.
mod extern_swift_freestanding_fn_with_owned_opaque_swift_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                extern "Swift" {
                    type MyType;
                    fn some_function (arg: MyType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub fn some_function (arg: MyType) {
                unsafe { __swift_bridge__some_function (arg) }
            }

            #[repr(C)]
            pub struct MyType(*mut std::ffi::c_void);

            impl Drop for MyType {
                fn drop (&mut self) {
                    unsafe { __swift_bridge__MyType__free(self.0) }
                }
            }

            #[allow(improper_ctypes)]
            extern "C" {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function (arg: MyType);

                #[link_name = "__swift_bridge__$MyType$_free"]
                fn __swift_bridge__MyType__free (this: *mut std::ffi::c_void);
            }
        })
    }

    const EXPECTED_SWIFT_CODE: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: UnsafeMutableRawPointer) {
    some_function(arg: Unmanaged<MyType>.fromOpaque(arg).takeRetainedValue())
}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ExactAfterTrim(r#""#);

    #[test]
    fn extern_swift_freestanding_fn_with_owned_opaque_swift_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT_CODE,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Swift" functions that returns an
/// opaque Swift type.
mod test_extern_swift_freestanding_function_owned_opaque_swift_type_return {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeType;

                    fn some_function() -> SomeType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub fn some_function () -> SomeType {
                    unsafe { __swift_bridge__some_function() }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function() -> SomeType;
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function () -> UnsafeMutableRawPointer {
    Unmanaged.passRetained(some_function()).toOpaque()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim("")
    }

    #[test]
    fn test_extern_swift_freestanding_function_owned_opaque_swift_type_return() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the proper code for extern "Swift" methods that returns an
/// opaque Swift type.
mod test_extern_swift_method_owned_opaque_swift_type_return {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Swift" {
                    type SomeType;

                    fn some_method(&self) -> SomeType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                impl SomeType {
                    pub fn some_method (&self) -> SomeType {
                        unsafe { __swift_bridge__SomeType_some_method(swift_bridge::PointerToSwiftType(self.0)) }
                    }
                }
            },
            quote! {
                #[link_name = "__swift_bridge__$SomeType$some_method"]
                fn __swift_bridge__SomeType_some_method(this: swift_bridge::PointerToSwiftType) -> SomeType;
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
@_cdecl("__swift_bridge__$SomeType$some_method")
func __swift_bridge__SomeType_some_method (_ this: UnsafeMutableRawPointer) -> UnsafeMutableRawPointer {
    Unmanaged.passRetained(Unmanaged<SomeType>.fromOpaque(this).takeUnretainedValue().some_method()).toOpaque()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ExactAfterTrim("")
    }

    #[test]
    fn test_extern_swift_method_owned_opaque_swift_type_return() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
