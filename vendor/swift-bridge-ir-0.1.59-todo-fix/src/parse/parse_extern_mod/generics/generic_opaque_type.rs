use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Generics, Token};

pub(crate) struct GenericOpaqueType {
    pub attributes: Vec<Attribute>,
    #[allow(unused)]
    pub type_token: Token![type],
    pub ident: Ident,
    #[allow(unused)]
    pub generics: Generics,
    #[allow(unused)]
    pub semicolon: Token![;],
}

impl Parse for GenericOpaqueType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = Vec::new();

        if input.peek(Token![#]) {
            attributes = input.call(Attribute::parse_outer)?;
        }

        Ok(GenericOpaqueType {
            attributes,
            type_token: input.parse()?,
            ident: input.parse()?,
            generics: input.parse()?,
            semicolon: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::parse_ok;
    use quote::quote;

    /// Verify that we can parse a generic opaque type that has attributes.
    /// We use a type that has a `#[swift_bridge(...)]` attribute as well as
    /// a `#[doc(...)]` attribute
    #[test]
    fn parse_generic_opaque_type_with_attributes() {
        let tokens = quote! {
            #[swift_bridge(Copy(4))]
            /// Doc attribute.
            type SomeType<u32>;
        };

        let generics: GenericOpaqueType = syn::parse2(tokens).unwrap();

        assert_eq!(generics.attributes.len(), 2);
    }

    /// Verify that we can parse an opaque Rust type with a `#[swift_bridge(declare_generic)]`
    /// attribute.
    #[test]
    fn parses_declare_generic_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type MyType<A>;
                }
            }
        };
        let module = parse_ok(tokens);
        assert!(
            module.types.types()[0]
                .as_opaque()
                .unwrap()
                .attributes
                .declare_generic
        );
    }

    /// Verify that we can parse a function that has a generic opaque type argument.
    #[test]
    fn parse_function_with_generic_opaque_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type MyType<A>;
                    type MyType<u32>;

                    fn some_function(arg: &MyType<u32>);
                }
            }
        };
        let module = parse_ok(tokens);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].sig.inputs.len(), 1);
    }

    /// Verify that we can parse a function that has a generic opaque type argument.
    #[test]
    fn parse_method_with_generic_opaque_type_self() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type MyType<A>;
                    type MyType<u32>;

                    fn some_function(self: MyType<u32>);
                }
            }
        };
        let module = parse_ok(tokens);
        assert_eq!(module.functions.len(), 1);

        let ty = &module.functions[0]
            .associated_type
            .as_ref()
            .unwrap()
            .as_opaque()
            .unwrap()
            .ty;
        assert_eq!(ty.to_string(), "MyType");
    }
}
