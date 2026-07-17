use crate::bridged_type::{
    BridgeableType, CFfiStruct, OnlyEncoding, TypePosition, UnusedOptionNoneValue,
};
use crate::parse::{HostLang, OpaqueRustTypeGenerics};
use crate::{TypeDeclarations, SWIFT_BRIDGE_PREFIX};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use syn::{Path, Type};

#[derive(Clone)]
pub(crate) struct OpaqueForeignType {
    pub ty: Ident,
    pub host_lang: HostLang,
    pub reference: bool,
    pub mutable: bool,
    pub has_swift_bridge_copy_annotation: bool,
    pub generics: OpaqueRustTypeGenerics,
}

impl BridgeableType for OpaqueForeignType {
    fn is_built_in_type(&self) -> bool {
        false
    }

    fn only_encoding(&self) -> Option<OnlyEncoding> {
        None
    }

    fn is_result(&self) -> bool {
        false
    }

    fn as_result(&self) -> Option<&super::bridgeable_result::BuiltInResult> {
        None
    }

    fn as_option(&self) -> Option<&super::bridged_option::BridgedOption> {
        None
    }

    fn is_passed_via_pointer(&self) -> bool {
        true
    }

    fn generate_custom_rust_ffi_types(
        &self,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> Option<Vec<TokenStream>> {
        None
    }

    fn generate_custom_c_ffi_types(&self, _types: &TypeDeclarations) -> Option<CFfiStruct> {
        None
    }

    fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        let ty_name = &self.ty;
        let generics = self
            .generics
            .angle_bracketed_concrete_generics_tokens(types);

        if self.host_lang.is_rust() {
            quote! {
                super:: #ty_name #generics
            }
        } else {
            quote! {
                #ty_name
            }
        }
    }

    fn to_swift_type(
        &self,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        if self.host_lang.is_rust() {
            match type_pos {
                TypePosition::FnArg(func_host_lang, _) | TypePosition::FnReturn(func_host_lang) => {
                    if func_host_lang.is_rust() {
                        let mut class_name = self.ty.to_string();

                        if !self.has_swift_bridge_copy_annotation {
                            if self.reference {
                                class_name += "Ref";
                            }

                            if self.mutable {
                                class_name += "Mut";
                            }
                        }

                        format!(
                            "{}{}",
                            class_name,
                            self.generics
                                .angle_bracketed_generic_concrete_swift_types_string(
                                    types,
                                    swift_bridge_path
                                )
                        )
                    } else {
                        format!("UnsafeMutableRawPointer")
                    }
                }
                TypePosition::SharedStructField => {
                    let class_name = self.ty.to_string();
                    if !self.has_swift_bridge_copy_annotation {
                        if self.mutable || self.reference {
                            todo!();
                        }
                    }

                    format!(
                        "{}{}",
                        class_name,
                        self.generics
                            .angle_bracketed_generic_concrete_swift_types_string(
                                types,
                                swift_bridge_path
                            )
                    )
                }
                TypePosition::ResultFfiReturnType => {
                    unimplemented!()
                }
                TypePosition::ThrowingInit(_) => unimplemented!(),
            }
        } else {
            match type_pos {
                TypePosition::FnArg(func_host_lang, _) | TypePosition::FnReturn(func_host_lang) => {
                    if func_host_lang.is_rust() {
                        self.ty.to_string()
                    } else {
                        "UnsafeMutableRawPointer".to_string()
                    }
                }
                TypePosition::SharedStructField => {
                    //
                    unimplemented!()
                }
                TypePosition::ResultFfiReturnType => {
                    unimplemented!()
                }
                TypePosition::ThrowingInit(_) => unimplemented!(),
            }
        }
    }

    fn to_c_type(&self, _types: &TypeDeclarations) -> String {
        if self.host_lang.is_rust() {
            if self.has_swift_bridge_copy_annotation {
                format!("struct {}", self.copy_ffi_repr_type_string())
            } else {
                "void*".to_string()
            }
        } else {
            "void*".to_string()
        }
    }

    fn to_c_include(&self, _types: &TypeDeclarations) -> Option<Vec<&'static str>> {
        None
    }

    fn to_ffi_compatible_rust_type(
        &self,
        _swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let ty_name = &self.ty;

        if self.has_swift_bridge_copy_annotation {
            let ty = self.copy_rust_repr_type();
            quote! { #ty }
        } else {
            if self.host_lang.is_rust() {
                let generics = self
                    .generics
                    .angle_bracketed_concrete_generics_tokens(types);

                if self.reference {
                    let ptr = if self.mutable {
                        quote! { *mut }
                    } else {
                        quote! { *const }
                    };

                    quote_spanned! {ty_name.span()=> #ptr super::#ty_name }
                } else {
                    quote! { *mut super::#ty_name #generics }
                }
            } else {
                quote! { #ty_name }
            }
        }
    }

    fn to_ffi_compatible_option_rust_type(
        &self,
        _swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        let type_name = &self.ty;

        if self.has_swift_bridge_copy_annotation {
            let option_ty = self.option_copy_rust_repr_type();
            quote! { #option_ty }
        } else {
            let generics = self
                .generics
                .angle_bracketed_concrete_generics_tokens(types);

            if self.reference {
                quote! { *const super::#type_name #generics }
            } else {
                quote! { *mut super::#type_name #generics }
            }
        }
    }

    fn to_ffi_compatible_option_swift_type(
        &self,
        _type_pos: TypePosition,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> String {
        todo!()
    }

    fn to_ffi_compatible_option_c_type(&self) -> String {
        if self.has_swift_bridge_copy_annotation {
            self.option_copy_ffi_repr_type_string()
        } else {
            "void*".to_string()
        }
    }

    fn convert_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        _swift_bridge_path: &Path,
        types: &TypeDeclarations,
        span: Span,
    ) -> TokenStream {
        let ty_name = &self.ty;

        if self.host_lang.is_rust() {
            if self.has_swift_bridge_copy_annotation {
                let copy_ty = self.copy_rust_repr_type();
                quote! {
                    #copy_ty::from_rust_repr(#expression)
                }
            } else if self.reference {
                let ptr = if self.mutable {
                    quote! { *mut }
                } else {
                    quote! { *const }
                };

                quote! {
                    #expression as #ptr super::#ty_name
                }
            } else {
                let generics = self
                    .generics
                    .angle_bracketed_concrete_generics_tokens(types);
                quote_spanned! {span=>
                    Box::into_raw(Box::new({
                        let val: super::#ty_name #generics = #expression;
                        val
                    })) as *mut super::#ty_name #generics
                }
            }
        } else {
            quote! {
                #expression
            }
        }
    }

    fn convert_option_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        _swift_bridge_path: &Path,
    ) -> TokenStream {
        if self.has_swift_bridge_copy_annotation {
            let copy_repr = self.copy_rust_repr_type();
            let option_copy_repr = self.option_copy_rust_repr_type();

            quote! {
                if let Some(val) = #expression {
                    #option_copy_repr {
                        is_some: true,
                        val: std::mem::MaybeUninit::new(#copy_repr::from_rust_repr(val))
                    }
                } else {
                    #option_copy_repr {
                        is_some: false,
                        val: std::mem::MaybeUninit::uninit()
                    }
                }
            }
        } else if self.reference {
            let ty = &self.ty;

            quote! {
                if let Some(val) = #expression {
                    val as *const super::#ty
                } else {
                    std::ptr::null()
                }
            }
        } else {
            match self.host_lang {
                HostLang::Rust => {
                    quote! {
                        if let Some(val) = #expression {
                            Box::into_raw(Box::new(val))
                        } else {
                            std::ptr::null_mut()
                        }
                    }
                }
                HostLang::Swift => {
                    let ty = &self.ty;

                    // Here we are converting a Swift type from its Rust representation to its FFI
                    // representation.
                    // When we drop the Rust representation we do not want to free the backing Swift
                    // type since we are passing ownership to Swift.
                    // So, we are transitioning this Swift type from its
                    // `RustRepr -> FfiRepr -> SwiftRepr`.
                    // This means that the Swift side will be responsible for freeing the Swift type
                    // whenever it is done with it.
                    // Here we use `ManuallyDrop` to avoid freeing the Swift type.
                    quote! {
                        if let Some(val) = #expression {
                            let val = std::mem::ManuallyDrop::new(val);
                            val.0 as *mut super::#ty
                        } else {
                            std::ptr::null_mut()
                        }
                    }
                }
            }
        }
    }

    fn convert_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        _types: &TypeDeclarations,
        type_pos: TypePosition,
    ) -> String {
        let ty_name = &self.ty;

        if self.host_lang.is_rust() {
            if self.has_swift_bridge_copy_annotation {
                format!("{}.intoFfiRepr()", expression)
            } else if self.reference {
                format!("{}.ptr", expression)
            } else {
                match type_pos {
                    TypePosition::FnArg(func_host_lang, _)
                    | TypePosition::FnReturn(func_host_lang) => {
                        if func_host_lang.is_rust() {
                            if self.reference {
                                format!("{{return {}.ptr;}}()", expression)
                            } else {
                                format!(
                                    "{{{}.isOwned = false; return {}.ptr;}}()",
                                    expression, expression
                                )
                            }
                        } else {
                            if self.reference {
                                format!("{{return {}.ptr;}}()", expression)
                            } else {
                                format!(
                                    "{{{}.isOwned = false; return {}.ptr;}}()",
                                    expression, expression
                                )
                            }
                        }
                    }
                    TypePosition::SharedStructField => {
                        format!(
                            "{{{}.isOwned = false; return {}.ptr;}}()",
                            expression, expression
                        )
                    }
                    TypePosition::ResultFfiReturnType => {
                        unimplemented!()
                    }
                    TypePosition::ThrowingInit(_) => unimplemented!(),
                }
            }
        } else {
            match type_pos {
                TypePosition::FnArg(func_host_lang, _) => {
                    if func_host_lang.is_rust() {
                        format!("Unmanaged.passRetained({}).toOpaque()", expression)
                    } else {
                        format!(
                            "Unmanaged<{type_name}>.fromOpaque({value}).takeRetainedValue()",
                            type_name = ty_name,
                            value = expression
                        )
                    }
                }
                TypePosition::FnReturn(_func_host_lang) => {
                    format!("Unmanaged.passRetained({}).toOpaque()", expression)
                }
                TypePosition::SharedStructField => {
                    todo!("Opaque types in shared struct fields are not yet supported")
                }
                TypePosition::ResultFfiReturnType => {
                    unimplemented!()
                }
                TypePosition::ThrowingInit(_) => unimplemented!(),
            }
        }
    }

    fn convert_option_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        _type_pos: TypePosition,
    ) -> String {
        if self.has_swift_bridge_copy_annotation {
            let ffi_repr = self.copy_ffi_repr_type_string();
            let option_ffi_repr = self.option_copy_ffi_repr_type_string();

            format!("{option_ffi_repr}(is_some: {expression} != nil, val: {{ if let val = {expression} {{ return val.intoFfiRepr() }} else {{ return {ffi_repr}() }} }}() )", expression = expression,
                        option_ffi_repr = option_ffi_repr,
                        ffi_repr = ffi_repr
                    )
        } else if self.reference {
            format!(
                "{{ if let val = {expression} {{ return val.ptr }} else {{ return nil }} }}()",
                expression = expression,
            )
        } else {
            match self.host_lang {
                HostLang::Rust => {
                    format!("{{ if let val = {expression} {{ val.isOwned = false; return val.ptr }} else {{ return nil }} }}()", expression = expression,)
                }
                HostLang::Swift => {
                    format!("{{ if let val = {expression} {{ return Unmanaged.passRetained(val).toOpaque() }} else {{ return nil }} }}()")
                }
            }
        }
    }

    fn convert_ffi_expression_to_rust_type(
        &self,
        expression: &TokenStream,
        _span: Span,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        if self.host_lang.is_rust() {
            if self.has_swift_bridge_copy_annotation {
                let maybe_ref = if self.reference {
                    quote! {&}
                } else {
                    quote! {}
                };
                quote! {
                    #maybe_ref #expression.into_rust_repr()
                }
            } else if self.reference {
                let maybe_mut = if self.mutable {
                    quote! { mut }
                } else {
                    quote! {}
                };

                quote! {
                    unsafe {  & #maybe_mut * #expression }
                }
            } else {
                quote! {
                    unsafe { * Box::from_raw(  #expression ) }
                }
            }
        } else {
            if self.reference {
                todo!("Handle referenced self Swift types")
            } else {
                quote! {
                    #expression
                }
            }
        }
    }

    fn convert_ffi_option_expression_to_rust_type(&self, expression: &TokenStream) -> TokenStream {
        if self.has_swift_bridge_copy_annotation {
            quote! {
                if #expression.is_some {
                    Some(unsafe{ #expression.val.assume_init() }.into_rust_repr())
                } else {
                    None
                }
            }
        } else if self.reference {
            quote! {
                if #expression.is_null() {
                    None
                } else {
                    Some(unsafe {& * #expression} )
                }
            }
        } else {
            match self.host_lang {
                HostLang::Rust => {
                    quote! {
                        if #expression.is_null() {
                            None
                        } else {
                            Some(unsafe { *Box::from_raw(#expression) } )
                        }
                    }
                }
                HostLang::Swift => {
                    let ty = &self.ty;
                    quote! {
                        {
                            let val = #expression;
                            if val.is_null() {
                                None
                            } else {
                                Some(#ty(val as *mut std::ffi::c_void))
                            }
                        }
                    }
                }
            }
        }
    }

    fn convert_ffi_expression_to_swift_type(
        &self,
        expression: &str,
        type_pos: TypePosition,
        _types: &TypeDeclarations,
        _swift_bridge_path: &Path,
    ) -> String {
        let mut ty_name = self.ty.to_string();

        if self.reference {
            ty_name += "Ref";
        }
        if self.mutable {
            ty_name += "Mut";
        }

        if self.host_lang.is_rust() {
            if self.has_swift_bridge_copy_annotation {
                format!(
                    "{ty_name}(bytes: {value})",
                    ty_name = ty_name,
                    value = expression,
                )
            } else {
                match type_pos {
                    TypePosition::FnReturn(fn_host_lang) if fn_host_lang.is_swift() => {
                        format!("Unmanaged.passRetained({expression}).toOpaque()")
                    }
                    _ => {
                        format!(
                            "{ty_name}(ptr: {value})",
                            ty_name = ty_name,
                            value = expression,
                        )
                    }
                }
            }
        } else {
            format!(
                "Unmanaged<{ty_name}>.fromOpaque({value}).takeRetainedValue()",
                ty_name = ty_name,
                value = expression
            )
        }
    }

    fn convert_ffi_option_expression_to_swift_type(&self, expression: &str) -> String {
        if self.has_swift_bridge_copy_annotation {
            let type_name = self.swift_name();
            format!(
                "{{ let val = {expression}; if val.is_some {{ return {type_name}(bytes: val.val) }} else {{ return nil }} }}()",
                expression = expression,
                type_name = type_name
            )
        } else {
            let type_name = self.swift_name();
            match self.host_lang {
                HostLang::Rust => {
                    format!(
                        "{{ let val = {expression}; if val != nil {{ return {type_name}(ptr: val!) }} else {{ return nil }} }}()",
                        expression = expression,
                        type_name = type_name
                    )
                }
                HostLang::Swift => {
                    format!(
                        "{{ if let val = {expression} {{ return Unmanaged<{type_name}>.fromOpaque(val).takeRetainedValue() }} else {{ return nil }} }}()"
                    )
                }
            }
        }
    }

    fn convert_ffi_result_ok_value_to_rust_value(
        &self,
        result: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        let ty = &self.ty;

        match self.host_lang {
            HostLang::Rust => {
                quote! {
                    unsafe { *Box::from_raw(#result.ok_or_err as *mut super::#ty) }
                }
            }
            HostLang::Swift => {
                quote! {
                    unsafe { #ty(#result.ok_or_err) }
                }
            }
        }
    }

    fn convert_ffi_result_err_value_to_rust_value(
        &self,
        result: &TokenStream,
        _swift_bridge_path: &Path,
        _types: &TypeDeclarations,
    ) -> TokenStream {
        let ty = &self.ty;

        match self.host_lang {
            HostLang::Rust => {
                quote! {
                    unsafe { *Box::from_raw(#result.ok_or_err as *mut super::#ty) }
                }
            }
            HostLang::Swift => {
                quote! {
                    unsafe { #ty(#result.ok_or_err) }
                }
            }
        }
    }

    fn unused_option_none_val(&self, _swift_bridge_path: &Path) -> UnusedOptionNoneValue {
        let ty_name = &self.ty;

        if self.reference {
            todo!("Support returning Option<&T> where T is an opaque type")
        } else {
            UnusedOptionNoneValue {
                rust: quote! { std::ptr::null::<#ty_name>() as *mut super::#ty_name },
                swift: "TODO..Support Swift Option<T>::None value".into(),
            }
        }
    }

    fn can_parse_token_stream_str(_tokens: &str) -> bool
    where
        Self: Sized,
    {
        // TODO: Make this unreachable!() once we finish moving towards `BridgeableType` instead
        //  of the old `BridgedType`.
        // unreachable!()
        true
    }

    fn from_type(ty: &Type, types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        match ty {
            Type::Path(path) => {
                if let Some(ty) = types.get_with_type_path(path) {
                    ty.to_opaque_type(false, false)
                } else {
                    Self::parse_token_stream_str(
                        path.path.segments.to_token_stream().to_string().as_str(),
                        types,
                    )
                }
            }
            Type::Reference(ty_ref) => match ty_ref.elem.deref() {
                Type::Path(p) => {
                    if let Some(ty) = types.get_with_type_path(p) {
                        ty.to_opaque_type(true, ty_ref.mutability.is_some())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_token_stream_str(tokens: &str, types: &TypeDeclarations) -> Option<Self>
    where
        Self: Sized,
    {
        let bridged_type = types.get(tokens)?;
        bridged_type.to_opaque_type(false, false)
    }

    fn is_null(&self) -> bool {
        false
    }

    fn is_str(&self) -> bool {
        false
    }

    fn contains_owned_string_recursive(&self, _types: &TypeDeclarations) -> bool {
        false
    }

    fn contains_ref_string_recursive(&self) -> bool {
        false
    }

    fn has_swift_bridge_copy_annotation(&self) -> bool {
        self.has_swift_bridge_copy_annotation
    }

    fn to_alpha_numeric_underscore_name(&self, _types: &TypeDeclarations) -> String {
        if self.generics.len() >= 1 {
            todo!()
        }
        self.ty.to_string()
    }
}

impl OpaqueForeignType {
    pub fn swift_name(&self) -> String {
        if self.reference {
            format!("{}Ref", self.ty)
        } else {
            format!("{}", self.ty)
        }
    }

    /// The name of the type used to pass a `#[swift_bridge(Copy(...))]` type over FFI
    ///
    /// __swift_bridge__SomeType
    pub fn copy_rust_repr_type(&self) -> Ident {
        let ty = format!(
            "{}{}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.underscore_prefixed_generics_string()
        );
        Ident::new(&ty, self.ty.span())
    }

    /// The name of the type used to pass a `Option<T>` where T is
    /// `#[swift_bridge(Copy(...))]` over FFI
    ///
    /// __swift_bridge__Option_SomeType
    pub fn option_copy_rust_repr_type(&self) -> Ident {
        let ty = format!(
            "{}Option_{}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.underscore_prefixed_generics_string()
        );
        Ident::new(&ty, self.ty.span())
    }

    /// The FFI name of the type used to pass a `Option<T>` where T is
    /// `#[swift_bridge(Copy(...))]` over FFI
    ///
    /// __swift_bridge__$Option$SomeType
    pub fn option_copy_ffi_repr_type_string(&self) -> String {
        format!(
            "{}$Option${}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.dollar_prefixed_generics_string()
        )
    }

    /// The name of the type used to pass a `#[swift_bridge(Copy(...))]` type over FFI
    pub fn copy_ffi_repr_type_string(&self) -> String {
        format!(
            "{}${}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.dollar_prefixed_generics_string()
        )
    }
}

impl Debug for OpaqueForeignType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpaqueForeignType")
            .field("ty", &self.ty.to_token_stream())
            .field("host_lang", &self.host_lang)
            .field("reference", &self.reference)
            .field("mutable", &self.mutable)
            .finish()
    }
}

impl PartialEq for OpaqueForeignType {
    fn eq(&self, other: &Self) -> bool {
        self.ty.to_token_stream().to_string() == other.ty.to_token_stream().to_string()
            && self.host_lang == other.host_lang
            && self.reference == other.reference
            && self.mutable == other.mutable
    }
}

impl Deref for OpaqueForeignType {
    type Target = Ident;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}
