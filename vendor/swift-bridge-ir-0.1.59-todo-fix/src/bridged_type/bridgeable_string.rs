use crate::bridged_type::{
    BridgeableType, CFfiStruct, OnlyEncoding, TypePosition, UnusedOptionNoneValue,
};
use crate::TypeDeclarations;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{Path, Type};

#[derive(Debug)]
pub(crate) struct BridgedString;

impl BridgeableType for BridgedString {
    fn is_built_in_type(&self) -> bool {
        true
    }

    fn only_encoding(&self) -> Option<OnlyEncoding> {
        None
    }

    fn is_result(&self) -> bool {
        false
    }

    fn as_result(&self) -> Option<&super::bridgeable_result::BuiltInResult> {
        None
    }

    fn as_option(&self) -> Option<&super::bridged_option::BridgedOption> {
        None
    }

    fn is_passed_via_pointer(&self) -> bool {
        true
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

    fn to_rust_type_path(&self, _types: &TypeDeclarations) -> TokenStream {
        // FIXME: Change to `::std::string::String`
        quote! { String }
    }

    fn to_swift_type(
        &self,
        type_pos: TypePosition,
        _types: &TypeDeclarations,
        _swift_bridge_path: &Path,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(func_host_lang, _) => {
                if func_host_lang.is_rust() {
                    "GenericIntoRustString".to_string()
                } else {
                    "UnsafeMutableRawPointer".to_string()
                }
            }
            TypePosition::FnReturn(func_host_lang) => {
                if func_host_lang.is_rust() {
                    "RustString".to_string()
                } else {
                    "UnsafeMutableRawPointer".to_string()
                }
            }
            TypePosition::SharedStructField => "RustString".to_string(),
            TypePosition::ResultFfiReturnType => "UnsafeMutableRawPointer?".to_string(),
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn to_c_type(&self, _types: &TypeDeclarations) -> String {
        "void*".to_string()
    }

    fn to_c_include(&self, _types: &TypeDeclarations) -> Option<Vec<&'static str>> {
        None
    }

    fn to_ffi_compatible_rust_type(
        &self,
        swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        quote! { *mut #swift_bridge_path::string::RustString }
    }

    fn to_ffi_compatible_option_rust_type(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        self.to_ffi_compatible_rust_type(swift_bridge_path, types)
    }

    fn to_ffi_compatible_option_swift_type(
        &self,
        type_pos: TypePosition,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(host_lang, _) => {
                if host_lang.is_rust() {
                    todo!()
                } else {
                    "UnsafeMutableRawPointer?".to_string()
                }
            }
            TypePosition::FnReturn(host_lang) => {
                if host_lang.is_rust() {
                    todo!()
                } else {
                    "UnsafeMutableRawPointer?".to_string()
                }
            }
            TypePosition::SharedStructField => "UnsafeMutableRawPointer?".to_string(),
            TypePosition::ResultFfiReturnType => {
                todo!()
            }
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn to_ffi_compatible_option_c_type(&self) -> String {
        "void*".to_string()
    }

    fn convert_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        swift_bridge_path: &Path,
        _types: &TypeDeclarations,
        _span: Span,
    ) -> TokenStream {
        quote! {
            #swift_bridge_path::string::RustString( #expression ).box_into_raw()
        }
    }

    fn convert_option_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        swift_bridge_path: &Path,
    ) -> TokenStream {
        let unused_none_value = BridgedString.unused_option_none_val(swift_bridge_path).rust;

        quote! {
            if let Some(val) = #expression {
                #swift_bridge_path::string::RustString(val).box_into_raw()
            } else {
                #unused_none_value
            }
        }
    }

    fn convert_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        _types: &TypeDeclarations,
        _type_pos: TypePosition,
    ) -> String {
        format!(
            "{{ let rustString = {value}.intoRustString(); rustString.isOwned = false; return rustString.ptr }}()",
            value = expression
        )
    }

    fn convert_option_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        type_pos: TypePosition,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(_func_host_lang, _) => {
                format!(
                    "{{ if let rustString = optionalStringIntoRustString({expression}) {{ rustString.isOwned = false; return rustString.ptr }} else {{ return nil }} }}()",
                    expression = expression
                )
            }
            TypePosition::FnReturn(func_host_lang) => {
                if func_host_lang.is_rust() {
                    todo!()
                } else {
                    format!(
                        "{{ if let rustString = optionalStringIntoRustString({expression}) {{ rustString.isOwned = false; return rustString.ptr }} else {{ return nil }} }}()",
                        expression = expression
                    )
                }
            }
            TypePosition::SharedStructField => {
                format!(
                    "{{ if let rustString = optionalStringIntoRustString({expression}) {{ rustString.isOwned = false; return rustString.ptr }} else {{ return nil }} }}()",
                    expression = expression
                )
            }
            TypePosition::ResultFfiReturnType => {
                unimplemented!()
            }
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn convert_ffi_expression_to_rust_type(
        &self,
        expression: &TokenStream,
        span: Span,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        quote_spanned! {span=>
            unsafe { Box::from_raw(#expression).0 }
        }
    }

    fn convert_ffi_option_expression_to_rust_type(&self, expression: &TokenStream) -> TokenStream {
        quote! {
            {
                let val = #expression;

                if val.is_null() {
                    None
                } else {
                    Some( unsafe { Box::from_raw(val).0 } )
                }
            }
        }
    }

    fn convert_ffi_expression_to_swift_type(
        &self,
        expression: &str,
        type_pos: TypePosition,
        _types: &TypeDeclarations,
        _swift_bridge_path: &Path,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(_, _)
            | TypePosition::FnReturn(_)
            | TypePosition::SharedStructField => {
                format!("RustString(ptr: {})", expression)
            }
            TypePosition::ResultFfiReturnType => {
                format!("RustString(ptr: {}!)", expression)
            }
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    fn convert_ffi_option_expression_to_swift_type(&self, expression: &str) -> String {
        format!("{{ let val = {expression}; if val != nil {{ return RustString(ptr: val!) }} else {{ return nil }} }}()", expression = expression,)
    }

    fn convert_ffi_result_ok_value_to_rust_value(
        &self,
        result: &TokenStream,
        swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        quote! {
            unsafe {
                Box::from_raw(#result.ok_or_err as *mut #swift_bridge_path::string::RustString).0
            }
        }
    }

    fn convert_ffi_result_err_value_to_rust_value(
        &self,
        result: &TokenStream,
        swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        quote! {
            unsafe {
                Box::from_raw(#result.ok_or_err as *mut #swift_bridge_path::string::RustString).0
            }
        }
    }

    fn unused_option_none_val(&self, swift_bridge_path: &Path) -> UnusedOptionNoneValue {
        UnusedOptionNoneValue {
            rust: quote! {
                std::ptr::null::<#swift_bridge_path::string::RustString>() as *mut #swift_bridge_path::string::RustString
            },
            swift: "nil".to_string(),
        }
    }

    fn can_parse_token_stream_str(tokens: &str) -> bool
    where
        Self: Sized,
    {
        tokens == "String"
    }

    fn from_type(ty: &Type, types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        match ty {
            Type::Path(path) => Self::parse_token_stream_str(
                path.path.segments.to_token_stream().to_string().as_str(),
                types,
            ),
            _ => None,
        }
    }

    fn parse_token_stream_str(_tokens: &str, _types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        Some(BridgedString)
    }

    fn is_null(&self) -> bool {
        false
    }

    fn is_str(&self) -> bool {
        false
    }

    fn contains_owned_string_recursive(&self, _types: &TypeDeclarations) -> bool {
        true
    }

    fn contains_ref_string_recursive(&self) -> bool {
        false
    }

    fn has_swift_bridge_copy_annotation(&self) -> bool {
        false
    }

    fn to_alpha_numeric_underscore_name(&self, _types: &TypeDeclarations) -> String {
        "String".to_string()
    }
}
