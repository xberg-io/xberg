use crate::bridged_type::{fn_arg_name, BridgeableType, BridgedType, StdLibType, TypePosition};
use crate::parse::{HostLang, TypeDeclaration};
use crate::parsed_extern_fn::FailableInitializerType;
use crate::{ParsedExternFn, TypeDeclarations, SWIFT_BRIDGE_PREFIX};
use quote::ToTokens;
use std::ops::Deref;
use syn::{Path, ReturnType, Type};

pub(super) fn gen_func_swift_calls_rust(
    function: &ParsedExternFn,
    types: &TypeDeclarations,
    swift_bridge_path: &Path,
) -> String {
    let fn_name = function.sig.ident.to_string();
    let params = function.to_swift_param_names_and_types(false, types, swift_bridge_path);
    let call_args = function.to_swift_call_args(true, false, types, swift_bridge_path);
    let call_fn = if function.sig.asyncness.is_some() {
        let maybe_args = if function.sig.inputs.is_empty() {
            "".to_string()
        } else {
            format!(", {}", call_args)
        };

        format!("{}(wrapperPtr, onComplete{})", fn_name, maybe_args)
    } else {
        format!("{}({})", fn_name, call_args)
    };

    let maybe_type_name_segment = if let Some(ty) = function.associated_type.as_ref() {
        match ty {
            TypeDeclaration::Shared(_) => {
                //
                todo!()
            }
            TypeDeclaration::Opaque(ty) => {
                format!("${}", ty.to_string())
            }
        }
    } else {
        "".to_string()
    };

    let maybe_static_class_func = if function.associated_type.is_some()
        && (!function.is_method() && !function.is_swift_initializer)
    {
        if function.is_copy_method_on_opaque_type() {
            "static "
        } else {
            "class "
        }
    } else {
        ""
    };

    let public_func_fn_name = if function.is_swift_initializer {
        if function.is_copy_method_on_opaque_type() {
            "public init".to_string()
        } else {
            if let Some(crate::parsed_extern_fn::FailableInitializerType::Throwing) =
                function.swift_failable_initializer
            {
                "public convenience init".to_string()
            } else if let Some(crate::parsed_extern_fn::FailableInitializerType::Option) =
                function.swift_failable_initializer
            {
                "public convenience init?".to_string()
            } else {
                "public convenience init".to_string()
            }
        }
    } else {
        if let Some(swift_name) = &function.swift_name_override {
            format!("public func {}", swift_name.value())
        } else {
            format!("public func {}", fn_name.as_str())
        }
    };

    let maybe_throws =
        if let Some(FailableInitializerType::Throwing) = function.swift_failable_initializer {
            " throws"
        } else {
            ""
        };
    let indentation = if function.associated_type.is_some() {
        "    "
    } else {
        ""
    };

    let call_rust = format!(
        "{prefix}{type_name_segment}${call_fn}",
        prefix = SWIFT_BRIDGE_PREFIX,
        type_name_segment = maybe_type_name_segment,
        call_fn = call_fn
    );
    let mut call_rust = if function.sig.asyncness.is_some() {
        call_rust
    } else if function.is_swift_initializer {
        if let Some(FailableInitializerType::Throwing) = function.swift_failable_initializer {
            let built_in = function.return_ty_built_in(types).unwrap();
            built_in.convert_ffi_value_to_swift_value(
                &call_rust,
                TypePosition::ThrowingInit(function.host_lang),
                types,
                swift_bridge_path,
            )
        } else {
            call_rust
        }
    } else if let Some(built_in) = function.return_ty_built_in(types) {
        built_in.convert_ffi_value_to_swift_value(
            &call_rust,
            TypePosition::FnReturn(function.host_lang),
            types,
            swift_bridge_path,
        )
    } else {
        if function.host_lang.is_swift() {
            call_rust
        } else {
            match &function.sig.output {
                ReturnType::Default => {
                    // () is a built in type so this would have been handled in the previous block.
                    unreachable!()
                }
                ReturnType::Type(_, ty) => {
                    let ty_name = match ty.deref() {
                        Type::Reference(reference) => reference.elem.to_token_stream().to_string(),
                        Type::Path(path) => path.path.segments.to_token_stream().to_string(),
                        _ => todo!(),
                    };

                    match types.get(&ty_name).unwrap() {
                        TypeDeclaration::Shared(_) => call_rust,
                        TypeDeclaration::Opaque(opaque) => {
                            if opaque.host_lang.is_rust() {
                                let (is_owned, ty) = match ty.deref() {
                                    Type::Reference(reference) => ("false", &reference.elem),
                                    _ => ("true", ty),
                                };

                                let ty = ty.to_token_stream().to_string();
                                format!("{}(ptr: {}, isOwned: {})", ty, call_rust, is_owned)
                            } else {
                                let ty = ty.to_token_stream().to_string();
                                format!(
                                    "Unmanaged<{}>.fromOpaque({}).takeRetainedValue()",
                                    ty, call_rust
                                )
                            }
                        }
                    }
                }
            }
        }
    };
    let returns_null = BridgedType::new_with_return_type(&function.func.sig.output, types)
        .map(|b| b.is_null())
        .unwrap_or(false);

    let maybe_return = if returns_null || function.is_swift_initializer {
        ""
    } else {
        "return "
    };

    for arg in function.func.sig.inputs.iter() {
        let bridged_arg = BridgedType::new_with_fn_arg(arg, types);
        if bridged_arg.is_none() {
            continue;
        }
        let bridged_arg = bridged_arg.unwrap();

        let arg_name = fn_arg_name(arg).unwrap().to_string();

        // TODO: Refactor to make less duplicative
        match bridged_arg {
            BridgedType::StdLib(StdLibType::Str) => {
                call_rust = format!(
                    r#"{maybe_return}{arg}.toRustStr({{ {arg}AsRustStr in
{indentation}        {call_rust}
{indentation}    }})"#,
                    maybe_return = maybe_return,
                    indentation = indentation,
                    arg = arg_name,
                    call_rust = call_rust
                );
            }
            BridgedType::StdLib(StdLibType::Option(briged_opt)) if briged_opt.ty.is_str() => {
                call_rust = format!(
                    r#"{maybe_return}optionalRustStrToRustStr({arg}, {{ {arg}AsRustStr in
{indentation}        {call_rust}
{indentation}    }})"#,
                    maybe_return = maybe_return,
                    indentation = indentation,
                    arg = arg_name,
                    call_rust = call_rust
                );
            }
            _ => {}
        }
    }

    if function.is_swift_initializer {
        if function.is_copy_method_on_opaque_type() {
            call_rust = format!("self.bytes = {}", call_rust)
        } else {
            if let Some(FailableInitializerType::Option) = function.swift_failable_initializer {
                call_rust = format!(
                    "guard let val = {} else {{ return nil }}; self.init(ptr: val)",
                    call_rust
                )
            } else if function.swift_failable_initializer.is_none() {
                call_rust = format!("self.init(ptr: {})", call_rust)
            }
        }
    }

    let maybe_return = if function.is_swift_initializer {
        "".to_string()
    } else {
        function.to_swift_return_type(types, swift_bridge_path)
    };

    let maybe_generics = function.maybe_swift_generics(types);

    let func_definition = if function.sig.asyncness.is_some() {
        let func_ret_ty = function.return_ty_built_in(types).unwrap();
        let rust_fn_ret_ty = func_ret_ty.to_swift_type(
            TypePosition::FnReturn(HostLang::Rust),
            types,
            swift_bridge_path,
        );
        let maybe_on_complete_sig_ret_val = if func_ret_ty.is_null() {
            "".to_string()
        } else {
            format!(
                ", rustFnRetVal: {}",
                func_ret_ty.to_swift_type(
                    TypePosition::ResultFfiReturnType,
                    types,
                    swift_bridge_path
                )
            )
        };
        let callback_wrapper_ty = format!("CbWrapper{}${}", maybe_type_name_segment, fn_name);
        let (run_wrapper_cb, error, maybe_try, with_checked_continuation_function_name) =
            if let Some(result) = func_ret_ty.as_result() {
                let run_wrapper_cb = result.generate_swift_calls_async_rust_callback(
                    "rustFnRetVal",
                    TypePosition::FnReturn(HostLang::Rust),
                    types,
                    swift_bridge_path,
                );
                (
                    run_wrapper_cb,
                    "Error".to_string(),
                    " try ".to_string(),
                    "withCheckedThrowingContinuation".to_string(),
                )
            } else {
                let on_complete_ret_val = if func_ret_ty.is_null() {
                    "()".to_string()
                } else {
                    func_ret_ty.convert_ffi_value_to_swift_value(
                        "rustFnRetVal",
                        TypePosition::ResultFfiReturnType,
                        types,
                        swift_bridge_path,
                    )
                };
                (
                    format!(r#"wrapper.cb(.success({on_complete_ret_val}))"#),
                    "Never".to_string(),
                    " ".to_string(),
                    "withCheckedContinuation".to_string(),
                )
            };
        let callback_wrapper = format!(
            r#"{indentation}class {cb_wrapper_ty} {{
{indentation}    var cb: (Result<{rust_fn_ret_ty}, {error}>) -> ()
{indentation}
{indentation}    public init(cb: @escaping (Result<{rust_fn_ret_ty}, {error}>) -> ()) {{
{indentation}        self.cb = cb
{indentation}    }}
{indentation}}}"#,
            indentation = indentation,
            cb_wrapper_ty = callback_wrapper_ty
        );

        let fn_body = format!(
            r#"func onComplete(cbWrapperPtr: UnsafeMutableRawPointer?{maybe_on_complete_sig_ret_val}) {{
    let wrapper = Unmanaged<{cb_wrapper_ty}>.fromOpaque(cbWrapperPtr!).takeRetainedValue()
    {run_wrapper_cb}
}}

return{maybe_try}await {with_checked_continuation_function_name}({{ (continuation: CheckedContinuation<{rust_fn_ret_ty}, {error}>) in
    let callback = {{ rustFnRetVal in
        continuation.resume(with: rustFnRetVal)
    }}

    let wrapper = {cb_wrapper_ty}(cb: callback)
    let wrapperPtr = Unmanaged.passRetained(wrapper).toOpaque()

    {call_rust}
}})"#,
            rust_fn_ret_ty = rust_fn_ret_ty,
            error = error,
            maybe_on_complete_sig_ret_val = maybe_on_complete_sig_ret_val,
            cb_wrapper_ty = callback_wrapper_ty,
            call_rust = call_rust,
        );

        let mut fn_body_indented = "".to_string();
        for line in fn_body.lines() {
            if line.len() > 0 {
                fn_body_indented += &format!("{}    {}\n", indentation, line);
            } else {
                fn_body_indented += "\n"
            }
        }
        let fn_body_indented = fn_body_indented.trim_end();

        format!(
            r#"{indentation}{maybe_static_class_func}{swift_class_func_name}{maybe_generics}({params}) async{maybe_ret} {{
{fn_body_indented}
{indentation}}}
{callback_wrapper}"#,
            indentation = indentation,
            maybe_static_class_func = maybe_static_class_func,
            swift_class_func_name = public_func_fn_name,
            maybe_generics = maybe_generics,
            params = params,
            maybe_ret = maybe_return,
            fn_body_indented = fn_body_indented,
            callback_wrapper = callback_wrapper
        )
    } else {
        format!(
            r#"{indentation}{maybe_static_class_func}{swift_class_func_name}{maybe_generics}({params}){maybe_throws}{maybe_ret} {{
{indentation}    {call_rust}
{indentation}}}"#,
            indentation = indentation,
            maybe_static_class_func = maybe_static_class_func,
            swift_class_func_name = public_func_fn_name,
            maybe_generics = maybe_generics,
            params = params,
            maybe_ret = maybe_return,
            call_rust = call_rust,
            maybe_throws = maybe_throws,
        )
    };

    func_definition
}
