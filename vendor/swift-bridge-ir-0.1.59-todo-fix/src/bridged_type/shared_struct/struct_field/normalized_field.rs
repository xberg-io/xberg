use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::str::FromStr;
use syn::Type;

pub(crate) struct NormalizedStructField {
    pub accessor: NormalizedStructFieldAccessor,
    pub ty: Type,
}

pub(crate) enum NormalizedStructFieldAccessor {
    Named(Ident),
    Unnamed(usize),
}

impl NormalizedStructField {
    /// ```
    /// struct A(
    ///     // name_and_colon for this field is ""
    ///     u8
    /// );
    ///
    /// struct B {
    ///     // name_and_colon for this field is "field: u8"
    ///     field: u8
    /// }
    /// ```
    pub fn maybe_name_and_colon(&self) -> TokenStream {
        match &self.accessor {
            NormalizedStructFieldAccessor::Named(name) => {
                quote! {
                    #name:
                }
            }
            NormalizedStructFieldAccessor::Unnamed(_idx) => {
                quote! {}
            }
        }
    }

    /// Create a string for setting the value of a struct's field.
    ///
    /// Example if named field -> "field_name: someValue".
    /// Example if unnamed field -> "someValue".
    pub fn struct_field_setter_string(&self, value: String) -> String {
        match &self.accessor {
            NormalizedStructFieldAccessor::Named(name) => {
                format!("{}: {}", name.to_string(), value)
            }
            NormalizedStructFieldAccessor::Unnamed(_) => {
                format!("{}", value)
            }
        }
    }

    /// Create a string for setting the value of a struct's ffi field.
    ///
    /// Example if named field -> "field_name: someValue".
    /// Example if unnamed field -> "_0: someValue".
    pub fn struct_ffi_field_setter_string(&self, value: String) -> String {
        match &self.accessor {
            NormalizedStructFieldAccessor::Named(name) => {
                format!("{}: {}", name.to_string(), value)
            }
            NormalizedStructFieldAccessor::Unnamed(idx) => {
                format!("_{}: {}", idx, value)
            }
        }
    }

    /// Access a struct's field
    ///
    /// // Example named field access
    /// val -> val.field
    /// // Example tuple access
    /// val -> val.1
    pub fn append_field_accessor(&self, expression: &TokenStream) -> TokenStream {
        match &self.accessor {
            NormalizedStructFieldAccessor::Named(name) => {
                quote! { #expression.#name }
            }
            NormalizedStructFieldAccessor::Unnamed(idx) => {
                let idx = TokenStream::from_str(&idx.to_string()).unwrap();
                quote! { #expression.#idx }
            }
        }
    }

    pub fn ffi_field_name(&self) -> String {
        match &self.accessor {
            NormalizedStructFieldAccessor::Named(name) => name.to_string(),
            NormalizedStructFieldAccessor::Unnamed(idx) => {
                format!("_{}", idx)
            }
        }
    }
}
