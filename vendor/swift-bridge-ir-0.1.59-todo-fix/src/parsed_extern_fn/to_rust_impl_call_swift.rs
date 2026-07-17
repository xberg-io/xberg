use crate::bridged_type::{pat_type_pat_is_self, BridgeableType, BridgedType};
use crate::parse::{SharedTypeDeclaration, TypeDeclaration, TypeDeclarations};
use crate::parsed_extern_fn::ParsedExternFn;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{FnArg, PatType, Path, ReturnType, Type, TypeReference};

/// Generates the
///
/// `pub fn new () -> Foo { ... }`
///
/// in the following:
///
/// ```
/// # use std::ffi::c_void;
/// struct Foo(*mut c_void);
///
/// impl Foo {
///     // We're generating this function.
///     pub fn new () -> Foo {
///         Foo(unsafe{ __swift_bridge__Foo_new() })
///     }
/// }
/// extern "C" {
///     #[link_name = "__swift_bridge__$Foo$new"]
///     fn __swift_bridge__Foo_new() -> *mut c_void;
/// }
/// ```
impl ParsedExternFn {
    pub fn to_rust_fn_that_calls_a_swift_extern(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let sig = &self.func.sig;
        let fn_name = &sig.ident;
        let is_async = sig.asyncness.is_some();

        let ret = &sig.output;

        let ret = match &ret {
            ReturnType::Default => {
                quote! {#ret}
            }
            ReturnType::Type(arrow, _ty) => {
                if let Some(built_in) = BridgedType::new_with_return_type(&sig.output, types) {
                    let ty = built_in.maybe_convert_pointer_to_super_pointer(types);
                    let return_ty_span = sig.output.span();

                    quote_spanned! {return_ty_span=> #arrow #ty}
                } else {
                    quote! { #ret }
                }
            }
        };

        let params = self.params_with_explicit_self_types_removed(types);
        let call_args = self.to_call_rust_args(swift_bridge_path, types);
        let linked_fn_name = self.extern_swift_linked_fn_new();

        if is_async {
            self.generate_async_rust_fn_that_calls_swift(
                fn_name,
                &ret,
                &params,
                &call_args,
                &linked_fn_name,
                swift_bridge_path,
                types,
            )
        } else {
            // Check if this is a Result type
            let return_ty = BridgedType::new_with_return_type(&sig.output, types);
            let maybe_result = return_ty.as_ref().and_then(|ty| ty.as_result());

            if let Some(result) = maybe_result {
                // Result type: call Swift and convert FFI result to Rust Result
                let ffi_result_expr = quote! {
                    unsafe { #linked_fn_name(#call_args) }
                };
                let convert_result = result.convert_ffi_value_to_rust_value(
                    &ffi_result_expr,
                    sig.output.span(),
                    swift_bridge_path,
                    types,
                );

                quote! {
                    pub fn #fn_name(#params) #ret {
                        #convert_result
                    }
                }
            } else {
                let mut inner = quote! {
                    unsafe { #linked_fn_name(#call_args) }
                };

                if let Some(built_in) = BridgedType::new_with_return_type(&sig.output, types) {
                    inner = built_in.convert_ffi_expression_to_rust_type(
                        &inner,
                        sig.output.span(),
                        swift_bridge_path,
                        types,
                    );
                } else {
                    todo!("Push to ParsedErrors")
                }

                quote! {
                    pub fn #fn_name(#params) #ret {
                        #inner
                    }
                }
            }
        }
    }

    /// Generate an async Rust function that calls an async Swift function.
    ///
    /// The pattern is:
    /// 1. Create a oneshot channel
    /// 2. Define callback(s) that send result through the channel
    /// 3. Call Swift with callback wrapper and callback fn pointer(s)
    /// 4. Await the receiver and convert result to Rust type
    fn generate_async_rust_fn_that_calls_swift(
        &self,
        fn_name: &Ident,
        ret: &TokenStream,
        params: &TokenStream,
        call_args: &TokenStream,
        linked_fn_name: &Ident,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let sig = &self.func.sig;
        let return_ty = BridgedType::new_with_return_type(&sig.output, types);
        let maybe_result = return_ty.as_ref().and_then(|ty| ty.as_result());

        // Generate comma before call_args if there are any
        let maybe_comma_call_args = if call_args.is_empty() {
            quote! {}
        } else {
            quote! { , #call_args }
        };

        if let Some(result) = maybe_result {
            // Result type: use two callbacks (on_success and on_error)
            let err_ffi_ty = result
                .err_ty
                .to_ffi_compatible_rust_type(swift_bridge_path, types);

            let err_convert = result.err_ty.convert_ffi_expression_to_rust_type(
                &quote! { err_val },
                sig.output.span(),
                swift_bridge_path,
                types,
            );

            let rust_ok_ty = result.ok_ty.to_rust_type_path(types);
            let rust_err_ty = result.err_ty.to_rust_type_path(types);

            // Handle Result<(), E> where ok_ty can be encoded with zero bytes
            let (on_success_params, on_success_body) =
                if result.ok_ty.can_be_encoded_with_zero_bytes() {
                    // Result<(), E>: on_success callback has no ok_val parameter
                    (
                        quote! { callback_wrapper: *mut std::ffi::c_void },
                        quote! {
                            unsafe {
                                #swift_bridge_path::async_swift_support::complete_swift_async(
                                    callback_wrapper,
                                    std::result::Result::<#rust_ok_ty, #rust_err_ty>::Ok(())
                                );
                            }
                        },
                    )
                } else {
                    // Result<T, E> where T is not (): on_success has ok_val parameter
                    let ok_ffi_ty = result
                        .ok_ty
                        .to_ffi_compatible_rust_type(swift_bridge_path, types);
                    let ok_convert = result.ok_ty.convert_ffi_expression_to_rust_type(
                        &quote! { ok_val },
                        sig.output.span(),
                        swift_bridge_path,
                        types,
                    );
                    (
                        quote! {
                            callback_wrapper: *mut std::ffi::c_void,
                            ok_val: #ok_ffi_ty
                        },
                        quote! {
                            let ok_val: #rust_ok_ty = #ok_convert;
                            unsafe {
                                #swift_bridge_path::async_swift_support::complete_swift_async(
                                    callback_wrapper,
                                    std::result::Result::<#rust_ok_ty, #rust_err_ty>::Ok(ok_val)
                                );
                            }
                        },
                    )
                };

            quote! {
                pub async fn #fn_name(#params) #ret {
                    let (future, callback_wrapper) = #swift_bridge_path::async_swift_support::create_swift_async_call::<
                        std::result::Result<#rust_ok_ty, #rust_err_ty>
                    >();

                    extern "C" fn on_success(#on_success_params) {
                        #on_success_body
                    }

                    extern "C" fn on_error(
                        callback_wrapper: *mut std::ffi::c_void,
                        err_val: #err_ffi_ty
                    ) {
                        let err_val: #rust_err_ty = #err_convert;
                        unsafe {
                            #swift_bridge_path::async_swift_support::complete_swift_async(
                                callback_wrapper,
                                std::result::Result::<#rust_ok_ty, #rust_err_ty>::Err(err_val)
                            );
                        }
                    }

                    unsafe {
                        #linked_fn_name(
                            callback_wrapper,
                            on_success,
                            on_error
                            #maybe_comma_call_args
                        )
                    };

                    future.await
                }
            }
        } else {
            // Non-Result type: use single callback
            let (callback_params, ffi_ty_for_channel, convert_and_complete, final_result) =
                if let Some(built_in) = return_ty.as_ref() {
                    if built_in.can_be_encoded_with_zero_bytes() {
                        // () return type
                        (
                            quote! { callback_wrapper: *mut std::ffi::c_void },
                            quote! { () },
                            quote! {
                                unsafe {
                                    #swift_bridge_path::async_swift_support::complete_swift_async(
                                        callback_wrapper,
                                        ()
                                    );
                                }
                            },
                            quote! { future.await },
                        )
                    } else {
                        let ffi_ty = built_in.to_ffi_compatible_rust_type(swift_bridge_path, types);
                        let rust_ty = built_in.to_rust_type_path(types);
                        let convert = built_in.convert_ffi_expression_to_rust_type(
                            &quote! { result_val },
                            sig.output.span(),
                            swift_bridge_path,
                            types,
                        );

                        (
                            quote! {
                                callback_wrapper: *mut std::ffi::c_void,
                                result_val: #ffi_ty
                            },
                            rust_ty,
                            quote! {
                                let result_val = #convert;
                                unsafe {
                                    #swift_bridge_path::async_swift_support::complete_swift_async(
                                        callback_wrapper,
                                        result_val
                                    );
                                }
                            },
                            quote! { future.await },
                        )
                    }
                } else {
                    // No return type
                    (
                        quote! { callback_wrapper: *mut std::ffi::c_void },
                        quote! { () },
                        quote! {
                            unsafe {
                                #swift_bridge_path::async_swift_support::complete_swift_async(
                                    callback_wrapper,
                                    ()
                                );
                            }
                        },
                        quote! { future.await },
                    )
                };

            quote! {
                pub async fn #fn_name(#params) #ret {
                    let (future, callback_wrapper) = #swift_bridge_path::async_swift_support::create_swift_async_call::<
                        #ffi_ty_for_channel
                    >();

                    extern "C" fn callback(#callback_params) {
                        #convert_and_complete
                    }

                    unsafe {
                        #linked_fn_name(
                            callback_wrapper,
                            callback
                            #maybe_comma_call_args
                        )
                    };

                    #final_result
                }
            }
        }
    }

    /// #\[export_name = "__swift_bridge__$SomeType$some_method$param1"]
    /// pub extern "C" fn SomeType_some_method_param1(boxed_fn: *mut dyn FnOnce(u8) -> (), arg0: u8) {
    ///     unsafe { Box::from_raw(boxed_fn) }(arg0)
    /// }
    /// #\[export_name = "__swift_bridge__$SomeType$some_method$_free$param1"]
    /// pub extern "C" fn free_SomeType_some_method_param1(boxed_fn: *mut dyn FnOnce(u8) -> ()) {
    ///     unsafe { Box::from_raw(boxed_fn) }
    /// }
    pub fn callbacks_support(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let sig = &self.func.sig;
        let fn_name = &sig.ident;

        let mut boxed_fn_support = vec![];
        for (idx, boxed_fn) in self.args_filtered_to_boxed_fns(types) {
            if boxed_fn.does_not_have_params_or_return() {
                continue;
            }

            let maybe_associated_ty = self
                .associated_type
                .as_ref()
                .and_then(|t| t.as_opaque())
                .map(|o| format!("{}_", o.ty.to_string()))
                .unwrap_or("".to_string());

            let boxed_fn_name = format!("{}{}_param{idx}", maybe_associated_ty, fn_name);
            let boxed_fn_name = Ident::new(&boxed_fn_name, fn_name.span());

            let boxed_fn_ffi_repr = boxed_fn.to_ffi_compatible_rust_type(types);

            let free_boxed_fn_name = format!("free_{}{}_param{idx}", maybe_associated_ty, fn_name);
            let free_boxed_fn_name = Ident::new(&free_boxed_fn_name, fn_name.span());

            let params = boxed_fn.params_to_ffi_compatible_rust_types(swift_bridge_path, types);
            let call_args = boxed_fn.to_rust_call_args(swift_bridge_path, types);

            let call_boxed_fn_link_name = self.call_boxed_fn_link_name(idx);
            let free_boxed_fn_link_name = self.free_boxed_fn_link_name(idx);

            let maybe_params = if boxed_fn.params.len() > 0 {
                quote! {
                    , #(#params),*
                }
            } else {
                quote! {}
            };

            let maybe_ret = if boxed_fn.ret.is_null() {
                quote! {}
            } else {
                let ret = boxed_fn
                    .ret
                    .to_ffi_compatible_rust_type(swift_bridge_path, types);
                quote! { -> #ret }
            };

            let arg_name = self.arg_name_tokens_at_idx(idx).unwrap();
            let arg_name = Ident::new(&format!("{}_{}", fn_name, arg_name), arg_name.span());

            let call_boxed_fn = quote! {
                unsafe { Box::from_raw(#arg_name)(#(#call_args),*) }
            };
            let call_boxed_fn = boxed_fn.ret.convert_rust_expression_to_ffi_type(
                &call_boxed_fn,
                swift_bridge_path,
                types,
                // TODO: Add a UI test and then add a better span
                Span::call_site(),
            );
            let call_boxed_fn = quote! {
                #[export_name = #call_boxed_fn_link_name]
                pub extern "C" fn #boxed_fn_name(#arg_name: #boxed_fn_ffi_repr #maybe_params) #maybe_ret {
                    #call_boxed_fn
                }
            };

            let free_boxed_fn = quote! {
                #[export_name = #free_boxed_fn_link_name]
                pub extern "C" fn #free_boxed_fn_name(#arg_name: #boxed_fn_ffi_repr) {
                    let _ = unsafe { Box::from_raw(#arg_name) };
                }
            };

            boxed_fn_support.push(call_boxed_fn);
            boxed_fn_support.push(free_boxed_fn);
        }

        quote! {
            #(#boxed_fn_support)*
        }
    }

    // All of the params but with explicit types removed from `self`.
    //
    // `self: Foo` becomes `self`,
    // `self: &Foo` -> `&self`,
    // `self: &mut Foo` -> `&mut self`
    fn params_with_explicit_self_types_removed(&self, types: &TypeDeclarations) -> TokenStream {
        let params = self
            .sig
            .inputs
            .iter()
            .map(|fn_arg| {
                if let Some(reference) = pat_ty_type_reference_if_arg_self(fn_arg) {
                    let ref_token = reference.and_token;
                    let maybe_mut = reference.mutability;

                    quote! {
                        #ref_token #maybe_mut self
                    }
                } else {
                    match fn_arg {
                        FnArg::Receiver(_) => {
                            quote! { #fn_arg }
                        }
                        FnArg::Typed(pat_ty) => {
                            let pat = &pat_ty.pat;

                            if let Some(built_in) = BridgedType::new_with_fn_arg(fn_arg, types) {
                                let ty = built_in.maybe_convert_pointer_to_super_pointer(types);

                                let maybe_unused = if built_in.can_be_encoded_with_zero_bytes() {
                                    "_"
                                } else {
                                    ""
                                };
                                let pat = Ident::new(
                                    &format!(
                                        "{}{}",
                                        maybe_unused,
                                        pat.to_token_stream().to_string()
                                    ),
                                    pat.span(),
                                );

                                quote! { #pat: #ty}
                            } else {
                                match types.get_with_pat_type(pat_ty).unwrap() {
                                    TypeDeclaration::Shared(SharedTypeDeclaration::Struct(_)) => {
                                        // quote! { #pat: #fn_arg}
                                        todo!("Add a test that hits this code path")
                                    }
                                    TypeDeclaration::Shared(SharedTypeDeclaration::Enum(_)) => {
                                        todo!("Add a test that hits this code path")
                                    }
                                    TypeDeclaration::Opaque(opaque) => {
                                        let ty = &opaque.ty;
                                        if opaque.host_lang.is_rust() {
                                            quote! { #pat: super:: #ty}
                                        } else {
                                            quote! { #pat: #ty }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        quote! {
            #(#params),*
        }
    }
}

// self: &Foo would return &Foo
// _foo: &Foo would not return &Foo
fn pat_ty_type_reference_if_arg_self(fn_arg: &FnArg) -> Option<&TypeReference> {
    match fn_arg {
        FnArg::Typed(pat_ty) if pat_type_pat_is_self(pat_ty) => {
            if let Some(reference) = pat_ty_type_reference(pat_ty) {
                Some(reference)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn pat_ty_type_reference(pat_ty: &PatType) -> Option<&TypeReference> {
    match pat_ty.ty.deref() {
        Type::Reference(reference) => Some(reference),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_tokens_eq, parse_ok};

    /// Verify that we generate a Rust associated function that calls a Swift static class method.
    #[test]
    fn static_class_method() {
        let start = quote! {
            mod foo {
                extern "Swift" {
                    type Foo;

                    #[swift_bridge(associated_to = Foo)]
                    fn message (val: u8);
                }
            }
        };
        let expected = quote! {
            pub fn message (val: u8) {
                unsafe { __swift_bridge__Foo_message(val) }
            }
        };

        assert_impl_fn_tokens_eq(start, &expected);
    }

    /// Verify that we generate a Rust associated function for a Swift class init function.
    #[test]
    fn class_initializer() {
        let start = quote! {
            mod foo {
                extern "Swift" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn new () -> Foo;
                }
            }
        };
        let expected = quote! {
            pub fn new () -> Foo {
                unsafe { __swift_bridge__Foo_new() }
            }
        };

        assert_impl_fn_tokens_eq(start, &expected);
    }

    /// Verify that we convert FfiSlice<T> -> &[T]
    #[test]
    fn converts_slice() {
        let start = quote! {
            mod foo {
                extern "Swift" {
                    type Foo;

                    fn as_slice (&self) -> &[u8];
                }
            }
        };
        let expected = quote! {
            pub fn as_slice (&self) -> &[u8] {
                unsafe { __swift_bridge__Foo_as_slice(swift_bridge::PointerToSwiftType(self.0)) }.as_slice()
            }
        };

        assert_impl_fn_tokens_eq(start, &expected);
    }

    /// Verify that we can call a Swift method with a &str and have it return an &str.
    /// This verifies that our type conversions are being inserted for Swift methods.
    #[test]
    fn call_with_str_arg_and_return_str() {
        let start = quote! {
            mod foo {
                extern "Swift" {
                    type Foo;

                    fn some_function (&self, arg: &str) -> &str;
                }
            }
        };
        let expected = quote! {
            pub fn some_function (&self, arg: &str) -> &str {
                unsafe {
                    __swift_bridge__Foo_some_function(
                        swift_bridge::PointerToSwiftType(self.0),
                        swift_bridge::string::RustStr::from_str(arg)
                    )
                }.to_str()
            }
        };

        assert_impl_fn_tokens_eq(start, &expected);
    }

    // impl Foo {
    //    // We're testing to make sure that we generated this function or method properly.
    //    fn some_function() {
    //        ...
    //    }
    // }
    fn assert_impl_fn_tokens_eq(module: TokenStream, expected_impl_fn_tokens: &TokenStream) {
        let module = parse_ok(module);
        let tokens = module.functions[0]
            .to_rust_fn_that_calls_a_swift_extern(&module.swift_bridge_path, &module.types);
        assert_tokens_eq(&tokens, &expected_impl_fn_tokens);
    }
}
