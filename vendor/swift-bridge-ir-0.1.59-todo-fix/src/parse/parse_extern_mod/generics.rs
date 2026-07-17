pub(crate) use self::generic_opaque_type::*;

mod generic_opaque_type;

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::TypeParam;

    use crate::test_utils::parse_ok;
    use crate::SwiftBridgeModule;

    /// Verify that we can parse generic extern "Rust" types
    #[test]
    fn parse_generic_extern_rust_type() {
        let tokens = quote! {
            #[swift_bridge:bridge]
            mod foo {
                extern "Rust" {
                    type AnotherType;
                    type SomeType<u32>;
                }
            }
        };

        let module = parse_ok(tokens);

        assert_eq!(get_generics(&module, "SomeType<u32>").len(), 1);
    }

    /// Verify that we can parse multiple generic types.
    #[test]
    fn multiple_generics() {
        let tokens = quote! {
            #[swift_bridge:bridge]
            mod foo {
                extern "Rust" {
                    #[swift_bridge(declare_generic)]
                    type SomeType<A>;

                    type SomeType<u32>;
                    type SomeType<u64>;
                }
            }
        };

        let module = parse_ok(tokens);
        assert_eq!(module.types.types().len(), 3);

        assert_eq!(get_generics(&module, "SomeType<u32>").len(), 1);
        assert_eq!(get_generics(&module, "SomeType<u64>").len(), 1);
    }

    fn get_generics<'a>(module: &'a SwiftBridgeModule, type_name: &str) -> &'a Vec<TypeParam> {
        &module
            .types
            .get(type_name)
            .unwrap()
            .unwrap_opaque()
            .generics
    }
}
