use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we use the `#[swift_bridge(args_into = (arg1, another_arg))]` attribute.
mod function_args_into_attribute {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                #[swift_bridge(swift_name = "SomeStruct")]
                struct FfiSomeStruct {
                    field: u16
                }

                #[swift_bridge(swift_name = "AnotherStruct")]
                struct FfiAnotherStruct(u8);

                extern "Rust" {
                    #[swift_bridge(args_into = (some_arg, another_arg))]
                    fn some_function(some_arg: FfiSomeStruct, another_arg: FfiAnotherStruct, arg3: u8);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function(
                some_arg: __swift_bridge__FfiSomeStruct,
                another_arg: __swift_bridge__FfiAnotherStruct,
                arg3: u8
            ) {
                super::some_function(some_arg.into_rust_repr().into(), another_arg.into_rust_repr().into(), arg3)
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ some_arg: SomeStruct, _ another_arg: AnotherStruct, _ arg3: UInt8) {
    __swift_bridge__$some_function(some_arg.intoFfiRepr(), another_arg.intoFfiRepr(), arg3)
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn function_args_into_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we explicitly case to the struct when generating `return_into` code for a
/// Rust function that returns a shared struct.
/// This explicit conversion into the struct avoids a type inference issues when converting the
/// shared struct to its FFI representation so that we can return it.
mod return_into_attribute_for_shared_struct {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "StructRename1")]
                struct StructName1 {
                    field: u8
                }

                extern "Rust" {
                    #[swift_bridge(return_into)]
                    fn some_function() -> StructName1;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function() -> __swift_bridge__StructName1 {
                 { let val: StructName1 = super::some_function().into(); val }.into_ffi_repr()
            }
        })
    }

    #[test]
    fn shared_struct_swift_name_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: ExpectedSwiftCode::SkipTest,
            expected_c_header: ExpectedCHeader::SkipTest,
        }
        .test();
    }
}

/// Verify that we explicitly case to the struct when generating `return_into` code for a
/// Rust function that returns a shared struct.
/// This explicit conversion into the struct avoids a type inference issues when converting the
/// shared struct to its FFI representation so that we can return it.
mod return_into_attribute_for_transparent_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant
                }

                extern "Rust" {
                    #[swift_bridge(return_into)]
                    fn some_function() -> SomeEnum;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function() -> __swift_bridge__SomeEnum {
                 { let val: SomeEnum = super::some_function().into(); val }.into_ffi_repr()
            }
        })
    }

    #[test]
    fn shared_struct_swift_name_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: ExpectedSwiftCode::SkipTest,
            expected_c_header: ExpectedCHeader::SkipTest,
        }
        .test();
    }
}

/// Verify that we can use `return_with` to convert a return type.
mod return_with {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(return_with = path::to::convert_fn)]
                    fn some_function() -> u32;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function() -> u32 {
                super::path::to::convert_fn(super::some_function())
            }
        })
    }

    #[test]
    fn return_with() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: ExpectedSwiftCode::SkipTest,
            expected_c_header: ExpectedCHeader::SkipTest,
        }
        .test();
    }
}

/// Verify that we can annotate that a function should serve as the Identifiable protocol extension.
mod protocol_identifiable {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;
                    type AnotherType;

                    #[swift_bridge(Identifiable)]
                    fn some_function(self: &SomeType) -> i16;

                    #[swift_bridge(Identifiable)]
                    fn id(self: &AnotherType) -> u32;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::SkipTest
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        // We add the class declarations to our assertions to ensure that our extensions come
        // directory after our class declarations.
        // This helps when an end-user that is looking at the generated code more easily see
        // which protocols a class implements.
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public class SomeTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension SomeTypeRef: Identifiable {
    public var id: Int16 {
        return self.some_function()
    }
}"#,
            r#"
public class AnotherTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension AnotherTypeRef: Identifiable {}"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn protocol_identifiable() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use the get attribute
mod get {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    #[swift_bridge(get(field))]
                    fn some_function(&self) -> u16;

                    #[swift_bridge(get(&field))]
                    fn some_function_ref(&self) -> i16;

                    #[swift_bridge(get(&mut field))]
                    fn some_function_ref_mut(&mut self) -> u8;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function(
                    this: *mut super::SomeType
                ) -> u16 {
                    (unsafe { &*this }).field
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function_ref(
                    this: *mut super::SomeType
                ) -> i16 {
                    &(unsafe { &*this }).field
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function_ref_mut(
                    this: *mut super::SomeType
                ) -> u8 {
                    &mut (unsafe { &mut *this }).field
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::SkipTest
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn get() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use the get attribute
mod get_with {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    #[swift_bridge(get_with(field = a::b::c))]
                    fn some_function(&self);

                    #[swift_bridge(get_with(&field = a::b::c))]
                    fn some_function_ref(&self);

                    #[swift_bridge(get_with(&mut field = a::b::c))]
                    fn some_function_ref_mut(&mut self);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function(
                    this: *mut super::SomeType
                ) {
                    super::a::b::c( (unsafe { &*this }).field )
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function_ref(
                    this: *mut super::SomeType
                ) {
                    super::a::b::c( & (unsafe { &*this }).field )
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_function_ref_mut(
                    this: *mut super::SomeType
                ) {
                    super::a::b::c( &mut (unsafe { &mut *this }).field )
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::SkipTest
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn get_with() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Tests that the swift_name function attribute generates the correct code
/// when using extern "Rust" (calling Rust code from Swift) and when using
/// extern "Swift" (calling Swift code from Rust).
mod function_attribute_swift_name_extern_rust {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(swift_name = "callRustFromSwift")]
                    fn call_rust_from_swift() -> String;
                }
                extern "Swift" {
                    #[swift_bridge(swift_name = "callSwiftFromRust")]
                    fn call_swift_from_rust() -> String;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$call_rust_from_swift"]
            pub extern "C" fn __swift_bridge__call_rust_from_swift() -> * mut swift_bridge::string::RustString {
                swift_bridge::string::RustString(super::call_rust_from_swift()).box_into_raw()
            }
            pub fn call_swift_from_rust() -> String {
                unsafe { Box::from_raw(unsafe {__swift_bridge__call_swift_from_rust () }).0 }
            }

            #[allow(improper_ctypes)]
            extern "C" {
                #[link_name = "__swift_bridge__$call_swift_from_rust"]
                fn __swift_bridge__call_swift_from_rust() -> * mut swift_bridge::string::RustString;
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public func callRustFromSwift() -> RustString {
    RustString(ptr: __swift_bridge__$call_rust_from_swift())
}
@_cdecl("__swift_bridge__$call_swift_from_rust")
func __swift_bridge__call_swift_from_rust () -> UnsafeMutableRawPointer {
    { let rustString = callSwiftFromRust().intoRustString(); rustString.isOwned = false; return rustString.ptr }()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::SkipTest
    }

    #[test]
    fn function_args_into_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
