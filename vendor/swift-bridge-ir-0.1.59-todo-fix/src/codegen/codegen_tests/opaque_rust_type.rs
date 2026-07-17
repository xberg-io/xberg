use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Test code generation for an extern "Rust" type.
mod extern_rust_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    type SomeType;
                }
            }
        }
    }

    /// Verify that we generate a function that frees the memory behind an opaque pointer to a Rust
    /// type.
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[export_name = "__swift_bridge__$SomeType$_free"]
            pub extern "C" fn __swift_bridge__SomeType__free (
                this: *mut super::SomeType
            ) {
                let this = unsafe { Box::from_raw(this) };
                drop(this);
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public class SomeType: SomeTypeRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$SomeType$_free(ptr)
        }
    }
}
public class SomeTypeRefMut: SomeTypeRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class SomeTypeRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
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

/// Test code generation for an extern "Rust" type that implements Hashable.
mod extern_rust_hashable_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Hashable)]
                    type HashableType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
        #[export_name = "__swift_bridge__$HashableType$_hash"]
        pub extern "C" fn __swift_bridge__HashableType__hash (
            this: *const super::HashableType,
        ) -> u64 {
            use std::hash::{Hash, Hasher};
            use std::collections::hash_map::DefaultHasher;
            let mut s = DefaultHasher::new();
            (unsafe {&*this}).hash(&mut s);
            s.finish()
        }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
extension HashableTypeRef: Hashable{
    public func hash(into hasher: inout Hasher){
        hasher.combine(__swift_bridge__$HashableType$_hash(self.ptr))
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
uint64_t __swift_bridge__$HashableType$_hash(void* self);
    "#,
            r#"
"#,
        ])
    }

    #[test]
    fn extern_rust_hashable_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for an extern "Rust" type that implements Equatable.
mod extern_rust_equatable_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Equatable)]
                    type EquatableType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
        #[export_name = "__swift_bridge__$EquatableType$_partial_eq"]
        pub extern "C" fn __swift_bridge__EquatableType__partial_eq (
            lhs: *const super::EquatableType,
            rhs: *const super::EquatableType
        ) -> bool {
            unsafe { &*lhs == &*rhs }
        }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
extension EquatableTypeRef: Equatable {
    public static func == (lhs: EquatableTypeRef, rhs: EquatableTypeRef) -> Bool {
        __swift_bridge__$EquatableType$_partial_eq(rhs.ptr, lhs.ptr)
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
bool __swift_bridge__$EquatableType$_partial_eq(void* lhs, void* rhs);
    "#,
            r#"
#include <stdint.h>
#include <stdbool.h>
"#,
        ])
    }

    #[test]
    fn extern_rust_equatable_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for an extern "Rust" type that implements Copy.
mod extern_rust_copy_type {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(32))]
                    type SomeType;
                }
            }
        }
    }

    // We use a somewhat hacky approach to asserting that the Copy size is correct at compile time.
    // In the future we'd prefer something like
    //  `assert_eq!(std::mem::size_of::<super::SomeType>(), 32usize);`
    // If compile time assertions are ever supported by Rust.
    // https://github.com/rust-lang/rfcs/issues/2790
    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsManyAndDoesNotContainMany {
            contains: vec![
                quote! {
                    const _: () = {
                        let _: [u8; std::mem::size_of::<super::SomeType>()] = [0; 32usize];
                        fn _assert_copy() {
                            swift_bridge::copy_support::assert_copy::<super::SomeType>();
                        }
                    };
                },
                quote! {
                    #[repr(C)]
                    #[doc(hidden)]
                    pub struct __swift_bridge__SomeType([u8; 32usize]);
                    impl __swift_bridge__SomeType {
                        #[inline(always)]
                        fn into_rust_repr(self) -> super::SomeType {
                            unsafe { std::mem::transmute(self) }
                        }

                        #[inline(always)]
                        fn from_rust_repr(repr: super::SomeType) -> Self {
                            unsafe { std::mem::transmute(repr) }
                        }
                    }

                    #[repr(C)]
                    #[doc(hidden)]
                    pub struct __swift_bridge__Option_SomeType {
                        is_some: bool,
                        val: std::mem::MaybeUninit<__swift_bridge__SomeType>
                    }
                },
            ],
            // Copy types don't need a function for freeing memory.
            does_not_contain: vec![quote! {
                __swift_bridge__SomeType__free
            }],
        }
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
public struct SomeType {
    fileprivate var bytes: __swift_bridge__$SomeType

    func intoFfiRepr() -> __swift_bridge__$SomeType {
        bytes
    }
}
"#,
            r#"
extension __swift_bridge__$SomeType {
    func intoSwiftRepr() -> SomeType {
        SomeType(bytes: self)
    }
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
typedef struct __swift_bridge__$SomeType { uint8_t bytes[32]; } __swift_bridge__$SomeType;
typedef struct __swift_bridge__$Option$SomeType { bool is_some; __swift_bridge__$SomeType val; } __swift_bridge__$Option$SomeType;
    "#,
            r#"
#include <stdint.h>
#include <stdbool.h>
"#,
        ])
    }

    #[test]
    fn extern_rust_copy_type() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we properly generate a method for a Copy opaque Rust type.
mod extern_rust_copy_type_method {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(32))]
                    type SomeType;

                    fn some_method(self);
                    fn some_method_ref(&self);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_method (
                    this: __swift_bridge__SomeType
                ) {
                    this.into_rust_repr().some_method()
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__SomeType_some_method_ref (
                    this: __swift_bridge__SomeType
                ) {
                    this.into_rust_repr().some_method_ref()
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
extension SomeType {
    public func some_method() {
        __swift_bridge__$SomeType$some_method(self.bytes)
    }
}
"#,
            r#"
extension SomeType {
    public func some_method_ref() {
        __swift_bridge__$SomeType$some_method_ref(self.bytes)
    }
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
void __swift_bridge__$SomeType$some_method(struct __swift_bridge__$SomeType this);
    "#,
            r#"
void __swift_bridge__$SomeType$some_method_ref(struct __swift_bridge__$SomeType this);
    "#,
        ])
    }

    #[test]
    fn extern_rust_copy_type_method() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we properly generate Equatable and Hashable extensions for a Copy opaque Rust type.
///
/// In May 2025 it was discovered that the Swift `Equatable` protocol was not being implemented for
/// Rust `Copy` types. This test case confirms tht we emit an `Equatable` protocol implementation.
/// Furthermore, in October 2025 it was discovered that the Swift `Hashable` protocol was not being implemented for
/// Rust `Copy` types. This test case confirms tht we emit an `Hashable` protocol implementation.
mod extern_rust_copy_type_equatable {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(Copy(16), Equatable, Hashable)]
                    type SomeType;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::SkipTest
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
extension SomeType: Equatable {
    public static func == (lhs: Self, rhs: Self) -> Bool {
        var lhs = lhs
        var rhs = rhs
        return withUnsafePointer(to: &lhs.bytes, {(lhs_p: UnsafePointer<__swift_bridge__$SomeType>) in
            return withUnsafePointer(to: &rhs.bytes, {(rhs_p: UnsafePointer<__swift_bridge__$SomeType>) in
                return __swift_bridge__$SomeType$_partial_eq(
                    UnsafeMutablePointer(mutating: lhs_p),
                    UnsafeMutablePointer(mutating: rhs_p)
                )
            })
        })
    }
}

extension SomeType: Hashable {
    public func hash(into hasher: inout Hasher){
        var this = self
        return withUnsafePointer(to: &this.bytes, {(ptr: UnsafePointer<__swift_bridge__$SomeType>) in
            hasher.combine(__swift_bridge__$SomeType$_hash(
                UnsafeMutablePointer(mutating: ptr)
            ))
        })
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdint.h>
#include <stdbool.h>
"#,
            r#"
bool __swift_bridge__$SomeType$_partial_eq(__swift_bridge__$SomeType* lhs, __swift_bridge__$SomeType* rhs);
"#,
            r#"
uint64_t __swift_bridge__$SomeType$_hash(__swift_bridge__$SomeType* self);
"#,
        ])
    }

    #[test]
    fn extern_rust_copy_type_equatable() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Test code generation for freestanding Swift function that takes an opaque Rust type argument.
mod extern_swift_freestanding_fn_with_owned_opaque_rust_type_arg {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                extern "Rust" {
                    type MyType;
                }

                extern "Swift" {
                    fn some_function (arg: MyType);
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub fn some_function (arg: super::MyType) {
                unsafe { __swift_bridge__some_function( Box::into_raw(Box::new({
                    let val: super::MyType = arg;
                    val
                })) as *mut super::MyType ) }
            }

            #[allow(improper_ctypes)]
            extern "C" {
                #[link_name = "__swift_bridge__$some_function"]
                fn __swift_bridge__some_function (arg: *mut super::MyType);
            }
        })
    }

    const EXPECTED_SWIFT: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
@_cdecl("__swift_bridge__$some_function")
func __swift_bridge__some_function (_ arg: UnsafeMutableRawPointer) {
    some_function(arg: MyType(ptr: arg))
}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ContainsAfterTrim(
        r#"
typedef struct MyType MyType;
"#,
    );

    #[test]
    fn extern_swift_freestanding_fn_with_owned_opaque_rust_type_arg() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Verify that we generated a Swift class with a failable init method.
mod extern_rust_class_with_failable_init {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn new() -> Option<Foo>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            # [export_name = "__swift_bridge__$Foo$new"]
            pub extern "C" fn __swift_bridge__Foo_new () -> * mut super :: Foo {
                if let Some (val) = super :: Foo :: new () {
                    Box :: into_raw (Box :: new (val))
                } else {
                    std :: ptr :: null_mut ()
                }
            }
        })
    }

    const EXPECTED_SWIFT: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
public class Foo: FooRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Foo$_free(ptr)
        }
    }
}
extension Foo {
    public convenience init?() {
        guard let val = __swift_bridge__$Foo$new() else { return nil }; self.init(ptr: val)
    }
}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ContainsAfterTrim(
        r#"
void* __swift_bridge__$Foo$new(void);
"#,
    );

    #[test]
    fn extern_rust_class_with_failable_init() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}

/// Verify that we can create a Swift class with a throwing init.
mod extern_rust_class_with_throwing_init {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            mod foo {
                enum SomeErrEnum {
                    Variant1,
                    Variant2,
                }
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn new() -> Result<Foo, SomeErrEnum>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            # [export_name = "__swift_bridge__$Foo$new"]
            pub extern "C" fn __swift_bridge__Foo_new() -> ResultFooAndSomeErrEnum{
                match super :: Foo :: new() {
                    Ok(ok) => ResultFooAndSomeErrEnum::Ok(Box::into_raw(Box::new({
                        let val: super::Foo = ok;
                        val
                    })) as *mut super::Foo),
                    Err(err) => ResultFooAndSomeErrEnum::Err(err.into_ffi_repr()),
                }
            }
        })
    }

    const EXPECTED_SWIFT: ExpectedSwiftCode = ExpectedSwiftCode::ContainsAfterTrim(
        r#"
public class Foo: FooRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Foo$_free(ptr)
        }
    }
}
extension Foo {
    public convenience init() throws {
        let val = __swift_bridge__$Foo$new(); if val.tag == __swift_bridge__$ResultFooAndSomeErrEnum$ResultOk { self.init(ptr: val.payload.ok) } else { throw val.payload.err.intoSwiftRepr() }
    }
}
"#,
    );

    const EXPECTED_C_HEADER: ExpectedCHeader = ExpectedCHeader::ContainsAfterTrim(
        r#"
typedef enum __swift_bridge__$ResultFooAndSomeErrEnum$Tag {__swift_bridge__$ResultFooAndSomeErrEnum$ResultOk, __swift_bridge__$ResultFooAndSomeErrEnum$ResultErr} __swift_bridge__$ResultFooAndSomeErrEnum$Tag;
union __swift_bridge__$ResultFooAndSomeErrEnum$Fields {void* ok; struct __swift_bridge__$SomeErrEnum err;};
typedef struct __swift_bridge__$ResultFooAndSomeErrEnum{__swift_bridge__$ResultFooAndSomeErrEnum$Tag tag; union __swift_bridge__$ResultFooAndSomeErrEnum$Fields payload;} __swift_bridge__$ResultFooAndSomeErrEnum;        
"#,
    );

    #[test]
    fn extern_rust_class_with_throwing_init() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: EXPECTED_SWIFT,
            expected_c_header: EXPECTED_C_HEADER,
        }
        .test();
    }
}
