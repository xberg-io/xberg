//! Layout-guided extraction utilities.
//!
//! Provides table recognition, layout validation, and heading classification
//! helpers used by the PDF structure pipeline.

mod heading;
pub(super) mod layout_validation;
pub(super) mod table_recognition;
pub(crate) mod tables;

pub(super) use heading::{looks_like_bare_url, looks_like_figure_label};
#[cfg(feature = "layout-detection")]
pub(super) use table_recognition::recognize_tables_slanet;
#[cfg(feature = "layout-detection")]
pub(super) use table_recognition::{NativeTatrRecognitionOptions, recognize_tables_for_native_page};
pub(super) use tables::extract_tables_from_layout_hints;
