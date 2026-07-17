use crate::bridge_module_attributes::CfgAttr;
use crate::SwiftBridgeModule;

mod generate_c_header;
mod generate_rust_tokens;
mod generate_swift;

#[cfg(test)]
mod codegen_tests;

/// The corresponding Swift code and C header for a bridge module.
pub struct SwiftCodeAndCHeader {
    /// The generated Swift code.
    pub swift: String,
    /// The generated C header.
    pub c_header: String,
}

/// Configuration for how we will generate our Swift code.
pub struct CodegenConfig {
    /// Look up whether or not a feature is enabled for the crate that holds the bridge module.
    /// This helps us decide whether or not to generate code for parts of the module
    /// that are annotated with `#[cfg(feature = "some-feature")]`
    pub crate_feature_lookup: Box<dyn Fn(&str) -> bool>,
}

#[cfg(test)]
impl CodegenConfig {
    pub(crate) fn no_features_enabled() -> Self {
        CodegenConfig {
            crate_feature_lookup: Box::new(|_| false),
        }
    }
}

impl SwiftBridgeModule {
    /// Generate the corresponding Swift code and C header for a bridge module.
    pub fn generate_swift_code_and_c_header(&self, config: CodegenConfig) -> SwiftCodeAndCHeader {
        SwiftCodeAndCHeader {
            swift: self.generate_swift(&config),
            c_header: self.generate_c_header(&config),
        }
    }

    /// Whether or not the module's conditional compilation flags willl lead it to being included
    /// in the final binary.
    /// If not, when we won't generate any C or Swift code for it.
    fn module_will_be_compiled(&self, config: &CodegenConfig) -> bool {
        for cfg_attr in &self.cfg_attrs {
            match cfg_attr {
                CfgAttr::Feature(feature_name) => {
                    if !(config.crate_feature_lookup)(&feature_name.value()) {
                        return false;
                    }
                }
            }
        }

        true
    }
}
