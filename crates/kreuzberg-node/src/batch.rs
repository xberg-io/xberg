use napi::bindgen_prelude::*;
use napi_derive::napi;

use std::path::PathBuf;

use crate::WORKER_POOL;
use crate::config::{JsExtractionConfig, JsFileExtractionConfig};
use crate::error_handling::convert_error;
use crate::result::{JsExtractionResult, resolve_config, resolve_file_config};

#[napi]
pub fn batch_extract_files_sync(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
    file_configs: Option<Vec<Option<JsFileExtractionConfig>>>,
) -> Result<Vec<JsExtractionResult>> {
    if let Some(ref fcs) = file_configs
        && paths.len() != fcs.len()
    {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "paths length ({}) must match fileConfigs length ({})",
                paths.len(),
                fcs.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let items: Vec<(PathBuf, Option<kreuzberg::FileExtractionConfig>)> = match file_configs {
        Some(fcs) => paths
            .into_iter()
            .zip(fcs)
            .map(|(path, fc)| Ok((PathBuf::from(path), resolve_file_config(fc)?)))
            .collect::<Result<Vec<_>>>()?,
        None => paths.into_iter().map(|path| (PathBuf::from(path), None)).collect(),
    };

    kreuzberg::batch_extract_file_sync(items, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple files (asynchronous).
///
/// Asynchronously processes multiple files in parallel. Non-blocking alternative
/// to `batchExtractFilesSync` with same performance benefits.
///
/// # Parameters
///
/// * `paths` - Array of file paths to extract
/// * `config` - Optional extraction configuration (applied to all files)
/// * `file_configs` - Optional per-file extraction configs (must match paths length if provided)
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractFiles } from '@kreuzberg/node';
///
/// const files = ['report1.pdf', 'report2.pdf', 'report3.pdf'];
/// const results = await batchExtractFiles(files, null);
/// console.log(`Processed ${results.length} files`);
/// ```
#[napi]
pub async fn batch_extract_files(
    paths: Vec<String>,
    config: Option<JsExtractionConfig>,
    file_configs: Option<Vec<Option<JsFileExtractionConfig>>>,
) -> Result<Vec<JsExtractionResult>> {
    if let Some(ref fcs) = file_configs
        && paths.len() != fcs.len()
    {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "paths length ({}) must match fileConfigs length ({})",
                paths.len(),
                fcs.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let items: Vec<(PathBuf, Option<kreuzberg::FileExtractionConfig>)> = match file_configs {
        Some(fcs) => paths
            .into_iter()
            .zip(fcs)
            .map(|(path, fc)| Ok((PathBuf::from(path), resolve_file_config(fc)?)))
            .collect::<Result<Vec<_>>>()?,
        None => paths.into_iter().map(|path| (PathBuf::from(path), None)).collect(),
    };

    let results = WORKER_POOL
        .spawn_blocking(move || kreuzberg::batch_extract_file_sync(items, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    results.into_iter().map(JsExtractionResult::try_from).collect()
}

/// Batch extract from multiple byte arrays (synchronous).
///
/// Synchronously processes multiple in-memory buffers in parallel. Requires
/// corresponding MIME types for each buffer.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
/// * `file_configs` - Optional per-item extraction configs (must match data_list length if provided)
///
/// # Returns
///
/// Array of `ExtractionResult` in the same order as inputs.
///
/// # Errors
///
/// Throws if data_list and mime_types lengths don't match.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytesSync } from '@kreuzberg/node';
///
/// const buffers = [buffer1, buffer2, buffer3];
/// const mimeTypes = ['application/pdf', 'image/png', 'text/plain'];
/// const results = batchExtractBytesSync(buffers, mimeTypes, null);
/// ```
#[napi]
pub fn batch_extract_bytes_sync(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
    file_configs: Option<Vec<Option<JsFileExtractionConfig>>>,
) -> Result<Vec<JsExtractionResult>> {
    if data_list.len() != mime_types.len() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match mime_types length ({})",
                data_list.len(),
                mime_types.len()
            ),
        ));
    }

    if let Some(ref fcs) = file_configs
        && data_list.len() != fcs.len()
    {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match fileConfigs length ({})",
                data_list.len(),
                fcs.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let items: Vec<(Vec<u8>, String, Option<kreuzberg::FileExtractionConfig>)> = match file_configs {
        Some(fcs) => data_list
            .iter()
            .zip(mime_types)
            .zip(fcs)
            .map(|((data, mime), fc)| Ok((data.to_vec(), mime, resolve_file_config(fc)?)))
            .collect::<Result<Vec<_>>>()?,
        None => data_list
            .iter()
            .zip(mime_types)
            .map(|(data, mime)| (data.to_vec(), mime, None))
            .collect(),
    };

    kreuzberg::batch_extract_bytes_sync(items, &rust_config)
        .map_err(convert_error)
        .and_then(|results| results.into_iter().map(JsExtractionResult::try_from).collect())
}

/// Batch extract from multiple byte arrays (asynchronous).
///
/// Asynchronously processes multiple in-memory buffers in parallel. Non-blocking
/// alternative to `batchExtractBytesSync`.
///
/// # Parameters
///
/// * `data_list` - Array of buffers to extract
/// * `mime_types` - Array of MIME types (must match data_list length)
/// * `config` - Optional extraction configuration
/// * `file_configs` - Optional per-item extraction configs (must match data_list length if provided)
///
/// # Returns
///
/// Promise resolving to array of `ExtractionResult`.
///
/// # Example
///
/// ```typescript
/// import { batchExtractBytes } from '@kreuzberg/node';
///
/// const responses = await Promise.all([
///   fetch('https://example.com/doc1.pdf'),
///   fetch('https://example.com/doc2.pdf')
/// ]);
/// const buffers = await Promise.all(
///   responses.map(r => r.arrayBuffer().then(b => Buffer.from(b)))
/// );
/// const results = await batchExtractBytes(
///   buffers,
///   ['application/pdf', 'application/pdf'],
///   null
/// );
/// ```
#[napi]
pub async fn batch_extract_bytes(
    data_list: Vec<Buffer>,
    mime_types: Vec<String>,
    config: Option<JsExtractionConfig>,
    file_configs: Option<Vec<Option<JsFileExtractionConfig>>>,
) -> Result<Vec<JsExtractionResult>> {
    if data_list.len() != mime_types.len() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match mime_types length ({})",
                data_list.len(),
                mime_types.len()
            ),
        ));
    }

    if let Some(ref fcs) = file_configs
        && data_list.len() != fcs.len()
    {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "data_list length ({}) must match fileConfigs length ({})",
                data_list.len(),
                fcs.len()
            ),
        ));
    }

    let rust_config = resolve_config(config)?;

    let items: Vec<(Vec<u8>, String, Option<kreuzberg::FileExtractionConfig>)> = match file_configs {
        Some(fcs) => data_list
            .iter()
            .zip(mime_types.into_iter())
            .zip(fcs)
            .map(|((data, mime), fc)| Ok((data.to_vec(), mime, resolve_file_config(fc)?)))
            .collect::<Result<Vec<_>>>()?,
        None => data_list
            .iter()
            .zip(mime_types.into_iter())
            .map(|(data, mime)| (data.to_vec(), mime, None))
            .collect(),
    };

    let results = WORKER_POOL
        .spawn_blocking(move || kreuzberg::batch_extract_bytes_sync(items, &rust_config))
        .await
        .map_err(|e| Error::from_reason(format!("Worker thread error: {}", e)))?
        .map_err(convert_error)?;

    results.into_iter().map(JsExtractionResult::try_from).collect()
}
