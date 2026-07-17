use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate the functions that Swift calls uses inside of the corresponding class for an opaque
/// Rust type's Vectorizable implementation.
///
/// So inside of `extension MyRustType: Vectorizable {}` on the Swift side.
pub(in super::super) fn generate_vec_of_opaque_rust_type_functions(ty: &Ident) -> TokenStream {
    // examples:
    // "__swift_bridge__$Vec_MyRustType$new"
    // "__swift_bridge__$Vec_MyRustType$drop"
    let make_export_name = |fn_name| format!("__swift_bridge__$Vec_{}${}", ty, fn_name);
    let export_name_new = make_export_name("new");
    let export_name_drop = make_export_name("drop");
    let export_name_len = make_export_name("len");
    let export_name_get = make_export_name("get");
    let export_name_get_mut = make_export_name("get_mut");
    let export_name_push = make_export_name("push");
    let export_name_pop = make_export_name("pop");
    let export_name_as_ptr = make_export_name("as_ptr");

    quote! {
        const _: () = {
            #[doc(hidden)]
            #[export_name = #export_name_new]
            pub extern "C" fn _new() -> *mut Vec<super::#ty> {
                Box::into_raw(Box::new(Vec::new()))
            }

            #[doc(hidden)]
            #[export_name = #export_name_drop]
            pub extern "C" fn _drop(vec: *mut Vec<super::#ty>) {
                let vec = unsafe { Box::from_raw(vec) };
                drop(vec)
            }

            #[doc(hidden)]
            #[export_name = #export_name_len]
            pub extern "C" fn _len(vec: *const Vec<super::#ty>) -> usize {
                unsafe { &*vec }.len()
            }

            #[doc(hidden)]
            #[export_name = #export_name_get]
            pub extern "C" fn _get(vec: *const Vec<super::#ty>, index: usize) -> *const super::#ty {
                let vec = unsafe { & *vec };
                if let Some(val) = vec.get(index) {
                    val as *const super::#ty
                } else {
                    std::ptr::null()
                }
            }

            #[doc(hidden)]
            #[export_name = #export_name_get_mut]
            pub extern "C" fn _get_mut(vec: *mut Vec<super::#ty>, index: usize) -> *mut super::#ty {
                let vec = unsafe { &mut *vec };
                if let Some(val) = vec.get_mut(index) {
                    val as *mut super::#ty
                } else {
                    std::ptr::null::<super::#ty>() as *mut super::#ty
                }
            }

            #[doc(hidden)]
            #[export_name = #export_name_push]
            pub extern "C" fn _push(vec: *mut Vec<super::#ty>, val: *mut super::#ty) {
                unsafe { &mut *vec }.push( unsafe { *Box::from_raw(val) } )
            }

            #[doc(hidden)]
            #[export_name = #export_name_pop]
            pub extern "C" fn _pop(vec: *mut Vec<super::#ty>) -> *mut super::#ty {
                let vec = unsafe { &mut *vec };
                if let Some(val) = vec.pop() {
                    Box::into_raw(Box::new(val))
                } else {
                    std::ptr::null::<super::#ty>() as *mut super::#ty
                }
            }

            #[doc(hidden)]
            #[export_name = #export_name_as_ptr]
            pub extern "C" fn _as_ptr(vec: *const Vec<super::#ty>) -> *const super::#ty {
                unsafe { & *vec }.as_ptr()
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_tokens_eq;
    use proc_macro2::Span;

    /// Verify that we can generate the functions for an opaque Rust type that get exposed to Swift
    /// in order to power the `extension MyRustType: Vectorizable { }` implementation on the Swift
    /// side.
    #[test]
    fn generates_vectorizable_impl_for_opaque_rust_type() {
        let expected = quote! {
            const _: () = {
                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$new"]
                pub extern "C" fn _new() -> *mut Vec<super::ARustType> {
                    Box::into_raw(Box::new(Vec::new()))
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$drop"]
                pub extern "C" fn _drop(vec: *mut Vec<super::ARustType>) {
                    let vec = unsafe { Box::from_raw(vec) };
                    drop(vec)
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$len"]
                pub extern "C" fn _len(vec: *const Vec<super::ARustType>) -> usize {
                    unsafe { &*vec }.len()
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$get"]
                pub extern "C" fn _get(vec: *const Vec<super::ARustType>, index: usize) -> *const super::ARustType {
                    let vec = unsafe { & *vec };
                    if let Some(val) = vec.get(index) {
                        val as *const super::ARustType
                    } else {
                        std::ptr::null()
                    }
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$get_mut"]
                pub extern "C" fn _get_mut(vec: *mut Vec<super::ARustType>, index: usize) -> *mut super::ARustType {
                    let vec = unsafe { &mut *vec };
                    if let Some(val) = vec.get_mut(index) {
                        val as *mut super::ARustType
                    } else {
                        std::ptr::null::<super::ARustType>() as *mut super::ARustType
                    }
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$push"]
                pub extern "C" fn _push(vec: *mut Vec<super::ARustType>, val: *mut super::ARustType) {
                    unsafe { &mut *vec }.push(unsafe { * Box::from_raw(val) })
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$pop"]
                pub extern "C" fn _pop(vec: *mut Vec<super::ARustType>) -> *mut super::ARustType {
                    let vec = unsafe { &mut *vec };
                    if let Some(val) = vec.pop() {
                        Box::into_raw(Box::new(val))
                    } else {
                        std::ptr::null::<super::ARustType>() as *mut super::ARustType
                    }
                }

                #[doc(hidden)]
                #[export_name = "__swift_bridge__$Vec_ARustType$as_ptr"]
                pub extern "C" fn _as_ptr(vec: *const Vec<super::ARustType>) -> *const super::ARustType {
                    unsafe { & *vec }.as_ptr()
                }
            };
        };

        assert_tokens_eq(
            &generate_vec_of_opaque_rust_type_functions(&Ident::new(
                "ARustType",
                Span::call_site(),
            )),
            &expected,
        );
    }
}
