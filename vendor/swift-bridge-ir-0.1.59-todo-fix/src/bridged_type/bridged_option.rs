use crate::bridged_type::built_in_primitive::BuiltInPrimitive;
use crate::bridged_type::{BridgedType, CustomBridgedType, SharedType, StdLibType, TypePosition};
use crate::parse::TypeDeclarations;
use proc_macro2::TokenStream;
use quote::quote;
use std::ops::Deref;
use syn::Path;

/// Option<T>
#[derive(Debug)]
pub(crate) struct BridgedOption {
    pub ty: Box<BridgedType>,
}

impl BridgedOption {
    pub(super) fn convert_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        swift_bridge_path: &Path,
    ) -> TokenStream {
        let option_rust_primitive_to_ffi_primitive =
            move |ffi_option_name: TokenStream, unused_none: TokenStream| {
                quote! {
                    if let Some(val) = #expression {
                        #swift_bridge_path::option::#ffi_option_name { val, is_some: true }
                    } else {
                        #swift_bridge_path::option::#ffi_option_name { val: #unused_none, is_some: false }
                    }
                }
            };

        match self.ty.deref() {
            BridgedType::Bridgeable(b) => {
                b.convert_option_rust_expression_to_ffi_type(expression, swift_bridge_path)
            }
            BridgedType::StdLib(stdlib_type) => match stdlib_type {
                StdLibType::Null => {
                    todo!("Option<()> is not yet supported")
                }
                StdLibType::U8 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionU8}, quote! {123})
                }
                StdLibType::I8 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionI8}, quote! {123})
                }
                StdLibType::U16 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionU16}, quote! {123})
                }
                StdLibType::I16 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionI16}, quote! {123})
                }
                StdLibType::U32 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionU32}, quote! {123})
                }
                StdLibType::I32 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionI32}, quote! {123})
                }
                StdLibType::U64 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionU64}, quote! {123})
                }
                StdLibType::I64 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionI64}, quote! {123})
                }
                StdLibType::Usize => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionUsize}, quote! {123})
                }
                StdLibType::Isize => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionIsize}, quote! {123})
                }
                StdLibType::F32 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionF32}, quote! {123.4})
                }
                StdLibType::F64 => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionF64}, quote! {123.4})
                }
                StdLibType::Bool => {
                    option_rust_primitive_to_ffi_primitive(quote! {OptionBool}, quote! {false})
                }
                StdLibType::Pointer(_) => {
                    todo!("Support Option<*const T> and Option<*mut T>")
                }
                StdLibType::RefSlice(_) => {
                    todo!("Support Option<&[T]> and Option<&mut [T]>")
                }
                StdLibType::Str => {
                    quote! {
                        if let Some(val) = #expression {
                            #swift_bridge_path::string::RustStr::from_str(val)
                        } else {
                            #swift_bridge_path::string::RustStr { start: std::ptr::null::<u8>(), len: 0}
                        }
                    }
                }
                StdLibType::Vec(_) => {
                    quote! {
                        if let Some(value) = #expression {
                            Box::into_raw(Box::new(value))
                        } else {
                            std::ptr::null_mut()
                        }
                    }
                }
                StdLibType::Option(_) => {
                    todo!("Support Option<Option<T>>")
                }
                StdLibType::Result(_) => {
                    todo!("Support Option<Result<T, E>>")
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!("Option<Box<dyn FnOnce(A, B) -> C>> is not yet supported")
                }
                StdLibType::Tuple(_) => todo!(),
            },
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(shared_struct))) => {
                let option_name = shared_struct.ffi_option_name_tokens();
                quote! {
                    #option_name::from_rust_repr(#expression)
                }
            }
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(shared_enum))) => {
                let option_name = shared_enum.ffi_option_name_tokens();
                quote! {
                    #option_name::from_rust_repr(#expression)
                }
            }
        }
    }

    pub(super) fn convert_ffi_expression_to_rust_type(
        &self,
        expression: &TokenStream,
    ) -> TokenStream {
        match self.ty.deref() {
            BridgedType::Bridgeable(b) => b.convert_ffi_option_expression_to_rust_type(expression),
            BridgedType::StdLib(stdlib_ty) => match stdlib_ty {
                StdLibType::Null => {
                    todo!("Option<()> is not yet supported")
                }
                StdLibType::U8
                | StdLibType::I8
                | StdLibType::U16
                | StdLibType::I16
                | StdLibType::U32
                | StdLibType::I32
                | StdLibType::U64
                | StdLibType::I64
                | StdLibType::Usize
                | StdLibType::Isize
                | StdLibType::F32
                | StdLibType::F64
                | StdLibType::Bool => {
                    quote! {
                        {
                            let val = #expression;
                            if val.is_some {
                                Some(val.val)
                            } else {
                                None
                            }
                        }
                    }
                }
                StdLibType::Pointer(_) => {
                    todo!("Option<*const T> and Option<*mut T> are not yet supported.")
                }
                StdLibType::RefSlice(_) => {
                    todo!("Option<*const T> and Option<*mut T> are not yet supported.")
                }
                StdLibType::Str => {
                    quote! {
                        {
                            let val = #expression;

                            if val.start.is_null() {
                                None
                            } else {
                                Some( val.to_str() )
                            }
                        }
                    }
                }
                StdLibType::Vec(_) => {
                    quote! {
                        if #expression.is_null() { None } else { Some( unsafe { * Box::from_raw(#expression) } ) }
                    }
                }
                StdLibType::Option(_) => {
                    todo!("Option<Option<T>> is not yet supported")
                }
                StdLibType::Result(_) => {
                    todo!("Option<Result<T, E>> is not yet supported")
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!("Option<Box<dyn FnOnce(A, B) -> C>> is not yet supported")
                }
                StdLibType::Tuple(_) => todo!(),
            },
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(_shared_struct))) => {
                quote! {
                    #expression.into_rust_repr()
                }
            }
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(_shared_enum))) => {
                quote! {
                    #expression.into_rust_repr()
                }
            }
        }
    }

    pub(super) fn convert_ffi_expression_to_swift_type(&self, expression: &str) -> String {
        match self.ty.deref() {
            BridgedType::Bridgeable(b) => b.convert_ffi_option_expression_to_swift_type(expression),
            BridgedType::StdLib(stdlib_type) => match stdlib_type {
                StdLibType::Null => {
                    todo!("Option<()> is not yet supported")
                }
                StdLibType::U8
                | StdLibType::I8
                | StdLibType::U16
                | StdLibType::I16
                | StdLibType::U32
                | StdLibType::I32
                | StdLibType::U64
                | StdLibType::I64
                | StdLibType::Usize
                | StdLibType::Isize
                | StdLibType::F32
                | StdLibType::F64
                | StdLibType::Bool => {
                    format!("{expression}.intoSwiftRepr()")
                }
                StdLibType::Pointer(_) => {
                    todo!("Support Option<*const T> and Option<*mut T>")
                }
                StdLibType::RefSlice(_) => {
                    todo!("Support Option<&[T]>")
                }
                StdLibType::Str => {
                    format!(
                            "{{ let val = {val}; if val.start != nil {{ return val; }} else {{ return nil; }} }}()",
                            val = expression,
                        )
                }
                StdLibType::Vec(_) => {
                    format!(
                        "{{ let val = {expression}; if val != nil {{ return RustVec(ptr: val!) }} else {{ return nil }} }}()"
                    ,
                    expression = expression
                    )
                }
                StdLibType::Option(_) => {
                    todo!("Support Option<Option<T>>")
                }
                StdLibType::Result(_) => {
                    todo!("Option<Result<T, E>> is not yet supported")
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!("Option<Box<dyn FnOnce(A, B) -> C>> is not yet supported")
                }
                StdLibType::Tuple(_) => todo!(),
            },
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(_shared_struct))) => {
                format!("{expression}.intoSwiftRepr()", expression = expression)
            }
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(_shared_enum))) => {
                format!("{expression}.intoSwiftRepr()", expression = expression)
            }
        }
    }

    pub fn convert_swift_expression_to_ffi_type(
        &self,
        expression: &str,
        type_pos: TypePosition,
    ) -> String {
        match self.ty.deref() {
            BridgedType::Bridgeable(b) => {
                b.convert_option_swift_expression_to_ffi_type(expression, type_pos)
            }
            BridgedType::StdLib(stdlib_type) => match stdlib_type {
                StdLibType::Null => {
                    todo!("Option<()> is not yet supported")
                }
                StdLibType::U8
                | StdLibType::I8
                | StdLibType::U16
                | StdLibType::I16
                | StdLibType::U32
                | StdLibType::I32
                | StdLibType::U64
                | StdLibType::I64
                | StdLibType::Usize
                | StdLibType::Isize
                | StdLibType::F32
                | StdLibType::F64
                | StdLibType::Bool => {
                    format!("{expression}.intoFfiRepr()")
                }
                StdLibType::Pointer(_) => {
                    todo!("Option<*const T> and Option<*mut T> are not yet supported")
                }
                StdLibType::RefSlice(_) => {
                    todo!("Option<&[T]> is not yet supported")
                }
                StdLibType::Str => match type_pos {
                    TypePosition::FnArg(host_lang, _) => {
                        if host_lang.is_rust() {
                            format!("{expression}AsRustStr", expression = expression)
                        } else {
                            todo!()
                        }
                    }
                    TypePosition::FnReturn(host_lang) => {
                        if host_lang.is_rust() {
                            format!("{expression}AsRustStr", expression = expression)
                        } else {
                            todo!()
                        }
                    }
                    TypePosition::SharedStructField => {
                        todo!()
                    }
                    TypePosition::ResultFfiReturnType => {
                        todo!()
                    }
                    TypePosition::ThrowingInit(_) => todo!(),
                },
                StdLibType::Vec(_) => {
                    format!(
                        "{{ if let val = {expression} {{ val.isOwned = false; return val.ptr }} else {{ return nil }} }}()"
                    , expression = expression
                    )
                }
                StdLibType::Option(_) => {
                    todo!("Option<Option<T> is not yet supported")
                }
                StdLibType::Result(_) => {
                    todo!("Option<Result<T, E>> is not yet supported")
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!("Option<Box<dyn FnOnce(A, B) -> C>> is not yet supported")
                }
                StdLibType::Tuple(_) => todo!(),
            },
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(shared_struct))) => {
                let ffi_name = shared_struct.ffi_option_name_string();
                format!(
                    "{ffi_name}.fromSwiftRepr({expression})",
                    ffi_name = ffi_name,
                    expression = expression
                )
            }
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(shared_enum))) => {
                let ffi_name = shared_enum.ffi_option_name_string();
                format!(
                    "{ffi_name}.fromSwiftRepr({expression})",
                    ffi_name = ffi_name,
                    expression = expression
                )
            }
        }
    }

    pub fn to_swift_type(
        &self,
        swift_bridge_path: &Path,
        type_pos: TypePosition,
        types: &TypeDeclarations,
    ) -> String {
        match type_pos {
            TypePosition::FnArg(func_host_lang, _) => {
                if func_host_lang.is_swift() {
                    self.to_ffi_compatible_swift_type(type_pos, swift_bridge_path, &types)
                } else {
                    format!(
                        "Optional<{}>",
                        self.ty.to_swift_type(type_pos, types, swift_bridge_path)
                    )
                }
            }
            TypePosition::FnReturn(func_host_lang) => {
                if func_host_lang.is_swift() {
                    self.to_ffi_compatible_swift_type(type_pos, swift_bridge_path, &types)
                } else {
                    format!(
                        "Optional<{}>",
                        self.ty.to_swift_type(type_pos, types, swift_bridge_path)
                    )
                }
            }
            TypePosition::SharedStructField => {
                format!(
                    "Optional<{}>",
                    self.ty.to_swift_type(type_pos, types, swift_bridge_path)
                )
            }
            TypePosition::ResultFfiReturnType => {
                unimplemented!()
            }
            TypePosition::ThrowingInit(_) => unimplemented!(),
        }
    }

    fn to_ffi_compatible_swift_type(
        &self,
        type_pos: TypePosition,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> String {
        match self.ty.deref() {
            BridgedType::StdLib(stdlib_type) => match stdlib_type {
                StdLibType::Null => {
                    todo!()
                }
                StdLibType::U8
                | StdLibType::I8
                | StdLibType::U16
                | StdLibType::I16
                | StdLibType::U32
                | StdLibType::I32
                | StdLibType::U64
                | StdLibType::I64
                | StdLibType::Usize
                | StdLibType::Isize
                | StdLibType::F32
                | StdLibType::F64
                | StdLibType::Bool => BuiltInPrimitive::new_with_stdlib_type(stdlib_type)
                    .unwrap()
                    .to_option_ffi_repr_name()
                    .to_string(),
                StdLibType::Pointer(_) => {
                    todo!()
                }
                StdLibType::RefSlice(_) => {
                    todo!()
                }
                StdLibType::Str => "RustStr".to_string(),
                StdLibType::Vec(_) => {
                    todo!()
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!()
                }
                StdLibType::Option(_) => {
                    todo!()
                }
                StdLibType::Result(_) => {
                    todo!()
                }
                StdLibType::Tuple(_) => {
                    todo!()
                }
            },
            BridgedType::Foreign(_) => {
                todo!()
            }
            BridgedType::Bridgeable(bridgeable) => {
                bridgeable.to_ffi_compatible_option_swift_type(type_pos, swift_bridge_path, types)
            }
        }
    }
}

impl BridgedOption {
    pub fn to_c(&self) -> String {
        match self.ty.deref() {
            BridgedType::Bridgeable(b) => b.to_ffi_compatible_option_c_type(),
            BridgedType::StdLib(stdlib_type) => match stdlib_type {
                StdLibType::Null => {
                    todo!("Option<()> is not yet supported")
                }
                StdLibType::U8 => "struct __private__OptionU8".to_string(),
                StdLibType::I8 => "struct __private__OptionI8".to_string(),
                StdLibType::U16 => "struct __private__OptionU16".to_string(),
                StdLibType::I16 => "struct __private__OptionI16".to_string(),
                StdLibType::U32 => "struct __private__OptionU32".to_string(),
                StdLibType::I32 => "struct __private__OptionI32".to_string(),
                StdLibType::U64 => "struct __private__OptionU64".to_string(),
                StdLibType::I64 => "struct __private__OptionI64".to_string(),
                StdLibType::Usize => "struct __private__OptionUsize".to_string(),
                StdLibType::Isize => "struct __private__OptionIsize".to_string(),
                StdLibType::F32 => "struct __private__OptionF32".to_string(),
                StdLibType::F64 => "struct __private__OptionF64".to_string(),
                StdLibType::Bool => "struct __private__OptionBool".to_string(),
                StdLibType::Pointer(_) => {
                    todo!("Option<*const T> and Option<*mut T> are not yet supported")
                }
                StdLibType::RefSlice(_) => {
                    todo!("Option<&[T]> is not yet supported")
                }
                StdLibType::Str => "struct RustStr".to_string(),
                StdLibType::Vec(_) => "void*".to_string(),
                StdLibType::Option(_) => {
                    todo!("Option<Option<T>> is not yet supported")
                }
                StdLibType::Result(_) => {
                    todo!("Option<Result<T, E>> is not yet supported")
                }
                StdLibType::BoxedFnOnce(_) => {
                    todo!("Option<Box<dyn FnOnce(A, B) -> C>> is not yet supported")
                }
                StdLibType::Tuple(_) => todo!(),
            },
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Struct(shared_struct))) => {
                format!("struct {}", shared_struct.ffi_option_name_string())
            }
            BridgedType::Foreign(CustomBridgedType::Shared(SharedType::Enum(shared_enum))) => {
                format!("struct {}", shared_enum.ffi_option_name_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that we can parse an `Option<&'static str>' bridged type
    /// This ensures that our logic that removes the spaces in order to normalize generic type
    /// strings (i.e. "SomeType < u32 >" -> "SomeType<u32>") does not remove spaces from types
    /// where the spaces matter such as "&'static str".
    #[test]
    fn parse_option_static_str() {
        let type_str = "Option < & 'static str >";

        let parsed = BridgedType::new_with_str(type_str, &TypeDeclarations::default()).unwrap();
        match parsed {
            BridgedType::StdLib(StdLibType::Option(opt)) => match *opt.ty {
                BridgedType::StdLib(StdLibType::Str) => {}
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}
