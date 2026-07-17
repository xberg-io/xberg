use crate::bridged_type::{BridgeableType, BridgedType, CFfiStruct, TypePosition};
use crate::parse::HostLang;
use crate::{TypeDeclarations, SWIFT_BRIDGE_PREFIX};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::Path;

/// Rust: Result<T, E>
/// Swift: RustResult<T, E>
///
/// We don't use Swift's `Result` type since when we tried we saw a strange error
///  `'Sendable' class 'ResultTestOpaqueRustType' cannot inherit from another class other than 'NSObject'`
///  which meant that we could not use the `public class ResultTestOpaqueRustType: ResultTestOpaqueRustTypeRefMut {`
///  pattern that we use to prevent calling mutable methods on immutable references.
///  We only saw this error after `extension: ResultTestOpaqueRustType: Error {}` .. which was
///  necessary because Swift's Result type requires that the error implements the `Error` protocol.
#[derive(Debug)]
pub(crate) struct BuiltInResult {
    pub ok_ty: Box<dyn BridgeableType>,
    pub err_ty: Box<dyn BridgeableType>,
}

impl BuiltInResult {
    pub(super) fn to_ffi_compatible_rust_type(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        if self.is_custom_result_type() {
            let ty = format_ident!("{}", self.custom_c_struct_name(types));
            return quote! {
                #ty
            };
        }

        if self.ok_ty.can_be_encoded_with_zero_bytes() {
            return self
                .err_ty
                .to_ffi_compatible_rust_type(swift_bridge_path, types);
        };

        // TODO: Choose the kind of Result representation based on whether or not the ok and error
        //  types are primitives.
        //  See `swift-bridge/src/std_bridge/result`
        let result_kind = quote! {
                ResultPtrAndPtr
        };

        quote! {
            #swift_bridge_path::result::#result_kind
        }
    }

    pub(super) fn convert_rust_expression_to_ffi_type(
        &self,
        expression: &TokenStream,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
        span: Span,
    ) -> TokenStream {
        let convert_ok = self.ok_ty.convert_rust_expression_to_ffi_type(
            &quote! { ok },
            swift_bridge_path,
            types,
            span,
        );

        let convert_err = self.err_ty.convert_rust_expression_to_ffi_type(
            &quote! { err },
            swift_bridge_path,
            types,
            span,
        );

        if self.is_custom_result_type() {
            if self.err_ty.can_be_encoded_with_zero_bytes() {
                todo!();
            }
            if self.ok_ty.can_be_encoded_with_zero_bytes() {
                let ffi_enum_name = self.to_ffi_compatible_rust_type(swift_bridge_path, types);
                let err_ffi = self.err_ty.convert_rust_expression_to_ffi_type(
                    &quote!(err),
                    swift_bridge_path,
                    types,
                    span,
                );
                return quote! {
                    match #expression {
                        Ok(ok) => #ffi_enum_name::Ok,
                        Err(err) => #ffi_enum_name::Err(#err_ffi),
                    }
                };
            }
            let ffi_enum_name = self.to_ffi_compatible_rust_type(swift_bridge_path, types);
            let ok_ffi = self.ok_ty.convert_rust_expression_to_ffi_type(
                &quote!(ok),
                swift_bridge_path,
                types,
                span,
            );
            let err_ffi = self.err_ty.convert_rust_expression_to_ffi_type(
                &quote!(err),
                swift_bridge_path,
                types,
                span,
            );
            return quote! {
                match #expression {
                    Ok(ok) => #ffi_enum_name::Ok(#ok_ffi),
                    Err(err) => #ffi_enum_name::Err(#err_ffi),
                }
            };
        }

        if self.ok_ty.can_be_encoded_with_zero_bytes() {
            quote! {
                match #expression {
                    Ok(ok) => std::ptr::null_mut(),
                    Err(err) => #convert_err
                }
            }
        } else {
            quote! {
                match #expression {
                    Ok(ok) => {
                        #swift_bridge_path::result::ResultPtrAndPtr {
                            is_ok: true,
                            ok_or_err: #convert_ok as *mut std::ffi::c_void
                        }
                    }
                    Err(err) => {
                        #swift_bridge_path::result::ResultPtrAndPtr {
                            is_ok: false,
                            ok_or_err: #convert_err as *mut std::ffi::c_void
                        }
                    }
                }
            }
        }
    }

    pub fn convert_ffi_value_to_rust_value(
        &self,
        expression: &TokenStream,
        span: Span,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> TokenStream {
        if self.is_custom_result_type() {
            // Custom result type: Rust enum like ResultOkAndErr::Ok(ok_val) / ResultOkAndErr::Err(err_val)
            let ffi_enum_name = self.to_ffi_compatible_rust_type(swift_bridge_path, types);
            let (ok_pattern, convert_ok) = if self.ok_ty.can_be_encoded_with_zero_bytes() {
                (quote! {}, quote! { () })
            } else {
                let convert = self.ok_ty.convert_ffi_expression_to_rust_type(
                    &quote! { ok },
                    span,
                    swift_bridge_path,
                    types,
                );
                (quote! { (ok) }, convert)
            };
            let convert_err = self.err_ty.convert_ffi_expression_to_rust_type(
                &quote! { err },
                span,
                swift_bridge_path,
                types,
            );

            quote_spanned! {span=>
                match #expression {
                    #ffi_enum_name::Ok #ok_pattern => std::result::Result::Ok(#convert_ok),
                    #ffi_enum_name::Err(err) => std::result::Result::Err(#convert_err),
                }
            }
        } else if self.ok_ty.can_be_encoded_with_zero_bytes() {
            // Result<(), E> - null pointer for Ok, pointer for Err
            let convert_err = self.err_ty.convert_ffi_result_err_value_to_rust_value(
                expression,
                swift_bridge_path,
                types,
            );

            quote_spanned! {span=>
                if #expression.is_null() {
                    std::result::Result::Ok(())
                } else {
                    std::result::Result::Err(#convert_err)
                }
            }
        } else {
            // Standard ResultPtrAndPtr
            let convert_ok = self.ok_ty.convert_ffi_result_ok_value_to_rust_value(
                expression,
                swift_bridge_path,
                types,
            );

            let convert_err = self.err_ty.convert_ffi_result_err_value_to_rust_value(
                expression,
                swift_bridge_path,
                types,
            );

            quote_spanned! {span=>
                if #expression.is_ok {
                    std::result::Result::Ok(#convert_ok)
                } else {
                    std::result::Result::Err(#convert_err)
                }
            }
        }
    }

    pub fn to_rust_type_path(&self, types: &TypeDeclarations) -> TokenStream {
        let ok = self.ok_ty.to_rust_type_path(types);
        let err = self.err_ty.to_rust_type_path(types);

        quote! { Result<#ok, #err> }
    }

    pub fn to_swift_type(
        &self,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        match type_pos {
            TypePosition::FnReturn(_) => {
                self.ok_ty.to_swift_type(type_pos, types, swift_bridge_path)
            }
            TypePosition::FnArg(_, _) | TypePosition::SharedStructField => {
                format!(
                    "RustResult<{}, {}>",
                    self.ok_ty.to_swift_type(type_pos, types, swift_bridge_path),
                    self.err_ty
                        .to_swift_type(type_pos, types, swift_bridge_path),
                )
            }
            TypePosition::ResultFfiReturnType => {
                if self.err_ty.can_be_encoded_with_zero_bytes() {
                    todo!()
                }
                if self.is_custom_result_type() {
                    return format!(
                        "{}${}",
                        SWIFT_BRIDGE_PREFIX,
                        self.custom_c_struct_name(types)
                    );
                }
                if self.ok_ty.can_be_encoded_with_zero_bytes() {
                    return "UnsafeMutableRawPointer?".to_string();
                }
                "__private__ResultPtrAndPtr".to_string()
            }
            TypePosition::ThrowingInit(_) => todo!(),
        }
    }

    pub fn convert_ffi_value_to_swift_value(
        &self,
        expression: &str,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        if self.is_custom_result_type() {
            if self.err_ty.can_be_encoded_with_zero_bytes() {
                todo!();
            }
            let c_ok_name = self.c_ok_tag_name(types);
            let c_err_name = self.c_err_tag_name(types);
            let ok_swift_type = if self.ok_ty.can_be_encoded_with_zero_bytes() {
                "".to_string()
            } else {
                " ".to_string()
                    + &self.ok_ty.convert_ffi_expression_to_swift_type(
                        "val.payload.ok",
                        type_pos,
                        types,
                        swift_bridge_path,
                    )
            };
            let err_swift_type = self.err_ty.convert_ffi_expression_to_swift_type(
                "val.payload.err",
                type_pos,
                types,
                swift_bridge_path,
            );

            return match type_pos {
                TypePosition::FnArg(_, _) => todo!(),
                TypePosition::FnReturn(_) => format!(
                        "try {{ let val = {expression}; switch val.tag {{ case {c_ok_name}: return{ok_swift_type} case {c_err_name}: throw {err_swift_type} default: fatalError() }} }}()",
                    expression = expression,
                    c_ok_name = c_ok_name,
                    c_err_name = c_err_name,
                    ok_swift_type = ok_swift_type,
                    err_swift_type = err_swift_type
                ),
                TypePosition::SharedStructField => todo!(),
                TypePosition::ResultFfiReturnType => todo!(),
                TypePosition::ThrowingInit(lang) => {
                    match lang {
                        HostLang::Rust => format!(
                            "let val = {expression}; if val.tag == {c_ok_name} {{ self.init(ptr: val.payload.ok) }} else {{ throw {err_swift_type} }}",
                        expression = expression,
                        c_ok_name = c_ok_name,
                        err_swift_type = err_swift_type
                    ),
                        HostLang::Swift => todo!(),
                    }
                }
            };
        }

        if let Some(ok) = self.ok_ty.only_encoding() {
            let mut ok = ok.swift;

            // There is a Swift compiler bug in Xcode 13 where using an explicit `()` here somehow leads
            // the Swift compiler to a compile time error:
            // "Unable to infer complex closure return type; add explicit type to disambiguate"
            //
            // It's asking us to add a `{ () -> () in .. }` explicit type to the beginning of our closure.
            //
            // To solve this bug we can either add that explicit closure type, or remove the explicit
            // `return ()` in favor of a `return`.. Not sure why making the return type less explicit
            //  solves the compile time error.. But it does..
            //
            // As mentioned, this doesn't seem to happen in Xcode 14.
            // So, we can remove this if statement whenever we stop supporting Xcode 13.

            if self.ok_ty.is_null() {
                ok = "".to_string();
            } else {
                ok = " ".to_string() + &ok;
            }
            let err = self.err_ty.convert_ffi_expression_to_swift_type(
                "val!",
                type_pos,
                types,
                swift_bridge_path,
            );
            return format!("try {{ let val = {expression}; if val != nil {{ throw {err} }} else {{ return{ok} }} }}()", expression = expression, err = err, ok = ok);
        }

        let ok = self.ok_ty.convert_ffi_expression_to_swift_type(
            "val.ok_or_err!",
            type_pos,
            types,
            swift_bridge_path,
        );
        let err = self.err_ty.convert_ffi_expression_to_swift_type(
            "val.ok_or_err!",
            type_pos,
            types,
            swift_bridge_path,
        );

        format!(
            "try {{ let val = {expression}; if val.is_ok {{ return {ok} }} else {{ throw {err} }} }}()",
            expression = expression,
            err = err
        )
    }

    pub fn convert_swift_expression_to_ffi_compatible(
        &self,
        expression: &str,
        types: &TypeDeclarations,
        type_pos: TypePosition,
    ) -> String {
        let convert_ok = self
            .ok_ty
            .convert_swift_expression_to_ffi_type("ok", types, type_pos);
        let convert_err = self
            .err_ty
            .convert_swift_expression_to_ffi_type("err", types, type_pos);

        if self.ok_ty.can_be_encoded_with_zero_bytes() {
            format!(
                "{{ switch {val} {{ case .Ok(let ok): return __private__ResultPtrAndPtr(is_ok: true, ok_or_err: {convert_ok}) case .Err(let err): return __private__ResultPtrAndPtr(is_ok: false, ok_or_err: {convert_err}) }} }}()",
                val = expression
            )
        } else {
            format!(
                "{{ switch {val} {{ case .Ok(let ok): return __private__ResultPtrAndPtr(is_ok: true, ok_or_err: {convert_ok}) case .Err(let err): return __private__ResultPtrAndPtr(is_ok: false, ok_or_err: {convert_err}) }} }}()",
                val = expression
            )
        }
    }

    pub fn to_c(&self, types: &TypeDeclarations) -> String {
        if self.is_custom_result_type() {
            return format!(
                "struct {}${}",
                SWIFT_BRIDGE_PREFIX,
                self.custom_c_struct_name(types)
            );
        }
        // TODO: Choose the kind of Result representation based on whether or not the ok and error
        //  types are primitives.
        //  See `swift-bridge/src/std_bridge/result`
        if self.ok_ty.can_be_encoded_with_zero_bytes() {
            format!("{}", self.err_ty.to_c_type(types))
        } else {
            format!("struct __private__ResultPtrAndPtr")
        }
    }

    pub fn generate_custom_rust_ffi_types(
        &self,
        swift_bridge_path: &Path,
        types: &TypeDeclarations,
    ) -> Option<Vec<TokenStream>> {
        if !self.is_custom_result_type() {
            return None;
        }
        if self.err_ty.can_be_encoded_with_zero_bytes() {
            todo!()
        }
        let ty = self.to_ffi_compatible_rust_type(swift_bridge_path, types);
        let ok = if self.ok_ty.can_be_encoded_with_zero_bytes() {
            quote! {}
        } else {
            let ty = self
                .ok_ty
                .to_ffi_compatible_rust_type(swift_bridge_path, types);
            quote! {(#ty)}
        };

        let err = self
            .err_ty
            .to_ffi_compatible_rust_type(swift_bridge_path, types);
        let mut custom_rust_ffi_types = vec![];
        // TODO: remove `#[allow(unused)]` when rustc no longer issues dead code warnings for `#[repr(C)]`
        //  structs or enums: https://github.com/rust-lang/rust/issues/126706
        custom_rust_ffi_types.push(quote! {
            #[repr(C)]
            pub enum #ty {
                #[allow(unused)]
                Ok #ok,
                #[allow(unused)]
                Err(#err),
            }
        });
        let ok_custom_rust_ffi_types = self
            .ok_ty
            .generate_custom_rust_ffi_types(swift_bridge_path, types);
        let err_custom_rust_ffi_types = self
            .err_ty
            .generate_custom_rust_ffi_types(swift_bridge_path, types);
        if let Some(ok_custom_rust_ffi_types) = ok_custom_rust_ffi_types {
            for custom_rust_ffi_type in ok_custom_rust_ffi_types {
                custom_rust_ffi_types.push(custom_rust_ffi_type);
            }
        }
        if let Some(err_custom_rust_ffi_types) = err_custom_rust_ffi_types {
            for custom_rust_ffi_type in err_custom_rust_ffi_types {
                custom_rust_ffi_types.push(custom_rust_ffi_type);
            }
        }
        return Some(custom_rust_ffi_types);
    }

    pub fn generate_custom_c_ffi_types(&self, types: &TypeDeclarations) -> Option<CFfiStruct> {
        if !self.is_custom_result_type() {
            return None;
        }
        if self.err_ty.can_be_encoded_with_zero_bytes() {
            todo!();
        }
        let c_type = format!(
            "{}${}",
            SWIFT_BRIDGE_PREFIX,
            self.custom_c_struct_name(types)
        );
        let c_enum_name = c_type.clone();
        let c_tag_name = format!("{}$Tag", c_type.clone());
        let c_fields_name = format!("{}$Fields", c_type);

        let ok_c_field_name = if self.ok_ty.can_be_encoded_with_zero_bytes() {
            "".to_string()
        } else {
            format!("{} ok; ", self.ok_ty.to_c_type(types))
        };
        let err_c_field_name = self.err_ty.to_c_type(types);
        let ok_c_tag_name = self.c_ok_tag_name(types);
        let err_c_tag_name = self.c_err_tag_name(types);
        let c_ffi_type = format!(
            "typedef enum {c_tag_name} {{{ok_c_tag_name}, {err_c_tag_name}}} {c_tag_name};
union {c_fields_name} {{{ok_c_field_name}{err_c_field_name} err;}};
typedef struct {c_enum_name}{{{c_tag_name} tag; union {c_fields_name} payload;}} {c_enum_name};",
            c_enum_name = c_enum_name,
            c_tag_name = c_tag_name,
            c_fields_name = c_fields_name,
            ok_c_field_name = ok_c_field_name,
            err_c_field_name = err_c_field_name,
            ok_c_tag_name = ok_c_tag_name,
            err_c_tag_name = err_c_tag_name,
        );
        let mut custom_c_ffi_type = CFfiStruct {
            c_ffi_type,
            fields: Vec::with_capacity(2),
        };
        if let Some(ok_custom_c_ffi_type) = self.ok_ty.generate_custom_c_ffi_types(types) {
            custom_c_ffi_type.fields.push(ok_custom_c_ffi_type);
        }
        if let Some(err_custom_c_ffi_type) = self.err_ty.generate_custom_c_ffi_types(types) {
            custom_c_ffi_type.fields.push(err_custom_c_ffi_type);
        }
        return Some(custom_c_ffi_type);
    }

    pub fn is_custom_result_type(&self) -> bool {
        // ResultPtrAndPtr
        if self.ok_ty.is_passed_via_pointer() && self.err_ty.is_passed_via_pointer() {
            return false;
        }

        // ResultVoidAndPtr or ResultPtrAndVoid
        if (self.ok_ty.only_encoding().is_some() && self.err_ty.is_passed_via_pointer())
            || (self.ok_ty.is_passed_via_pointer() && self.err_ty.only_encoding().is_some())
        {
            return false;
        }

        return true;
    }

    pub fn generate_swift_calls_async_rust_callback(
        &self,
        expression: &str,
        type_pos: TypePosition,
        types: &TypeDeclarations,
        swift_bridge_path: &Path,
    ) -> String {
        if self.is_custom_result_type() {
            let ok = if self.ok_ty.can_be_encoded_with_zero_bytes() {
                "()".to_string()
            } else {
                self.ok_ty.convert_ffi_expression_to_swift_type(
                    &format!("{expression}.payload.ok"),
                    type_pos,
                    types,
                    swift_bridge_path,
                )
            };
            let err = self.err_ty.convert_ffi_expression_to_swift_type(
                &format!("{expression}.payload.err"),
                type_pos,
                types,
                swift_bridge_path,
            );
            return format!(
                r#"switch {expression}.tag {{ case {c_ok_tag_name}: wrapper.cb(.success({ok})) case {c_err_tag_name}: wrapper.cb(.failure({err})) default: fatalError() }}"#,
                expression = expression,
                ok = ok,
                err = err,
                c_ok_tag_name = self.c_ok_tag_name(types),
                c_err_tag_name = self.c_err_tag_name(types)
            );
        }
        let ok = self.ok_ty.to_swift_type(type_pos, types, swift_bridge_path);
        let err = self
            .err_ty
            .to_swift_type(type_pos, types, swift_bridge_path);

        let (ok_val, err_val, condition) = if self.ok_ty.can_be_encoded_with_zero_bytes() {
            (
                ok,
                format!("{err}(ptr: rustFnRetVal!)"),
                "rustFnRetVal == nil",
            )
        } else {
            (
                format!("{ok}(ptr: rustFnRetVal.ok_or_err!)"),
                format!("{err}(ptr: rustFnRetVal.ok_or_err!)"),
                "rustFnRetVal.is_ok",
            )
        };

        format!(
            r#"if {condition} {{
        wrapper.cb(.success({ok_val}))
    }} else {{
        wrapper.cb(.failure({err_val}))
    }}"#
        )
    }
}

impl BuiltInResult {
    /// Go from `Result < A , B >` to a `BuiltInResult`.
    pub fn from_str_tokens(string: &str, types: &TypeDeclarations) -> Option<Self> {
        // A , B >
        let trimmed = string.trim_start_matches("Result < ");
        // A , B
        let trimmed = trimmed.trim_end_matches(" >");

        // [A, B]
        let ok_and_err = trimmed.rsplit_once(",")?;
        let ok = ok_and_err.0.trim();
        let err = ok_and_err.1.trim();

        let ok = BridgedType::new_with_str(ok, types)?;
        let err = BridgedType::new_with_str(err, types)?;

        Some(BuiltInResult {
            ok_ty: Box::new(ok),
            err_ty: Box::new(err),
        })
    }
}

impl BuiltInResult {
    fn custom_c_struct_name(&self, types: &TypeDeclarations) -> String {
        let ok = &self.ok_ty;
        let err = &self.err_ty;

        let ok = ok.to_alpha_numeric_underscore_name(types);
        let err = err.to_alpha_numeric_underscore_name(types);

        format!("Result{ok}And{err}")
    }
    pub fn c_ok_tag_name(&self, types: &TypeDeclarations) -> String {
        format!(
            "{}${}$ResultOk",
            SWIFT_BRIDGE_PREFIX,
            self.custom_c_struct_name(types)
        )
    }

    pub fn c_err_tag_name(&self, types: &TypeDeclarations) -> String {
        format!(
            "{}${}$ResultErr",
            SWIFT_BRIDGE_PREFIX,
            self.custom_c_struct_name(types)
        )
    }

    pub fn c_fields_name(&self, types: &TypeDeclarations) -> String {
        format!(
            "{}${}$Fields",
            SWIFT_BRIDGE_PREFIX,
            self.custom_c_struct_name(types)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    /// Verify that we can parse a `Result<(), ()>`
    #[test]
    fn result_from_null_type() {
        let tokens = quote! { Result<(), ()> }.to_token_stream().to_string();

        let result = BuiltInResult::from_str_tokens(&tokens, &TypeDeclarations::default()).unwrap();

        assert!(result.ok_ty.is_null());
        assert!(result.err_ty.is_null());
    }
}
