use crate::parse::OpaqueCopy;
use proc_macro2::Ident;
use quote::ToTokens;
use std::ops::Deref;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, LitInt, Meta};

#[derive(Default, Clone)]
pub(crate) struct OpaqueTypeAllAttributes {
    pub swift_bridge: OpaqueTypeSwiftBridgeAttributes,
    /// A doc comment.
    // TODO: Use this to generate doc comment for the generated Swift type.
    #[allow(unused)]
    pub doc_comment: Option<String>,
}

#[derive(Default, Clone)]
pub(crate) struct OpaqueTypeSwiftBridgeAttributes {
    /// Whether or not the `#[swift_bridge(already_declared)]` attribute was present on the type.
    /// If it was, we won't generate Swift and C type declarations for this type, since we
    /// will elsewhere.
    pub already_declared: bool,
    /// `#[swift_bridge(Copy(...)]`
    /// Describes the type's Copy semantics.
    pub copy: Option<OpaqueCopy>,
    /// `#[swift_bridge(declare_generic)]`
    /// Used to declare a generic type.
    pub declare_generic: bool,
    /// `#[swift_bridge(Equatable)]`
    /// Used to determine if Equatable need to be implemented.
    pub equatable: bool,
    /// `#[swift_bridge(Hashable)]`
    /// Used to determine if Hashable need to be implemented.
    pub hashable: bool,
    /// `#[swift_bridge(Sendable)]`
    /// Used to determine if the generated type should implement the Swift's `Sendable` protocol or
    /// Rust's `Send+Sync` trait.
    ///
    /// When applied to a Rust type the generated Swift code will implement `Sendable`.
    /// Compile time checks get emitted to ensure that the Rust type is `Send + Sync`.
    ///
    /// When applied to a Swift type the generated Swift code will implement `Send + Sync`.
    /// Compile time checks get emitted to ensure that the Swift type is `Sendable`.
    pub sendable: bool,
    /// `#[swift_bridge(__experimental_swift_ownership)]`
    /// Enables experimental support for Swift ownership.
    /// This attribute will eventually be removed once we've stabilized our support for Swift
    /// ownership.
    /// issue: https://github.com/chinedufn/swift-bridge/issues/155
    pub experimental_swift_ownership: bool,
}

impl OpaqueTypeAllAttributes {
    pub(super) fn from_attributes(attribs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut attributes = OpaqueTypeAllAttributes::default();

        for attr in attribs.iter() {
            let attribute_name = attr.path.to_token_stream().to_string();

            match attribute_name.as_str() {
                "doc" => {
                    let meta = attr.parse_meta()?;
                    let doc = match meta {
                        Meta::NameValue(name_val) => match name_val.lit {
                            syn::Lit::Str(comment) => comment.value(),
                            _ => {
                                todo!("Push parse error that doc attribute is in incorrect format")
                            }
                        },
                        _ => {
                            todo!("Push parse error that doc attribute is in incorrect format")
                        }
                    };

                    attributes.doc_comment = Some(doc);
                }
                "swift_bridge" => {
                    attributes.swift_bridge = attr.parse_args()?;
                }
                _ => todo!("Push unsupported attribute error."),
            };
        }

        Ok(attributes)
    }
}

impl OpaqueTypeSwiftBridgeAttributes {
    pub(super) fn store_attrib(&mut self, attrib: OpaqueTypeAttr) {
        match attrib {
            OpaqueTypeAttr::AlreadyDeclared => self.already_declared = true,
            OpaqueTypeAttr::Copy { size } => self.copy = Some(OpaqueCopy { size_bytes: size }),
            OpaqueTypeAttr::DeclareGeneric => self.declare_generic = true,
            OpaqueTypeAttr::Equatable => self.equatable = true,
            OpaqueTypeAttr::Hashable => self.hashable = true,
            OpaqueTypeAttr::Sendable => self.sendable = true,
            OpaqueTypeAttr::ExperimentalSwiftOwnership => self.experimental_swift_ownership = true,
        }
    }
}

pub(crate) enum OpaqueTypeAttr {
    AlreadyDeclared,
    Copy { size: usize },
    DeclareGeneric,
    Equatable,
    Hashable,
    Sendable,
    ExperimentalSwiftOwnership,
}

impl Parse for OpaqueTypeSwiftBridgeAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = OpaqueTypeSwiftBridgeAttributes::default();

        let punctuated =
            syn::punctuated::Punctuated::<OpaqueTypeAttr, syn::Token![,]>::parse_terminated(input)?;

        for attr in punctuated.into_iter() {
            attributes.store_attrib(attr);
        }

        Ok(attributes)
    }
}

impl Parse for OpaqueTypeAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        let attrib = match key.to_string().as_str() {
            "already_declared" => OpaqueTypeAttr::AlreadyDeclared,
            // Copy(10)
            "Copy" => {
                let content;
                syn::parenthesized!(content in input);

                let size: LitInt = content.parse()?;
                OpaqueTypeAttr::Copy {
                    // TODO: Return an error if the integer cannot be parsed into a usize.
                    size: size.to_string().parse().unwrap(),
                }
            }
            "declare_generic" => OpaqueTypeAttr::DeclareGeneric,
            "Equatable" => OpaqueTypeAttr::Equatable,
            "Hashable" => OpaqueTypeAttr::Hashable,
            "Sendable" => OpaqueTypeAttr::Sendable,
            "__experimental_swift_ownership" => OpaqueTypeAttr::ExperimentalSwiftOwnership,
            _ => {
                let attrib = key.to_string();
                Err(syn::Error::new_spanned(
                    key,
                    format!(r#"Unrecognized attribute "{}"."#, attrib),
                ))?
            }
        };

        Ok(attrib)
    }
}

impl Deref for OpaqueTypeAllAttributes {
    type Target = OpaqueTypeSwiftBridgeAttributes;

    fn deref(&self) -> &Self::Target {
        &self.swift_bridge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::parse_ok;
    use proc_macro2::TokenStream;
    use quote::quote;

    /// Verify that we can parse the `already_declared` attribute.
    #[test]
    fn parse_already_declared_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(already_declared)]
                    type AnotherType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "AnotherType");
        assert!(attribs.already_declared);
    }

    /// Verify that we can parse the `hashable` attribute.
    #[test]
    fn parse_hashable_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(Hashable)]
                    type SomeType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "SomeType");
        assert_eq!(attribs.hashable, true);
    }

    /// Verify that we can parse the `equatable` attribute.
    #[test]
    fn parse_equatable_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(Equatable)]
                    type SomeType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "SomeType");
        assert_eq!(attribs.equatable, true);
    }

    /// Verify that we can parse the `copy` attribute.
    #[test]
    fn parse_copy_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(Copy(4))]
                    type SomeType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "SomeType");
        assert_eq!(attribs.copy.unwrap().size_bytes, 4);
    }

    /// Verify that we can parse multiple atributes from an opaque type.
    #[test]
    fn parse_multiple_attributes() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(already_declared, Copy(4))]
                    type SomeType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "SomeType");

        assert!(attribs.copy.is_some());
        assert!(attribs.already_declared)
    }

    /// Verify that we can parse a doc comment from an extern "Rust" opaque type.
    #[test]
    fn parse_opaque_rust_type_doc_comment() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    /// Some comment
                    type AnotherType;
                }
            }
        };

        let attribs = unwrap_opaque_type_attributes(tokens, "AnotherType");
        assert_eq!(attribs.doc_comment.as_ref().unwrap(), " Some comment");
    }

    /// Verify that we parse a Rust opaque type's experimental Swift ownership attribute.
    #[test]
    fn parse_experimental_swift_ownership_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(__experimental_swift_ownership)]
                    type SomeType;
                }
            }
        };
        let attribs = unwrap_opaque_type_attributes(tokens, "SomeType");

        assert_eq!(attribs.experimental_swift_ownership, true);
    }

    /// Verify that we parse a Rust or Swift opaque type's `Sendable` attribute.
    #[test]
    fn parse_sendable_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(Sendable)]
                    type SomeRustType;
                }

                extern "Swift" {
                    #[swift_bridge(Sendable)]
                    type SomeSwiftType;
                }
            }
        };

        for ty_name in ["SomeRustType", "SomeSwiftType"] {
            let attribs = unwrap_opaque_type_attributes(tokens.clone(), ty_name);
            assert_eq!(attribs.sendable, true);
        }
    }

    fn unwrap_opaque_type_attributes(
        tokens: TokenStream,
        type_name: &'static str,
    ) -> OpaqueTypeAllAttributes {
        let module = parse_ok(tokens);
        module
            .types
            .get(type_name)
            .unwrap()
            .unwrap_opaque()
            .clone()
            .attributes
    }
}
