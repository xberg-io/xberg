//! More tests can be found in
//! crates/swift-bridge-ir/src/codegen/codegen_tests/shared_struct_codegen_tests.rs

use crate::bridged_type::{BridgedType, SharedStruct};
use crate::{SwiftBridgeModule, SWIFT_BRIDGE_PREFIX};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

impl SwiftBridgeModule {
    /// Generate the tokens for a shared struct.
    pub(super) fn generate_shared_struct_tokens(
        &self,
        shared_struct: &SharedStruct,
    ) -> Option<TokenStream> {
        if shared_struct.already_declared {
            return None;
        }

        let struct_name = &shared_struct.name;
        let swift_bridge_path = &self.swift_bridge_path;

        let struct_ffi_name = format!("{}{}", SWIFT_BRIDGE_PREFIX, struct_name);
        let struct_ffi_name = Ident::new(&struct_ffi_name, struct_name.span());

        let option_struct = shared_struct.ffi_option_name_tokens();

        let struct_fields: Vec<TokenStream> = shared_struct
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let maybe_name_and_colon = norm_field.maybe_name_and_colon();
                let ty = &norm_field.ty;

                quote! {
                    pub #maybe_name_and_colon #ty
                }
            })
            .collect();
        let struct_fields = shared_struct.fields.wrap_declaration_fields(&struct_fields);

        let repr_c_struct_fields: Vec<TokenStream> = shared_struct
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let maybe_name_and_colon = norm_field.maybe_name_and_colon();
                let ty = &norm_field.ty;

                let ty = BridgedType::new_with_type(ty, &self.types).unwrap();
                let ty = ty.to_ffi_compatible_rust_type(&self.swift_bridge_path, &self.types);

                quote! {
                    #maybe_name_and_colon #ty
                }
            })
            .collect();
        let repr_c_struct_fields = shared_struct
            .fields
            .wrap_declaration_fields(&repr_c_struct_fields);

        let into_ffi_repr_impl = shared_struct.generate_into_ffi_repr_method(
            &quote! {self},
            &self.types,
            &self.swift_bridge_path,
            // TODO: Add a UI test and then add a better span
            Span::call_site(),
        );

        let convert_ffi_to_rust = shared_struct.convert_ffi_repr_to_rust(
            &quote! { self },
            swift_bridge_path,
            &self.types,
        );

        let struct_ffi_repr = if shared_struct.fields.is_empty() {
            // Using a u8 is arbitrary... We just need a field since empty structs aren't FFI safe.
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct #struct_ffi_name {
                    _private: u8
                }
            }
        } else {
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct #struct_ffi_name #repr_c_struct_fields
            }
        };

        let mut derives: Vec<TokenStream> = vec![];
        if shared_struct.derives.copy {
            derives.push(quote! {Copy});
        }
        if shared_struct.derives.clone {
            derives.push(quote! {Clone});
        }

        let definition = quote! {
            #[derive(#(#derives),*)]
            pub struct #struct_name #struct_fields

            #struct_ffi_repr

            impl #swift_bridge_path::SharedStruct for #struct_name {
                type FfiRepr = #struct_ffi_name;
            }

            #into_ffi_repr_impl

            impl #struct_ffi_name {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> #struct_name {
                    #convert_ffi_to_rust
                }
            }

            #[repr(C)]
            #[doc(hidden)]
            pub struct #option_struct {
                is_some: bool,
                val: std::mem::MaybeUninit<#struct_ffi_name>,
            }

            impl #option_struct {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> Option<#struct_name> {
                    if self.is_some {
                        Some(unsafe { self.val.assume_init().into_rust_repr() })
                    } else {
                        None
                    }
                }

                #[doc(hidden)]
                #[inline(always)]
                pub fn from_rust_repr(val: Option<#struct_name>) -> #option_struct {
                    if let Some(val) = val {
                        #option_struct {
                            is_some: true,
                            val: std::mem::MaybeUninit::new(val.into_ffi_repr())
                        }
                    } else {
                        #option_struct {
                            is_some: false,
                            val: std::mem::MaybeUninit::uninit()
                        }
                    }
                }
            }
        };

        Some(definition)
    }
}
