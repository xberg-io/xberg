use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::LitStr;
use syn::Token;

/// A `cfg` attribute on a bridge module.
///
/// ```no_run
/// #[swift_bridge::bridge]
/// // This is a cfg attribute.
/// #[cfg(feature = "some-feature")]
/// mod ffi {
/// }
/// ```
pub enum CfgAttr {
    /// #\[cfg(feature = "...")\]
    Feature(LitStr),
}

impl Parse for CfgAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let ident: Ident = content.parse()?;

        if &ident == "feature" {
            content.parse::<Token![=]>()?;

            let feature_name = content.parse::<LitStr>()?;

            Ok(CfgAttr::Feature(feature_name))
        } else {
            todo!("Return an unsupported cfg kind error")
        }
    }
}
