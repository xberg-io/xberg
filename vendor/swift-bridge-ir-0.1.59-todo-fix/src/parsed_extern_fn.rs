use crate::bridged_type::boxed_fn::BridgeableBoxedFnOnce;
use crate::bridged_type::{pat_type_pat_is_self, BridgeableType, BridgedType, StdLibType};
use crate::parse::{HostLang, SharedTypeDeclaration, TypeDeclaration, TypeDeclarations};
use crate::SWIFT_BRIDGE_PREFIX;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::{FnArg, ForeignItemFn, Lifetime, LitStr, Path, ReturnType, Token, Type};

mod to_extern_c_fn;
mod to_extern_c_param_names_and_types;
mod to_rust_impl_call_swift;
mod to_swift_func;

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum SwiftFuncGenerics {
    String,
    Str,
}

impl SwiftFuncGenerics {
    pub fn as_bound(&self) -> &'static str {
        match self {
            SwiftFuncGenerics::String => "GenericIntoRustString: IntoRustString",
            SwiftFuncGenerics::Str => "GenericToRustStr: ToRustStr",
        }
    }
}

/// Represents different types of Swift's initializers that can fail
#[derive(Clone)]
pub(crate) enum FailableInitializerType {
    Throwing,
    Option,
}

/// A method or associated function associated with a type.
///
/// fn bar (&self);
/// fn buzz (self: &Foo) -> u8;
///
/// #\[swift_bridge(init)\]
/// fn new () -> Foo;
///
/// ... etc
pub(crate) struct ParsedExternFn {
    pub func: ForeignItemFn,
    /// The type that this function is associated to.
    ///
    /// ```
    /// # const  _: &str = stringify!(
    /// #[swift_bridge::bridge]
    /// mod ffi {
    ///     extern "Rust" {
    ///         type SomeType;
    ///
    ///         // This function is associated to `SomeType` since it has a receiver `&self`.
    ///         fn some_function(&self);
    ///     }
    /// }
    /// # );
    /// ```
    pub associated_type: Option<TypeDeclaration>,
    pub host_lang: HostLang,
    /// Whether or not this function is a Swift initializer.
    pub is_swift_initializer: bool,
    /// Whether or not this function is a Swift failable initializer.
    /// For more details, see:
    /// [Swift Documentation - Failable Initializers](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/initialization/#Failable-Initializers)
    pub swift_failable_initializer: Option<FailableInitializerType>,
    /// Whether or not this function should be used for the associated type's Swift
    /// `Identifiable` protocol implementation.
    pub is_swift_identifiable: bool,
    pub rust_name_override: Option<syn::LitStr>,
    pub swift_name_override: Option<syn::LitStr>,
    /// If true, we call `.into()` on the expression that the function returns before returning it.
    ///
    /// ```no_run,ignore
    /// // Declaration
    /// fn some_function() -> SomeType;
    ///
    /// // Approximate generated Code
    /// extern "C" fn some_function() -> SomeType {
    ///     super::some_function().into()
    /// }
    /// ```
    pub return_into: bool,
    pub return_with: Option<Path>,
    /// Call `.into()` before passing this argument to the function that handles it.
    ///
    /// ```no_run,ignore
    /// // Declaration
    /// #[swift_bridge(args_into = (some_arg, another_arg)]
    /// fn some_function(some_arg: u8, another_arg: MyStruct);
    ///
    /// // Approximate generated code
    /// extern "C" fn some_function(some_arg: u8, another_arg: MyStruct) {
    ///     super::some_function(some_arg, another_arg.into())
    /// }
    /// ```
    pub args_into: Option<Vec<Ident>>,
    /// Get one of the associated type's fields
    pub get_field: Option<GetField>,
    pub argument_labels: HashMap<Ident, LitStr>,
}

pub(crate) enum GetField {
    Direct(GetFieldDirect),
    With(GetFieldWith),
}

pub struct GetFieldDirect {
    pub(crate) maybe_ref: Option<Token![&]>,
    pub(crate) maybe_mut: Option<Token![mut]>,
    pub(crate) field_name: Ident,
}

pub struct GetFieldWith {
    pub(crate) maybe_ref: Option<Token![&]>,
    pub(crate) maybe_mut: Option<Token![mut]>,
    pub(crate) field_name: Ident,
    pub(crate) path: Path,
}

#[cfg(test)]
impl GetField {
    pub(crate) fn unwrap_direct(&self) -> &GetFieldDirect {
        match self {
            GetField::Direct(d) => d,
            _ => panic!(),
        }
    }

    pub(crate) fn unwrap_with(&self) -> &GetFieldWith {
        match self {
            GetField::With(d) => d,
            _ => panic!(),
        }
    }
}

impl ParsedExternFn {
    pub fn is_method(&self) -> bool {
        self.func.sig.receiver().is_some()
    }

    pub fn self_reference(&self) -> Option<(Token![&], Option<Lifetime>)> {
        match self.func.sig.receiver()? {
            FnArg::Receiver(receiver) => receiver.reference.clone(),
            FnArg::Typed(pat_ty) => match pat_ty.ty.deref() {
                Type::Reference(type_ref) => Some((type_ref.and_token, type_ref.lifetime.clone())),
                _ => None,
            },
        }
    }

    pub fn self_mutability(&self) -> Option<Token![mut]> {
        match self.func.sig.receiver()? {
            FnArg::Receiver(receiver) => receiver.mutability,
            FnArg::Typed(pat_ty) => match pat_ty.ty.deref() {
                Type::Reference(type_ref) => type_ref.mutability,
                _ => None,
            },
        }
    }

    pub(crate) fn rust_fn_sig_return_tokens(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
        custom_type_definitions: &mut HashMap<String, TokenStream>,
    ) -> TokenStream {
        let sig = &self.func.sig;

        if let Some(ret) = BridgedType::new_with_return_type(&sig.output, types) {
            if ret.can_be_encoded_with_zero_bytes() {
                return quote! {};
            }
            if let Some(tokens) = ret.generate_custom_rust_ffi_types(swift_bridge_path, types) {
                for token in tokens.into_iter() {
                    custom_type_definitions.insert(token.to_string(), token);
                }
            }
            let ty = ret.to_ffi_compatible_rust_type(swift_bridge_path, types);
            quote! { -> #ty }
        } else {
            todo!("Push to ParseErrors")
        }
    }

    pub(crate) fn maybe_async_rust_fn_return_ty(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> Option<TokenStream> {
        let sig = &self.func.sig;

        if let Some(ret) = BridgedType::new_with_return_type(&sig.output, types) {
            let ty = ret.to_ffi_compatible_rust_type(swift_bridge_path, types);
            if ty.to_string() == "()" {
                None
            } else {
                Some(quote! { , #ty })
            }
        } else {
            todo!("Push to ParseErrors")
        }
    }

    pub fn extern_swift_linked_fn_new(&self) -> Ident {
        let sig = &self.func.sig;

        let prefix = if let Some(associated_ty) = self.associated_type.as_ref() {
            match associated_ty {
                TypeDeclaration::Shared(_) => {
                    todo!()
                }
                TypeDeclaration::Opaque(associated_ty) => {
                    format!("{}_", associated_ty.ty)
                }
            }
        } else {
            "".to_string()
        };

        Ident::new(
            &format!("{}{}{}", SWIFT_BRIDGE_PREFIX, prefix, sig.ident),
            sig.ident.span(),
        )
    }

    pub fn args_into_contains_arg(&self, arg: &FnArg) -> bool {
        if self.args_into.is_none() {
            return false;
        }
        let args_into = self.args_into.as_ref().unwrap();

        match arg {
            FnArg::Receiver(_) => false,
            FnArg::Typed(arg) => {
                let arg_string = arg.pat.to_token_stream().to_string();

                args_into
                    .iter()
                    .any(|arg_name| &arg_name.to_string() == &arg_string)
            }
        }
    }
}

impl ParsedExternFn {
    // extern Rust:
    // fn foo (&self, arg1: u8, arg2: u32, bar: &SomeType)
    //  becomes..
    // arg1, arg2, & unsafe { Box::from_raw(bar) }
    //
    // extern Swift:
    // fn foo (&self, arg1: u8, arg2: u32, &SomeType)
    //  becomes..
    // self.0, arg1, arg2, & unsafe { Box::from_raw(bar) }
    pub fn to_call_rust_args(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let mut args = vec![];
        let inputs = &self.func.sig.inputs;
        for fn_arg in inputs.into_iter() {
            match fn_arg {
                FnArg::Receiver(_receiver) => {
                    if self.host_lang.is_swift() {
                        args.push(quote! {#swift_bridge_path::PointerToSwiftType(self.0)});
                    }
                }
                FnArg::Typed(pat_ty) => {
                    if pat_type_pat_is_self(pat_ty) {
                        if self.host_lang.is_swift() {
                            args.push(quote! {#swift_bridge_path::PointerToSwiftType(self.0)});
                        }

                        continue;
                    }

                    let pat = &pat_ty.pat;

                    let mut arg = quote! {#pat};

                    if let Some(built_in) = BridgedType::new_with_type(&pat_ty.ty, types) {
                        if self.host_lang.is_rust() {
                            arg = if let Some(repr) = built_in.only_encoding() {
                                repr.rust
                            } else {
                                built_in.convert_ffi_expression_to_rust_type(
                                    &arg,
                                    pat_ty.ty.span(),
                                    swift_bridge_path,
                                    types,
                                )
                            };

                            if self.args_into_contains_arg(fn_arg) {
                                arg = quote_spanned! {pat_ty.span()=>
                                    #arg.into()
                                };
                            }
                        } else {
                            if built_in.can_be_encoded_with_zero_bytes() {
                                continue;
                            }

                            arg = built_in.convert_rust_expression_to_ffi_type(
                                &arg,
                                swift_bridge_path,
                                types,
                                // TODO: Add a UI test and then add a better span
                                Span::call_site(),
                            );
                        };
                    } else {
                        todo!("Push to ParsedErrors")
                    }

                    args.push(arg);
                }
            };
        }

        quote! {
            #(#args),*
        }
    }

    // fn foo (&self, arg1: u8, arg2: u32)
    //  becomes..
    // void* self, uint8_t u8, uint32_t arg2
    pub fn to_c_header_params(&self, types: &TypeDeclarations) -> String {
        let mut params = vec![];
        let inputs = &self.func.sig.inputs;
        for arg in inputs {
            match arg {
                FnArg::Receiver(_receiver) => {
                    self.push_self_param(&mut params);
                }
                FnArg::Typed(pat_ty) => {
                    let pat = &pat_ty.pat;

                    if pat_type_pat_is_self(pat_ty) {
                        self.push_self_param(&mut params);
                    } else {
                        let built_in = BridgedType::new_with_type(&pat_ty.ty, types).unwrap();

                        if built_in.can_be_encoded_with_zero_bytes() {
                            continue;
                        }

                        let ty = built_in.to_c(types);

                        let arg_name = pat.to_token_stream().to_string();
                        params.push(format!("{} {}", ty, arg_name));
                    }
                }
            };
        }

        if params.len() == 0 {
            "void".to_string()
        } else {
            params.join(", ")
        }
    }

    pub fn to_c_header_return(&self, types: &TypeDeclarations) -> String {
        match &self.func.sig.output {
            ReturnType::Default => "void".to_string(),
            ReturnType::Type(_, ty) => {
                if let Some(ty) = BridgedType::new_with_type(&ty, types) {
                    if ty.can_be_encoded_with_zero_bytes() {
                        return "void".to_string();
                    }

                    ty.to_c(types)
                } else {
                    let ty_string = match ty.deref() {
                        Type::Reference(reference) => reference.elem.to_token_stream().to_string(),
                        Type::Path(path) => path.path.to_token_stream().to_string(),
                        _ => todo!(),
                    };

                    match types.get(&ty_string).unwrap() {
                        TypeDeclaration::Shared(SharedTypeDeclaration::Struct(shared_struct)) => {
                            format!("struct {}", shared_struct.swift_name_string())
                        }
                        TypeDeclaration::Shared(SharedTypeDeclaration::Enum(_shared_enum)) => {
                            //
                            todo!("Enum C header")
                        }
                        TypeDeclaration::Opaque(opaque) => {
                            if opaque.host_lang.is_rust() {
                                "void*".to_string()
                            } else {
                                "void*".to_string()
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn c_includes(&self, types: &TypeDeclarations) -> Option<Vec<&'static str>> {
        let mut includes = vec![];

        if let ReturnType::Type(_, ty) = &self.func.sig.output {
            if let Some(ty) = BridgedType::new_with_type(&ty, types) {
                if let Some(include) = ty.to_c_include(types) {
                    includes.push(include);
                }
            }
        }

        for param in &self.func.sig.inputs {
            if let FnArg::Typed(pat_ty) = param {
                if let Some(ty) = BridgedType::new_with_type(&pat_ty.ty, types) {
                    if let Some(include) = ty.to_c_include(types) {
                        includes.push(include);
                    }
                }
            }
        }

        if includes.len() > 0 {
            Some(includes.into_iter().flatten().collect())
        } else {
            None
        }
    }

    fn push_self_param(&self, params: &mut Vec<String>) {
        let param = if self.is_copy_method_on_opaque_type() {
            format!(
                "struct {}${} this",
                SWIFT_BRIDGE_PREFIX,
                &self
                    .associated_type
                    .as_ref()
                    .unwrap()
                    .as_opaque()
                    .unwrap()
                    .ty
            )
        } else {
            "void* self".to_string()
        };

        params.push(param);
    }
}

impl ParsedExternFn {
    pub fn link_name(&self) -> String {
        let host_type = self
            .associated_type
            .as_ref()
            .map(|h| {
                match h {
                    TypeDeclaration::Shared(_) => {
                        //
                        todo!()
                    }
                    TypeDeclaration::Opaque(h) => {
                        format!("${}", h.to_string())
                    }
                }
            })
            .unwrap_or("".to_string());
        format!(
            "{}{}${}",
            SWIFT_BRIDGE_PREFIX,
            host_type,
            self.func.sig.ident.to_string()
        )
    }

    pub fn call_boxed_fn_link_name(&self, boxed_fn_idx: usize) -> String {
        format!("{}$param{}", self.link_name(), boxed_fn_idx)
    }
    pub fn free_boxed_fn_link_name(&self, boxed_fn_idx: usize) -> String {
        format!("{}$_free$param{}", self.link_name(), boxed_fn_idx)
    }

    /// Generates something like:
    /// void __swift_bridge__$some_function$param0(void* boxed_fn, uint8_t arg);
    /// void __swift_bridge__$some_function$_free$param0(void* boxed_fn);
    pub fn boxed_fn_to_c_header_fns(
        &self,
        idx: usize,
        boxed_fn: &BridgeableBoxedFnOnce,
        types: &TypeDeclarations,
    ) -> String {
        let call_boxed_fn_link_name = self.call_boxed_fn_link_name(idx);
        let free_boxed_fn_link_name = self.free_boxed_fn_link_name(idx);

        let boxed_fn_arg_name = self.arg_name_at_idx(idx).unwrap();
        let boxed_fn_arg_name = format!("{}_{}", self.sig.ident, boxed_fn_arg_name);

        let maybe_args = if boxed_fn.params.is_empty() {
            "".to_string()
        } else {
            let args = boxed_fn.params_to_c_types(types);
            format!(", {args}")
        };

        let ret = boxed_fn.ret.to_c(types);

        format!(
            r#"
{ret} {call_boxed_fn_link_name}(void* {boxed_fn_arg_name}{maybe_args});
void {free_boxed_fn_link_name}(void* {boxed_fn_arg_name});"#
        )
    }

    pub fn prefixed_fn_name(&self) -> Ident {
        let host_type_prefix = self
            .associated_type
            .as_ref()
            .map(|h| {
                match h {
                    TypeDeclaration::Shared(_) => {
                        //
                        todo!()
                    }
                    TypeDeclaration::Opaque(h) => {
                        format!("{}_", h.to_token_stream().to_string())
                    }
                }
            })
            .unwrap_or_default();
        let fn_name = &self.func.sig.ident;
        let prefixed_fn_name = Ident::new(
            &format!(
                "{}{}{}",
                SWIFT_BRIDGE_PREFIX,
                host_type_prefix,
                fn_name.to_string()
            ),
            fn_name.span(),
        );

        prefixed_fn_name
    }

    /// Get all of the `Box<dyn Fn(A, B) -> C>` arguments.
    /// We include the arguments position.
    pub fn args_filtered_to_boxed_fns(
        &self,
        type_decls: &TypeDeclarations,
    ) -> Vec<(usize, BridgeableBoxedFnOnce)> {
        self.func
            .sig
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(idx, arg)| {
                let ty = BridgedType::new_with_fn_arg(arg, type_decls)?;

                match ty {
                    BridgedType::StdLib(StdLibType::BoxedFnOnce(boxed_fn)) => Some((idx, boxed_fn)),
                    _ => None,
                }
            })
            .collect()
    }

    /// `let cb1 = __private__RustFnOnceCallback$some_function$param0(ptr: callback); let cb0 = ...`
    pub fn fnonce_callback_initializers(
        &self,
        fn_name: &str,
        maybe_associated_ty: &str,
        types: &TypeDeclarations,
    ) -> String {
        let mut initializers = "".to_string();
        let mut maybe_space = "";

        for (idx, fn_once) in self.args_filtered_to_boxed_fns(types) {
            let arg_name = self.arg_name_at_idx(idx).unwrap();

            if fn_once.params.is_empty() && fn_once.ret.is_null() {
                initializers += &format!(
                "{maybe_space}let cb{idx} = __private__RustFnOnceCallbackNoArgsNoRet(ptr: {arg_name});"
            );
            } else {
                initializers += &format!("{maybe_space}let cb{idx} = __private__RustFnOnceCallback{maybe_associated_ty}${fn_name}$param{idx}(ptr: {arg_name});");
            }

            maybe_space = " ";
        }

        initializers
    }

    /// Get the name of the argument at the given index.
    ///
    /// So, `fn some_function (foo: u32, bar: u8);`
    ///  would return "foo" for index 0 and "bar" for index 1.
    pub fn arg_name_tokens_at_idx(&self, idx: usize) -> Option<TokenStream> {
        self.func
            .sig
            .inputs
            .iter()
            .nth(idx)
            .and_then(|arg| match arg {
                FnArg::Typed(arg) => Some(arg.pat.to_token_stream()),
                FnArg::Receiver(_) => None,
            })
    }

    /// Get the name of the argument at the given index.
    ///
    /// So, `fn some_function (foo: u32, bar: u8);`
    ///  would return "foo" for index 0 and "bar" for index 1.
    pub fn arg_name_at_idx(&self, idx: usize) -> Option<String> {
        self.arg_name_tokens_at_idx(idx).map(|a| a.to_string())
    }

    /// Generate the generate bounds for a Swift function.
    /// For example:
    /// "<GenericRustString: IntoRustString>"
    pub fn maybe_swift_generics(&self, types: &TypeDeclarations) -> String {
        let mut maybe_generics = HashSet::new();

        for arg in self.sig.inputs.iter() {
            let bridged_arg = BridgedType::new_with_fn_arg(arg, types);
            if bridged_arg.is_none() {
                continue;
            }

            let bridged_arg = bridged_arg.unwrap();
            if bridged_arg.contains_owned_string_recursive(types) {
                maybe_generics.insert(SwiftFuncGenerics::String);
            } else if bridged_arg.contains_ref_string_recursive() {
                maybe_generics.insert(SwiftFuncGenerics::Str);
            }
        }

        let maybe_generics = if maybe_generics.is_empty() {
            "".to_string()
        } else {
            let mut m = vec![];

            let generics: Vec<SwiftFuncGenerics> = maybe_generics.into_iter().collect();

            for generic in generics {
                m.push(generic.as_bound())
            }

            format!("<{}>", m.join(", "))
        };

        maybe_generics
    }
}

impl Deref for ParsedExternFn {
    type Target = ForeignItemFn;

    fn deref(&self) -> &Self::Target {
        &self.func
    }
}

pub(crate) fn fn_arg_is_mutable_reference(fn_arg: &FnArg) -> bool {
    match fn_arg {
        FnArg::Receiver(receiver) => receiver.reference.is_some() && receiver.mutability.is_some(),
        FnArg::Typed(pat_ty) => match pat_ty.ty.deref() {
            Type::Reference(type_ref) => type_ref.mutability.is_some(),
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assert_tokens_contain, assert_tokens_eq, parse_ok};

    /// Verify that when generating rust call args we do not include the receiver.
    #[test]
    fn does_not_include_self_in_rust_call_args() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make1 (self);
                    fn make2 (&self);
                    fn make3 (&mut self);
                    fn make4 (self: Foo);
                    fn make5 (self: &Foo);
                    fn make6 (self: &mut Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        let methods = &module.functions;
        assert_eq!(methods.len(), 6);

        for method in methods {
            let rust_call_args =
                &method.to_call_rust_args(&module.swift_bridge_path, &module.types);
            assert_eq!(
                rust_call_args.to_string(),
                "",
                "\n Function Tokens:\n{:#?}",
                method.func.to_token_stream()
            );
        }
    }

    /// Verify that when we get an owned opaque type as an argument we unbox it.
    #[test]
    fn unboxes_owned_opaque_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn some_function (arg: Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        assert_tokens_eq(
            &module.functions[0].to_call_rust_args(&module.swift_bridge_path, &module.types),
            &quote! {unsafe { *Box::from_raw(arg) }},
        );
    }

    /// Verify that we properly take and return String arguments
    #[test]
    fn extern_rust_strings() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function (arg1: String) -> String;
                }
            }
        };
        let expected = quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg1: *mut swift_bridge::string::RustString
            ) -> *mut swift_bridge::string::RustString {
                swift_bridge::string::RustString(super::some_function(
                    unsafe { Box::from_raw(arg1).0 }
                )).box_into_raw()
            }
        };

        assert_tokens_contain(&parse_ok(tokens).to_token_stream(), &expected);
    }

    /// Verify that we properly take and return &str arguments
    #[test]
    fn extern_rust_strs() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function (arg: &str) -> &str;
                }
            }
        };
        let expected = quote! {
            #[export_name = "__swift_bridge__$some_function"]
            pub extern "C" fn __swift_bridge__some_function (
                arg: swift_bridge::string::RustStr
            ) -> swift_bridge::string::RustStr {
                swift_bridge::string::RustStr::from_str(
                    super::some_function(arg.to_str())
                )
            }
        };

        assert_tokens_contain(&parse_ok(tokens).to_token_stream(), &expected);
    }
}
