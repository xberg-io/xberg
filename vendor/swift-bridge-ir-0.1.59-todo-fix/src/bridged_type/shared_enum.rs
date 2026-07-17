use crate::SWIFT_BRIDGE_PREFIX;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::fmt::{Debug, Formatter};
use syn::LitStr;

mod enum_variant;
pub(crate) use self::enum_variant::EnumVariant;

use super::StructFields;

#[derive(Default, Clone)]
pub(crate) struct DeriveAttrs {
    pub debug: bool,
}

#[derive(Clone)]
pub(crate) struct SharedEnum {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub already_declared: bool,
    pub swift_name: Option<LitStr>,
    pub derive: DeriveAttrs,
}

impl SharedEnum {
    /// SomeEnum
    pub fn swift_name_string(&self) -> String {
        if let Some(swift_name) = self.swift_name.as_ref() {
            swift_name.value().to_string()
        } else {
            format!("{}", self.name)
        }
    }

    /// __swift_bridge__$SomeEnum
    pub fn ffi_name_string(&self) -> String {
        format!("{}${}", SWIFT_BRIDGE_PREFIX, self.swift_name_string())
    }

    /// __swift_bridge__$SomeEnumTag
    pub fn ffi_tag_name_string(&self) -> String {
        format!("{}Tag", self.ffi_name_string())
    }

    /// __swift_bridge__SomeEnum
    pub fn ffi_name_tokens(&self) -> TokenStream {
        let name = Ident::new(
            &format!("{}{}", SWIFT_BRIDGE_PREFIX, self.name),
            self.name.span(),
        );
        quote! { #name }
    }

    /// __swift_bridge__Option_SomeEnum
    pub fn ffi_option_name_tokens(&self) -> TokenStream {
        let name = Ident::new(
            &format!("{}Option_{}", SWIFT_BRIDGE_PREFIX, self.name),
            self.name.span(),
        );
        quote! { #name }
    }

    /// __swift_bridge__$SomeEnumFields
    pub fn ffi_union_name_string(&self) -> String {
        format!("{}Fields", self.ffi_name_string())
    }

    /// { __swift_bridge__$SomeEnum$FieldOfVariant1 Variant1; __swift_bridge__$SomeEnum$FieldOfVariant2 Variant2;}
    pub fn ffi_union_field_names_string(&self) -> String {
        let mut union_fields = "{".to_string();
        for variant in self.variants.iter() {
            if let StructFields::Unit = variant.fields {
                continue;
            }
            let union_field = format!(
                " {} {};",
                variant.union_name_string(&self.ffi_name_string()),
                variant.name
            );
            union_fields += &union_field;
        }
        union_fields += "}";
        union_fields
    }

    /// __swift_bridge__$Option$SomeEnum
    pub fn ffi_option_name_string(&self) -> String {
        format!(
            "{}$Option${}",
            SWIFT_BRIDGE_PREFIX,
            self.swift_name_string()
        )
    }
}

impl SharedEnum {
    /// Whether or not any of the enum's variants contain data.
    ///
    /// `EnumWithData { VariantA(u8), VariantB }` -> true
    /// `EnumWithData { VariantA(u8), VariantB(u16) }` -> true
    /// `EnumWithNoData { VariantA, VariantB }` -> false
    pub fn has_one_or_more_variants_with_data(&self) -> bool {
        self.variants.iter().any(|v| !v.fields.is_empty())
    }

    /// Returns true if None of the variants are data-carrying.
    ///
    /// `enum { VariantA, VariantB }` would return true.
    /// `enum { VariantA(u8), VariantB }` would return false.
    pub fn all_variants_empty(&self) -> bool {
        self.variants.iter().all(|v| v.fields.is_empty())
    }
}

impl PartialEq for SharedEnum {
    fn eq(&self, other: &Self) -> bool {
        self.name.to_string() == other.name.to_string() && self.variants == other.variants
    }
}

impl Debug for SharedEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedEnum")
            .field("name", &self.name.to_string())
            .field("variants", &self.variants)
            .finish()
    }
}
