//! An intermediate representation of the FFI layer.
//!
//! Things annotated with the `#[swift_bridge::bridge]` attribute get parsed into this IR.
//!
//! This IR is then used to generate the C header files, Objective-C bridging headers, Swift code,
//! and Rust code needed to power Rust + Swift interop.

#![deny(missing_docs)]

use proc_macro2::Ident;
use syn::{Path, Visibility};

use crate::bridge_module_attributes::CfgAttr;
use crate::parse::TypeDeclarations;
use crate::parsed_extern_fn::ParsedExternFn;

pub use self::bridge_macro_attributes::{SwiftBridgeModuleAttr, SwiftBridgeModuleAttrs};
pub use self::codegen::CodegenConfig;

mod errors;
mod parse;

mod bridge_macro_attributes;
mod bridge_module_attributes;
mod bridged_type;
mod parsed_extern_fn;

mod codegen;

#[cfg(test)]
mod test_utils;

const SWIFT_BRIDGE_PREFIX: &'static str = "__swift_bridge__";

/// Represents a type definition within an `extern "Rust"` module, as well as all of its methods.
///
/// ```no_run,ignore
/// #[swift_bridge::bridge]
/// mod ffi {
///     extern "Rust" {
///         type Stack;
///
///         fn push(&mut self, val: u8);
///
///         fn pop(self: &mut Stack) -> Option<u8>;
///
///         fn as_ptr(&self) -> *const u8;
///
///         fn len(self: &Stack) -> usize;
///
///         fn consume(self);
///     }
///
///     extern "Swift" {
///         // TODO: Examples
///     }
/// }
/// ```
pub struct SwiftBridgeModule {
    name: Ident,
    vis: Visibility,
    types: TypeDeclarations,
    functions: Vec<ParsedExternFn>,
    swift_bridge_path: Path,
    cfg_attrs: Vec<CfgAttr>,
}

impl SwiftBridgeModule {
    /// Set the path used for `swift_bridge` types such as `swift_bridge::RustString`.
    /// We set this to `crate` when we're inside of the `swift_bridge` crate.
    pub fn set_swift_bridge_path(&mut self, path: Path) {
        self.swift_bridge_path = path;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn foo() {
        //
    }
}
