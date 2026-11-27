//! # xberg-libheif
//!
//! Vendored from [libheif-rs](https://github.com/Cykooz/libheif-rs) v2.7.0 — MIT,
//! Kirill Kuzminykh (Cykooz). See `ATTRIBUTIONS.md` at the repository root.
//!
//! Safe wrapper around the `libheif-sys` crate for parsing HEIF / HEIC / AVIF
//! files. The public API mirrors upstream libheif-rs v2.7.0.

#![deny(clippy::print_stdout, clippy::print_stderr)]
#![cfg_attr(test, allow(clippy::print_stdout, clippy::print_stderr))]
#![cfg_attr(docsrs, feature(doc_cfg))]
extern crate core;

pub use color_profile::*;
pub use context::HeifContext;
pub use decoder::*;
pub use encoder::*;
pub use enums::*;
pub use errors::{HeifError, HeifErrorCode, HeifErrorSubCode, Result};
pub use heif::*;
pub use image::*;
pub use image_handle::{AuxiliaryImagesFilter, ImageHandle, ItemId};
pub use metadata::ImageMetadata;
pub use reader::{Reader, StreamReader};
#[cfg(feature = "v1_19")]
pub use security_limits::*;
#[cfg(feature = "v1_20")]
pub use track::*;
pub use utils::check_file_type;

// Every libheif call goes through `lh`: the linked `libheif-sys` declarations, or with the
// `dynamic` feature their runtime-loaded wrappers. In both, a root function (one reachable
// without a libheif object) returns `Result`, so a missing library is an error on every path. ~keep
#[cfg(feature = "dynamic")]
use dynamic as lh;
#[cfg(not(feature = "dynamic"))]
use linked as lh;

/// Fails with why libheif cannot be loaded (`dynamic` feature); always succeeds when linked.
#[inline]
pub(crate) fn ensure_libheif() -> Result<()> {
    #[cfg(feature = "dynamic")]
    {
        dynamic::ensure_loaded()
    }
    #[cfg(not(feature = "dynamic"))]
    {
        Ok(())
    }
}

/// `Ok` when libheif can be called: always with a linked libheif; with the `dynamic` feature,
/// once it loads, else the cached error naming every path searched
/// ([`HeifErrorCode::PluginLoadingError`]). Lets a caller tell a missing library from a file
/// libheif rejects.
pub fn ensure_available() -> Result<()> {
    ensure_libheif()
}

mod color_profile;
mod context;
mod decoder;
#[cfg(feature = "dynamic")]
mod dynamic;
mod encoder;
mod enums;
mod errors;
mod heif;
mod image;
mod image_handle;
#[cfg(feature = "image")]
pub mod integration;
#[cfg(not(feature = "dynamic"))]
mod linked;
mod metadata;
mod reader;
#[cfg(feature = "v1_18")]
pub mod regions;
#[cfg(feature = "v1_19")]
mod security_limits;
#[cfg(feature = "v1_20")]
mod track;
mod utils;
