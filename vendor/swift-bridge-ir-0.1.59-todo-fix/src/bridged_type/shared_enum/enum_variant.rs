use crate::bridged_type::{BridgedType, StructFields, TypePosition};
use crate::parse::TypeDeclarations;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::fmt::{Debug, Formatter};
use syn::spanned::Spanned;
use syn::Path;

#[derive(Clone)]
pub(crate) struct EnumVariant {
    pub name: Ident,
    #[allow(unused)]
    pub fields: StructFields,
}

impl EnumVariant {
    pub(crate) fn convert_rust_expression_to_ffi_repr(
        &self,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
        enum_name: &Ident,
        ffi_enum_name: &Ident,
    ) -> TokenStream {
        let variant_name = &self.name;
        let rust_fields: Vec<TokenStream> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| norm_field.ffi_field_name())
            .map(|norm_field| format_ident!("{}", norm_field))
            .map(|norm_field| quote! {#norm_field})
            .collect();
        let converted_fields: Vec<TokenStream> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let maybe_name_and_colon = norm_field.maybe_name_and_colon();
                let access_field = format_ident!("{}", norm_field.ffi_field_name());
                let access_field = quote! {#access_field};
                let ty = BridgedType::new_with_type(&norm_field.ty, types).unwrap();
                let converted_field = ty.convert_rust_expression_to_ffi_type(
                    &access_field,
                    swift_bridge_path,
                    types,
                    norm_field.ty.span(),
                );
                let tokens = quote! {
                    #maybe_name_and_colon #converted_field
                };
                tokens
            })
            .collect();

        let rust_fields = self.wrap_fields(&rust_fields);
        let converted_fields = self.wrap_fields(&converted_fields);

        if self.fields.is_empty() {
            quote! {
                #enum_name :: #variant_name => #ffi_enum_name :: #variant_name
            }
        } else {
            quote! {
                #enum_name :: #variant_name #rust_fields => #ffi_enum_name :: #variant_name #converted_fields
            }
        }
    }
    pub(crate) fn convert_ffi_repr_to_rust(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
        enum_name: &Ident,
        ffi_enum_name: &Ident,
    ) -> TokenStream {
        let variant_name = &self.name;
        let ffi_fields: Vec<TokenStream> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| format_ident!("{}", norm_field.ffi_field_name()))
            .map(|norm_field| quote! {#norm_field})
            .collect();
        let converted_fields: Vec<TokenStream> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let maybe_name_and_colon = norm_field.maybe_name_and_colon();
                let access_field = format_ident!("{}", norm_field.ffi_field_name());
                let access_field = quote! {#access_field};
                let ty = BridgedType::new_with_type(&norm_field.ty, types).unwrap();
                let converted_field = ty.convert_ffi_expression_to_rust_type(
                    &access_field,
                    norm_field.ty.span(),
                    swift_bridge_path,
                    types,
                );

                quote! {
                    #maybe_name_and_colon #converted_field
                }
            })
            .collect();

        let ffi_fields = self.wrap_fields(&ffi_fields);
        let converted_fields = self.wrap_fields(&converted_fields);

        if converted_fields.is_empty() {
            quote! {
                #ffi_enum_name :: #variant_name => #enum_name :: #variant_name
            }
        } else {
            quote! {
                #ffi_enum_name :: #variant_name #ffi_fields => #enum_name :: #variant_name #converted_fields
            }
        }
    }
    pub(crate) fn convert_ffi_expression_to_swift(
        &self,
        types: &TypeDeclarations,
        enum_name: String,
        swift_bridge_path: &Path,
    ) -> String {
        let converted_fields: Vec<String> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let field_name = norm_field.ffi_field_name();
                let ty = BridgedType::new_with_type(&norm_field.ty, types).unwrap();
                let field = ty.convert_ffi_value_to_swift_value(
                    &format!(
                        "self.payload.{variant_name}.{field_name}",
                        variant_name = self.name,
                        field_name = field_name
                    ),
                    TypePosition::SharedStructField,
                    types,
                    swift_bridge_path,
                );
                norm_field.struct_field_setter_string(field)
            })
            .collect();
        let converted_fields = converted_fields.join(", ");

        if self.fields.is_empty() {
            format!(
                "            case __swift_bridge__${enum_name}${variant_name}:
                return {enum_name}.{variant_name}\n",
                enum_name = enum_name,
                variant_name = self.name
            )
        } else {
            format!(
                "            case __swift_bridge__${enum_name}${variant_name}:
                return {enum_name}.{variant_name}({converted_fields})\n",
                enum_name = enum_name,
                variant_name = self.name,
                converted_fields = converted_fields
            )
        }
    }

    pub(crate) fn convert_swift_to_ffi_repr(
        &self,
        types: &TypeDeclarations,
        enum_name: String,
        ffi_enum_name: String,
        all_variants_empty: bool,
    ) -> String {
        if all_variants_empty {
            return format!(
                "            case {enum_name}.{variant_name}:
                return {ffi_enum_name}(tag: {ffi_enum_name}${variant_name})\n",
                enum_name = enum_name,
                variant_name = self.name,
                ffi_enum_name = ffi_enum_name
            );
        }
        let converted_fields: Vec<String> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let ffi_field_name = norm_field.ffi_field_name();
                let ty = BridgedType::new_with_type(&norm_field.ty, types).unwrap();
                let variant_field = ty.convert_swift_expression_to_ffi_type(
                    &format!("{}", ffi_field_name),
                    types,
                    TypePosition::SharedStructField,
                );
                norm_field.struct_ffi_field_setter_string(variant_field)
            })
            .collect();
        let converted_fields = converted_fields.join(", ");

        let associated_values: Vec<String> = self
            .fields
            .normalized_fields()
            .iter()
            .map(|norm_field| {
                let field_name = norm_field.ffi_field_name();
                format!("let {field_name}", field_name = field_name)
            })
            .collect();
        let associated_values = associated_values.join(", ");

        if self.fields.is_empty() {
            format!("            case {enum_name}.{variant_name}:
                return {{var val = {ffi_enum_name}(); val.tag = {ffi_enum_name}${variant_name}; return val }}()\n", enum_name = enum_name, variant_name = self.name, ffi_enum_name = ffi_enum_name)
        } else {
            format!("            case {enum_name}.{variant_name}({associated_values}):
                return {ffi_enum_name}(tag: {ffi_enum_name}${variant_name}, payload: {ffi_enum_name}Fields({variant_name}: {ffi_enum_name}$FieldOf{variant_name}({converted_fields})))\n", ffi_enum_name = ffi_enum_name, associated_values = associated_values, enum_name = enum_name, variant_name = self.name, converted_fields = converted_fields)
        }
    }

    fn wrap_fields(&self, fields: &[TokenStream]) -> TokenStream {
        match &self.fields {
            StructFields::Named(_) => {
                quote! {
                    { #(#fields),* }
                }
            }
            StructFields::Unnamed(_) => {
                quote! {
                    ( #(#fields),* )
                }
            }
            StructFields::Unit => {
                quote! {}
            }
        }
    }

    pub(crate) fn union_name_string(&self, parent_enum_ffi_name: &String) -> String {
        format!("{}$FieldOf{}", parent_enum_ffi_name, self.name.to_string())
    }
}

impl PartialEq for EnumVariant {
    fn eq(&self, other: &Self) -> bool {
        self.name.to_string() == other.name.to_string()
    }
}

impl Debug for EnumVariant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnumVariant")
            .field("name", &self.name.to_string())
            .field("fields", &self.fields)
            .finish()
    }
}
