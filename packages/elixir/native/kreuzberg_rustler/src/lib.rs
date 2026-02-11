#![allow(clippy::let_unit_value)]

//! Kreuzberg Rustler - Elixir NIF bindings for Kreuzberg document intelligence
//!
//! This module provides Elixir Native Implemented Functions (NIFs) for document extraction,
//! MIME type detection, configuration, and cache management.
//!
//! # Architecture
//!
//! The bindings are organized into focused modules:
//! - `atoms` - Elixir atom definitions
//! - `conversion` - Type conversion between Rust and Elixir
//! - `config` - Configuration parsing and validation
//! - `extraction` - Single document extraction NIFs
//! - `batch` - Batch extraction NIFs
//! - `utilities` - Validation, MIME detection, cache, and config NIFs

mod atoms;
pub mod batch;
pub(crate) mod config;
pub(crate) mod conversion;
pub mod extraction;
pub mod plugins;
mod types;
pub mod utilities;
mod utils;

// The rustler::init! macro will automatically discover NIF functions in public submodules
rustler::init!("Elixir.Kreuzberg.Native", load = on_load);

#[allow(non_local_definitions)]
fn on_load(_env: rustler::Env, _info: rustler::Term) -> bool {
    true
}
