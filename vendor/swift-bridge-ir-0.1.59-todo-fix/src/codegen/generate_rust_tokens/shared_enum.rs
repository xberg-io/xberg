//! More tests can be found in
//! crates/swift-bridge-ir/src/codegen/codegen_tests/shared_enum_codegen_tests.rs

use crate::bridged_type::{BridgedType, SharedEnum, StructFields};
use crate::codegen::generate_rust_tokens::vec::vec_of_transparent_enum::generate_vec_of_transparent_enum_functions;
use crate::parse::TypeDeclarations;
use crate::{SwiftBridgeModule, SWIFT_BRIDGE_PREFIX};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

impl SwiftBridgeModule {
    /// Generate the tokens for a shared enum.
    pub(super) fn generate_shared_enum_tokens(
        &self,
        shared_enum: &SharedEnum,
        types: &TypeDeclarations,
    ) -> Option<TokenStream> {
        if shared_enum.already_declared {
            return None;
        }

        let enum_name = &shared_enum.name;
        let swift_bridge_path = &self.swift_bridge_path;

        let enum_ffi_name = format!("{}{}", SWIFT_BRIDGE_PREFIX, enum_name);
        let enum_ffi_name = Ident::new(&enum_ffi_name, enum_name.span());

        let option_enum = shared_enum.ffi_option_name_tokens();

        let mut enum_variants = vec![];
        let mut enum_ffi_variants = vec![];

        for variant in shared_enum.variants.iter() {
            let variant_name = &variant.name;
            let enum_variant = match &variant.fields {
                StructFields::Named(named_fields) => {
                    let mut names = vec![];
                    for named_field in named_fields {
                        let field_name = &named_field.name;
                        let ty = BridgedType::new_with_type(&named_field.ty, &self.types)
                            .unwrap()
                            .to_rust_type_path(types);
                        let field = quote! {#field_name : #ty};
                        names.push(field);
                    }
                    quote! {
                        #variant_name {#(#names),*}
                    }
                }
                StructFields::Unnamed(unamed_fields) => {
                    let mut names = vec![];
                    for unnamed_field in unamed_fields {
                        let ty =
                            BridgedType::new_with_type(&unnamed_field.ty, &self.types).unwrap();
                        names.push(ty.to_rust_type_path(types));
                    }
                    quote! {
                        #variant_name (#(#names),*)
                    }
                }
                StructFields::Unit => {
                    quote! {
                        #variant_name
                    }
                }
            };
            enum_variants.push(enum_variant);
        }

        for variant in shared_enum.variants.iter() {
            let variant_name = &variant.name;
            let enum_ffi_variant = match &variant.fields {
                StructFields::Named(named_fields) => {
                    let mut names = vec![];
                    for named_field in named_fields {
                        let field_name = &named_field.name;
                        let ty = BridgedType::new_with_type(&named_field.ty, &self.types)
                            .unwrap()
                            .to_ffi_compatible_rust_type(&self.swift_bridge_path, &self.types);
                        let field = quote! {#field_name : #ty};
                        names.push(field);
                    }
                    quote! {
                        #variant_name {#(#names),*}
                    }
                }
                StructFields::Unnamed(unamed_fields) => {
                    let mut names = vec![];
                    for unnamed_field in unamed_fields {
                        let ty =
                            BridgedType::new_with_type(&unnamed_field.ty, &self.types).unwrap();
                        names.push(
                            ty.to_ffi_compatible_rust_type(&self.swift_bridge_path, &self.types),
                        );
                    }
                    quote! {
                        #variant_name (#(#names),*)
                    }
                }
                StructFields::Unit => {
                    quote! {
                        #variant_name
                    }
                }
            };
            enum_ffi_variants.push(enum_ffi_variant);
        }

        let mut convert_rust_variants_to_ffi = vec![];
        let mut convert_ffi_variants_to_rust = vec![];

        for variant in shared_enum.variants.iter() {
            let convert_rust_variant_to_ffi = variant.convert_rust_expression_to_ffi_repr(
                &self.types,
                &self.swift_bridge_path,
                &format_ident!("{}", enum_name),
                &format_ident!("{}", enum_ffi_name),
            );
            convert_rust_variants_to_ffi.push(convert_rust_variant_to_ffi);
        }

        for variant in shared_enum.variants.iter() {
            let convert_ffi_variant_to_rust = variant.convert_ffi_repr_to_rust(
                &self.swift_bridge_path,
                &self.types,
                &format_ident!("{}", enum_name),
                &format_ident!("{}", enum_ffi_name),
            );
            convert_ffi_variants_to_rust.push(convert_ffi_variant_to_rust);
        }

        // Auto derives
        let mut derives = if shared_enum.has_one_or_more_variants_with_data() {
            vec![]
        } else {
            vec![quote! {Copy}, quote! {Clone}]
        };

        // User derives
        let mut derive_impl_ffi_bridges = vec![];

        // We currently only allow derive(Debug) on non data carrying enums in order
        // to prevent a potential memory safety issue.
        // https://github.com/chinedufn/swift-bridge/pull/194#discussion_r1134386788
        if shared_enum.derive.debug && !shared_enum.has_one_or_more_variants_with_data() {
            derives.push(quote! {::std::fmt::Debug});

            // __swift_bridge__$SomeEnum$Debug
            let export_name = format!("{}$Debug", shared_enum.ffi_name_string());
            // __swift_bridge__SomeEnum_Debug
            let fn_name = format_ident!("{}_Debug", enum_ffi_name);
            derive_impl_ffi_bridges.push(quote! {
                #[export_name = #export_name]
                pub extern "C" fn #fn_name(this: #enum_ffi_name) -> *mut swift_bridge::string::RustString {
                    swift_bridge::string::RustString(format!("{:?}", this.into_rust_repr())).box_into_raw()
                }
            });
        }

        let vec_support = if shared_enum.has_one_or_more_variants_with_data() {
            // Enums with variants that contain data are not yet supported.
            quote! {}
        } else {
            generate_vec_of_transparent_enum_functions(&shared_enum)
        };

        let definition = quote! {
            #[derive(#(#derives),*)]
            pub enum #enum_name {
                #(#enum_variants),*
            }

            #[repr(C)]
            #[doc(hidden)]
            pub enum #enum_ffi_name {
                #(#enum_ffi_variants),*
            }

            impl #swift_bridge_path::SharedEnum for #enum_name {
                type FfiRepr = #enum_ffi_name;
            }

            impl #enum_name {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> #enum_ffi_name {
                    match self {
                        #(#convert_rust_variants_to_ffi),*
                    }
                }
            }

            impl #enum_ffi_name {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> #enum_name {
                    match self {
                        #(#convert_ffi_variants_to_rust),*
                    }
                }
            }

            #[repr(C)]
            #[doc(hidden)]
            pub struct #option_enum {
                is_some: bool,
                val: std::mem::MaybeUninit<#enum_ffi_name>,
            }

            impl #option_enum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> Option<#enum_name> {
                    if self.is_some {
                        Some(unsafe { self.val.assume_init().into_rust_repr() })
                    } else {
                        None
                    }
                }

                #[doc(hidden)]
                #[inline(always)]
                pub fn from_rust_repr(val: Option<#enum_name>) -> #option_enum {
                    if let Some(val) = val {
                        #option_enum {
                            is_some: true,
                            val: std::mem::MaybeUninit::new(val.into_ffi_repr())
                        }
                    } else {
                        #option_enum {
                            is_some: false,
                            val: std::mem::MaybeUninit::uninit()
                        }
                    }
                }
            }

            #vec_support

            #(#derive_impl_ffi_bridges),*
        };

        Some(definition)
    }
}
