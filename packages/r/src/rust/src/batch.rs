//! Batch extraction functions

use crate::config::parse_config;
use crate::error::{kreuzberg_error, to_r_error};
use crate::result::extraction_result_to_list;
use extendr_api::prelude::*;
use std::path::PathBuf;

/// Build file items from paths and optional per-file configs.
fn build_file_items(
    paths: Vec<String>,
    file_configs: Option<&List>,
) -> extendr_api::Result<Vec<(PathBuf, Option<kreuzberg::FileExtractionConfig>)>> {
    if let Some(fc_list) = file_configs {
        if fc_list.len() != paths.len() {
            return Err(format!(
                "file_configs length ({}) must match paths length ({})",
                fc_list.len(),
                paths.len()
            )
            .into());
        }
    }

    let items = match file_configs {
        Some(fc_list) => paths
            .into_iter()
            .zip(fc_list.values())
            .map(|(path, fc_val)| {
                let fc = if fc_val.is_null() {
                    None
                } else {
                    let fc_str: String = fc_val.try_into().map_err(to_r_error)?;
                    let fc: kreuzberg::FileExtractionConfig = serde_json::from_str(&fc_str).map_err(to_r_error)?;
                    Some(fc)
                };
                Ok((PathBuf::from(path), fc))
            })
            .collect::<extendr_api::Result<Vec<_>>>()?,
        None => paths.into_iter().map(|p| (PathBuf::from(p), None)).collect(),
    };
    Ok(items)
}

/// Build bytes items from data, mime types, and optional per-item configs.
fn build_bytes_items(
    data_list: &List,
    mime_types: Strings,
    file_configs: Option<&List>,
) -> extendr_api::Result<Vec<(Vec<u8>, String, Option<kreuzberg::FileExtractionConfig>)>> {
    let mime_vec: Vec<String> = mime_types.iter().map(|s| s.to_string()).collect();

    if data_list.len() != mime_vec.len() {
        return Err(format!(
            "data_list length ({}) must match mime_types length ({})",
            data_list.len(),
            mime_vec.len()
        )
        .into());
    }

    if let Some(fc_list) = file_configs {
        if fc_list.len() != data_list.len() {
            return Err(format!(
                "file_configs length ({}) must match data_list length ({})",
                fc_list.len(),
                data_list.len()
            )
            .into());
        }
    }

    let items = match file_configs {
        Some(fc_list) => data_list
            .values()
            .zip(mime_vec.into_iter())
            .zip(fc_list.values())
            .map(|((v, mime), fc_val)| {
                let raw = Raw::try_from(v).map_err(to_r_error)?;
                let fc = if fc_val.is_null() {
                    None
                } else {
                    let fc_str: String = fc_val.try_into().map_err(to_r_error)?;
                    let fc: kreuzberg::FileExtractionConfig = serde_json::from_str(&fc_str).map_err(to_r_error)?;
                    Some(fc)
                };
                Ok((raw.as_slice().to_vec(), mime, fc))
            })
            .collect::<extendr_api::Result<Vec<_>>>()?,
        None => data_list
            .values()
            .zip(mime_vec.into_iter())
            .map(|(v, mime)| {
                let raw = Raw::try_from(v).map_err(to_r_error)?;
                Ok((raw.as_slice().to_vec(), mime, None))
            })
            .collect::<extendr_api::Result<Vec<_>>>()?,
    };
    Ok(items)
}

pub fn batch_extract_files_sync_impl(
    paths: Strings,
    file_configs: Nullable<&List>,
    config_json: Nullable<&str>,
) -> extendr_api::Result<List> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let config = parse_config(config_json)?;
        let path_vec: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
        let items = build_file_items(path_vec, file_configs.0)?;
        let results = kreuzberg::batch_extract_file_sync(items, &config).map_err(kreuzberg_error)?;
        let r_results: Vec<Robj> = results
            .into_iter()
            .map(|r| extraction_result_to_list(r).map(|l| l.into_robj()))
            .collect::<extendr_api::Result<Vec<_>>>()?;
        Ok(List::from_values(r_results))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (paths, file_configs, config_json);
        Err("Batch file extraction is not supported on WebAssembly".into())
    }
}

pub fn batch_extract_files_impl(
    paths: Strings,
    file_configs: Nullable<&List>,
    config_json: Nullable<&str>,
) -> extendr_api::Result<List> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let config = parse_config(config_json)?;
        let path_vec: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
        let items = build_file_items(path_vec, file_configs.0)?;
        let runtime = tokio::runtime::Runtime::new().map_err(to_r_error)?;
        let results = runtime
            .block_on(async { kreuzberg::batch_extract_file(items, &config).await })
            .map_err(kreuzberg_error)?;
        let r_results: Vec<Robj> = results
            .into_iter()
            .map(|r| extraction_result_to_list(r).map(|l| l.into_robj()))
            .collect::<extendr_api::Result<Vec<_>>>()?;
        Ok(List::from_values(r_results))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (paths, file_configs, config_json);
        Err("Async batch file extraction is not supported on WebAssembly".into())
    }
}

pub fn batch_extract_bytes_sync_impl(
    data_list: List,
    mime_types: Strings,
    file_configs: Nullable<&List>,
    config_json: Nullable<&str>,
) -> extendr_api::Result<List> {
    let config = parse_config(config_json)?;
    let items = build_bytes_items(&data_list, mime_types, file_configs.0)?;
    let results = kreuzberg::batch_extract_bytes_sync(items, &config).map_err(kreuzberg_error)?;
    let r_results: Vec<Robj> = results
        .into_iter()
        .map(|r| extraction_result_to_list(r).map(|l| l.into_robj()))
        .collect::<extendr_api::Result<Vec<_>>>()?;
    Ok(List::from_values(r_results))
}

pub fn batch_extract_bytes_impl(
    data_list: List,
    mime_types: Strings,
    file_configs: Nullable<&List>,
    config_json: Nullable<&str>,
) -> extendr_api::Result<List> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let config = parse_config(config_json)?;
        let items = build_bytes_items(&data_list, mime_types, file_configs.0)?;
        let runtime = tokio::runtime::Runtime::new().map_err(to_r_error)?;
        let results = runtime
            .block_on(async { kreuzberg::batch_extract_bytes(items, &config).await })
            .map_err(kreuzberg_error)?;
        let r_results: Vec<Robj> = results
            .into_iter()
            .map(|r| extraction_result_to_list(r).map(|l| l.into_robj()))
            .collect::<extendr_api::Result<Vec<_>>>()?;
        Ok(List::from_values(r_results))
    }
    #[cfg(target_arch = "wasm32")]
    {
        batch_extract_bytes_sync_impl(data_list, mime_types, file_configs, config_json)
    }
}
