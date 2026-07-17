use crate::codegen::generate_swift::generate_swift_class_methods;
use crate::parse::OpaqueForeignTypeDeclaration;
use crate::{ParsedExternFn, TypeDeclarations, SWIFT_BRIDGE_PREFIX};
use std::collections::HashMap;
use syn::Path;

pub(super) fn generate_opaque_copy_struct(
    ty: &OpaqueForeignTypeDeclaration,
    associated_funcs_and_methods: &HashMap<String, Vec<&ParsedExternFn>>,
    types: &TypeDeclarations,
    swift_bridge_path: &Path,
) -> String {
    let type_name = &ty.ty.to_string();

    let class_methods = generate_swift_class_methods(
        &type_name,
        associated_funcs_and_methods,
        types,
        swift_bridge_path,
    );

    let mut extensions = "".to_string();

    append_methods_extension(&mut extensions, type_name, &class_methods.initializers);
    append_methods_extension(
        &mut extensions,
        type_name,
        &class_methods.owned_self_methods,
    );
    append_methods_extension(&mut extensions, type_name, &class_methods.ref_self_methods);

    if class_methods.owned_self_methods.len() > 0 {};

    let struct_definition = if !ty.attributes.already_declared {
        generate_struct_definition(ty, types, swift_bridge_path)
    } else {
        "".to_string()
    };

    format!(
        r#"{struct_definition}{extensions}"#,
        struct_definition = struct_definition,
        extensions = extensions
    )
}

fn generate_struct_definition(
    ty: &OpaqueForeignTypeDeclaration,
    types: &TypeDeclarations,
    swift_bridge_path: &Path,
) -> String {
    let type_name = ty.ty.to_string();
    let generics = ty.generics.angle_bracketed_generic_placeholders_string();

    let declare_struct = if ty.generics.is_empty() {
        format!(
            r#"public struct {type_name} {{
    fileprivate var bytes: {prefix}${type_name}

    func intoFfiRepr() -> {prefix}${type_name} {{
        bytes
    }}
}}"#,
            prefix = SWIFT_BRIDGE_PREFIX,
            type_name = type_name,
        )
    } else {
        format!(
            r#"public struct {type_name}{generics} {{
    fileprivate var bytes: SwiftBridgeGenericCopyTypeFfiRepr
}}"#,
            type_name = type_name,
            generics = generics
        )
    };

    let ffi_repr_conversion = if ty.generics.is_empty() {
        format!(
            r#"extension {prefix}${type_name} {{
    func intoSwiftRepr() -> {type_name} {{
        {type_name}(bytes: self)
    }}
}}"#,
            prefix = SWIFT_BRIDGE_PREFIX,
            type_name = type_name,
        )
    } else {
        let ffi_repr_name = ty.ffi_repr_name_string();
        let bounds = ty
            .generics
            .rust_opaque_type_swift_generic_bounds(types, swift_bridge_path);

        format!(
            r#"extension {type_name}
where {bounds} {{
    func intoFfiRepr() -> {ffi_repr_name} {{
        self.bytes as! {ffi_repr_name}
    }}
}}
extension {ffi_repr_name} {{
    func intoSwiftRepr() -> {type_name}{generics} {{
        {type_name}(bytes: self)
    }}
}}
extension {ffi_repr_name}: SwiftBridgeGenericCopyTypeFfiRepr {{}}"#,
            ffi_repr_name = ffi_repr_name,
            type_name = type_name,
            bounds = bounds,
            generics = ty
                .generics
                .angle_bracketed_generic_concrete_swift_types_string(types, swift_bridge_path),
        )
    };

    let ext_equatable = if ty.attributes.equatable {
        format!(
            r#"
extension {type_name}: Equatable {{
    public static func == (lhs: Self, rhs: Self) -> Bool {{
        var lhs = lhs
        var rhs = rhs
        return withUnsafePointer(to: &lhs.bytes, {{(lhs_p: UnsafePointer<{ffi_repr_name}>) in
            return withUnsafePointer(to: &rhs.bytes, {{(rhs_p: UnsafePointer<{ffi_repr_name}>) in
                return __swift_bridge__${type_name}$_partial_eq(
                    UnsafeMutablePointer(mutating: lhs_p),
                    UnsafeMutablePointer(mutating: rhs_p)
                )
            }})
        }})
    }}
}}
"#,
            type_name = type_name,
            ffi_repr_name = ty.ffi_repr_name_string()
        )
    } else {
        String::new()
    };

    let ext_hashable = if ty.attributes.hashable {
        format!(
            r#"
extension {type_name}: Hashable {{
    public func hash(into hasher: inout Hasher){{
        var this = self
        return withUnsafePointer(to: &this.bytes, {{(ptr: UnsafePointer<{ffi_repr_name}>) in
            hasher.combine(__swift_bridge__${type_name}$_hash(
                UnsafeMutablePointer(mutating: ptr)
            ))
        }})
    }}
}}
"#,
            type_name = type_name,
            ffi_repr_name = ty.ffi_repr_name_string()
        )
    } else {
        String::new()
    };

    format!(
        r#"{declare_struct}
{ffi_repr_conversion}
{ext_equatable}{ext_hashable}"#,
    )
}

fn append_methods_extension(extensions: &mut String, type_name: &str, methods: &[String]) {
    if methods.len() == 0 {
        return;
    }

    *extensions += &format!(
        r#"
extension {type_name} {{
"#,
        type_name = type_name
    );

    for (idx, method) in methods.iter().enumerate() {
        if idx > 0 {
            *extensions += "\n";
        }

        *extensions += method;
    }

    *extensions += "\n}";
}
