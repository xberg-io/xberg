use crate::bridged_type::{BridgedType, TypePosition};
use crate::parse::HostLang;
use crate::TypeDeclarations;
use proc_macro2::TokenStream;
use quote::quote;
use std::ops::Deref;
use syn::{Path, TypeParam};

pub(crate) const GENERIC_PLACEHOLDERS: [&'static str; 8] = ["A", "B", "C", "D", "E", "F", "G", "H"];

#[derive(Clone)]
pub(crate) struct OpaqueRustTypeGenerics {
    pub generics: Vec<TypeParam>,
}

impl OpaqueRustTypeGenerics {
    pub(crate) fn new() -> Self {
        OpaqueRustTypeGenerics { generics: vec![] }
    }

    /// For Rust type `SomeType<u32, u64>`:
    /// A == UInt32, B == UInt64
    pub(crate) fn rust_opaque_type_swift_generic_bounds(
        &self,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        if self.generics.len() == 0 {
            return "".to_string();
        }

        let bounds: Vec<String> = self
            .generics
            .iter()
            .enumerate()
            .map(|(idx, g)| {
                format!(
                    "{} == {}",
                    GENERIC_PLACEHOLDERS[idx],
                    BridgedType::new_with_str(&g.ident.to_string(), types)
                        .unwrap()
                        // TODO: FnReturn isn't the real position.. Add a
                        //  new variant that makes more sense for our use case (generic bounds).
                        .to_swift_type(
                            TypePosition::FnReturn(HostLang::Rust),
                            types,
                            swift_bridge_path
                        )
                )
            })
            .collect();

        bounds.join(", ")
    }

    /// "<A, B, C>" if there are generics.
    /// "" if there are no generics.
    pub(crate) fn angle_bracketed_generic_placeholders_string(&self) -> String {
        if self.generics.len() == 0 {
            return "".to_string();
        }

        let generics: Vec<String> = self
            .generics
            .iter()
            .enumerate()
            .map(|(idx, _)| GENERIC_PLACEHOLDERS[idx].to_string())
            .collect();
        let generics = generics.join(", ");
        format!("<{}>", generics)
    }

    /// "<UInt8, Int64, RustStr>" if there are generics.
    /// "" if there are no generics.
    pub(crate) fn angle_bracketed_generic_concrete_swift_types_string(
        &self,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        if self.generics.len() == 0 {
            return "".to_string();
        }

        let bounds: Vec<String> = self
            .generics
            .iter()
            .map(|g| {
                format!(
                    "{}",
                    BridgedType::new_with_str(&g.ident.to_string(), types)
                        .unwrap()
                        // TODO: FnReturn isn't the real position.. Add a
                        //  new variant that makes more sense for our use case (generic bounds).
                        .to_swift_type(
                            TypePosition::FnReturn(HostLang::Rust),
                            types,
                            swift_bridge_path
                        )
                )
            })
            .collect();

        format!("<{}>", bounds.join(", "))
    }

    /// "<u8, i16, super::SomeType>" if there are generics.
    /// "" if there are no generics.
    pub(crate) fn angle_bracketed_concrete_generics_tokens(
        &self,
        types: &TypeDeclarations,
    ) -> TokenStream {
        if self.generics.len() == 0 {
            return quote! {};
        }

        let generics: Vec<TokenStream> = self
            .generics
            .iter()
            .map(|g| {
                let ty = BridgedType::new_with_str(&g.ident.to_string(), types).unwrap();
                let path = ty.to_rust_type_path(types);
                quote! { #path }
            })
            .collect();
        quote! {<#(#generics),*>}
    }

    /// "$A$B$C" if there are generics.
    /// "" if there are no generics.
    pub(crate) fn dollar_prefixed_generics_string(&self) -> String {
        let mut generics = String::with_capacity(self.generics.len() * 2);
        if self.generics.len() == 0 {
            return generics;
        }

        for generic in self.generics.iter() {
            generics += &format!("${}", generic.ident);
        }

        generics
    }

    /// "_A_B_C" if there are generics.
    /// "" if there are no generics.
    pub(crate) fn underscore_prefixed_generics_string(&self) -> String {
        let mut generics = String::with_capacity(self.generics.len() * 2);
        if self.generics.len() == 0 {
            return generics;
        }

        for generic in self.generics.iter() {
            generics += &format!("_{}", generic.ident);
        }

        generics
    }
}

impl Deref for OpaqueRustTypeGenerics {
    type Target = Vec<TypeParam>;

    fn deref(&self) -> &Self::Target {
        &self.generics
    }
}
