use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token};

/// The `...` in
/// `#\[swift_bridge::bridge(...)\]`
pub struct SwiftBridgeModuleAttrs {
    #[allow(missing_docs)]
    pub attributes: Vec<SwiftBridgeModuleAttr>,
}

/// An attribute within a `#\[swift_bridge::bridge(...)\]`
pub enum SwiftBridgeModuleAttr {
    /// Sets the path for the actual swift bridge crate that contains different helpers such
    /// as `RustString`.
    /// `#\[swift_bridge::bridge(swift_bridge_path = swift_bridge)\]`
    SwiftBridgePath(Path),
}

impl Parse for SwiftBridgeModuleAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(SwiftBridgeModuleAttrs { attributes: vec![] });
        }

        let opts = syn::punctuated::Punctuated::<_, Token![,]>::parse_terminated(input)?;

        Ok(SwiftBridgeModuleAttrs {
            attributes: opts.into_iter().collect(),
        })
    }
}

impl Parse for SwiftBridgeModuleAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let _equals = input.parse::<Token![=]>()?;

        let attr = match key.to_string().as_str() {
            "swift_bridge_path" => SwiftBridgeModuleAttr::SwiftBridgePath(input.parse()?),
            _ => {
                return Err(syn::Error::new(input.span(), "Unknown attribute."));
            }
        };

        Ok(attr)
    }
}
