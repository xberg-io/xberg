use crate::bridged_type::DeriveAttrs;
use crate::errors::ParseError;
use crate::parse::move_input_cursor_to_next_comma;
use proc_macro2::Ident;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, LitStr, Path, Token};

#[derive(Default)]
pub(super) struct SharedEnumAllAttributes {
    pub errors: Vec<ParseError>,
    pub swift_bridge: SharedEnumSwiftBridgeAttributes,
    pub derive: DeriveAttrs,
}

impl SharedEnumAllAttributes {
    pub(super) fn from_attributes(attribs: &[Attribute]) -> Result<Self, syn::Error> {
        let mut attributes = SharedEnumAllAttributes::default();

        for attr in attribs {
            let attribute_name = attr.path.to_token_stream().to_string();

            match attribute_name.as_str() {
                "derive" => {
                    let parser =
                        syn::punctuated::Punctuated::<Path, syn::Token![,]>::parse_terminated;
                    let args = attr.parse_args_with(parser)?;
                    for arg in args.into_iter() {
                        match arg.get_ident().unwrap().to_string().as_str() {
                            "Debug" => attributes.derive.debug = true,
                            _ => todo!("Unsupported derive macro; please see https://github.com/chinedufn/swift-bridge/issues/190#issuecomment-1463234027"),
                        }
                    }
                }
                "swift_bridge" => {
                    attributes.swift_bridge = attr.parse_args()?;
                    attributes
                        .errors
                        .append(&mut attributes.swift_bridge.errors);
                }
                _ => todo!("Push unsupported attribute error."),
            };
        }

        Ok(attributes)
    }
}

pub(super) enum EnumAttr {
    AlreadyDeclared,
    Error(ParseError),
    SwiftName(LitStr),
}

#[derive(Default)]
pub(super) struct SharedEnumSwiftBridgeAttributes {
    pub errors: Vec<ParseError>,
    pub already_declared: bool,
    pub swift_name: Option<LitStr>,
}

impl SharedEnumSwiftBridgeAttributes {
    pub(super) fn store_attrib(&mut self, attrib: EnumAttr) -> syn::Result<()> {
        match attrib {
            EnumAttr::AlreadyDeclared => self.already_declared = true,
            EnumAttr::Error(error) => self.errors.push(error),
            EnumAttr::SwiftName(name) => self.swift_name = Some(name),
        };
        Ok(())
    }
}

impl Parse for SharedEnumSwiftBridgeAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = SharedEnumSwiftBridgeAttributes::default();

        let punctuated =
            syn::punctuated::Punctuated::<EnumAttr, syn::Token![,]>::parse_terminated(input)?;

        for attr in punctuated.into_iter() {
            attributes.store_attrib(attr)?;
        }

        Ok(attributes)
    }
}

impl Parse for EnumAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        let attr = match key.to_string().as_str() {
            "already_declared" => EnumAttr::AlreadyDeclared,
            "swift_name" => {
                input.parse::<Token![=]>()?;

                let name = input.parse()?;
                EnumAttr::SwiftName(name)
            }
            _ => {
                move_input_cursor_to_next_comma(input);
                EnumAttr::Error(ParseError::EnumUnrecognizedAttribute { attribute: key })
            }
        };

        Ok(attr)
    }
}
