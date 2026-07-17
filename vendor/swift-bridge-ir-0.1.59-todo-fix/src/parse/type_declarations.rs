use crate::bridged_type::{
    BridgedType, CustomBridgedType, OpaqueForeignType, SharedEnum, SharedStruct, SharedType,
};
use crate::parse::parse_extern_mod::OpaqueTypeAllAttributes;
use crate::parse::HostLang;
use crate::SWIFT_BRIDGE_PREFIX;
use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use std::collections::HashMap;
use std::ops::Deref;
use syn::{PatType, Type, TypePath};

mod generics;
pub(crate) use self::generics::*;

#[derive(Default)]
pub(crate) struct TypeDeclarations {
    decls: HashMap<String, TypeDeclaration>,
    order: Vec<String>,
}

#[derive(Clone)]
pub(crate) enum TypeDeclaration {
    Shared(SharedTypeDeclaration),
    Opaque(OpaqueForeignTypeDeclaration),
}

#[derive(Clone)]
pub(crate) enum SharedTypeDeclaration {
    Struct(SharedStruct),
    Enum(SharedEnum),
}

impl TypeDeclaration {
    pub fn to_bridged_type(&self, reference: bool, mutable: bool) -> BridgedType {
        match self {
            TypeDeclaration::Shared(SharedTypeDeclaration::Struct(shared_struct)) => {
                BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(
                    shared_struct.clone(),
                )))
            }
            TypeDeclaration::Shared(SharedTypeDeclaration::Enum(shared_enum)) => {
                BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(
                    shared_enum.clone(),
                )))
            }
            TypeDeclaration::Opaque(_o) => {
                BridgedType::Bridgeable(Box::new(self.to_opaque_type(reference, mutable).unwrap()))
            }
        }
    }

    pub fn to_opaque_type(&self, reference: bool, mutable: bool) -> Option<OpaqueForeignType> {
        match self {
            TypeDeclaration::Opaque(opaque) => Some(OpaqueForeignType {
                ty: opaque.ty.clone(),
                host_lang: opaque.host_lang,
                reference,
                mutable,
                has_swift_bridge_copy_annotation: opaque.attributes.copy.is_some(),
                generics: opaque.generics.clone(),
            }),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct OpaqueForeignTypeDeclaration {
    pub ty: Ident,
    pub host_lang: HostLang,
    pub attributes: OpaqueTypeAllAttributes,
    pub generics: OpaqueRustTypeGenerics,
}

impl OpaqueForeignTypeDeclaration {
    pub(crate) fn ffi_repr_type_tokens(&self) -> TokenStream {
        if self.attributes.copy.is_some() {
            self.ffi_copy_repr_ident().to_token_stream()
        } else {
            let ty_name = &self.ty;
            quote::quote! {
                *mut super::#ty_name
            }
        }
    }

    /// The name of the FFI representation for an opaque Rust type.
    /// __swift_bridge__$SomeType
    pub(crate) fn ffi_repr_name_string(&self) -> String {
        format!(
            "{}${}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.dollar_prefixed_generics_string()
        )
    }

    /// The C FFI link name of the function used to free memory for this opaque Rust type.
    ///
    /// For `type SomeType<u32>` this would be:
    /// "__swift_bridge__$SomeType$u32$_free"
    pub(crate) fn free_rust_opaque_type_ffi_name(&self) -> String {
        format!(
            "{}${}{}$_free",
            SWIFT_BRIDGE_PREFIX,
            self.to_string(),
            self.generics.dollar_prefixed_generics_string(),
        )
    }

    /// The Rust function used to free memory for this opaque Rust type.
    ///
    /// For `type SomeType<u32>` this would be:
    /// "__swift_bridge__SomeType_u32__free"
    pub(crate) fn free_rust_opaque_type_ident(&self) -> Ident {
        Ident::new(
            &format!(
                "{}{}{}__free",
                SWIFT_BRIDGE_PREFIX,
                self.ty.to_string(),
                self.generics.underscore_prefixed_generics_string(),
            ),
            self.ty.span(),
        )
    }

    /// The identifier for the `#[repr(C)] __swift_bridge__SomeStruct([u8; 123usize])`
    /// type that is generated to pass a Copy type over FFI.
    pub(crate) fn ffi_copy_repr_ident(&self) -> Ident {
        Ident::new(
            &format!(
                "{}{}{}",
                SWIFT_BRIDGE_PREFIX,
                self.ty,
                self.generics.underscore_prefixed_generics_string()
            ),
            self.ty.span(),
        )
    }

    /// The identifier for the FFI representation of an `Option<T>` where `T` is
    /// the `#[repr(C)] __swift_bridge__SomeStruct([u8; 123usize])`
    /// type that is generated to pass a Copy type over FFI.
    pub(crate) fn ffi_option_copy_repr_ident(&self) -> Ident {
        Ident::new(
            &format!(
                "{}Option_{}{}",
                SWIFT_BRIDGE_PREFIX,
                self.ty,
                self.generics.underscore_prefixed_generics_string()
            ),
            self.ty.span(),
        )
    }

    /// The String for the FFI representation of the type used to pass an Option Copy Opaque Rust
    /// type over FFI.
    pub(crate) fn ffi_option_copy_repr_string(&self) -> String {
        format!(
            "{}$Option${}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.dollar_prefixed_generics_string()
        )
    }

    /// The String for the FFI representation of the type used to pass a Copy Opaque Rust type
    /// over FFI.
    pub(crate) fn ffi_copy_repr_string(&self) -> String {
        format!(
            "{}${}{}",
            SWIFT_BRIDGE_PREFIX,
            self.ty,
            self.generics.dollar_prefixed_generics_string()
        )
    }
}

#[derive(Copy, Clone)]
pub(crate) struct OpaqueCopy {
    /// The size of the opaque type, in bytes.
    pub(crate) size_bytes: usize,
}

impl Deref for OpaqueForeignTypeDeclaration {
    type Target = Ident;

    fn deref(&self) -> &Self::Target {
        &self.ty
    }
}

impl OpaqueForeignTypeDeclaration {
    // "__swift_bridge__$TypeName$_free"
    pub fn free_swift_class_link_name(&self) -> String {
        format!("{}${}$_free", SWIFT_BRIDGE_PREFIX, self.ty.to_string())
    }

    // "__swift_bridge__TypeName__free"
    pub fn free_swift_class_func_name(&self) -> String {
        format!("{}{}__free", SWIFT_BRIDGE_PREFIX, self.ty.to_string())
    }

    pub fn ty_name_ident(&self) -> &Ident {
        &self.ty
    }

    pub fn ty_name_string(&self) -> String {
        self.ty.to_string()
    }
}

impl TypeDeclarations {
    pub(crate) fn get<Q: ?Sized>(&self, type_name: &Q) -> Option<&TypeDeclaration>
    where
        Q: std::hash::Hash + Eq,
        String: std::borrow::Borrow<Q>,
    {
        self.decls.get(type_name)
    }

    pub(crate) fn get_with_pat_type(&self, pat_ty: &PatType) -> Option<&TypeDeclaration> {
        self.get_with_type(&pat_ty.ty)
    }

    pub(crate) fn get_with_type_path(&self, type_path: &TypePath) -> Option<&TypeDeclaration> {
        let ty = type_path.path.to_token_stream().to_string();

        // Handles generics. i.e. "SomeType < u32, u64 >" -> SomeType<u32,u64>
        let ty = ty.replace(" ", "");

        self.get(&ty)
    }

    pub(crate) fn get_with_type(&self, ty: &Type) -> Option<&TypeDeclaration> {
        let ty = match ty {
            Type::Reference(reference) => reference.elem.to_token_stream().to_string(),
            Type::Path(path) => path.to_token_stream().to_string(),
            _ => todo!("Handle other cases"),
        };
        self.get(&ty)
    }

    pub(crate) fn insert(&mut self, type_name: String, ty: TypeDeclaration) {
        self.decls.insert(type_name.clone(), ty);
        self.order.push(type_name);
    }

    pub fn types(&self) -> Vec<&TypeDeclaration> {
        self.order
            .iter()
            .map(|ty| self.decls.get(ty).unwrap())
            .collect()
    }
}

impl TypeDeclaration {
    pub(crate) fn as_opaque(&self) -> Option<&OpaqueForeignTypeDeclaration> {
        match self {
            TypeDeclaration::Opaque(o) => Some(o),
            _ => None,
        }
    }
}

#[cfg(test)]
impl TypeDeclaration {
    pub fn unwrap_shared_struct(&self) -> &SharedStruct {
        match self {
            TypeDeclaration::Shared(SharedTypeDeclaration::Struct(s)) => s,
            _ => panic!(),
        }
    }

    pub fn unwrap_shared_enum(&self) -> &SharedEnum {
        match self {
            TypeDeclaration::Shared(SharedTypeDeclaration::Enum(e)) => e,
            _ => panic!(),
        }
    }

    pub fn unwrap_opaque(&self) -> &OpaqueForeignTypeDeclaration {
        self.as_opaque().unwrap()
    }
}
