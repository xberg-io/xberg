use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

#[derive(Default)]
pub(super) struct ArgumentAttributes {
    /// LitStr: argument_name
    pub label: Option<LitStr>,
}

enum ArgumentAttr {
    /// LitStr: argument_name
    ArgumentLabel(LitStr),
}

impl Parse for ArgumentAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = ArgumentAttributes::default();
        let punctuated =
            syn::punctuated::Punctuated::<ArgumentAttr, syn::Token![,]>::parse_terminated(input)?;
        for attr in punctuated.into_iter() {
            match attr {
                ArgumentAttr::ArgumentLabel(label) => {
                    attributes.label = Some(label);
                }
            }
        }
        Ok(attributes)
    }
}

impl Parse for ArgumentAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let attribute = match key.to_string().as_str() {
            "label" => {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                ArgumentAttr::ArgumentLabel(value)
            }
            _ => {
                let attrib = key.to_string();
                Err(syn::Error::new_spanned(
                    key,
                    format!(r#"Unrecognized attribute "{}"."#, attrib,),
                ))?
            }
        };
        Ok(attribute)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::parse_ok;
    use quote::{format_ident, quote};

    /// Verify that we can parse a function that has a argument label.
    #[test]
    fn parse_extern_rust_argument_attributes() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    fn some_function(
                        #[swift_bridge(label = "argumentLabel1")] parameter_name1: i32,
                        parameter_name2: String,
                    );
                }
            }
        };

        let module = parse_ok(tokens);
        assert!(module.functions.len() == 1);
        assert_eq!(module.functions[0].argument_labels.len(), 1);
        let argument_label = module.functions[0]
            .argument_labels
            .get(&format_ident!("parameter_name1"))
            .unwrap();
        assert_eq!(argument_label.value().to_string(), "argumentLabel1");
    }
}
