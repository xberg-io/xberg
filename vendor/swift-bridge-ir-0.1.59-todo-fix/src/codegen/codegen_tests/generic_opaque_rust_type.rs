use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we can declare a generic type.
mod declare_generic {
    use super::*;

    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;
                }
            }
        }
    }

    // We don't declare any extern functions (such as the function to free memory) for the
    // generic declaration. We only declare functions for the concrete types.
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::DoesNotContain(quote! {
            fn
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public class SomeType<A>: SomeTypeRefMut<A> {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            (self as! SwiftBridgeGenericFreer).rust_free()
        }
    }
}
public class SomeTypeRefMut<A>: SomeTypeRef<A> {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class SomeTypeRef<A> {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
"#,
        )
    }

    // Nothing to bridge for a declared generic. We only need to bridge its monomorphizations.
    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::DoesNotContainAfterTrim("SomeType")
    }

    #[test]
    fn declare_generic() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can declare a generic with concrete types.
mod monomorphized_generic_opaque_rust_type {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$SomeType$u32$_free"]
            pub extern "C" fn __swift_bridge__SomeType_u32__free (
                this: *mut super::SomeType<u32>
            ) {
                let this = unsafe { Box::from_raw(this) };
                drop(this);
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
extension SomeType: SwiftBridgeGenericFreer
where A == UInt32 {
    public func rust_free() {
        __swift_bridge__$SomeType$u32$_free(ptr)
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$SomeType$u32$_free(void* self);
    "#,
        )
    }

    #[test]
    fn monomorphized_generic_opaque_rust_type() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a generic opaque Rust type as a function argument.
mod generic_opaque_rust_type_arg {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                    fn some_function(arg: SomeType<u32>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: *mut super::SomeType<u32>
            ) {
                super::some_function(unsafe { * Box::from_raw(arg) })
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeType<UInt32>) {
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
    fn generic_opaque_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can return a generic opaque Rust type from a function.
mod generic_opaque_rust_type_return {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                    fn some_function() -> SomeType<u32>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function () -> *mut super::SomeType<u32> {
                Box::into_raw(Box::new({
                    let val: super::SomeType<u32> = super::some_function();
                    val
                })) as *mut super::SomeType<u32>
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> SomeType<UInt32> {
    SomeType(ptr: __swift_bridge__$some_function())
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void* __swift_bridge__$some_function(void);
    "#,
        )
    }

    #[test]
    fn generic_opaque_rust_type_return() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can declare a generic opaque Rust type.
mod generic_opaque_rust_type_copy {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(6))]
                    type SomeType<u32, u16>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsManyAndDoesNotContainMany {
            contains: vec![
                quote! {
                    const _ : () = {
                        let _ : [u8 ; std :: mem :: size_of :: < super :: SomeType<u32, u16> > ()] = [0 ; 6usize] ;
                        fn _assert_copy () {
                            swift_bridge::copy_support::assert_copy::<super::SomeType<u32,u16>>();
                        }
                    }
                },
                quote! {
                    #[repr(C)]
                    #[doc(hidden)]
                    pub struct __swift_bridge__SomeType_u32_u16([u8; 6usize]);
                    impl __swift_bridge__SomeType_u32_u16 {
                        #[inline(always)]
                        fn into_rust_repr(self) -> super::SomeType<u32,u16> {
                            unsafe { std::mem::transmute(self) }
                        }

                        #[inline(always)]
                        fn from_rust_repr(repr: super::SomeType<u32,u16>) -> Self {
                            unsafe { std::mem::transmute(repr) }
                        }
                    }
                },
            ],
            // Copy types don't need to be freed.
            does_not_contain: vec![quote! {
                free
            }],
        }
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public struct SomeType<A, B> {
    fileprivate var bytes: SwiftBridgeGenericCopyTypeFfiRepr
}"#,
            r#"
extension SomeType
where A == UInt32, B == UInt16 {
    func intoFfiRepr() -> __swift_bridge__$SomeType$u32$u16 {
        self.bytes as! __swift_bridge__$SomeType$u32$u16
    }
}
extension __swift_bridge__$SomeType$u32$u16 {
    func intoSwiftRepr() -> SomeType<UInt32, UInt16> {
        SomeType(bytes: self)
    }
}
extension __swift_bridge__$SomeType$u32$u16: SwiftBridgeGenericCopyTypeFfiRepr {}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
typedef struct __swift_bridge__$SomeType$u32$u16 { uint8_t bytes[6]; } __swift_bridge__$SomeType$u32$u16;
        "#,
        )
    }

    #[test]
    fn generic_opaque_rust_type_copy() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use a generic opaque Rust Copy type as an argument.
mod generic_opaque_rust_type_copy_arg {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(6))]
                    type SomeType<u32, u16>;

                    fn some_function(arg: SomeType<u32, u16>);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function(
                arg: __swift_bridge__SomeType_u32_u16
            ) {
                super::some_function(arg.into_rust_repr())
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeType<UInt32, UInt16>) {
    __swift_bridge__$some_function(arg.intoFfiRepr())
}
        "#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
void __swift_bridge__$some_function(struct __swift_bridge__$SomeType$u32$u16 arg);
        "#,
        )
    }

    #[test]
    fn generic_opaque_rust_type_copy_arg() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can return a generic opaque Rust Copy type.
mod generic_opaque_rust_type_copy_return {
    use super::*;

    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(6))]
                    type SomeType<u32, u16>;

                    fn some_function() -> SomeType<u32, u16>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function(
            ) -> __swift_bridge__SomeType_u32_u16 {
                __swift_bridge__SomeType_u32_u16::from_rust_repr(super::some_function())
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function() -> SomeType<UInt32, UInt16> {
    SomeType(bytes: __swift_bridge__$some_function())
}
        "#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
struct __swift_bridge__$SomeType$u32$u16 __swift_bridge__$some_function(void);
        "#,
        )
    }

    #[test]
    fn generic_opaque_rust_type_copy_return() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can declare a generic opaque Rust type that has an inner opaque Rust concrete
/// type.
mod generic_opaque_rust_type_inner_opaque_ty {
    use super::*;
    fn bridge_module() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;
                    type SomeType<AnotherType>;
                    type AnotherType;
                }
            }
        }
    }

    // No need to free memory for Copy types
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$SomeType$AnotherType$_free"]
            pub extern "C" fn __swift_bridge__SomeType_AnotherType__free (
                this: *mut super::SomeType<super::AnotherType>
            ) {
                let this = unsafe { Box::from_raw(this) };
                drop(this);
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
    fn generic_opaque_rust_type_inner_opaque_ty() {
        CodegenTest {
            bridge_module: bridge_module().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
