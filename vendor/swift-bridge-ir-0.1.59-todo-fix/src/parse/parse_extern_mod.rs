use self::argument_attributes::ArgumentAttributes;
pub(crate) use self::opaque_type_attributes::OpaqueTypeAllAttributes;
use crate::bridged_type::{
    bridgeable_type_from_fn_arg, pat_type_pat_is_self, BridgeableType, BridgedType,
};
use crate::errors::{FunctionAttributeParseError, IdentifiableParseError, ParseError, ParseErrors};
use crate::parse::parse_extern_mod::function_attributes::FunctionAttributes;
use crate::parse::parse_extern_mod::generics::GenericOpaqueType;
use crate::parse::type_declarations::{
    OpaqueForeignTypeDeclaration, TypeDeclaration, TypeDeclarations,
};
use crate::parse::{HostLang, OpaqueRustTypeGenerics};
use crate::parsed_extern_fn::{fn_arg_is_mutable_reference, FailableInitializerType};
use crate::ParsedExternFn;
use proc_macro2::Ident;
use quote::{format_ident, ToTokens};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use syn::{
    FnArg, ForeignItem, ForeignItemFn, GenericParam, ItemForeignMod, LitStr, Pat, ReturnType, Type,
};

mod argument_attributes;
mod function_attributes;
mod generics;
mod opaque_type_attributes;

pub(super) struct ForeignModParser<'a> {
    pub errors: &'a mut ParseErrors,
    /// All of the type declarations across all of the extern "..." foreign modules in the
    /// `mod` module that this foreign module is in.
    pub type_declarations: &'a mut TypeDeclarations,
    pub functions: &'a mut Vec<ParsedExternFn>,
    pub unresolved_types: &'a mut Vec<Type>,
}

impl<'a> ForeignModParser<'a> {
    pub fn parse(mut self, mut foreign_mod: ItemForeignMod) -> Result<(), syn::Error> {
        if foreign_mod.abi.name.is_none() {
            self.errors.push(ParseError::AbiNameMissing {
                extern_token: foreign_mod.abi.extern_token,
            });
            return Ok(());
        }

        let abi_name = foreign_mod.abi.name.unwrap();

        let host_lang = match abi_name.value().as_str() {
            "Rust" => HostLang::Rust,
            "Swift" => HostLang::Swift,
            _ => {
                self.errors.push(ParseError::AbiNameInvalid { abi_name });
                return Ok(());
            }
        };

        foreign_mod.items.sort_by(|a, _b| {
            if matches!(a, ForeignItem::Type(_)) {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        let mut local_type_declarations = HashMap::new();
        for foreign_mod_item in foreign_mod.items {
            match foreign_mod_item {
                ForeignItem::Type(foreign_ty) => {
                    // TODO: Normalize with the code used to parse generic foreign item types

                    let ty_name = foreign_ty.ident.to_string();

                    if let Some(ty) = BridgedType::new_with_str(
                        &foreign_ty.ident.to_string(),
                        &self.type_declarations,
                    ) {
                        if ty.is_built_in_type() {
                            self.errors.push(ParseError::DeclaredBuiltInType {
                                ty: foreign_ty.clone(),
                            });
                        }
                    }

                    let foreign_type = OpaqueForeignTypeDeclaration {
                        ty: foreign_ty.ident.clone(),
                        host_lang,
                        attributes: OpaqueTypeAllAttributes::from_attributes(&foreign_ty.attrs)?,
                        generics: OpaqueRustTypeGenerics::new(),
                    };
                    self.type_declarations.insert(
                        ty_name.clone(),
                        TypeDeclaration::Opaque(foreign_type.clone()),
                    );
                    local_type_declarations.insert(ty_name, foreign_type);
                }
                ForeignItem::Fn(func) => {
                    let mut attributes = FunctionAttributes::default();

                    for attr in func.attrs.iter() {
                        attributes = attr.parse_args()?;
                    }

                    for arg in func.sig.inputs.iter() {
                        if let FnArg::Typed(pat_ty) = arg {
                            let ty = &pat_ty.ty;
                            if BridgedType::new_with_type(&ty, &self.type_declarations).is_none() {
                                self.unresolved_types.push(ty.deref().clone());
                            }
                        }
                    }

                    let return_type = &func.sig.output;
                    let mut swift_failable_initializer: Option<FailableInitializerType> = None;
                    if let ReturnType::Type(_, return_ty) = return_type {
                        let bridged_return_type =
                            BridgedType::new_with_type(return_ty.deref(), &self.type_declarations);

                        if let Some(ty) = &bridged_return_type {
                            if ty.as_option().is_some() && attributes.is_swift_initializer {
                                swift_failable_initializer = Some(FailableInitializerType::Option);
                            } else if ty.as_result().is_some() && attributes.is_swift_initializer {
                                swift_failable_initializer =
                                    Some(FailableInitializerType::Throwing);
                            }
                        }
                        if bridged_return_type.is_none() {
                            self.unresolved_types.push(return_ty.deref().clone());
                        }
                    }
                    let first_input = func.sig.inputs.iter().next();

                    let associated_type = self.get_associated_type(
                        first_input,
                        func.clone(),
                        &attributes,
                        &mut local_type_declarations,
                        swift_failable_initializer.clone(),
                    )?;

                    if attributes.is_swift_identifiable {
                        let args = &func.sig.inputs;

                        let mut is_ref_self_no_args = args.len() == 1;
                        if is_ref_self_no_args {
                            is_ref_self_no_args = match args.iter().next().unwrap() {
                                FnArg::Receiver(receiver) => {
                                    receiver.reference.is_some() && receiver.mutability.is_none()
                                }
                                FnArg::Typed(pat_ty) => {
                                    pat_type_pat_is_self(pat_ty)
                                        && pat_ty.ty.to_token_stream().to_string().starts_with("&")
                                }
                            };
                        }

                        let has_return_type = matches!(&func.sig.output, ReturnType::Type(_, _));

                        if !is_ref_self_no_args {
                            self.errors.push(ParseError::FunctionAttribute(
                                FunctionAttributeParseError::Identifiable(
                                    IdentifiableParseError::MustBeRefSelf {
                                        fn_ident: func.sig.ident.clone(),
                                    },
                                ),
                            ));
                        }
                        if !has_return_type {
                            self.errors.push(ParseError::FunctionAttribute(
                                FunctionAttributeParseError::Identifiable(
                                    IdentifiableParseError::MissingReturnType {
                                        fn_ident: func.sig.ident.clone(),
                                    },
                                ),
                            ));
                        }
                    }
                    let mut argument_labels: HashMap<Ident, LitStr> = HashMap::new();
                    for arg in func.sig.inputs.iter() {
                        let is_mutable_ref = fn_arg_is_mutable_reference(arg);

                        let is_copy_opaque_type =
                            if let Some(TypeDeclaration::Opaque(o)) = associated_type.as_ref() {
                                o.attributes.copy.is_some()
                            } else if let Some(ty) =
                                bridgeable_type_from_fn_arg(arg, &self.type_declarations)
                            {
                                ty.has_swift_bridge_copy_annotation()
                            } else {
                                false
                            };

                        if is_mutable_ref && is_copy_opaque_type {
                            self.errors
                                .push(ParseError::ArgCopyAndRefMut { arg: arg.clone() });
                        }
                        match arg {
                            syn::FnArg::Typed(ty) => {
                                for attr in ty.attrs.iter() {
                                    let attribute: ArgumentAttributes = attr.parse_args()?;
                                    if let Some(label) = attribute.label {
                                        argument_labels.insert(
                                            format_ident!(
                                                "{}",
                                                ty.pat.to_token_stream().to_string()
                                            ),
                                            label,
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(ref args) = attributes.args_into {
                        let mut func_sig_args = HashSet::with_capacity(args.len());
                        for fn_arg in func.sig.inputs.iter() {
                            match fn_arg {
                                FnArg::Receiver(_) => {}
                                FnArg::Typed(pat_ty) => {
                                    let fn_arg_name = pat_ty.pat.to_token_stream().to_string();
                                    func_sig_args.insert(fn_arg_name);
                                }
                            }
                        }

                        for arg in args.iter() {
                            let arg_name = arg.to_token_stream().to_string();

                            if !func_sig_args.contains(&arg_name) {
                                self.errors.push(ParseError::ArgsIntoArgNotFound {
                                    func: func.clone(),
                                    missing_arg: arg.clone(),
                                })
                            }
                        }
                    }

                    let func = ParsedExternFn {
                        func,
                        associated_type,
                        is_swift_initializer: attributes.is_swift_initializer,
                        swift_failable_initializer,
                        is_swift_identifiable: attributes.is_swift_identifiable,
                        host_lang,
                        rust_name_override: attributes.rust_name,
                        swift_name_override: attributes.swift_name,
                        return_into: attributes.return_into,
                        return_with: attributes.return_with,
                        args_into: attributes.args_into,
                        get_field: attributes.get_field,
                        argument_labels: argument_labels,
                    };
                    self.functions.push(func);
                }
                ForeignItem::Verbatim(foreign_item_verbatim) => {
                    if let Ok(generic_foreign_type) =
                        syn::parse2::<GenericOpaqueType>(foreign_item_verbatim)
                    {
                        let ty_name = generic_foreign_type.ident.to_string();

                        let foreign_ty = OpaqueForeignTypeDeclaration {
                            ty: generic_foreign_type.ident,
                            host_lang,
                            attributes: OpaqueTypeAllAttributes::from_attributes(
                                &generic_foreign_type.attributes,
                            )?,
                            generics: OpaqueRustTypeGenerics {
                                generics: generic_foreign_type
                                    .generics
                                    .params
                                    .clone()
                                    .into_iter()
                                    .map(|p| match p {
                                        GenericParam::Type(generic_ty) => generic_ty,
                                        _ => todo!(
                                            "Push a ParseError for non-concrete generic types"
                                        ),
                                    })
                                    .collect(),
                            },
                        };
                        let generics: Vec<String> = foreign_ty
                            .generics
                            .generics
                            .iter()
                            .map(|g| g.ident.to_string())
                            .collect();
                        let generics: String = generics.join(",");
                        let ty_name = format!("{}<{}>", ty_name, generics);
                        self.type_declarations
                            .insert(ty_name.clone(), TypeDeclaration::Opaque(foreign_ty.clone()));
                        local_type_declarations.insert(ty_name, foreign_ty);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn get_associated_type(
        &mut self,
        first: Option<&FnArg>,
        func: ForeignItemFn,
        attributes: &FunctionAttributes,
        local_type_declarations: &mut HashMap<String, OpaqueForeignTypeDeclaration>,
        swift_failable_initializer: Option<FailableInitializerType>,
    ) -> syn::Result<Option<TypeDeclaration>> {
        let associated_type = match first {
            Some(FnArg::Receiver(recv)) => {
                if let Some(_) = attributes.associated_to {
                    self.errors.push(ParseError::InvalidAssociatedTo {
                        self_: first.unwrap().clone(),
                    })
                }
                if local_type_declarations.len() == 1 {
                    let ty = local_type_declarations.iter_mut().next().unwrap().1;
                    let associated_type = Some(TypeDeclaration::Opaque(ty.clone()));
                    associated_type
                } else {
                    self.errors.push(ParseError::AmbiguousSelf {
                        self_: recv.clone(),
                    });
                    return Ok(None);
                }
            }
            Some(FnArg::Typed(arg)) => match arg.pat.deref() {
                Pat::Ident(pat_ident) => {
                    if pat_ident.ident.to_string() == "self" {
                        let self_ty = match arg.ty.deref() {
                            Type::Path(ty_path) => ty_path.path.segments.to_token_stream(),
                            Type::Reference(type_ref) => type_ref.elem.deref().to_token_stream(),
                            _ => {
                                todo!("Add a test that hits this branch")
                            }
                        };

                        let self_ty_string = self_ty.to_string();
                        // Handles generics. i.e. "SomeType< u32, u64 >" -> "SomeType<u32,u64>";
                        let self_ty_string = self_ty_string.replace(" ", "");

                        let ty = self.type_declarations.get(&self_ty_string).unwrap();
                        let associated_type = Some(ty.clone());
                        associated_type
                    } else {
                        let associated_type = self.get_associated_type(
                            None,
                            func.clone(),
                            attributes,
                            local_type_declarations,
                            swift_failable_initializer,
                        )?;
                        associated_type
                    }
                }
                _ => {
                    todo!(
                        r#"
One way to hit this block is with a `fn foo (&self: SomeType)`.
Note that this is an invalid signature since the `&` should be in front of `SomeType`, not `self`.
i.e., this would be correct: `fn foo (self: &SomeType)`
We should add a test case like this that hits this block, and verify that we push a parse error
indicating that the function signature is invalid.
For common mistakes such as the `&self: SomeType` example, we can have dedicated errors telling you
exactly how to fix it.
Otherwise we use a more general error that says that your argument is invalid.
"#
                    )
                }
            },
            None => {
                let associated_type = if let Some(associated_to) = &attributes.associated_to {
                    let ty = self
                        .type_declarations
                        .get(&associated_to.to_string())
                        .unwrap();
                    Some(ty.clone())
                } else if attributes.is_swift_initializer {
                    let ty_string = match &func.sig.output {
                        ReturnType::Default => {
                            todo!("Push error if initializer does not return a type")
                        }
                        ReturnType::Type(_, ty) => {
                            let ty_string = ty.deref().to_token_stream().to_string();
                            ty_string
                        }
                    };
                    if swift_failable_initializer.is_some() {
                        if ty_string.starts_with("Option <") {
                            // Safety: since we've already checked ty_string is formatted as "Option<~>" before calling this function.
                            let last_bracket = ty_string.rfind(">").unwrap();

                            let inner = &ty_string[0..last_bracket];
                            let inner = inner.trim_start_matches("Option < ").trim_end_matches(" ");

                            let ty = self.type_declarations.get(inner);
                            ty.map(|ty| ty.clone())
                        } else if ty_string.starts_with("Result <") {
                            // A , B >
                            let trimmed = ty_string.trim_start_matches("Result < ");
                            // A , B
                            let trimmed = trimmed.trim_end_matches(" >");

                            // [A, B]
                            let ok_and_err = trimmed.rsplit_once(",").unwrap();
                            let ok = ok_and_err.0.trim();

                            let ty = self.type_declarations.get(ok);

                            ty.map(|ty| ty.clone())
                        } else {
                            unreachable!();
                        }
                    } else {
                        let ty = self.type_declarations.get(&ty_string);

                        ty.map(|ty| ty.clone())
                    }
                } else {
                    None
                };

                associated_type
            }
        };

        Ok(associated_type)
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::ParseError;
    use crate::test_utils::{parse_errors, parse_ok};
    use crate::SwiftBridgeModule;
    use quote::{quote, ToTokens};
    use syn::parse_quote;

    /// Verify that we can parse a SwiftBridgeModule from an empty module.
    #[test]
    fn parse_empty_module() {
        let tokens = quote! {
            mod foo { }
        };
        let module: SwiftBridgeModule = parse_quote!(#tokens);

        assert_eq!(module.name.to_string(), "foo");
    }

    /// Verify that we store an error if no abi name was provided.
    #[test]
    fn error_if_no_abi_name_provided_for_an_extern_block() {
        let tokens = quote! {
            mod foo {
                extern {}
            }
        };
        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 1);
        match errors[0] {
            ParseError::AbiNameMissing { .. } => {}
            _ => panic!(),
        }
    }

    /// Verify that we store an error if the abi name isn't Rust or Swift.
    #[test]
    fn error_if_invalid_abi_name() {
        let tokens = quote! {
            mod foo {
                extern "SomeAbi" {}
            }
        };
        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ParseError::AbiNameInvalid { abi_name } => {
                assert_eq!(abi_name.value(), "SomeAbi");
            }
            _ => panic!(),
        }
    }

    /// Verify that we can parse a Rust type declaration.
    #[test]
    fn rust_type_declaration() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.types.types()[0].unwrap_opaque().to_string(), "Foo");
    }

    /// Verify that we return an error if the declared type is a built in type.
    #[test]
    fn error_if_declared_built_in_type() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type u8;
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 1);
    }

    /// Verify that we can parse a Rust type's methods.
    /// We test all of the possible ways we can specify self.
    #[test]
    fn parses_rust_self_methods() {
        let tests = vec![
            quote! { fn bar (self); },
            quote! { fn bar (&self); },
            quote! { fn bar (&mut self); },
            quote! { fn bar (self: Foo); },
            quote! { fn bar (self: &Foo); },
            quote! { fn bar (self: &mut Foo); },
        ];

        for fn_definition in tests {
            let tokens = quote! {
                mod foo {
                    extern "Rust" {
                        type Foo;

                        #fn_definition
                    }
                }
            };

            let module = parse_ok(tokens);

            let ty = &module.types.types()[0].unwrap_opaque();
            assert_eq!(ty.to_string(), "Foo");

            assert_eq!(
                module.functions.len(),
                1,
                "Failed not parse {} into an associated method.",
                quote! {#fn_definition}.to_string()
            );
        }
    }

    /// Verify that if a a type is defined in another foreign module block we can still use it.
    #[test]
    fn type_defined_in_another_foreign_module() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    fn foo () -> AnotherType;
                }

                extern "Rust" {
                    type AnotherType;
                }
            }
        };

        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 0,);
    }

    /// Verify that we can parse a freestanding Rust function declaration.
    #[test]
    fn rust_freestanding_function_no_args() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    fn bar () -> u8;
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.functions.len(), 1);
    }

    /// Verify that we can parse a freestanding Rust function declaration that has one arg.
    #[test]
    fn rust_freestanding_function_one_arg() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    fn bar (bazz: u32);
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(module.functions.len(), 1);
    }

    /// Verify that if a freestanding function has argument types that were not declared in the
    /// module we return an error.
    #[test]
    fn freestanding_function_argument_undeclared_type() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    fn a (bar: Bar);
                    fn b (bar: &Bar);
                    fn c (bar: &mut Bar);
                    // Counts as two errors.
                    fn d (multiple: Bar, args: Bar);
                }
            }
        };
        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 5);

        for error in errors.iter() {
            match error {
                ParseError::UndeclaredType { ty } => {
                    let ty_name = ty.to_token_stream().to_string();
                    // "& Bar" -> "Bar"
                    let ty_name = ty_name.split_whitespace().last().unwrap();

                    assert_eq!(ty_name, "Bar");
                }
                _ => panic!(),
            }
        }
    }

    /// Verify that a freestanding function can return a declared type.
    #[test]
    fn freestanding_function_return_declared_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod foo {
                extern "Rust" {
                    type Bar;

                    fn a () -> Bar;
                }
            }
        };
        let module = parse_ok(tokens);
        assert_eq!(module.functions.len(), 1);
    }

    /// Verify that if a freestanding function returns a type that was not declared in the module
    /// we return an error.
    #[test]
    fn freestanding_function_returns_undeclared_type() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type Foo;

                    fn a () -> Bar;
                    fn a () -> &Bar;
                    fn a () -> &mut Bar;
                }
            }
        };
        let errors = parse_errors(tokens);
        assert_eq!(errors.len(), 3);

        for error in errors.iter() {
            match error {
                ParseError::UndeclaredType { ty } => {
                    let ty_name = ty.to_token_stream().to_string();
                    // "& Bar" -> "Bar"
                    let ty_name = ty_name.split_whitespace().last().unwrap();

                    assert_eq!(ty_name, "Bar");
                }
                _ => panic!(),
            }
        }
    }

    /// Verify that if an extern Rust block has more than one type, we push errors for any methods
    /// that have an ambiguous self.
    #[test]
    fn error_if_method_has_ambiguous_self() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type SomeType;
                    type AnotherType;

                    fn a (self);
                    fn b (&self);
                    fn c (&mut self);
                }
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 3);

        for idx in 0..3 {
            match &errors[idx] {
                ParseError::AmbiguousSelf { self_: _ } => {}
                _ => panic!(),
            };
        }
    }

    /// Verify that annotated self methods get parsed.
    #[test]
    fn disambiguate_method() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type SomeType;
                    type AnotherType;

                    fn a (self: SomeType);
                    fn b (self: &SomeType);
                    fn c (self: &mut AnotherType);
                }
            }
        };

        let module = parse_ok(tokens);

        let functions = &module.functions;

        for (ty_name, expected_count) in vec![("SomeType", 2), ("AnotherType", 1)] {
            assert_eq!(
                functions
                    .iter()
                    .filter(|f| f.associated_type.as_ref().unwrap().unwrap_opaque().ty == ty_name)
                    .count(),
                expected_count
            );
        }
    }

    /// Verify that if we have multiple externs types can be inferred within each.
    #[test]
    fn infer_type_with_multiple_externs() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    type SomeType;

                    fn a (&self);
                }

                extern "Rust" {
                    type AnotherType;

                    fn b (&self);
                }
            }
        };

        let module = parse_ok(tokens);

        let functions = &module.functions;

        for (ty_name, expected_count) in vec![("SomeType", 1), ("AnotherType", 1)] {
            assert_eq!(
                functions
                    .iter()
                    .filter(|f| f.associated_type.as_ref().unwrap().unwrap_opaque().ty == ty_name)
                    .count(),
                expected_count
            );
        }
    }

    /// Verify that we we do not get any parsing errors when we use a type that is declared in
    /// an extern block that comes after the block that it is used in.
    #[test]
    fn type_declared_in_separate_extern_block_after_use() {
        let tokens = quote! {
            mod foo {
                extern "Rust" {
                    fn a () -> AnotherType;
                    fn b () -> Vec<AnotherType>;
                    // TODO: (Dec 2021) Uncomment this when we support Option<OpaqueRustType>
                    // fn c () -> Option<AnotherType>;
                    fn d (arg: AnotherType);
                }

                extern "Rust" {
                    type AnotherType;
                }
            }
        };

        assert_eq!(parse_errors(tokens).len(), 0,);
    }

    /// Verify that we push errors for unknown arguments in a function
    #[test]
    fn error_args_into_arg_not_found_in_function() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    #[swift_bridge(args_into = (foo, bar))]
                    fn some_function(foo: u8);
                }
            }
        };

        let errors = parse_errors(tokens);

        // Only "bar" should be missing argument.
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            ParseError::ArgsIntoArgNotFound {
                func: _,
                missing_arg,
            } => {
                assert_eq!(missing_arg, "bar")
            }
            _ => panic!(),
        }
    }

    /// Verify that we push errors for function arguments that are both mutable and opaque Copy.
    #[test]
    fn error_if_mutable_opaque_copy_type() {
        let tokens = quote! {
            #[swift_bridge:bridge]
            mod foo {
                extern "Rust" {
                    #[swift_bridge(Copy(2))]
                    type SomeType;

                    fn a(&mut self);
                    fn b(self: &mut SomeType);
                    fn c(arg: &mut SomeType);
                }
            }
        };

        let errors = parse_errors(tokens);

        assert_eq!(errors.len(), 3);

        for error in errors.iter() {
            match error {
                ParseError::ArgCopyAndRefMut { arg: _ } => {}
                _ => panic!(),
            }
        }
    }
}
