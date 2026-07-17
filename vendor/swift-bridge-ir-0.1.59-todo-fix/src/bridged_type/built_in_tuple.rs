use crate::bridged_type::shared_struct::UnnamedStructFields;
use crate::bridged_type::{
    BridgeableType, BuiltInResult, CFfiStruct, OnlyEncoding, TypePosition, UnusedOptionNoneValue,
};
use crate::parse::TypeDeclarations;
use crate::SWIFT_BRIDGE_PREFIX;
use std::fmt::Debug;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Path, Type};

#[derive(Debug)]
pub(crate) struct BuiltInTuple(UnnamedStructFields);

impl BuiltInTuple {
    fn prefixed_ty_name(&self, types: &TypeDeclarations) -> Ident {
        let combined_types = self.0.combine_field_types_into_ffi_name_string(types);
        let ty_name = format_ident!("{}_{}", "tuple", combined_types);
        Ident::new(
            &format!("{}{}", SWIFT_BRIDGE_PREFIX, ty_name),
            ty_name.span(),
        )
    }

    /// Get the FFI compatible Swift type.
    ///
    /// Example
    ///
    /// `(Int32, UInt32)` becomes `__swift_bridge__$tuple$I32U32`.
    fn to_ffi_compatible_swift_type(
        &self,
        expression: &str,
        types: &TypeDeclarations,
        type_pos: TypePosition,
    ) -> String {
        let converted_fields = self
            .0
            .convert_swift_expression_to_ffi_type(expression, types, type_pos);
        let converted_fields = converted_fields.join(", ");
        format!(
            "{}${}${}({})",
            SWIFT_BRIDGE_PREFIX,
            "tuple",
            self.0.combine_field_types_into_ffi_name_string(types),
            converted_fields
        )
    }
}

impl BridgeableType for BuiltInTuple {
    fn is_built_in_type(&self) -> bool {
        todo!();
    }

    fn only_encoding(&self) -> Option<OnlyEncoding> {
        todo!();
    }

    fn is_result(&self) -> bool {
        todo!();
    }

    fn as_result(&self) -> Option<&BuiltInResult> {
        todo!();
    }

    fn as_option(&self) -> Option<&super::bridged_option::BridgedOption> {
        todo!()
    }

    fn is_passed_via_pointer(&self) -> bool {
        todo!();
    }

    fn generate_custom_rust_ffi_types(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> Option<Vec<TokenStream>> {
        let combined_types_tokens = self
            .0
            .combine_field_types_into_ffi_name_tokens(swift_bridge_path, types);
        let prefixed_ty_name = self.prefixed_ty_name(types);
        Some(vec![quote! {
            #[repr(C)]
            #[doc(hidden)]
            pub struct #prefixed_ty_name ( #(#combined_types_tokens),* );
        }])
    }

    fn generate_custom_c_ffi_types(&self, types: &TypeDeclarations) -> Option<CFfiStruct> {
        let combined_types = self.0.combine_field_types_into_ffi_name_string(types);
        let fields: Vec<String> = self.0.combine_field_types_into_c_type(types);
        let fields = fields.join("; ");
        let fields = fields + ";";
        let c_decl = format!("typedef struct __swift_bridge__$tuple${combined_types} {{ {fields} }} __swift_bridge__$tuple${combined_types};");
        Some(CFfiStruct {
            c_ffi_type: c_decl,
            fields: vec![],
        })
    }

    fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        let rust_type_path = self.0.to_rust_type_path_tokens(types);
        quote! {
            ( #(#rust_type_path),*)
        }
    }

    fn to_swift_type(
        &self,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(host_lang, _) => {
                if host_lang.is_swift() {
                    let field_signatures = self.0.combine_field_types_into_ffi_name_string(types);
                    format!("__swift_bridge__$tuple${field_signatures}")
                } else {
                    self.0
                        .to_swift_tuple_signature(type_pos, types, swift_bridge_path)
                }
            }
            TypePosition::FnReturn(host_lang) => {
                if host_lang.is_swift() {
                    let field_signatures = self.0.combine_field_types_into_ffi_name_string(types);
                    format!("__swift_bridge__$tuple${field_signatures}")
                } else {
                    self.0
                        .to_swift_tuple_signature(type_pos, types, swift_bridge_path)
                }
            }
            TypePosition::SharedStructField => todo!(),
            TypePosition::ResultFfiReturnType => todo!(),
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn to_c_type(&self, types: &TypeDeclarations) -> String {
        let ty_name = format!(
            "{}${}${}",
            SWIFT_BRIDGE_PREFIX,
            "tuple",
            self.0.combine_field_types_into_ffi_name_string(types)
        );
        format!("struct {}", ty_name)
    }

    fn to_c_include(&self, types: &TypeDeclarations) -> Option<Vec<&'static str>> {
        self.0.to_c_include(types)
    }

    fn to_ffi_compatible_rust_type(
        &self,
        _swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let prefixed_ty_name = self.prefixed_ty_name(types);

        quote! { #prefixed_ty_name }
    }

    fn to_ffi_compatible_option_rust_type(
        &self,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        todo!()
    }

    fn to_ffi_compatible_option_swift_type(
        &self,
        _type_pos: TypePosition,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> String {
        todo!()
    }

    fn to_ffi_compatible_option_c_type(&self) -> String {
        todo!()
    }

    fn convert_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
        span: Span,
    ) -> TokenStream {
        let prefixed_ty_name = self.prefixed_ty_name(types);
        let converted_fields: Vec<TokenStream> =
            self.0
                .convert_rust_expression_to_ffi_type(expression, swift_bridge_path, types, span);
        return quote! {
            { let val = #expression; #prefixed_ty_name( #(#converted_fields),* ) }
        };
    }

    fn convert_option_rust_expression_to_ffi_type(
        &self,
        _expression: &TokenStream,
        _swift_bridge_path: &Path,
    ) -> TokenStream {
        todo!()
    }

    fn convert_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        types: &TypeDeclarations,
        type_pos: TypePosition,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(_, _) => {
                self.to_ffi_compatible_swift_type(expression, types, type_pos)
            }
            TypePosition::FnReturn(host_lang) => {
                if host_lang.is_swift() {
                    let ffi_type = self.to_ffi_compatible_swift_type("val", types, type_pos);
                    format!("{{ let val = {expression}; return {ffi_type}; }}()")
                } else {
                    self.to_ffi_compatible_swift_type(expression, types, type_pos)
                }
            }
            TypePosition::SharedStructField => todo!(),
            TypePosition::ResultFfiReturnType => todo!(),
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn convert_option_swift_expression_to_ffi_type(
        &self,
        _expression: &str,
        _type_pos: TypePosition,
    ) -> String {
        todo!()
    }

    fn convert_ffi_expression_to_rust_type(
        &self,
        expression: &TokenStream,
        span: Span,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let fields: Vec<TokenStream> = self.0.convert_ffi_expression_to_rust_type(
            &quote! {val},
            span,
            swift_bridge_path,
            types,
        );
        return quote_spanned! {
            span => {
                let val = #expression;
                ( #(#fields),* )
            }
        };
    }

    fn convert_ffi_option_expression_to_rust_type(&self, _expression: &TokenStream) -> TokenStream {
        todo!()
    }

    fn convert_ffi_expression_to_swift_type(
        &self,
        expression: &str,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        let converted_fields: Vec<String> = self.0.convert_ffi_expression_to_swift_type(
            expression,
            type_pos,
            types,
            swift_bridge_path,
        );
        let converted_fields = converted_fields.join(", ");

        return format!(
            "{{ let val = {}; return ({converted_fields}); }}()",
            expression
        );
    }

    fn convert_ffi_option_expression_to_swift_type(&self, _expression: &str) -> String {
        todo!()
    }

    fn convert_ffi_result_ok_value_to_rust_value(
        &self,
        _ok_ffi_value: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        todo!();
    }

    fn convert_ffi_result_err_value_to_rust_value(
        &self,
        _err_ffi_value: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        todo!();
    }

    fn unused_option_none_val(&self, _swift_bridge_path: &Path) -> UnusedOptionNoneValue {
        todo!();
    }

    fn can_parse_token_stream_str(_tokens: &str) -> bool
    where
        Self: Sized,
    {
        todo!();
    }

    fn from_type(_ty: &Type, _types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn parse_token_stream_str(_tokens: &str, _types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn is_null(&self) -> bool {
        todo!();
    }

    fn is_str(&self) -> bool {
        todo!();
    }

    fn contains_owned_string_recursive(&self, types: &TypeDeclarations) -> bool {
        self.0.contains_owned_string_recursive(types)
    }

    fn contains_ref_string_recursive(&self) -> bool {
        todo!()
    }

    fn has_swift_bridge_copy_annotation(&self) -> bool {
        todo!();
    }

    fn to_alpha_numeric_underscore_name(&self, types: &TypeDeclarations) -> String {
        let fields_name = self.0.to_alpha_numeric_underscore_name(types);
        "Tuple".to_string() + &fields_name
    }
}

impl BuiltInTuple {
    pub fn new_unnamed_with_types(types: Vec<Type>) -> Self {
        let unnamed_fields = UnnamedStructFields::new_with_types(types);
        Self(unnamed_fields)
    }
}
