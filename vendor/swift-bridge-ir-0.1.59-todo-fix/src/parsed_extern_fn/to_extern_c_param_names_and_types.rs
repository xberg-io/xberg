use crate::bridged_type::{pat_type_pat_is_self, BridgeableType, BridgedType};
use crate::parse::{HostLang, TypeDeclaration, TypeDeclarations};
use crate::parsed_extern_fn::ParsedExternFn;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{FnArg, Path, Type};

impl ParsedExternFn {
    pub fn to_extern_c_param_names_and_types(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
        custom_type_definitions: &mut HashMap<String, TokenStream>,
    ) -> TokenStream {
        let mut params = vec![];
        let inputs = &self.func.sig.inputs;
        for arg in inputs {
            match arg {
                FnArg::Receiver(_receiver) => match self.host_lang {
                    HostLang::Rust => {
                        let this = match self.associated_type.as_ref().unwrap() {
                            TypeDeclaration::Opaque(opaque) => {
                                let opaque_ty_ffi_repr = opaque.ffi_repr_type_tokens();
                                quote! { this: #opaque_ty_ffi_repr }
                            }
                            TypeDeclaration::Shared(_) => {
                                todo!("Methods on shared types are not yet supported.")
                            }
                        };

                        params.push(this);
                    }
                    HostLang::Swift => {
                        let this = quote! { this: #swift_bridge_path::PointerToSwiftType };
                        params.push(this);
                    }
                },
                FnArg::Typed(pat_ty) => {
                    let pat_ty_is_self = pat_type_pat_is_self(pat_ty);

                    if !pat_ty_is_self {
                        if let Some(built_in) = BridgedType::new_with_type(&pat_ty.ty, types) {
                            if let Some(tokens) =
                                built_in.generate_custom_rust_ffi_types(swift_bridge_path, types)
                            {
                                for token in tokens.into_iter() {
                                    custom_type_definitions.insert(token.to_string(), token);
                                }
                            }
                            if built_in.can_be_encoded_with_zero_bytes() {
                                continue;
                            }

                            let pat = &pat_ty.pat;
                            let ty = built_in.to_ffi_compatible_rust_type(swift_bridge_path, types);

                            params.push(quote! { #pat: #ty});

                            continue;
                        } else {
                            todo!("Push to ParsedErrors")
                        }
                    };

                    if self.host_lang.is_swift() {
                        let this = quote! { this: #swift_bridge_path::PointerToSwiftType };
                        params.push(this);

                        continue;
                    }

                    let this = Ident::new("this", pat_ty.span());
                    let arg_name = quote! {
                        #this
                    };

                    let mut bridged_type = &pat_ty.ty;

                    // `&Foo` becomes `Foo`
                    // `&mut Foo` becomes `Foo`
                    if let Type::Reference(ty_ref) = pat_ty.ty.deref() {
                        bridged_type = &ty_ref.elem;
                    };

                    let arg_ty = match types
                        .get(&bridged_type.to_token_stream().to_string())
                        .unwrap()
                    {
                        TypeDeclaration::Shared(_) => {
                            todo!("Support methods on shared types.")
                        }
                        TypeDeclaration::Opaque(opaque) => opaque.ffi_repr_type_tokens(),
                    };

                    params.push(quote! {
                         #arg_name: #arg_ty
                    });
                }
            };
        }

        quote! {
            #(#params),*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_tokens_contain, assert_tokens_eq, parse_ok};

    /// Verify that we rename `self` parameters to `this`
    #[test]
    fn renames_self_to_this_in_params() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make1 (self);
                    fn make2 (&self);
                    fn make3 (&mut self);
                    fn make4 (self: Foo);
                    fn make5 (self: &Foo);
                    fn make6 (self: &mut Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        let methods = &module.functions;
        assert_eq!(methods.len(), 6);

        for method in methods {
            assert_tokens_contain(
                &method.to_extern_c_param_names_and_types(
                    &module.swift_bridge_path,
                    &module.types,
                    &mut HashMap::new(),
                ),
                &quote! { this },
            );
        }
    }

    /// Verify that a String parameter gets turned into a *mut RustString
    #[test]
    fn converts_string_param_to_ruststring_pointer() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn foo (s: String);
                }
            }
        };
        let expected = quote! { s: *mut swift_bridge::string::RustString };
        assert_params_eq(tokens, &expected);
    }

    fn assert_params_eq(tokens: TokenStream, expected_params: &TokenStream) {
        let module = parse_ok(tokens);
        let funcs = &module.functions;

        assert_tokens_eq(
            &funcs[0].to_extern_c_param_names_and_types(
                &module.swift_bridge_path,
                &module.types,
                &mut HashMap::new(),
            ),
            expected_params,
        );
    }
}
