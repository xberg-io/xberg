//! File extraction functions
//!
//! Handles extraction from files and byte arrays (synchronous and asynchronous).

use crate::config::parse_extraction_config;
use crate::error_handling::kreuzberg_error;
use crate::result::extraction_result_to_ruby;

use magnus::{Error, RHash, RString, Ruby, Value, scan_args::scan_args};

/// Extract content from a file (synchronous)
pub fn extract_file_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let result = kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config).map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Extract content from bytes (synchronous)
pub fn extract_bytes_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RString, String), (), (), (), RHash, ()>(args)?;
    let (data, mime_type) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let bytes = unsafe { data.as_slice() };
    let result = kreuzberg::extract_bytes_sync(bytes, &mime_type, &config).map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Extract content from a file (asynchronous)
pub fn extract_file(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| crate::error_handling::runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let result = runtime
        .block_on(async { kreuzberg::extract_file(&path, mime_type.as_deref(), &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}

/// Extract content from bytes (asynchronous)
pub fn extract_bytes(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(RString, String), (), (), (), RHash, ()>(args)?;
    let (data, mime_type) = args.required;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| crate::error_handling::runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let bytes = unsafe { data.as_slice() };
    let result = runtime
        .block_on(async { kreuzberg::extract_bytes(bytes, &mime_type, &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}
