use crate::bridged_type::{
    shared_struct::StructDerives, SharedStruct, StructFields, StructSwiftRepr,
};
use crate::errors::{ParseError, ParseErrors};
use crate::parse::move_input_cursor_to_next_comma;
use proc_macro2::Ident;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{ItemStruct, LitStr, Meta, Token};

pub(crate) struct SharedStructDeclarationParser<'a> {
    pub item_struct: ItemStruct,
    pub errors: &'a mut ParseErrors,
}

enum StructAttr {
    SwiftRepr((StructSwiftRepr, LitStr)),
    SwiftName(LitStr),
    Error(StructAttrParseError),
    AlreadyDeclared,
}

enum StructAttrParseError {
    InvalidSwiftRepr(LitStr),
    UnrecognizedAttribute(Ident),
}

#[derive(Default)]
struct StructAttribs {
    swift_repr: Option<(StructSwiftRepr, LitStr)>,
    swift_name: Option<LitStr>,
    already_declared: bool,
    derives: StructDerives,
}

impl Default for StructDerives {
    fn default() -> Self {
        StructDerives {
            copy: false,
            clone: false,
        }
    }
}

struct ParsedAttribs(Vec<StructAttr>);
impl Parse for ParsedAttribs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ParsedAttribs(vec![]));
        }

        let opts = syn::punctuated::Punctuated::<_, syn::token::Comma>::parse_terminated(input)?;

        Ok(ParsedAttribs(opts.into_iter().collect()))
    }
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        let attr = match key.to_string().as_str() {
            "swift_repr" => {
                input.parse::<Token![=]>()?;

                let repr: LitStr = input.parse()?;
                match repr.value().as_str() {
                    "class" => StructAttr::SwiftRepr((StructSwiftRepr::Class, repr)),
                    "struct" => StructAttr::SwiftRepr((StructSwiftRepr::Structure, repr)),
                    _ => StructAttr::Error(StructAttrParseError::InvalidSwiftRepr(repr)),
                }
            }
            "swift_name" => {
                input.parse::<Token![=]>()?;

                let name = input.parse()?;
                StructAttr::SwiftName(name)
            }
            "already_declared" => StructAttr::AlreadyDeclared,
            _ => {
                move_input_cursor_to_next_comma(input);
                StructAttr::Error(StructAttrParseError::UnrecognizedAttribute(key))
            }
        };

        Ok(attr)
    }
}

impl<'a> SharedStructDeclarationParser<'a> {
    pub fn parse(self) -> Result<SharedStruct, syn::Error> {
        let item_struct = self.item_struct;

        let mut attribs = StructAttribs::default();

        for attr in item_struct.attrs {
            let attribute_name = attr.path.to_token_stream().to_string();

            match attribute_name.as_str() {
                "swift_bridge" => {
                    let sections: ParsedAttribs = attr.parse_args()?;

                    for attr in sections.0 {
                        match attr {
                            StructAttr::SwiftRepr((repr, lit_str)) => {
                                attribs.swift_repr = Some((repr, lit_str));
                            }
                            StructAttr::SwiftName(name) => {
                                attribs.swift_name = Some(name);
                            }
                            StructAttr::Error(err) => match err {
                                StructAttrParseError::InvalidSwiftRepr(val) => {
                                    self.errors.push(ParseError::StructInvalidSwiftRepr {
                                        swift_repr_attr_value: val.clone(),
                                    });
                                    attribs.swift_repr = Some((StructSwiftRepr::Structure, val));
                                }
                                StructAttrParseError::UnrecognizedAttribute(attribute) => {
                                    self.errors.push(ParseError::StructUnrecognizedAttribute {
                                        attribute,
                                    });
                                }
                            },
                            StructAttr::AlreadyDeclared => {
                                attribs.already_declared = true;
                            }
                        };
                    }
                }
                "derive" => match attr.parse_meta()? {
                    Meta::List(meta_list) => {
                        for derive in meta_list.nested {
                            match derive.to_token_stream().to_string().as_str() {
                                "Copy" => attribs.derives.copy = true,
                                "Clone" => attribs.derives.clone = true,
                                _ => {}
                            }
                        }
                    }
                    _ => todo!("Push parse error that derive attribute is in incorrect format"),
                },
                attr_name => {
                    todo!(
                        "Push unsupported attribute error. Found unsupported attribute \"{}\" on struct \"{}\". Consult the swift-bridge manual for supported struct attributes.",
                        attr_name,
                        item_struct.ident.to_string(),
                    )
                }
            }
        }

        let swift_repr = if item_struct.fields.len() == 0 {
            if let Some((swift_repr, lit_str)) = attribs.swift_repr {
                if swift_repr == StructSwiftRepr::Class {
                    self.errors.push(ParseError::EmptyStructHasSwiftReprClass {
                        struct_ident: item_struct.ident.clone(),
                        swift_repr_attr_value: lit_str,
                    });
                }
            }

            StructSwiftRepr::Structure
        } else if let Some((swift_repr, _)) = attribs.swift_repr {
            swift_repr
        } else {
            self.errors.push(ParseError::StructMissingSwiftRepr {
                struct_ident: item_struct.ident.clone(),
            });

            StructSwiftRepr::Structure
        };

        let shared_struct = SharedStruct {
            name: item_struct.ident,
            swift_repr,
            fields: StructFields::from_syn_fields(item_struct.fields),
            swift_name: attribs.swift_name,
            already_declared: attribs.already_declared,
            derives: attribs.derives,
        };

        Ok(shared_struct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{parse_errors, parse_ok};
    use quote::quote;

    /// Verify that we can parse a struct with no fields.
    /// Structs with no fields always have an implicit `swift_repr = "struct"`.
    #[test]
    fn parse_unit_struct() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                struct Foo;
                struct Bar();
                struct Bazz {}
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types().len(), 3);
        for (idx, name) in vec!["Foo", "Bar", "Bazz"].into_iter().enumerate() {
            let ty = &module.types.types()[idx].unwrap_shared_struct();

            assert_eq!(ty.name, name);
            assert_eq!(ty.swift_repr, StructSwiftRepr::Structure);
        }
    }

    /// Verify that we get an error if a Struct has one or more fields and no `swift_repr`
    /// attribute.
    #[test]
    fn error_if_missing_swift_repr() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                struct Foo {
                    bar: u8
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ParseError::StructMissingSwiftRepr { struct_ident } => {
                assert_eq!(struct_ident, "Foo");
            }
            _ => panic!(),
        };
    }

    /// Verify that we push an error if the Swift representation attribute is invalid.
    #[test]
    fn error_if_invalid_swift_repr() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "an-invalid-value")]
                struct Foo {
                    bar: u8
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ParseError::StructInvalidSwiftRepr {
                swift_repr_attr_value,
            } => {
                assert_eq!(swift_repr_attr_value.value(), "an-invalid-value");
            }
            _ => panic!(),
        };
    }

    /// Verify that we push an error if a struct with no fields has it's swift_repr set to "class",
    /// since there is no advantage to bearing that extra overhead.
    #[test]
    fn error_if_empty_struct_swift_repr_set_to_class() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "class")]
                struct Foo;

                #[swift_bridge(swift_repr = "class")]
                struct Bar;

                #[swift_bridge(swift_repr = "class")]
                struct Buzz;
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 3);

        for (idx, struct_name) in vec!["Foo", "Bar", "Buzz"].into_iter().enumerate() {
            match &errors[idx] {
                ParseError::EmptyStructHasSwiftReprClass {
                    struct_ident,
                    swift_repr_attr_value,
                } => {
                    assert_eq!(struct_ident, struct_name);
                    assert_eq!(swift_repr_attr_value.value(), "class");
                }
                _ => panic!(),
            };
        }
    }

    /// Verify that we can parse a struct with a named field.
    #[test]
    fn parse_struct_with_named_u8_field() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_repr = "struct")]
                struct Foo {
                    bar: u8
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();
        match &ty.fields {
            StructFields::Named(fields) => {
                let field = &fields[0];

                assert_eq!(field.name, "bar");
                assert_eq!(field.ty.to_token_stream().to_string(), "u8");
            }
            _ => panic!(),
        };
    }

    /// Verify that we parse the swift_name = "..."
    #[test]
    fn parse_swift_name_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "FfiFoo")]
                struct Foo;
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();
        assert_eq!(ty.swift_name.as_ref().unwrap().value(), "FfiFoo");
    }

    /// Verify that we parse the derive(...)
    #[test]
    fn parse_derive_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[derive(Copy, Clone)]
                struct Foo;

                #[derive(Clone)]
                struct Bar;
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();

        assert_eq!(ty.derives.copy, true);
        assert_eq!(ty.derives.clone, true);

        let ty2 = module.types.types()[1].unwrap_shared_struct();

        assert_eq!(ty2.derives.copy, false);
        assert_eq!(ty2.derives.clone, true);
    }

    /// Verify that we properly parse multiple comma separated struct attributes.
    #[test]
    fn parses_multiple_struct_attributes() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "FfiFoo", swift_repr = "class")]
                struct Foo {
                    fied: u8
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();
        assert_eq!(ty.swift_name.as_ref().unwrap().value(), "FfiFoo");
        assert_eq!(ty.swift_repr, StructSwiftRepr::Class);
    }

    /// Verify that we properly parse multiple comma separated struct attributes and derive attributes.
    #[test]
    fn parses_multiple_struct_attributes_and_derive() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "FfiFoo", swift_repr = "class")]
                #[derive(Copy, Clone)]
                struct Foo {
                    fied: u8
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();
        assert_eq!(ty.swift_name.as_ref().unwrap().value(), "FfiFoo");
        assert_eq!(ty.swift_repr, StructSwiftRepr::Class);
        assert_eq!(ty.derives.copy, true);
        assert_eq!(ty.derives.clone, true);
    }

    /// Verify that we can parse an `already_defined = "struct"` attribute.
    #[test]
    fn parses_struct_already_declared_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(already_declared, swift_repr = "struct")]
                struct SomeType;
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_struct();
        assert!(ty.already_declared);
    }

    /// Verify that we return an error if an attribute isn't recognized.
    #[test]
    fn error_if_attribute_unrecognized() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(unrecognized, invalid_attribute = "hi", swift_repr = "struct")]
                struct SomeType;
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 2);

        match &errors[0] {
            ParseError::StructUnrecognizedAttribute { attribute } => {
                assert_eq!(&attribute.to_string(), "unrecognized");
            }
            _ => panic!(),
        };
        match &errors[1] {
            ParseError::StructUnrecognizedAttribute { attribute } => {
                assert_eq!(&attribute.to_string(), "invalid_attribute");
            }
            _ => panic!(),
        };
    }
}
