use crate::parsed_extern_fn::{GetField, GetFieldDirect, GetFieldWith};
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Path, Token};

#[derive(Default)]
pub(super) struct FunctionAttributes {
    pub associated_to: Option<Ident>,
    pub is_swift_initializer: bool,
    pub is_swift_identifiable: bool,
    pub rust_name: Option<LitStr>,
    pub swift_name: Option<LitStr>,
    pub return_into: bool,
    pub return_with: Option<Path>,
    pub args_into: Option<Vec<Ident>>,
    pub get_field: Option<GetField>,
}

impl FunctionAttributes {
    pub fn store_attrib(&mut self, attrib: FunctionAttr) {
        match attrib {
            FunctionAttr::AssociatedTo(ident) => {
                self.associated_to = Some(ident);
            }
            FunctionAttr::Init => self.is_swift_initializer = true,
            FunctionAttr::RustName(name) => {
                self.rust_name = Some(name);
            }
            FunctionAttr::SwiftName(name) => {
                self.swift_name = Some(name);
            }
            FunctionAttr::ReturnInto => {
                self.return_into = true;
            }
            FunctionAttr::ReturnWith(path) => {
                self.return_with = Some(path);
            }
            FunctionAttr::ArgsInto(args) => self.args_into = Some(args),
            FunctionAttr::Identifiable => {
                self.is_swift_identifiable = true;
            }
            FunctionAttr::GetField(get_field) => self.get_field = Some(GetField::Direct(get_field)),
            FunctionAttr::GetFieldWith(get_field) => {
                self.get_field = Some(GetField::With(get_field))
            }
        }
    }
}

pub(super) enum FunctionAttr {
    AssociatedTo(Ident),
    SwiftName(LitStr),
    RustName(LitStr),
    Init,
    Identifiable,
    ReturnInto,
    ReturnWith(Path),
    ArgsInto(Vec<Ident>),
    GetField(GetFieldDirect),
    GetFieldWith(GetFieldWith),
}

impl Parse for FunctionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = FunctionAttributes::default();

        let punctuated =
            syn::punctuated::Punctuated::<FunctionAttr, syn::Token![,]>::parse_terminated(input)?;

        for attr in punctuated.into_iter() {
            attributes.store_attrib(attr);
        }

        Ok(attributes)
    }
}

impl Parse for FunctionAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;

        let attrib = match key.to_string().as_str() {
            // TODO: Replace this with "static_method_of" before we release 0.2.0.
            "associated_to" => {
                input.parse::<Token![=]>()?;
                let value: Ident = input.parse()?;

                FunctionAttr::AssociatedTo(value)
            }
            "swift_name" => {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;

                FunctionAttr::SwiftName(value)
            }
            "init" => FunctionAttr::Init,
            "Identifiable" => FunctionAttr::Identifiable,
            // TODO: Right before we release 0.2.0 we should remove this
            //  "into_return_type" variant since it is deprecated.
            //
            "return_into" | "into_return_type" => FunctionAttr::ReturnInto,
            "return_with" => {
                input.parse::<Token![=]>()?;
                FunctionAttr::ReturnWith(input.parse()?)
            }
            "rust_name" => {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;

                FunctionAttr::RustName(value)
            }
            "args_into" => {
                input.parse::<Token![=]>()?;

                let content;
                syn::parenthesized!(content in input);

                let args = syn::punctuated::Punctuated::<_, Token![,]>::parse_terminated(&content)?;
                FunctionAttr::ArgsInto(args.into_iter().collect())
            }
            "get" => {
                let content;
                syn::parenthesized!(content in input);

                let maybe_ref = content.parse::<Token![&]>().ok();
                let maybe_mut = content.parse::<Token![mut]>().ok();
                let field_name = content.parse::<Ident>()?;

                FunctionAttr::GetField(GetFieldDirect {
                    maybe_ref,
                    maybe_mut,
                    field_name,
                })
            }
            "get_with" => {
                let content;
                syn::parenthesized!(content in input);

                let maybe_ref = content.parse::<Token![&]>().ok();
                let maybe_mut = content.parse::<Token![mut]>().ok();
                let field_name = content.parse::<Ident>()?;
                content.parse::<Token![=]>()?;
                let path = content.parse::<Path>()?;

                FunctionAttr::GetFieldWith(GetFieldWith {
                    maybe_ref,
                    maybe_mut,
                    field_name,
                    path,
                })
            }
            _ => {
                let attrib = key.to_string();
                Err(syn::Error::new_spanned(
                    key,
                    format!(r#"Unrecognized attribute "{}"."#, attrib,),
                ))?
            }
        };

        Ok(attrib)
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::{FunctionAttributeParseError, IdentifiableParseError, ParseError};
    use crate::parsed_extern_fn::FailableInitializerType;
    use crate::test_utils::{parse_errors, parse_ok};
    use quote::{quote, ToTokens};

    /// Verify that we can parse the return_into attribute from extern "Rust" blocks.
    #[test]
    fn parse_extern_rust_return_into_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(return_into)]
                    fn some_function () -> Foo;
                }
            }
        };

        let module = parse_ok(tokens);

        assert!(module.functions[0].return_into);
    }

    /// Verify that we can parse the return_with attribute from extern "Rust" blocks.
    #[test]
    fn parse_extern_rust_return_with_attribute() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(return_with = path::to::convert_fn)]
                    fn some_function () -> u32;
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(
            module.functions[0]
                .return_with
                .to_token_stream()
                .to_string(),
            quote! {
                path::to::convert_fn
            }
            .to_string()
        );
    }

    /// Verify that we can parse an associated function.
    #[test]
    fn parse_associated_function() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(associated_to = Foo)]
                    fn bar ();
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = &module.types.types()[0].unwrap_opaque();
        assert_eq!(ty.to_string(), "Foo");

        assert_eq!(module.functions.len(), 1,);
    }

    /// Verify that we can parse an associated function that has arguments.
    #[test]
    fn associated_function_with_args() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(associated_to = Foo)]
                    fn bar (arg: u8);
                }
            }
        };

        let module = parse_ok(tokens);

        let ty = &module.types.types()[0].unwrap_opaque();
        assert_eq!(ty.to_string(), "Foo");

        assert_eq!(module.functions.len(), 1,);
    }

    /// Verify that we can parse an init function.
    #[test]
    fn initializer() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn bar () -> Foo;
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];
        assert!(func.is_swift_initializer);
    }

    /// Verify that we can parse an failable init function.
    #[test]
    fn failable_initializer() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn bar () -> Option<Foo>;
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];
        assert!(func.is_swift_initializer);
        matches!(
            func.swift_failable_initializer,
            Some(FailableInitializerType::Option)
        );
    }

    /// Verify that we can parse an throwing init function.
    #[test]
    fn throwing_initializer() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn bar () -> Result<Foo, i32>;
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];
        assert!(func.is_swift_initializer);
        matches!(
            func.swift_failable_initializer,
            Some(FailableInitializerType::Throwing)
        );
    }

    /// Verify that we can parse an init function that takes inputs.
    #[test]
    fn initializer_with_inputs() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(init)]
                    fn bar (bazz: u8) -> Foo;
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];
        assert!(func.is_swift_initializer);
    }

    /// Verify that we push an error if the initialize type is not defined.
    #[test]
    fn error_if_initialized_type_not_defined() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    #[swift_bridge(init)]
                    fn bar () -> Foo;
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ParseError::UndeclaredType { ty } => {
                assert_eq!(ty.to_token_stream().to_string(), "Foo")
            }
            _ => panic!(),
        }
    }

    /// Verify that we can parse a from attribute for a struct.
    #[test]
    fn parses_extern_rust_args_into_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(args_into = (some_arg, another_arg))]
                    fn some_function(some_arg: u8, another_arg: u16);
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];

        let args_into = func.args_into.as_ref().unwrap();
        assert_eq!(args_into.len(), 2);

        let assert_arg_into = |arg_name: &str| {
            assert!(args_into
                .iter()
                .find(|arg| { &arg.to_string() == arg_name })
                .is_some());
        };

        assert_arg_into("some_arg");
        assert_arg_into("another_arg");
    }

    /// Verify that we push a parse error if we put an Identifiable attribute on a function
    /// that isn't `(&self)`.
    #[test]
    fn error_if_identifiable_attribute_on_non_ref_self() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    #[swift_bridge(Identifiable)]
                    fn a(self) -> u16;

                    #[swift_bridge(Identifiable)]
                    fn b(&mut self) -> u16;

                    #[swift_bridge(Identifiable)]
                    fn c() -> u16;

                    #[swift_bridge(Identifiable)]
                    fn d(arg: i32) -> u16;
                }
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 4);

        for (idx, expected) in vec!["a", "b", "c", "d"].into_iter().enumerate() {
            match &errors[idx] {
                ParseError::FunctionAttribute(FunctionAttributeParseError::Identifiable(
                    IdentifiableParseError::MustBeRefSelf { fn_ident },
                )) => {
                    assert_eq!(fn_ident, expected);
                }
                _ => panic!(),
            };
        }
    }

    /// Verify that we push a parse error if we put an Identifiable attribute on a method
    /// that does not have an explicit return value.
    #[test]
    fn error_if_identifiable_attribute_on_non_returning_method() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    #[swift_bridge(Identifiable)]
                    fn a(&self);

                    #[swift_bridge(Identifiable)]
                    fn b(self: &SomeType);
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 2);

        for (idx, expected) in vec!["a", "b"].into_iter().enumerate() {
            match &errors[idx] {
                ParseError::FunctionAttribute(FunctionAttributeParseError::Identifiable(
                    IdentifiableParseError::MissingReturnType { fn_ident },
                )) => {
                    assert_eq!(fn_ident, expected);
                }
                _ => panic!(),
            };
        }
    }

    /// Verify that we can parse the `Identifiable` attribute
    #[test]
    fn parses_identifiable_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type SomeType;

                    #[swift_bridge(Identifiable)]
                    fn some_function(&self) -> u16;
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];

        assert!(func.is_swift_identifiable);
    }

    /// Verify that we can parse the `get` attribute.
    #[test]
    fn parses_get_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(get(field))]
                    fn some_function(&self);

                    #[swift_bridge(get(&field))]
                    fn some_function_ref(&self);

                    #[swift_bridge(get(&mut field))]
                    fn some_function_ref_mut(&mut self);
                }
            }
        };

        let module = parse_ok(tokens);

        let tests = vec![(false, false), (true, false), (true, true)];

        for (idx, (has_ref, has_mut)) in tests.into_iter().enumerate() {
            let funcs = &module.functions;
            let field = funcs[idx].get_field.as_ref().unwrap().unwrap_direct();
            assert_eq!(field.maybe_ref.is_some(), has_ref);
            assert_eq!(field.maybe_mut.is_some(), has_mut);
        }
    }

    /// Verify that we can parse the `get_with` attribute.
    #[test]
    fn parses_get_with_attribute() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;

                    #[swift_bridge(get_with(field = a::b::c))]
                    fn some_function(&self);

                    #[swift_bridge(get_with(&field = a::b::c))]
                    fn some_function_ref(&self);

                    #[swift_bridge(get_with(&mut field = a::b::c))]
                    fn some_function_ref_mut(&mut self);
                }
            }
        };

        let module = parse_ok(tokens);

        let tests = vec![(false, false), (true, false), (true, true)];

        for (idx, (has_ref, has_mut)) in tests.into_iter().enumerate() {
            let funcs = &module.functions;
            let field = funcs[idx].get_field.as_ref().unwrap().unwrap_with();
            assert_eq!(field.maybe_ref.is_some(), has_ref);
            assert_eq!(field.maybe_mut.is_some(), has_mut);
        }
    }

    /// Verify that we can parse a function that has multiple swift_bridge attributes.
    #[test]
    fn parses_multiple_function_swift_bridge_attributes() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {

                    #[swift_bridge(args_into = (a), return_into)]
                    fn some_function(a: u8);
                }
            }
        };

        let module = parse_ok(tokens);

        let func = &module.functions[0];
        assert_eq!(func.args_into.as_ref().unwrap().len(), 1);
        assert_eq!(func.return_into, true);
    }
}
