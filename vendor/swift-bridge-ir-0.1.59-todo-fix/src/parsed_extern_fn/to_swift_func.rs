use crate::bridged_type::{pat_type_pat_is_self, BridgeableType, BridgedType, TypePosition};
use crate::parse::TypeDeclarations;
use crate::parsed_extern_fn::ParsedExternFn;
use quote::{format_ident, ToTokens};
use std::ops::Deref;
use syn::{FnArg, Path, ReturnType, Type};

impl ParsedExternFn {
    pub fn to_swift_param_names_and_types(
        &self,
        include_receiver_if_present: bool,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        let mut params: Vec<String> = vec![];

        for (arg_idx, arg) in self.func.sig.inputs.iter().enumerate() {
            let param = match arg {
                FnArg::Receiver(_receiver) => {
                    if include_receiver_if_present {
                        params.push(format!("_ this: UnsafeMutableRawPointer"));
                    }

                    continue;
                }
                FnArg::Typed(pat_ty) => {
                    if pat_type_pat_is_self(pat_ty) {
                        if include_receiver_if_present {
                            params.push(format!("_ this: UnsafeMutableRawPointer"));
                        }

                        continue;
                    }

                    let arg_name = pat_ty.pat.to_token_stream().to_string();

                    let ty = if let Some(built_in) = BridgedType::new_with_type(&pat_ty.ty, types) {
                        if self.host_lang.is_swift() {
                            if built_in.can_be_encoded_with_zero_bytes() {
                                continue;
                            }
                        }

                        built_in.to_swift_type(
                            TypePosition::FnArg(self.host_lang, arg_idx),
                            types,
                            swift_bridge_path,
                        )
                    } else {
                        todo!("Push to ParsedErrors")
                    };

                    if let Some(argument_label) =
                        self.argument_labels.get(&format_ident!("{}", arg_name))
                    {
                        format!("{} {}: {}", argument_label.value().as_str(), arg_name, ty)
                    } else {
                        format!("_ {}: {}", arg_name, ty)
                    }
                }
            };
            params.push(param)
        }

        params.join(", ")
    }

    // fn foo (&self, arg1: u8, arg2: u32)
    //  might become (depending on whether we're including the receiver and/or the var name)
    //  - arg1, arg2
    //  - ptr, arg1, arg2
    //  - arg1: arg1, arg2: arg2
    //  - self: ptr, arg1: arg1, arg2: arg2
    pub fn to_swift_call_args(
        &self,
        include_receiver_if_present: bool,
        include_var_name: bool,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        let mut args = vec![];
        let inputs = &self.func.sig.inputs;
        for (arg_idx, arg) in inputs.iter().enumerate() {
            match arg {
                FnArg::Receiver(receiver) => {
                    if include_receiver_if_present {
                        self.push_receiver_as_arg(&mut args, receiver.reference.is_some());
                    }
                }
                FnArg::Typed(pat_ty) => {
                    let is_reference = match pat_ty.ty.deref() {
                        Type::Reference(_) => true,
                        _ => false,
                    };

                    if pat_type_pat_is_self(pat_ty) {
                        if include_receiver_if_present {
                            self.push_receiver_as_arg(&mut args, is_reference);
                        }

                        continue;
                    }

                    let pat = &pat_ty.pat;
                    let arg = pat.to_token_stream().to_string();
                    let arg_name = arg.clone();

                    let arg =
                        if let Some(bridged_ty) = BridgedType::new_with_type(&pat_ty.ty, types) {
                            if self.host_lang.is_rust() {
                                if bridged_ty.can_be_encoded_with_zero_bytes() {
                                    continue;
                                }

                                bridged_ty.convert_swift_expression_to_ffi_type(
                                    &arg,
                                    types,
                                    TypePosition::FnArg(self.host_lang, arg_idx),
                                )
                            } else {
                                if let Some(only) = bridged_ty.only_encoding() {
                                    only.swift
                                } else {
                                    bridged_ty.convert_ffi_value_to_swift_value(
                                        &arg,
                                        TypePosition::FnArg(self.host_lang, arg_idx),
                                        types,
                                        swift_bridge_path,
                                    )
                                }
                            }
                        } else {
                            todo!("Push to ParsedErrors")
                        };
                    let arg = if include_var_name {
                        if let Some(label) =
                            self.argument_labels.get(&format_ident!("{}", arg_name))
                        {
                            let label_str = label.value();
                            if label_str == "_" {
                                // No label - just the value
                                arg
                            } else {
                                // Custom label
                                format!("{}: {}", label_str, arg)
                            }
                        } else {
                            // Default: use parameter name as label
                            format!("{}: {}", arg_name, arg)
                        }
                    } else {
                        arg
                    };

                    args.push(arg);
                }
            };
        }
        args.join(", ")
    }

    pub fn to_swift_return_type(
        &self,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        match &self.func.sig.output {
            ReturnType::Default => "".to_string(),
            ReturnType::Type(_, ty) => {
                if let Some(built_in) = BridgedType::new_with_type(&ty, types) {
                    if self.host_lang.is_swift() {
                        if built_in.can_be_encoded_with_zero_bytes() {
                            return "".to_string();
                        }
                    }

                    let maybe_throws = if built_in.is_result() { "throws " } else { "" };

                    format!(
                        " {}-> {}",
                        maybe_throws,
                        built_in.to_swift_type(
                            TypePosition::FnReturn(self.host_lang,),
                            types,
                            swift_bridge_path
                        )
                    )
                } else {
                    todo!("Push ParsedErrors")
                }
            }
        }
    }

    fn push_receiver_as_arg(&self, args: &mut Vec<String>, is_reference: bool) {
        let arg = if self.is_copy_method_on_opaque_type() {
            "self.bytes"
        } else {
            if is_reference {
                "ptr"
            } else {
                "{isOwned = false; return ptr;}()"
            }
        };
        args.push(arg.to_string());
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::SwiftBridgeModuleAndErrors;
    use crate::SwiftBridgeModule;
    use proc_macro2::TokenStream;
    use quote::quote;

    /// Verify that if we are returning a declared type (non built-in) we return it as a Swift class
    /// instance.
    #[test]
    fn return_declared_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make1 () -> Foo;
                    fn make2 () -> &Foo;
                    fn make3 () -> &mut Foo;
                }
            }
        };
        let module = parse_ok(tokens);
        let functions = &module.functions;
        assert_eq!(functions.len(), 3);

        let expected = vec!["Foo", "FooRef", "FooRefMut"];

        for (idx, expected) in expected.into_iter().enumerate() {
            assert_eq!(
                functions[idx].to_swift_return_type(&module.types, &module.swift_bridge_path),
                format!(" -> {}", expected)
            );
        }
    }

    /// Verify that we ignore self when generating Swift function params.
    #[test]
    fn excludes_self_from_params() {
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
            assert_eq!(
                method.to_swift_param_names_and_types(
                    false,
                    &module.types,
                    &module.swift_bridge_path
                ),
                ""
            );
        }
    }

    /// Verify that we always use the corresponding class name for an argument of a custom type.
    #[test]
    fn strips_references_from_params_with_declared_type() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make1 (other: Foo);
                    fn make2 (other: &Foo);
                    fn make3 (other: &mut Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        let functions = &module.functions;
        assert_eq!(functions.len(), 3);

        let expected = vec!["Foo", "FooRef", "FooRefMut"];

        for (idx, expected) in expected.into_iter().enumerate() {
            assert_eq!(
                functions[idx].to_swift_param_names_and_types(
                    false,
                    &module.types,
                    &module.swift_bridge_path
                ),
                format!("_ other: {}", expected)
            );
        }
    }

    /// Verify that we use the `.ptr` field on a class instance when calling a Rust function from
    /// Swift.
    #[test]
    fn call_args_uses_pointer_from_class_instances() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make2 (other: &Foo);
                    fn make3 (other: &mut Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        let functions = &module.functions;
        assert_eq!(functions.len(), 2);

        for idx in 0..2 {
            assert_eq!(
                functions[idx].to_swift_call_args(
                    true,
                    false,
                    &module.types,
                    &module.swift_bridge_path
                ),
                "other.ptr"
            );
        }
    }

    /// Verify that if we pass an owned value to Rust the class instance is marked as no longer
    /// owned, since Rust now owns it.
    #[test]
    fn call_args_marks_instance_no_longer_owned_if_passed_owned_to_rust() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    type Foo;
                    fn make1(self);
                    fn make2(self: Foo);
                    fn make3(other: Foo);
                }
            }
        };
        let module = parse_ok(tokens);
        let functions = &module.functions;

        assert_eq!(
            functions[0].to_swift_call_args(true, false, &module.types, &module.swift_bridge_path),
            "{isOwned = false; return ptr;}()"
        );

        assert_eq!(
            functions[1].to_swift_call_args(true, false, &module.types, &module.swift_bridge_path),
            "{isOwned = false; return ptr;}()"
        );

        assert_eq!(
            functions[2].to_swift_call_args(true, false, &module.types, &module.swift_bridge_path),
            "{other.isOwned = false; return other.ptr;}()"
        );
    }

    /// Verify that we are calling .convert_swift_expression_to_ffi_compatible() on Swift -> FFI
    /// arguments.
    #[test]
    fn converts_unsafe_buffer_pointer_to_ffi_slice() {
        let tokens = quote! {
            #[swift_bridge::bridge]
            mod ffi {
                extern "Rust" {
                    fn some_function (someArg: &[u8]);
                }
            }
        };
        let module = parse_ok(tokens);
        let functions = &module.functions;

        assert_eq!(
            functions[0].to_swift_call_args(false, false, &module.types, &module.swift_bridge_path),
            "someArg.toFfiSlice()"
        );
    }

    fn parse_ok(tokens: TokenStream) -> SwiftBridgeModule {
        let module_and_errors: SwiftBridgeModuleAndErrors = syn::parse2(tokens).unwrap();
        module_and_errors.module
    }
}
