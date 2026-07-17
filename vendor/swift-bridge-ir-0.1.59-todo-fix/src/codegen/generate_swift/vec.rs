use proc_macro2::Ident;

/// Generate the `extension MyRustType: Vectorizable {}` for the Swift side.
pub(super) fn generate_vectorizable_extension(ty: &Ident) -> String {
    format!(
        r#"extension {ty}: Vectorizable {{
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {{
        __swift_bridge__$Vec_{ty}$new()
    }}

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {{
        __swift_bridge__$Vec_{ty}$drop(vecPtr)
    }}

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: {ty}) {{
        __swift_bridge__$Vec_{ty}$push(vecPtr, {{value.isOwned = false; return value.ptr;}}())
    }}

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {{
        let pointer = __swift_bridge__$Vec_{ty}$pop(vecPtr)
        if pointer == nil {{
            return nil
        }} else {{
            return ({ty}(ptr: pointer!) as! Self)
        }}
    }}

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<{ty}Ref> {{
        let pointer = __swift_bridge__$Vec_{ty}$get(vecPtr, index)
        if pointer == nil {{
            return nil
        }} else {{
            return {ty}Ref(ptr: pointer!)
        }}
    }}

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<{ty}RefMut> {{
        let pointer = __swift_bridge__$Vec_{ty}$get_mut(vecPtr, index)
        if pointer == nil {{
            return nil
        }} else {{
            return {ty}RefMut(ptr: pointer!)
        }}
    }}

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<{ty}Ref> {{
        UnsafePointer<{ty}Ref>(OpaquePointer(__swift_bridge__$Vec_{ty}$as_ptr(vecPtr)))
    }}

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {{
        __swift_bridge__$Vec_{ty}$len(vecPtr)
    }}
}}
"#,
        ty = ty.to_string()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::assert_trimmed_generated_equals_trimmed_expected;
    use proc_macro2::Span;

    /// Verify that we generate the `extension MyRustType: Vectorizable { }` implementation
    /// for the Swift side.
    #[test]
    fn generates_vectorizable_extension() {
        let expected = r#"
extension ARustType: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_ARustType$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_ARustType$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: ARustType) {
        __swift_bridge__$Vec_ARustType$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_ARustType$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (ARustType(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ARustTypeRef> {
        let pointer = __swift_bridge__$Vec_ARustType$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ARustTypeRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ARustTypeRefMut> {
        let pointer = __swift_bridge__$Vec_ARustType$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ARustTypeRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ARustTypeRef> {
        UnsafePointer<ARustTypeRef>(OpaquePointer(__swift_bridge__$Vec_ARustType$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_ARustType$len(vecPtr)
    }
}
"#;

        assert_trimmed_generated_equals_trimmed_expected(
            &generate_vectorizable_extension(&Ident::new("ARustType", Span::call_site())),
            &expected,
        );
    }
}
