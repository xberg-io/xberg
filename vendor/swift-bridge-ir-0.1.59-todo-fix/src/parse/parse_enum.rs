use crate::bridged_type::{EnumVariant, SharedEnum, StructFields};
use crate::errors::ParseErrors;
use syn::ItemEnum;

use self::enum_attributes::SharedEnumAllAttributes;

mod enum_attributes;

pub(crate) struct SharedEnumDeclarationParser<'a> {
    pub errors: &'a mut ParseErrors,
    pub item_enum: ItemEnum,
}

impl<'a> SharedEnumDeclarationParser<'a> {
    pub fn parse(self) -> Result<SharedEnum, syn::Error> {
        let item_enum = self.item_enum;

        let attribs = SharedEnumAllAttributes::from_attributes(&item_enum.attrs)?;
        self.errors.append(attribs.errors);

        let mut variants = vec![];

        for v in item_enum.variants {
            let variant = EnumVariant {
                name: v.ident,
                fields: StructFields::from_syn_fields(v.fields),
            };
            variants.push(variant);
        }

        let shared_enum = SharedEnum {
            name: item_enum.ident,
            variants,
            already_declared: attribs.swift_bridge.already_declared,
            swift_name: attribs.swift_bridge.swift_name,
            derive: attribs.derive,
        };

        Ok(shared_enum)
    }
}

#[cfg(test)]
mod tests {
    use crate::bridged_type::StructFields;
    use crate::errors::ParseError;
    use crate::test_utils::{parse_errors, parse_ok};
    use quote::{quote, ToTokens};

    /// Verify that we can parse an enum with no variants.
    #[test]
    fn parse_enum_no_variants() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types().len(), 1);
        let ty = &module.types.types()[0].unwrap_shared_enum();
        assert_eq!(ty.name, "SomeEnum");
    }

    /// Verify that we can parse an enum with one variant.
    #[test]
    fn enum_with_one_variant() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    SomeVariant
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types().len(), 1);

        let ty = &module.types.types()[0].unwrap_shared_enum();
        assert_eq!(ty.variants.len(), 1);

        assert_eq!(ty.variants[0].name, "SomeVariant");
    }

    /// Verify that we can parse an enum that has a variant that has a field.
    #[test]
    fn parse_enum_variant_field() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    SomeVariant(u8)
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types().len(), 1);

        let ty = &module.types.types()[0].unwrap_shared_enum();
        assert_eq!(ty.variants[0].fields.normalized_fields().len(), 1);

        match &ty.variants[0].fields {
            StructFields::Unnamed(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].ty.to_token_stream().to_string(), "u8");
            }
            _ => panic!(),
        }
    }

    /// Verify that we can parse the `#[swift_bridge(swift_name = "...")`] attribute.
    #[test]
    fn swift_name_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "FfiFoo")]
                enum Foo {
                    Variant1
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_enum();
        assert_eq!(ty.swift_name.as_ref().unwrap().value(), "FfiFoo");
    }

    /// Verify that we can parse the `#[swift_bridge(already_declared)`] attribute.
    #[test]
    fn already_declared_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(already_declared)]
                enum SomeEnum {}
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types().len(), 1);

        let ty = &module.types.types()[0].unwrap_shared_enum();
        assert!(ty.already_declared);
    }

    /// Verify that we return an error if an attribute isn't recognized.
    #[test]
    fn error_if_attribute_unrecognized() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(unrecognized, invalid_attribute = "hi")]
                enum SomeEnum{
                    Variant
                }
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 2);

        match &errors[0] {
            ParseError::EnumUnrecognizedAttribute { attribute } => {
                assert_eq!(&attribute.to_string(), "unrecognized");
            }
            _ => panic!(),
        };
        match &errors[1] {
            ParseError::EnumUnrecognizedAttribute { attribute } => {
                assert_eq!(&attribute.to_string(), "invalid_attribute");
            }
            _ => panic!(),
        };
    }

    /// Verify that we can parse #[derive(Debug)] on enums
    #[test]
    fn derive_debug() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[derive(Debug)]
                enum Foo {
                    Variant1
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = module.types.types()[0].unwrap_shared_enum();
        assert!(ty.derive.debug);
    }
}
