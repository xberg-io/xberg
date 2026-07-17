use crate::bridge_module_attributes::CfgAttr;
use crate::bridged_type::BridgedType;
use crate::errors::{ParseError, ParseErrors};
use crate::parse::parse_enum::SharedEnumDeclarationParser;
use crate::parse::parse_extern_mod::ForeignModParser;
use crate::parse::parse_struct::SharedStructDeclarationParser;
use crate::SwiftBridgeModule;
use proc_macro2::TokenTree;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Item, ItemMod, Token};

mod parse_enum;
mod parse_extern_mod;
mod parse_struct;

mod type_declarations;
pub(crate) use self::type_declarations::*;

impl Parse for SwiftBridgeModule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let module_and_errors: SwiftBridgeModuleAndErrors = input.parse()?;

        module_and_errors.errors.combine_all()?;

        Ok(module_and_errors.module)
    }
}

pub(crate) struct SwiftBridgeModuleAndErrors {
    pub module: SwiftBridgeModule,
    pub errors: ParseErrors,
}

/// The language that a bridge type or function's implementation lives in.
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum HostLang {
    /// The type or function is defined Rust.
    Rust,
    /// The type or function is defined Swift.
    Swift,
}

impl HostLang {
    pub fn is_rust(&self) -> bool {
        matches!(self, HostLang::Rust)
    }

    pub fn is_swift(&self) -> bool {
        matches!(self, HostLang::Swift)
    }
}

impl Parse for SwiftBridgeModuleAndErrors {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut errors = ParseErrors::new();

        if let Ok(item_mod) = input.parse::<ItemMod>() {
            let module_name = item_mod.ident;
            let vis = item_mod.vis;

            let mut functions = vec![];
            let mut type_declarations = TypeDeclarations::default();
            let mut unresolved_types = vec![];
            let mut cfg_attrs = vec![];

            for attr in item_mod.attrs {
                match attr.path.to_token_stream().to_string().as_str() {
                    "cfg" => {
                        let cfg: CfgAttr = syn::parse2(attr.tokens)?;
                        cfg_attrs.push(cfg);
                    }
                    _ => {}
                };
            }

            for outer_mod_item in item_mod.content.unwrap().1 {
                match outer_mod_item {
                    Item::ForeignMod(foreign_mod) => {
                        ForeignModParser {
                            errors: &mut errors,
                            type_declarations: &mut type_declarations,
                            functions: &mut functions,
                            unresolved_types: &mut unresolved_types,
                        }
                        .parse(foreign_mod)?;
                    }
                    Item::Struct(item_struct) => {
                        let shared_struct = SharedStructDeclarationParser {
                            item_struct,
                            errors: &mut errors,
                        }
                        .parse()?;
                        type_declarations.insert(
                            shared_struct.name.to_string(),
                            TypeDeclaration::Shared(SharedTypeDeclaration::Struct(shared_struct)),
                        );
                    }
                    Item::Enum(item_enum) => {
                        let shared_enum = SharedEnumDeclarationParser {
                            item_enum,
                            errors: &mut errors,
                        }
                        .parse()?;
                        type_declarations.insert(
                            shared_enum.name.to_string(),
                            TypeDeclaration::Shared(SharedTypeDeclaration::Enum(shared_enum)),
                        );
                    }
                    invalid_item => {
                        let error = ParseError::InvalidModuleItem { item: invalid_item };
                        errors.push(error);
                    }
                };
            }

            for unresolved_type in unresolved_types.into_iter() {
                if BridgedType::new_with_type(&unresolved_type, &type_declarations).is_some() {
                    continue;
                }

                errors.push(ParseError::UndeclaredType {
                    ty: unresolved_type.clone(),
                });
            }

            let module = SwiftBridgeModule {
                name: module_name,
                vis,
                types: type_declarations,
                functions,
                swift_bridge_path: syn::parse2(quote! { swift_bridge }).unwrap(),
                cfg_attrs,
            };
            Ok(SwiftBridgeModuleAndErrors { module, errors })
        } else {
            return Err(syn::Error::new_spanned(
                input.to_string(),
                "Only modules are supported.",
            ));
        }
    }
}

// Used to fast-forward our attribute parsing to the next attribute when we've run into an
// issue parsing the current attribute.
fn move_input_cursor_to_next_comma(input: ParseStream) {
    if !input.peek(Token![,]) {
        let _ = input.step(|cursor| {
            let mut current_cursor = *cursor;

            while let Some((tt, next)) = current_cursor.token_tree() {
                match &tt {
                    TokenTree::Punct(punct) if punct.as_char() == ',' => {
                        return Ok(((), current_cursor));
                    }
                    _ => current_cursor = next,
                }
            }

            Ok(((), current_cursor))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{parse_errors, parse_ok};

    /// Verify that we can parse a cfg feature from a module.
    #[test]
    fn parse_module_cfg_feature() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            #[cfg(feature = "some-feature")]
            mod foo {}
        };

        let module = parse_ok(tokens);

        assert_eq!(module.cfg_attrs.len(), 1);

        match &module.cfg_attrs[0] {
            CfgAttr::Feature(feature) => {
                assert_eq!(feature.value(), "some-feature")
            }
        };
    }

    /// Verify that we get an error when parsing an unsupported module item, such as a
    /// `use` statement.
    #[test]
    fn invalid_module_item() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod foo {
                use std;
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ParseError::InvalidModuleItem { item } => {
                assert!(matches!(item, Item::Use(_)))
            }
            _ => panic!(),
        }
    }
}
