//! Plugin registration functions for PHP-Rust FFI bridge.
//!
//! Allows PHP-based plugins (OCR backends, PostProcessors, Validators) to register with the Rust core
//! and be used during the extraction pipeline.
//!
//! # Architecture
//!
//! This module provides the FFI bridge that enables:
//! - **PHP OCR backends** (custom OCR implementations) to be used by Rust extraction
//! - **PHP PostProcessors** to enrich extraction results with metadata, keywords, entities, etc.
//! - **PHP Validators** to validate extraction results and enforce quality standards
//!
//! # OCR Backend Plugin System
//!
//! PHP OCR backends can be registered to process images and extract text.
//! OCR backends receive image data and a language code, and must return an ExtractionResult
//! with extracted text, metadata, and optional tables.
//!
//! # PostProcessor Plugin System
//!
//! PHP post-processors can be registered to process extraction results after extraction.
//! Post-processors receive an ExtractionResult and must return a modified ExtractionResult.
//! They can add metadata, transform content, extract entities, etc.
//!
//! # Validator Plugin System
//!
//! PHP validators can be registered to validate extraction results after extraction
//! but before returning to the user. Validators receive an ExtractionResult and can:
//! - Return `true` to indicate validation passed
//! - Return `false` to indicate validation failed
//! - Throw a `ValidationError` exception with details about the failure
//!
//! All registered validators must pass for extraction to succeed. If any validator
//! fails, the extraction fails with a validation error.

use ext_php_rs::convert::IntoZvalDyn;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::types::ExtractionResult;

thread_local! {
    /// Global storage for PHP OCR backend callbacks.
    ///
    /// Maps backend name -> (callback, supported_languages)
    ///
    /// # Note
    ///
    /// PHP is single-threaded, so we use thread_local! storage.
    static OCR_BACKEND_CALLBACKS: RefCell<HashMap<String, (Zval, Vec<String>)>> =
        RefCell::new(HashMap::new());

    /// Global registry of PHP post-processor callbacks.
    ///
    /// Maps post-processor name -> PHP callable (Zval).
    ///
    /// # Note
    ///
    /// Unlike validators which can be called directly, post-processors in PHP
    /// work differently than in Rust/Python. They don't integrate with the Rust
    /// post-processor registry because Zval is not Send+Sync. Instead, they must
    /// be called manually using `kreuzberg_run_post_processors()`.
    static POST_PROCESSOR_REGISTRY: RefCell<HashMap<String, Zval>> =
        RefCell::new(HashMap::new());

    /// Global registry of PHP validator callbacks.
    ///
    /// Maps validator name -> PHP callable (Zval).
    static VALIDATOR_REGISTRY: RefCell<HashMap<String, Zval>> =
        RefCell::new(HashMap::new());

    /// Global registry of PHP custom extractor callbacks.
    ///
    /// Maps MIME type -> PHP callable (Zval).
    ///
    /// # Note
    ///
    /// PHP is single-threaded, so we use thread_local! storage.
    /// Unlike Rust/Python extractors which use Arc<RwLock<>>, PHP extractors
    /// cannot be Send+Sync due to Zval constraints.
    static EXTRACTOR_REGISTRY: RefCell<HashMap<String, Zval>> =
        RefCell::new(HashMap::new());
}

/// Register a PHP post-processor callback.
///
/// Post-processors are called manually after extraction to enrich the result with additional
/// metadata, keywords, entities, or other transformations. They receive an ExtractionResult
/// and must return a modified ExtractionResult.
///
/// # Important
///
/// Unlike Python/Rust post-processors which are integrated into the extraction pipeline,
/// PHP post-processors must be called explicitly using `kreuzberg_run_post_processors()`
/// after extraction. This is because PHP callbacks (Zval) cannot be safely shared across
/// threads, which is required for integration with the Rust extraction pipeline.
///
/// # Parameters
///
/// - `name` (string): Unique post-processor name
/// - `callback` (callable): PHP callable that accepts and returns an ExtractionResult
///
/// # Returns
///
/// `null` on success
///
/// # Throws
///
/// Exception if:
/// - Post-processor name is empty
/// - Post-processor name already registered
/// - Callback is not callable
///
/// # Example
///
/// ```php
/// kreuzberg_register_post_processor('add_word_count', function($result) {
///     $wordCount = str_word_count($result->content);
///     $result->metadata[] = ['word_count', (string)$wordCount];
///     return $result;
/// });
///
/// // After extraction, run post-processors
/// $result = kreuzberg_extract_file('document.pdf');
/// $result = kreuzberg_run_post_processors($result);
/// ```
#[php_function]
pub fn kreuzberg_register_post_processor(name: String, callback: &Zval) -> PhpResult<()> {
    if name.is_empty() {
        return Err(PhpException::default("Post-processor name cannot be empty".to_string()));
    }

    if !callback.is_callable() {
        return Err(PhpException::default(format!(
            "Post-processor '{}': callback must be callable",
            name
        )));
    }

    POST_PROCESSOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        if registry.contains_key(&name) {
            return Err(PhpException::default(format!(
                "Post-processor '{}' is already registered",
                name
            )));
        }

        registry.insert(name.clone(), callback.shallow_clone());
        Ok(())
    })
}

/// Unregister a post-processor by name.
///
/// Removes the post-processor from the PHP callback registry.
///
/// # Parameters
///
/// - `name` (string): Post-processor name to unregister
///
/// # Returns
///
/// `null` on success
///
/// # Throws
///
/// Exception if post-processor is not found
///
/// # Example
///
/// ```php
/// kreuzberg_unregister_post_processor('add_word_count');
/// ```
#[php_function]
pub fn kreuzberg_unregister_post_processor(name: String) -> PhpResult<()> {
    POST_PROCESSOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        if registry.remove(&name).is_none() {
            return Err(PhpException::default(format!(
                "Post-processor '{}' is not registered",
                name
            )));
        }

        Ok(())
    })
}

/// List all registered post-processor names.
///
/// Returns a list of all PHP post-processor names currently registered.
///
/// # Returns
///
/// Array of post-processor names
///
/// # Example
///
/// ```php
/// $processors = kreuzberg_list_post_processors();
/// foreach ($processors as $name) {
///     echo "Registered: $name\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_list_post_processors() -> Vec<String> {
    POST_PROCESSOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry.keys().cloned().collect()
    })
}

/// Clear all registered post-processors.
///
/// Removes all post-processors from the PHP callback registry.
/// Useful for testing or resetting state.
///
/// # Returns
///
/// `null` on success
///
/// # Example
///
/// ```php
/// // In test cleanup
/// kreuzberg_clear_post_processors();
/// ```
#[php_function]
pub fn kreuzberg_clear_post_processors() {
    POST_PROCESSOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.clear();
    })
}

/// Run all registered post-processors on an extraction result.
///
/// This function must be called explicitly after extraction to apply all
/// registered post-processors. They are executed in registration order.
///
/// # Parameters
///
/// - `result` (ExtractionResult): The extraction result to process
///
/// # Returns
///
/// The modified ExtractionResult after all post-processors have run
///
/// # Throws
///
/// Exception if any post-processor fails
///
/// # Example
///
/// ```php
/// // Register post-processors
/// kreuzberg_register_post_processor('add_word_count', function($result) {
///     $result->metadata[] = ['word_count', (string)str_word_count($result->content)];
///     return $result;
/// });
///
/// // Extract and process
/// $result = kreuzberg_extract_file('document.pdf');
/// $result = kreuzberg_run_post_processors($result);
/// ```
#[php_function]
pub fn kreuzberg_run_post_processors(result: &mut ExtractionResult) -> PhpResult<()> {
    POST_PROCESSOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();

        if registry.is_empty() {
            return Ok(());
        }

        for (name, callback) in registry.iter() {
            let args = vec![result as &dyn IntoZvalDyn];
            let modified = callback
                .try_call(args)
                .map_err(|e| PhpException::default(format!("Post-processor '{}' failed to execute: {}", name, e)))?;

            if let Some(modified_result) = modified.extract::<&ExtractionResult>() {
                *result = modified_result.clone();
            }
        }

        Ok(())
    })
}

/// Register a PHP validator callback.
///
/// Validators are called after extraction to validate the result. They receive
/// an ExtractionResult array and must return a boolean (true = valid, false = invalid)
/// or throw a ValidationError exception with details.
///
/// # Parameters
///
/// - `name` (string): Unique validator name
/// - `callback` (callable): PHP callable that accepts an ExtractionResult array
///
/// # Returns
///
/// `null` on success
///
/// # Throws
///
/// Exception if:
/// - Validator name is empty
/// - Validator name already registered
/// - Callback is not callable
///
/// # Example
///
/// ```php
/// use Kreuzberg\Plugins\ValidatorInterface;
/// use Kreuzberg\Plugins\ValidationError;
///
/// class MinLengthValidator implements ValidatorInterface {
///     public function validate(array $result): bool {
///         if (strlen($result['content']) < 100) {
///             throw new ValidationError(
///                 'Content too short: ' . strlen($result['content']) . ' < 100 characters'
///             );
///         }
///         return true;
///     }
/// }
///
/// $validator = new MinLengthValidator();
/// kreuzberg_register_validator('min_length', [$validator, 'validate']);
/// ```
#[php_function]
pub fn kreuzberg_register_validator(name: String, callback: &Zval) -> PhpResult<()> {
    if name.is_empty() {
        return Err(PhpException::default("Validator name cannot be empty".to_string()));
    }

    if !callback.is_callable() {
        return Err(PhpException::default(format!(
            "Validator '{}': callback must be callable",
            name
        )));
    }

    VALIDATOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        if registry.contains_key(&name) {
            return Err(PhpException::default(format!(
                "Validator '{}' is already registered",
                name
            )));
        }

        registry.insert(name.clone(), callback.shallow_clone());
        Ok(())
    })
}

/// Unregister a validator by name.
///
/// Removes a previously registered validator from the registry.
///
/// # Parameters
///
/// - `name` (string): Validator name to unregister
///
/// # Returns
///
/// `null` on success
///
/// # Throws
///
/// Exception if validator is not found
///
/// # Example
///
/// ```php
/// kreuzberg_unregister_validator('min_length');
/// ```
#[php_function]
pub fn kreuzberg_unregister_validator(name: String) -> PhpResult<()> {
    VALIDATOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();

        if registry.remove(&name).is_none() {
            return Err(PhpException::default(format!("Validator '{}' is not registered", name)));
        }

        Ok(())
    })
}

/// List all registered validator names.
///
/// # Returns
///
/// Array of validator names
///
/// # Example
///
/// ```php
/// $validators = kreuzberg_list_validators();
/// print_r($validators); // ["min_length", "max_size", ...]
/// ```
#[php_function]
pub fn kreuzberg_list_validators() -> Vec<String> {
    VALIDATOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry.keys().cloned().collect()
    })
}

/// Clear all registered validators.
///
/// Removes all validators from the registry. Useful for test cleanup.
///
/// # Example
///
/// ```php
/// kreuzberg_clear_validators();
/// ```
#[php_function]
pub fn kreuzberg_clear_validators() {
    VALIDATOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.clear();
    })
}

/// Run all registered validators on an extraction result.
///
/// This is called internally by the extraction pipeline. It's exposed for testing
/// but should not be called directly by users.
///
/// # Parameters
///
/// - `result` (array): ExtractionResult array to validate
///
/// # Returns
///
/// `null` if all validators pass
///
/// # Throws
///
/// Exception if any validator fails
///
/// # Internal Use Only
///
/// This function is called automatically during extraction. Users should not call it directly.
#[php_function]
pub fn kreuzberg_run_validators(result: &mut Zval) -> PhpResult<()> {
    VALIDATOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();

        if registry.is_empty() {
            return Ok(());
        }

        for (name, callback) in registry.iter() {
            let args = vec![result as &dyn IntoZvalDyn];
            let validation_result = callback
                .try_call(args)
                .map_err(|e| PhpException::default(format!("Validator '{}' failed to execute: {}", name, e)))?;

            if let Some(is_valid) = validation_result.extract::<bool>()
                && !is_valid
            {
                return Err(PhpException::default(format!(
                    "Validation failed: validator '{}' returned false",
                    name
                )));
            }
        }

        Ok(())
    })
}

/// Register a custom extractor for a specific MIME type.
///
/// Registers a PHP callable as a custom extractor for the specified MIME type.
/// Custom extractors are called before built-in extractors, allowing overrides
/// and support for proprietary formats.
///
/// # Parameters
///
/// - `mime_type` (string): MIME type to handle (e.g., "text/custom", "application/proprietary")
/// - `callback` (callable): PHP callable accepting ($bytes, $mimeType) and returning extraction result array
///
/// # Returns
///
/// `null` on success
///
/// # Throws
///
/// Exception if:
/// - MIME type is empty
/// - Callback is not callable
///
/// # Example
///
/// ```php
/// kreuzberg_register_extractor('text/custom', function($bytes, $mimeType) {
///     return [
///         'content' => strtoupper($bytes),
///         'metadata' => ['custom' => 'value'],
///         'tables' => [],
///     ];
/// });
/// ```
#[php_function]
pub fn kreuzberg_register_extractor(mime_type: String, callback: &Zval) -> PhpResult<()> {
    if mime_type.trim().is_empty() {
        return Err(PhpException::default("MIME type cannot be empty".to_string()));
    }

    if !callback.is_callable() {
        return Err(PhpException::default(format!(
            "Extractor for '{}': callback must be callable",
            mime_type
        )));
    }

    EXTRACTOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.insert(mime_type.clone(), callback.shallow_clone());
        Ok(())
    })
}

/// Unregister a custom extractor for a specific MIME type.
///
/// Removes a previously registered custom extractor. Subsequent extractions
/// of that MIME type will fall back to built-in extractors.
///
/// # Parameters
///
/// - `mime_type` (string): MIME type to unregister
///
/// # Returns
///
/// `null` on success
///
/// # Example
///
/// ```php
/// kreuzberg_unregister_extractor('text/custom');
/// ```
#[php_function]
pub fn kreuzberg_unregister_extractor(mime_type: String) -> PhpResult<()> {
    EXTRACTOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.remove(&mime_type);
        Ok(())
    })
}

/// List all registered custom extractor MIME types.
///
/// Returns an array of all MIME types with registered custom extractors.
///
/// # Returns
///
/// Array of MIME type strings
///
/// # Example
///
/// ```php
/// $mimes = kreuzberg_list_extractors();
/// foreach ($mimes as $mime) {
///     echo "Custom extractor for: $mime\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_list_extractors() -> Vec<String> {
    EXTRACTOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry.keys().cloned().collect()
    })
}

/// Clear all registered custom extractors.
///
/// Removes all custom extractors from the registry. Useful for testing
/// or resetting state.
///
/// # Example
///
/// ```php
/// kreuzberg_clear_extractors();
/// ```
#[php_function]
pub fn kreuzberg_clear_extractors() {
    EXTRACTOR_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.clear();
    })
}

/// Test a plugin with sample data.
///
/// Tests a registered extractor plugin with sample data to verify it works correctly.
///
/// # Parameters
///
/// - `plugin_type` (string): "extractor" (only extractor plugins are supported)
/// - `plugin_name` (string): MIME type of the registered extractor
/// - `test_data` (array): Array with 'bytes' and 'mime_type' keys
///
/// # Returns
///
/// `true` if the test passed
///
/// # Throws
///
/// Exception if:
/// - Plugin type is unsupported
/// - Extractor is not registered
/// - Test data is invalid
/// - Extractor returns invalid format
///
/// # Example
///
/// ```php
/// $testData = [
///     'bytes' => 'test content',
///     'mime_type' => 'text/custom',
/// ];
///
/// if (kreuzberg_test_plugin('extractor', 'text/custom', $testData)) {
///     echo "Extractor works correctly\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_test_plugin(plugin_type: String, plugin_name: String, test_data: &mut Zval) -> PhpResult<bool> {
    if plugin_type != "extractor" {
        return Err(PhpException::default(format!(
            "Unsupported plugin type: {}",
            plugin_type
        )));
    }

    EXTRACTOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();

        let callback = registry
            .get(&plugin_name)
            .ok_or_else(|| PhpException::default(format!("Extractor '{}' is not registered", plugin_name)))?;

        let test_array = test_data
            .array()
            .ok_or_else(|| PhpException::default("test_data must be an array".to_string()))?;

        let bytes_val = test_array
            .get("bytes")
            .ok_or_else(|| PhpException::default("test_data must contain 'bytes' key".to_string()))?;

        let mime_type_val = test_array
            .get("mime_type")
            .ok_or_else(|| PhpException::default("test_data must contain 'mime_type' key".to_string()))?;

        let args = vec![bytes_val as &dyn IntoZvalDyn, mime_type_val as &dyn IntoZvalDyn];

        let result = callback.try_call(args).map_err(|e| {
            PhpException::default(format!(
                "Failed to call extractor callback for '{}': {:?}",
                plugin_name, e
            ))
        })?;

        if let Some(result_array) = result.array() {
            if result_array.get("content").is_none() {
                return Err(PhpException::default(
                    "Extractor result must contain 'content' key".to_string(),
                ));
            }

            if let Some(content_val) = result_array.get("content")
                && content_val.str().is_none()
            {
                return Err(PhpException::default("'content' field must be a string".to_string()));
            }

            Ok(true)
        } else {
            Err(PhpException::default("Extractor must return an array".to_string()))
        }
    })
}

/// Internal: Check if a custom extractor is registered for a MIME type.
///
/// This is an internal function used by the extraction pipeline to determine
/// whether to use a custom extractor or fall back to built-in extractors.
pub(crate) fn has_custom_extractor(mime_type: &str) -> bool {
    EXTRACTOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        registry.contains_key(mime_type)
    })
}

/// Internal: Call a custom extractor for a MIME type.
///
/// This is an internal function used by the extraction pipeline to invoke
/// a registered custom extractor.
pub(crate) fn call_custom_extractor(mime_type: &str, bytes: &[u8]) -> PhpResult<Zval> {
    EXTRACTOR_REGISTRY.with(|registry| {
        let registry = registry.borrow();

        let callback = registry
            .get(mime_type)
            .ok_or_else(|| PhpException::default(format!("No custom extractor registered for '{}'", mime_type)))?;

        let mut bytes_zval = Zval::new();
        bytes_zval.set_binary(bytes.to_vec());

        let mut mime_zval = Zval::new();
        mime_zval.set_string(mime_type, false)?;

        let args = vec![&bytes_zval as &dyn IntoZvalDyn, &mime_zval as &dyn IntoZvalDyn];
        let result = callback
            .try_call(args)
            .map_err(|e| PhpException::default(format!("Custom extractor callback failed: {:?}", e)))?;

        Ok(result)
    })
}
