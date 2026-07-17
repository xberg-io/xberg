use crate::bridged_type::{BridgedType, StdLibType, TypePosition};
use crate::parse::HostLang;
use crate::parsed_extern_fn::SwiftFuncGenerics;
use crate::TypeDeclarations;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashSet;
use std::str::FromStr;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Path, Type};

/// Box<dyn FnOnce(A, B, C) -> ()>
#[derive(Debug)]
pub(crate) struct BridgeableBoxedFnOnce {
    /// The functions parameters.
    pub params: Vec<BridgedType>,
    /// The functions return type.
    pub ret: Box<BridgedType>,
}

/// example: Vec<SomeType, AnotherType, u32>
pub(crate) struct FunctionArguments(pub Vec<Type>);
impl Parse for FunctionArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args: Punctuated<Type, syn::Token![,]> = Punctuated::parse_terminated(input)?;
        Ok(Self(args.into_iter().collect()))
    }
}

impl BridgeableBoxedFnOnce {
    pub fn does_not_have_params_or_return(&self) -> bool {
        self.params.is_empty() && self.ret.is_null()
    }

    /// Box<dyn FnOnce(A, B) -> C>
    pub fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        let args: Vec<TokenStream> = self
            .params
            .iter()
            .map(|a| a.to_rust_type_path(types))
            .collect();
        let ret = &self.ret.to_rust_type_path(types);
        quote! {
            Box<dyn FnOnce(#(#args),*) -> #ret>
        }
    }

    pub fn convert_rust_value_to_ffi_compatible_value(
        &self,
        expression: &TokenStream,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let args: Vec<TokenStream> = self
            .params
            .iter()
            .map(|a| a.to_rust_type_path(types))
            .collect();
        let ret = &self.ret.to_rust_type_path(types);

        quote! {
            Box::into_raw(Box::new(#expression)) as *mut Box<dyn FnOnce(#(#args),*) -> #ret>
        }
    }

    pub fn to_ffi_compatible_rust_type(&self, types: &TypeDeclarations) -> TokenStream {
        let params: Vec<TokenStream> = self
            .params
            .iter()
            .map(|a| a.to_rust_type_path(types))
            .collect();
        let ret = &self.ret.to_rust_type_path(types);
        quote! {
            *mut Box<dyn FnOnce(#(#params),*) -> #ret>
        }
    }

    /// Returns each of the parameters as an FFI friendly type.
    ///
    /// For example, `Box<dyn FnOnce(u8, SomeType)>` would give us:
    /// arg0: u8, arg1: *mut super::SomeType
    pub fn params_to_ffi_compatible_rust_types(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> Vec<TokenStream> {
        self.params
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let param_name = Ident::new(&format!("arg{}", idx), Span::call_site());
                let param_ty = ty.to_ffi_compatible_rust_type(swift_bridge_path, types);

                quote! {
                    #param_name: #param_ty
                }
            })
            .collect()
    }

    /// arg0: UInt8, arg1: SomeType, ...
    pub fn params_to_swift_types(
        &self,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        self.params
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let ty = ty.to_swift_type(
                    TypePosition::FnArg(HostLang::Rust, idx),
                    types,
                    swift_bridge_path,
                );

                format!("_ arg{idx}: {ty}")
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Box<dyn FnOnce(u8, SomeRustType)> becomes:
    /// uint8_t arg0, *void arg1
    pub fn params_to_c_types(&self, types: &TypeDeclarations) -> String {
        self.params
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let ty = ty.to_c(types);

                format!("{ty} arg{idx}")
            })
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Returns each `arg0, arg1, ... argN`.
    ///
    /// For example, `Box<dyn FnOnce(u8, SomeType)>` would give us:
    /// arg0, unsafe { *Box::from_raw(arg1) }
    pub fn to_rust_call_args(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> Vec<TokenStream> {
        self.params
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let arg_name = Ident::new(&format!("arg{}", idx), Span::call_site());
                ty.convert_ffi_expression_to_rust_type(
                    &arg_name.to_token_stream(),
                    arg_name.span(),
                    swift_bridge_path,
                    types,
                )
            })
            .collect()
    }

    /// Returns each `arg0, arg1, ... argN`.
    ///
    /// For example, `Box<dyn FnOnce(u8, SomeType)>` would give us:
    /// "arg0, arg1"
    pub fn to_swift_call_args(&self) -> String {
        self.params
            .iter()
            .enumerate()
            .map(|(idx, _ty)| format!("arg{}", idx))
            .collect::<Vec<String>>()
            .join(", ")
    }

    /// Box<dyn FnOnce(u8, SomeType)> would become:
    /// ", arg0, { arg1.isOwned = false; arg1 }()"
    pub fn to_from_swift_to_rust_ffi_call_args(&self, types: &TypeDeclarations) -> String {
        let mut args = "".to_string();

        if self.params.is_empty() {
            return args;
        }

        for (idx, ty) in self.params.iter().enumerate() {
            let arg_name = format!("arg{}", idx);
            args += &format!(
                ", {}",
                ty.convert_swift_expression_to_ffi_type(
                    &arg_name,
                    types,
                    TypePosition::FnArg(HostLang::Rust, idx)
                )
            );
        }

        args
    }

    pub fn to_swift_type(&self) -> &'static str {
        "UnsafeMutableRawPointer"
    }

    pub fn convert_ffi_value_to_swift_value(&self, type_pos: TypePosition) -> String {
        match type_pos {
            TypePosition::FnArg(_, param_idx) => {
                if self.does_not_have_params_or_return() {
                    format!("{{ cb{param_idx}.call() }}")
                } else if self.params.len() > 0 {
                    let args = self.to_swift_call_args();
                    format!("{{ {args} in cb{param_idx}.call({args}) }}")
                } else {
                    format!("{{ cb{param_idx}.call() }}")
                }
            }
            _ => todo!("Not yet supported"),
        }
    }

    /// Generate the generate bounds for the Swift side.
    /// For example:
    /// "<GenericRustString: IntoRustString>"
    pub fn maybe_swift_generics(&self, types: &TypeDeclarations) -> String {
        let mut maybe_generics = HashSet::new();

        for bridged_arg in &self.params {
            if bridged_arg.contains_owned_string_recursive(types) {
                maybe_generics.insert(SwiftFuncGenerics::String);
            } else if bridged_arg.contains_ref_string_recursive() {
                maybe_generics.insert(SwiftFuncGenerics::Str);
            }
        }

        let maybe_generics = if maybe_generics.is_empty() {
            "".to_string()
        } else {
            let mut m = vec![];

            let generics: Vec<SwiftFuncGenerics> = maybe_generics.into_iter().collect();

            for generic in generics {
                m.push(generic.as_bound())
            }

            format!("<{}>", m.join(", "))
        };

        maybe_generics
    }
}

impl BridgeableBoxedFnOnce {
    pub fn from_str_tokens(string: &str, types: &TypeDeclarations) -> Option<Self> {
        // ( A , B , C ) -> D >
        //   OR
        // ( A , B , C ) >
        let signature = string.trim_start_matches("Box < dyn FnOnce");

        let open_parens = signature.find("(").unwrap();
        let closing_parens = signature.find(")").unwrap();
        // A, B, C
        let args = &signature[open_parens + 1..closing_parens];

        let return_idx = signature.rfind("->");

        // D
        let ret = return_idx.map(|idx| &signature[(idx + 3)..signature.len() - 2]);

        let args = TokenStream::from_str(args).unwrap();
        let args: FunctionArguments = syn::parse2(args).unwrap();

        let ret = if let Some(ret) = ret {
            // Parse out the comma in:
            //   Box<dyn FnOnce() -> (),>
            let ret = ret.trim_end_matches(",");

            let ret = syn::parse2::<Type>(TokenStream::from_str(ret).unwrap()).unwrap();
            BridgedType::new_with_type(&ret, types)?
        } else {
            BridgedType::StdLib(StdLibType::Null)
        };

        let mut args_bridged_tys = Vec::with_capacity(args.0.len());
        for arg in args.0 {
            args_bridged_tys.push(BridgedType::new_with_type(&arg, types)?);
        }

        return Some(BridgeableBoxedFnOnce {
            params: args_bridged_tys,
            ret: Box::new(ret),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that we can parse a boxed fn once that does not have an `->` token
    #[test]
    fn boxed_fn_once_from_string_no_arrow() {
        let tokens = quote! {Box<dyn FnOnce()>}.to_token_stream().to_string();

        assert!(
            BridgeableBoxedFnOnce::from_str_tokens(&tokens, &TypeDeclarations::default())
                .unwrap()
                .ret
                .is_null()
        );
    }

    /// Verify that we can parse a boxed fn once that has an `->` token
    #[test]
    fn boxed_fn_once_from_string_with_arrow() {
        let tokens = quote! {Box<dyn FnOnce() -> u8>}
            .to_token_stream()
            .to_string();

        assert!(matches!(
            *BridgeableBoxedFnOnce::from_str_tokens(&tokens, &TypeDeclarations::default())
                .unwrap()
                .ret,
            BridgedType::StdLib(StdLibType::U8)
        ));
    }

    /// Verify that we can parse a boxed fn once that explicitly returns the null type.
    #[test]
    fn boxed_fn_once_from_string_returns_null() {
        let tokens = quote! {Box<dyn FnOnce() -> ()>}
            .to_token_stream()
            .to_string();

        assert!(
            BridgeableBoxedFnOnce::from_str_tokens(&tokens, &TypeDeclarations::default())
                .unwrap()
                .ret
                .is_null(),
        );
    }

    /// Verify that we can parse a boxed fn that does not have a space before the argument
    /// parentheses.
    /// Not sure what leads to this case.. but if we don't handle it the test suite will fail so
    /// we can always figure out what leads to not having the space before the parens in the future.
    #[test]
    fn no_space_before_arg_parens() {
        let tokens = "Box < dyn FnOnce() -> () >";

        assert!(
            BridgeableBoxedFnOnce::from_str_tokens(tokens, &TypeDeclarations::default())
                .unwrap()
                .ret
                .is_null(),
        );
    }

    /// Verify that we can parse a boxed fn that has a comma after the FnOnce.
    /// rustfmt adds a trailing comma when it puts a long function signature on its own line.
    #[test]
    fn comma_after_fn_once() {
        let tests = vec![
            quote! {Box<dyn FnOnce(),>},
            quote! {Box<dyn FnOnce() -> (),>},
            quote! {
                Box<
                    dyn FnOnce(Result<String, String>),
                >
            },
        ];

        for test in tests {
            let tokens = test.to_token_stream().to_string();

            assert!(
                BridgeableBoxedFnOnce::from_str_tokens(&tokens, &TypeDeclarations::default())
                    .unwrap()
                    .ret
                    .is_null(),
            );
        }
    }
}
