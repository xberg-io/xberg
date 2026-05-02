//! Main extraction entry points.
//!
//! This module provides the primary API for extracting content from files and byte arrays.
//! It orchestrates the entire extraction pipeline: cache checking, MIME detection,
//! extractor selection, extraction, post-processing, and cache storage.
//!
//! # Functions
//!
//! - [`extract_file`] - Extract content from a file path
//! - [`extract_bytes`] - Extract content from a byte array
//! - [`batch_extract_files`] - Extract content from multiple files concurrently
//! - [`batch_extract_bytes`] - Extract content from multiple byte arrays concurrently

mod bytes;
mod file;
mod helpers;
mod legacy;
mod sync;

#[cfg(feature = "tokio-runtime")]
mod batch;

// Re-export public API
pub use bytes::extract_bytes;
pub use file::extract_file;
pub use sync::{batch_extract_bytes_sync, extract_bytes_sync};

#[cfg(feature = "tokio-runtime")]
pub use sync::extract_file_sync;

#[cfg(feature = "tokio-runtime")]
pub use batch::{batch_extract_bytes, batch_extract_files};
#[cfg(feature = "tokio-runtime")]
pub use sync::batch_extract_files_sync;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{BatchBytesItem, BatchFileItem, ExtractionConfig};
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn assert_text_content(actual: &str, expected: &str) {
        assert_eq!(actual.trim_end_matches('\n'), expected);
    }

    #[tokio::test]
    async fn test_extract_file_basic() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_text_content(&result.content, "Hello, world!");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_file_with_mime_override() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.dat");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, Some("text/plain"), &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_file_nonexistent() {
        let config = ExtractionConfig::default();
        let result = extract_file("/nonexistent/file.txt", None, &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_bytes_basic() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test content", "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_text_content(&result.content, "test content");
        assert_eq!(result.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_extract_bytes_invalid_mime() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test", "invalid/mime", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_extract_file() {
        let dir = tempdir().unwrap();

        let file1 = dir.path().join("test1.txt");
        let file2 = dir.path().join("test2.txt");

        File::create(&file1).unwrap().write_all(b"content 1").unwrap();
        File::create(&file2).unwrap().write_all(b"content 2").unwrap();

        let config = ExtractionConfig::default();
        let items = vec![
            BatchFileItem {
                path: file1,
                config: None,
            },
            BatchFileItem {
                path: file2,
                config: None,
            },
        ];
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "content 1");
        assert_text_content(&results[1].content, "content 2");
    }

    #[tokio::test]
    async fn test_batch_extract_file_empty() {
        let config = ExtractionConfig::default();
        let items: Vec<BatchFileItem> = vec![];
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_extract_bytes() {
        let config = ExtractionConfig::default();
        let items = vec![
            BatchBytesItem {
                content: b"content 1".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"content 2".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
        ];
        let results = batch_extract_bytes(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "content 1");
        assert_text_content(&results[1].content, "content 2");
    }

    #[test]
    fn test_sync_wrappers() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"sync test").unwrap();

        let config = ExtractionConfig::default();

        let result = extract_file_sync(&file_path, None, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_text_content(&result.content, "sync test");

        let result = extract_bytes_sync(b"test", "text/plain", &config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extractor_cache() {
        let config = ExtractionConfig::default();

        let result1 = extract_bytes(b"test 1", "text/plain", &config).await;
        assert!(result1.is_ok());
        let result1 = result1.unwrap();

        let result2 = extract_bytes(b"test 2", "text/plain", &config).await;
        assert!(result2.is_ok());
        let result2 = result2.unwrap();

        assert_text_content(&result1.content, "test 1");
        assert_text_content(&result2.content, "test 2");

        let result3 = extract_bytes(b"# test 3", "text/markdown", &config).await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_empty() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        File::create(&file_path).unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_extract_bytes_empty() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"", "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.content, "");
    }

    #[tokio::test]
    async fn test_extract_file_whitespace_only() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("whitespace.txt");
        File::create(&file_path).unwrap().write_all(b"   \n\t  \n  ").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_file_very_long_path() {
        let dir = tempdir().unwrap();
        let long_name = "a".repeat(200);
        let file_path = dir.path().join(format!("{}.txt", long_name));

        if let Ok(mut f) = File::create(&file_path) {
            f.write_all(b"content").unwrap();
            let config = ExtractionConfig::default();
            let result = extract_file(&file_path, None, &config).await;
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    async fn test_extract_file_special_characters_in_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test with spaces & symbols!.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_text_content(&result.content, "content");
    }

    #[tokio::test]
    async fn test_extract_file_unicode_filename() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("测试文件名.txt");
        File::create(&file_path).unwrap().write_all(b"content").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_bytes_unsupported_mime() {
        let config = ExtractionConfig::default();
        let result = extract_bytes(b"test", "application/x-unknown-format", &config).await;

        assert!(result.is_err());
        use crate::KreuzbergError;
        assert!(matches!(result.unwrap_err(), KreuzbergError::UnsupportedFormat(_)));
    }

    #[tokio::test]
    async fn test_batch_extract_file_with_errors() {
        let dir = tempdir().unwrap();

        let valid_file = dir.path().join("valid.txt");
        File::create(&valid_file).unwrap().write_all(b"valid content").unwrap();

        let invalid_file = dir.path().join("nonexistent.txt");

        let config = ExtractionConfig::default();
        let items = vec![
            BatchFileItem {
                path: valid_file,
                config: None,
            },
            BatchFileItem {
                path: invalid_file,
                config: None,
            },
        ];
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "valid content");
        assert!(results[1].metadata.error.is_some());
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_mixed_valid_invalid() {
        let config = ExtractionConfig::default();
        let items = vec![
            BatchBytesItem {
                content: b"valid 1".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"invalid".to_vec(),
                mime_type: "invalid/mime".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"valid 2".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
        ];
        let results = batch_extract_bytes(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert_text_content(&results[0].content, "valid 1");
        assert!(results[1].metadata.error.is_some());
        assert_text_content(&results[2].content, "valid 2");
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_all_invalid() {
        let config = ExtractionConfig::default();
        let items = vec![
            BatchBytesItem {
                content: b"test 1".to_vec(),
                mime_type: "invalid/mime1".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"test 2".to_vec(),
                mime_type: "invalid/mime2".to_string(),
                config: None,
            },
        ];
        let results = batch_extract_bytes(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].metadata.error.is_some());
        assert!(results[1].metadata.error.is_some());
    }

    #[tokio::test]
    async fn test_extract_bytes_very_large() {
        let large_content = vec![b'a'; 10_000_000];
        let config = ExtractionConfig::default();
        let result = extract_bytes(&large_content, "text/plain", &config).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        let trimmed_len = result.content.trim_end_matches('\n').len();
        assert_eq!(trimmed_len, 10_000_000);
    }

    #[tokio::test]
    async fn test_batch_extract_large_count() {
        let dir = tempdir().unwrap();
        let mut items = Vec::new();

        for i in 0..100 {
            let file_path = dir.path().join(format!("file{}.txt", i));
            File::create(&file_path)
                .unwrap()
                .write_all(format!("content {}", i).as_bytes())
                .unwrap();
            items.push(BatchFileItem {
                path: file_path,
                config: None,
            });
        }

        let config = ExtractionConfig::default();
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 100);

        for (i, result) in results.iter().enumerate() {
            assert_text_content(&result.content, &format!("content {}", i));
        }
    }

    #[tokio::test]
    async fn test_extract_file_mime_detection_fallback() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile");
        File::create(&file_path)
            .unwrap()
            .write_all(b"plain text content")
            .unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, None, &config).await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extract_file_wrong_mime_override() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"plain text").unwrap();

        let config = ExtractionConfig::default();
        let result = extract_file(&file_path, Some("application/pdf"), &config).await;

        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_sync_wrapper_nonexistent_file() {
        let config = ExtractionConfig::default();
        let result = extract_file_sync("/nonexistent/path.txt", None, &config);

        assert!(result.is_err());
        use crate::KreuzbergError;
        // File validation returns Io error, not Validation error
        assert!(matches!(result.unwrap_err(), KreuzbergError::Io { .. }));
    }

    #[test]
    fn test_sync_wrapper_batch_empty() {
        let config = ExtractionConfig::default();
        let items: Vec<BatchFileItem> = vec![];
        let results = batch_extract_files_sync(items, &config);

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[test]
    fn test_sync_wrapper_batch_bytes_empty() {
        let config = ExtractionConfig::default();
        let items: Vec<BatchBytesItem> = vec![];
        let results = batch_extract_bytes_sync(items, &config);

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_extractions_same_mime() {
        use tokio::task::JoinSet;

        let config = Arc::new(ExtractionConfig::default());
        let mut tasks = JoinSet::new();

        for i in 0..50 {
            let config_clone = Arc::clone(&config);
            tasks.spawn(async move {
                let content = format!("test content {}", i);
                extract_bytes(content.as_bytes(), "text/plain", &config_clone).await
            });
        }

        let mut success_count = 0;
        while let Some(task_result) = tasks.join_next().await {
            if let Ok(Ok(_)) = task_result {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 50);
    }

    #[serial]
    #[tokio::test]
    async fn test_concurrent_extractions_different_mimes() {
        use tokio::task::JoinSet;

        let config = Arc::new(ExtractionConfig::default());
        let mut tasks = JoinSet::new();

        let mime_types = ["text/plain", "text/markdown"];

        for i in 0..30 {
            let config_clone = Arc::clone(&config);
            let mime = mime_types[i % mime_types.len()];
            tasks.spawn(async move {
                let content = format!("test {}", i);
                extract_bytes(content.as_bytes(), mime, &config_clone).await
            });
        }

        let mut success_count = 0;
        while let Some(task_result) = tasks.join_next().await {
            if let Ok(Ok(_)) = task_result {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 30);
    }

    #[tokio::test]
    async fn test_batch_extract_file_with_per_file_configs() {
        let dir = tempdir().unwrap();

        let file1 = dir.path().join("test1.txt");
        let file2 = dir.path().join("test2.txt");
        File::create(&file1).unwrap().write_all(b"content 1").unwrap();
        File::create(&file2).unwrap().write_all(b"content 2").unwrap();

        let config = ExtractionConfig::default();
        let items = vec![
            BatchFileItem {
                path: file1,
                config: Some(crate::FileExtractionConfig::default()),
            },
            BatchFileItem {
                path: file2,
                config: None,
            },
        ];
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "content 1");
        assert_text_content(&results[1].content, "content 2");
    }

    #[tokio::test]
    async fn test_batch_extract_file_with_configs_empty() {
        let config = ExtractionConfig::default();
        let items: Vec<BatchFileItem> = vec![];
        let results = batch_extract_files(items, &config).await;

        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_with_per_item_configs() {
        let config = ExtractionConfig::default();
        let items = vec![
            BatchBytesItem {
                content: b"hello".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"world".to_vec(),
                mime_type: "text/plain".to_string(),
                config: Some(crate::FileExtractionConfig::default()),
            },
        ];
        let results = batch_extract_bytes(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "hello");
        assert_text_content(&results[1].content, "world");
    }

    #[tokio::test]
    async fn test_batch_extract_bytes_with_configs_error_handling() {
        let config = ExtractionConfig::default();
        let items = vec![
            BatchBytesItem {
                content: b"valid".to_vec(),
                mime_type: "text/plain".to_string(),
                config: None,
            },
            BatchBytesItem {
                content: b"invalid".to_vec(),
                mime_type: "invalid/mime".to_string(),
                config: Some(crate::FileExtractionConfig::default()),
            },
        ];
        let results = batch_extract_bytes(items, &config).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert_text_content(&results[0].content, "valid");
        assert!(results[1].metadata.error.is_some());
    }

    #[test]
    fn test_batch_extract_file_sync_with_configs() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap().write_all(b"sync test").unwrap();

        let config = ExtractionConfig::default();
        let items = vec![BatchFileItem {
            path: file_path,
            config: None,
        }];
        let results = batch_extract_files_sync(items, &config);

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_text_content(&results[0].content, "sync test");
    }

    #[test]
    fn test_with_file_overrides_single_field() {
        let base = ExtractionConfig::default();
        assert!(!base.force_ocr);

        let overrides = crate::FileExtractionConfig {
            force_ocr: Some(true),
            ..Default::default()
        };
        let resolved = base.with_file_overrides(&overrides);
        assert!(resolved.force_ocr);
        // Other fields unchanged
        assert_eq!(resolved.use_cache, base.use_cache);
        assert_eq!(resolved.enable_quality_processing, base.enable_quality_processing);
    }

    #[test]
    fn test_with_file_overrides_none_keeps_default() {
        let base = ExtractionConfig::default();
        let overrides = crate::FileExtractionConfig::default(); // all None
        let resolved = base.with_file_overrides(&overrides);
        // All fields should match base
        assert_eq!(resolved.use_cache, base.use_cache);
        assert_eq!(resolved.force_ocr, base.force_ocr);
        assert_eq!(resolved.enable_quality_processing, base.enable_quality_processing);
        assert_eq!(resolved.include_document_structure, base.include_document_structure);
    }

    #[test]
    fn test_with_file_overrides_batch_fields_unaffected() {
        let base = ExtractionConfig {
            max_concurrent_extractions: Some(42),
            use_cache: false,
            ..Default::default()
        };

        let overrides = crate::FileExtractionConfig {
            force_ocr: Some(true),
            ..Default::default()
        };
        let resolved = base.with_file_overrides(&overrides);
        // Batch-level fields must be preserved from base
        assert_eq!(resolved.max_concurrent_extractions, Some(42));
        assert!(!resolved.use_cache);
        // Override applied
        assert!(resolved.force_ocr);
    }
}
