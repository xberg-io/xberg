use crate::bridged_type::{BridgedType, SharedEnum, StructFields, TypePosition};
use crate::SwiftBridgeModule;

impl SwiftBridgeModule {
    /// Generate the tokens for a shared enum.
    pub(super) fn generate_shared_enum_string(&self, shared_enum: &SharedEnum) -> Option<String> {
        if shared_enum.already_declared {
            return None;
        }

        let enum_name = shared_enum.swift_name_string();
        let enum_ffi_name = shared_enum.ffi_name_string();
        let option_ffi_name = shared_enum.ffi_option_name_string();

        let mut variants = "".to_string();
        let mut convert_swift_to_ffi_repr = "\n".to_string();
        let mut convert_ffi_repr_to_swift = "\n".to_string();
        let all_variants_empty = shared_enum.all_variants_empty();
        for variant in shared_enum.variants.iter() {
            let v = match &variant.fields {
                StructFields::Named(named_fields) => {
                    let mut params = vec![];
                    for named_field in named_fields {
                        let ty = BridgedType::new_with_type(&named_field.ty, &self.types)
                            .unwrap()
                            .to_swift_type(
                                TypePosition::SharedStructField,
                                &self.types,
                                &self.swift_bridge_path,
                            );
                        params.push(format!("{}: {}", named_field.name, ty))
                    }
                    let params = params.join(", ");
                    format!(
                        r#"
    case {name}({params})"#,
                        name = variant.name,
                        params = params,
                    )
                }
                StructFields::Unnamed(unnamed_fields) => {
                    let mut params = vec![];
                    for unnamed_field in unnamed_fields {
                        let ty = BridgedType::new_with_type(&unnamed_field.ty, &self.types)
                            .unwrap()
                            .to_swift_type(
                                TypePosition::SharedStructField,
                                &self.types,
                                &self.swift_bridge_path,
                            );
                        params.push(ty);
                    }
                    let params = params.join(", ");
                    format!(
                        r#"
    case {name}({params})"#,
                        name = variant.name,
                        params = params,
                    )
                }
                StructFields::Unit => {
                    format!(
                        r#"
    case {name}"#,
                        name = variant.name
                    )
                }
            };
            variants += &v;
        }
        if variants.len() > 0 {
            variants += "\n";
        }

        for variant in shared_enum.variants.iter() {
            let convert_swift_variant_to_ffi_repr = variant.convert_swift_to_ffi_repr(
                &self.types,
                format!("{}", enum_name),
                format!("{}", enum_ffi_name),
                all_variants_empty,
            );
            convert_swift_to_ffi_repr += &convert_swift_variant_to_ffi_repr;
        }
        if convert_swift_to_ffi_repr.len() > 0 {
            convert_swift_to_ffi_repr += "        ";
        }

        for variant in shared_enum.variants.iter() {
            let convert_ffi_variant_to_swift = variant.convert_ffi_expression_to_swift(
                &self.types,
                format!("{}", enum_name),
                &self.swift_bridge_path,
            );
            convert_ffi_repr_to_swift += &convert_ffi_variant_to_swift;
        }
        if convert_ffi_repr_to_swift.len() > 0 {
            convert_ffi_repr_to_swift += &format!(
                r#"            default:
                fatalError("Unreachable")
        "#
            );
        }

        let vectorizable_impl = if shared_enum.has_one_or_more_variants_with_data() {
            "".to_string()
        } else {
            format!(
                r#"
extension {enum_name}: Vectorizable {{
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {{
        __swift_bridge__$Vec_{enum_name}$new()
    }}

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {{
        __swift_bridge__$Vec_{enum_name}$drop(vecPtr)
    }}

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Self) {{
        __swift_bridge__$Vec_{enum_name}$push(vecPtr, value.intoFfiRepr())
    }}

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {{
        let maybeEnum = __swift_bridge__$Vec_{enum_name}$pop(vecPtr)
        return maybeEnum.intoSwiftRepr()
    }}

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {{
        let maybeEnum = __swift_bridge__$Vec_{enum_name}$get(vecPtr, index)
        return maybeEnum.intoSwiftRepr()
    }}

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<Self> {{
        let maybeEnum = __swift_bridge__$Vec_{enum_name}$get_mut(vecPtr, index)
        return maybeEnum.intoSwiftRepr()
    }}

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<Self> {{
        UnsafePointer<Self>(OpaquePointer(__swift_bridge__$Vec_{enum_name}$as_ptr(vecPtr)))
    }}

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {{
        __swift_bridge__$Vec_{enum_name}$len(vecPtr)
    }}
}}"#
            )
        };

        let derive_debug_impl = if shared_enum.derive.debug {
            format!(
                r#"
extension {enum_name}: CustomDebugStringConvertible {{
    public var debugDescription: String {{
        RustString(ptr: __swift_bridge__${enum_name}$Debug(self.intoFfiRepr())).toString()
    }}
}}"#
            )
        } else {
            "".to_string()
        };

        let swift_enum = format!(
            r#"public enum {enum_name} {{{variants}}}
extension {enum_name} {{
    func intoFfiRepr() -> {ffi_repr_name} {{
        switch self {{{convert_swift_to_ffi_repr}}}
    }}
}}
extension {enum_ffi_name} {{
    func intoSwiftRepr() -> {enum_name} {{
        switch self.tag {{{convert_ffi_repr_to_swift}}}
    }}
}}
extension {option_ffi_name} {{
    @inline(__always)
    func intoSwiftRepr() -> Optional<{enum_name}> {{
        if self.is_some {{
            return self.val.intoSwiftRepr()
        }} else {{
            return nil
        }}
    }}
    @inline(__always)
    static func fromSwiftRepr(_ val: Optional<{enum_name}>) -> {option_ffi_name} {{
        if let v = val {{
            return {option_ffi_name}(is_some: true, val: v.intoFfiRepr())
        }} else {{
            return {option_ffi_name}(is_some: false, val: {ffi_repr_name}())
        }}
    }}
}}{vectorizable_impl}{derive_debug_impl}"#,
            enum_name = enum_name,
            enum_ffi_name = enum_ffi_name,
            option_ffi_name = option_ffi_name,
            ffi_repr_name = shared_enum.ffi_name_string(),
            variants = variants,
            convert_swift_to_ffi_repr = convert_swift_to_ffi_repr,
            convert_ffi_repr_to_swift = convert_ffi_repr_to_swift
        );

        Some(swift_enum)
    }
}
