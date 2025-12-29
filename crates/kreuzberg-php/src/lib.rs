//! Kreuzberg PHP Bindings
//!
//! This module exposes the Rust core extraction API to PHP using ext-php-rs.
//!
//! # Architecture
//!
//! - All extraction logic is in the Rust core (crates/kreuzberg)
//! - PHP is a thin wrapper that adds language-specific features
//! - Zero duplication of core functionality
//! - Modern ext-php-rs patterns throughout

#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(dead_code, unused_imports)]

use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

mod config;
mod embeddings;
mod error;
mod extraction;
mod plugins;
mod types;
mod validation;

use config::*;
use embeddings::*;
use error::*;
use extraction::*;
use plugins::*;
use types::*;
use validation::*;

/// Get the Kreuzberg library version.
///
/// # Returns
///
/// Version string in semver format (e.g., "4.0.0-rc.20")
///
/// # Example
///
/// ```php
/// $version = kreuzberg_version();
/// echo "Kreuzberg version: $version\n";
/// ```
#[php_function]
pub fn kreuzberg_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Kreuzberg PHP extension module.
///
/// Exports all extraction functions, configuration types, error handling, and plugin management.
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
