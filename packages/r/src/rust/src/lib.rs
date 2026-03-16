//! Kreuzberg R Bindings (extendr)
//!
//! High-performance document intelligence framework bindings for R.

mod batch;
mod cache;
mod config;
mod error;
mod extraction;
mod helpers;
mod metadata;
mod plugins;
mod result;
mod validation;

use extendr_api::prelude::*;

// Re-export for use by other modules
pub use error::to_r_error;

/// Clear the extraction cache
/// @export
#[extendr]
fn clear_cache() -> extendr_api::Result<()> {
    cache::clear_cache_impl()
}

/// Get cache statistics
/// @export
#[extendr]
fn cache_stats() -> extendr_api::Result<List> {
    cache::cache_stats_impl()
}

// Extraction functions
#[extendr]
fn extract_file_sync_native(path: &str, mime_type: Nullable<&str>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    extraction::extract_file_sync_impl(path, mime_type, config_json)
}

#[extendr]
fn extract_file_native(path: &str, mime_type: Nullable<&str>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    extraction::extract_file_impl(path, mime_type, config_json)
}

#[extendr]
fn extract_bytes_sync_native(data: Raw, mime_type: &str, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    extraction::extract_bytes_sync_impl(data, mime_type, config_json)
}

#[extendr]
fn extract_bytes_native(data: Raw, mime_type: &str, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    extraction::extract_bytes_impl(data, mime_type, config_json)
}

// Batch extraction functions
#[extendr]
fn batch_extract_files_sync_native(paths: Strings, file_configs: Nullable<&List>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    batch::batch_extract_files_sync_impl(paths, file_configs, config_json)
}

#[extendr]
fn batch_extract_files_native(paths: Strings, file_configs: Nullable<&List>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    batch::batch_extract_files_impl(paths, file_configs, config_json)
}

#[extendr]
fn batch_extract_bytes_sync_native(data_list: List, mime_types: Strings, file_configs: Nullable<&List>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    batch::batch_extract_bytes_sync_impl(data_list, mime_types, file_configs, config_json)
}

#[extendr]
fn batch_extract_bytes_native(data_list: List, mime_types: Strings, file_configs: Nullable<&List>, config_json: Nullable<&str>) -> extendr_api::Result<List> {
    batch::batch_extract_bytes_impl(data_list, mime_types, file_configs, config_json)
}

// Metadata functions
#[extendr]
fn detect_mime_type_native(data: Raw) -> extendr_api::Result<String> {
    metadata::detect_mime_type_impl(data)
}

#[extendr]
fn detect_mime_type_from_path_native(path: &str) -> extendr_api::Result<String> {
    metadata::detect_mime_type_from_path_impl(path)
}

#[extendr]
fn get_extensions_for_mime_native(mime_type: &str) -> extendr_api::Result<Strings> {
    metadata::get_extensions_for_mime_impl(mime_type)
}

#[extendr]
fn validate_mime_type_native(mime_type: &str) -> extendr_api::Result<bool> {
    metadata::validate_mime_type_impl(mime_type)
}

// Plugin functions
#[extendr]
fn register_post_processor_native(name: &str, callback: Robj) -> extendr_api::Result<()> {
    plugins::register_post_processor_impl(name, callback)
}

#[extendr]
fn unregister_post_processor_native(name: &str) -> extendr_api::Result<()> {
    plugins::unregister_post_processor_impl(name)
}

#[extendr]
fn list_post_processors_native() -> extendr_api::Result<Strings> {
    plugins::list_post_processors_impl()
}

#[extendr]
fn clear_post_processors_native() -> extendr_api::Result<()> {
    plugins::clear_post_processors_impl()
}

#[extendr]
fn register_validator_native(name: &str, callback: Robj) -> extendr_api::Result<()> {
    plugins::register_validator_impl(name, callback)
}

#[extendr]
fn unregister_validator_native(name: &str) -> extendr_api::Result<()> {
    plugins::unregister_validator_impl(name)
}

#[extendr]
fn list_validators_native() -> extendr_api::Result<Strings> {
    plugins::list_validators_impl()
}

#[extendr]
fn clear_validators_native() -> extendr_api::Result<()> {
    plugins::clear_validators_impl()
}

#[extendr]
fn register_ocr_backend_native(name: &str, callback: Robj) -> extendr_api::Result<()> {
    plugins::register_ocr_backend_impl(name, callback)
}

#[extendr]
fn unregister_ocr_backend_native(name: &str) -> extendr_api::Result<()> {
    plugins::unregister_ocr_backend_impl(name)
}

#[extendr]
fn list_ocr_backends_native() -> extendr_api::Result<Strings> {
    plugins::list_ocr_backends_impl()
}

#[extendr]
fn clear_ocr_backends_native() -> extendr_api::Result<()> {
    plugins::clear_ocr_backends_impl()
}

#[extendr]
fn list_document_extractors_native() -> extendr_api::Result<Strings> {
    plugins::list_document_extractors_impl()
}

#[extendr]
fn unregister_document_extractor_native(name: &str) -> extendr_api::Result<()> {
    plugins::unregister_document_extractor_impl(name)
}

#[extendr]
fn clear_document_extractors_native() -> extendr_api::Result<()> {
    plugins::clear_document_extractors_impl()
}

// Config loading functions
#[extendr]
fn config_from_file_native(path: &str) -> extendr_api::Result<Nullable<String>> {
    config::from_file_impl(path)
}

#[extendr]
fn config_discover_native() -> extendr_api::Result<Nullable<String>> {
    config::discover_impl()
}

// Validation functions
#[extendr]
fn validate_ocr_backend_name_native(backend: &str) -> extendr_api::Result<bool> {
    validation::validate_ocr_backend_impl(backend)
}

#[extendr]
fn validate_language_code_native(code: &str) -> extendr_api::Result<bool> {
    validation::validate_language_code_impl(code)
}

#[extendr]
fn validate_output_format_native(format: &str) -> extendr_api::Result<bool> {
    validation::validate_output_format_impl(format)
}

extendr_module! {
    mod kreuzberg;

    fn clear_cache;
    fn cache_stats;

    fn extract_file_sync_native;
    fn extract_file_native;
    fn extract_bytes_sync_native;
    fn extract_bytes_native;

    fn batch_extract_files_sync_native;
    fn batch_extract_files_native;
    fn batch_extract_bytes_sync_native;
    fn batch_extract_bytes_native;

    fn detect_mime_type_native;
    fn detect_mime_type_from_path_native;
    fn get_extensions_for_mime_native;
    fn validate_mime_type_native;

    fn register_post_processor_native;
    fn unregister_post_processor_native;
    fn list_post_processors_native;
    fn clear_post_processors_native;
    fn register_validator_native;
    fn unregister_validator_native;
    fn list_validators_native;
    fn clear_validators_native;
    fn register_ocr_backend_native;
    fn unregister_ocr_backend_native;
    fn list_ocr_backends_native;
    fn clear_ocr_backends_native;
    fn list_document_extractors_native;
    fn unregister_document_extractor_native;
    fn clear_document_extractors_native;

    fn config_from_file_native;
    fn config_discover_native;

    fn validate_ocr_backend_name_native;
    fn validate_language_code_native;
    fn validate_output_format_native;
}
