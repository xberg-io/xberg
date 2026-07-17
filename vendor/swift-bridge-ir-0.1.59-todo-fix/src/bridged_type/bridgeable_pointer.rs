use crate::bridged_type::{
    BridgeableType, BridgedType, BuiltInResult, CFfiStruct, TypePosition, UnusedOptionNoneValue,
};
use crate::parse::TypeDeclarations;
use crate::Path;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::fmt::{Debug, Formatter};
use syn::Type;

#[derive(Debug, PartialEq)]
pub(crate) struct BuiltInPointer {
    pub kind: PointerKind,
    pub pointee: Pointee,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum PointerKind {
    Const,
    Mut,
}

/// The target of an `*const` or `*mut` pointer.
pub(crate) enum Pointee {
    BuiltIn(Box<BridgedType>),
    /// `*const SomeType`
    ///         ^^^^^^^^ This is the Pointee
    Void(Type),
}

impl BridgeableType for BuiltInPointer {
    fn is_built_in_type(&self) -> bool {
        todo!()
    }

    fn is_result(&self) -> bool {
        todo!()
    }

    fn as_result(&self) -> Option<&BuiltInResult> {
        todo!()
    }

    fn as_option(&self) -> Option<&super::bridged_option::BridgedOption> {
        todo!();
    }

    fn is_passed_via_pointer(&self) -> bool {
        todo!()
    }

    fn generate_custom_rust_ffi_types(
        &self,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> Option<Vec<TokenStream>> {
        None
    }

    fn generate_custom_c_ffi_types(&self, _types: &TypeDeclarations) -> Option<CFfiStruct> {
        None
    }

    fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        match &self.pointee {
            Pointee::BuiltIn(ty) => {
                let pointer_kind = self.kind.to_ffi_compatible_rust_type();
                let ty = ty.to_rust_type_path(types);
                quote! { #pointer_kind #ty}
            }
            Pointee::Void(_ty) => {
                let pointer_kind = self.kind.to_ffi_compatible_rust_type();
                let pointee = self.pointee.to_rust_type_path(types);

                quote! { #pointer_kind super:: #pointee }
            }
        }
    }

    fn to_swift_type(
        &self,
        _type_pos: TypePosition,
        _types: &TypeDeclarations,
        _swift_bridge_path: &Path,
    ) -> String {
        todo!()
    }

    fn to_c_type(&self, _types: &TypeDeclarations) -> String {
        todo!()
    }

    fn to_c_include(&self, _types: &TypeDeclarations) -> Option<Vec<&'static str>> {
        todo!()
    }

    fn to_ffi_compatible_rust_type(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let kind = self.kind.to_ffi_compatible_rust_type();

        let ty = match &self.pointee {
            Pointee::BuiltIn(ty) => ty.to_ffi_compatible_rust_type(swift_bridge_path, types),
            Pointee::Void(ty) => {
                quote! { super::#ty }
            }
        };

        quote! { #kind #ty}
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
        _expression: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
        _span: Span,
    ) -> TokenStream {
        todo!()
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
        _expression: &str,
        _types: &TypeDeclarations,
        _type_pos: TypePosition,
    ) -> String {
        todo!()
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
        _expression: &TokenStream,
        _span: Span,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        todo!()
    }

    fn convert_ffi_option_expression_to_rust_type(&self, _expression: &TokenStream) -> TokenStream {
        todo!()
    }

    fn convert_ffi_expression_to_swift_type(
        &self,
        _expression: &str,
        _type_pos: TypePosition,
        _types: &TypeDeclarations,
        _swift_bridge_path: &Path,
    ) -> String {
        todo!()
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
        todo!()
    }

    fn convert_ffi_result_err_value_to_rust_value(
        &self,
        _err_ffi_value: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        todo!()
    }

    fn unused_option_none_val(&self, _swift_bridge_path: &Path) -> UnusedOptionNoneValue {
        todo!()
    }

    fn can_parse_token_stream_str(_tokens: &str) -> bool
    where
        Self: Sized,
    {
        todo!()
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
        todo!()
    }

    fn is_str(&self) -> bool {
        todo!()
    }

    fn contains_owned_string_recursive(&self, _types: &TypeDeclarations) -> bool {
        todo!()
    }

    fn contains_ref_string_recursive(&self) -> bool {
        todo!()
    }

    fn has_swift_bridge_copy_annotation(&self) -> bool {
        todo!()
    }

    fn only_encoding(&self) -> Option<super::OnlyEncoding> {
        todo!()
    }

    fn to_alpha_numeric_underscore_name(&self, _types: &TypeDeclarations) -> String {
        todo!()
    }
}

impl PointerKind {
    fn to_ffi_compatible_rust_type(&self) -> TokenStream {
        match self {
            PointerKind::Const => {
                quote! { *const }
            }
            PointerKind::Mut => {
                quote! { *mut }
            }
        }
    }
}

impl Pointee {
    fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        match self {
            Pointee::BuiltIn(built_in) => built_in.to_rust_type_path(types),
            Pointee::Void(ty) => ty.to_token_stream(),
        }
    }
}

impl Debug for Pointee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Pointee::BuiltIn(built_in) => f.debug_tuple("BuiltIn").field(&built_in).finish(),
            Pointee::Void(ty) => f
                .debug_tuple("Void")
                .field(&ty.to_token_stream().to_string())
                .finish(),
        }
    }
}

impl PartialEq for Pointee {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BuiltIn(_left), Self::BuiltIn(_right)) => {
                // left == right
                todo!()
            }
            (Self::Void(left), Self::Void(right)) => {
                left.to_token_stream().to_string() == right.to_token_stream().to_string()
            }
            _ => false,
        }
    }
}
