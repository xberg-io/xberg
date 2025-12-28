//! Helper functions for reducing DRY violations in type conversions
//!
//! This module provides reusable patterns and helpers for converting between
//! Rust types and Elixir terms. While the main extraction result conversion
//! happens in lib.rs (using JSON serialization for simplicity), this module
//! provides infrastructure for future direct field-by-field conversions.

use rustler::{Encoder, Env, Error, NifResult, Term};
use std::collections::{BTreeMap, HashMap};

// =============================================================================
// CONSTANTS - Size and recursion limits
// =============================================================================

/// Maximum recursion depth for extraction results (e.g., OCR in images)
/// Prevents stack overflow from deeply nested OCR results in images
#[allow(dead_code)]
const MAX_RECURSION_DEPTH: usize = 10;

/// Maximum binary size (100 MB) to prevent excessive memory allocation
#[allow(dead_code)]
const MAX_BINARY_SIZE: usize = 100 * 1024 * 1024;

/// Maximum number of tables per extraction to prevent resource exhaustion
#[allow(dead_code)]
const MAX_TABLES: usize = 10_000;

/// Maximum number of images per extraction to prevent resource exhaustion
#[allow(dead_code)]
const MAX_IMAGES: usize = 10_000;

/// Maximum number of chunks per extraction to prevent memory issues
#[allow(dead_code)]
const MAX_CHUNKS: usize = 100_000;

/// Maximum number of pages per extraction to prevent memory issues
#[allow(dead_code)]
const MAX_PAGES: usize = 100_000;

// =============================================================================
// HELPER STRUCTS - Map building with builder pattern
// =============================================================================

/// Builder pattern for constructing Elixir maps with reduced boilerplate.
///
/// Provides a fluent API for building maps with optional fields and type conversions.
/// Reduces repeated `map_put` chains and error handling.
///
/// This builder is useful for direct field-by-field construction of maps when
/// working with complex nested structures.
///
/// # Example
///
/// ```ignore
/// let map = MapBuilder::new(env)
///     .put(atoms::name(), "John")
///     .put_if(true, atoms::age(), 30u64)
///     .put_usize(atoms::count(), item_count)
///     .build()?;
/// ```
#[allow(dead_code)]
pub struct MapBuilder<'a> {
    env: Env<'a>,
    map: Term<'a>,
}

#[allow(dead_code)]
impl<'a> MapBuilder<'a> {
    /// Create a new empty map builder.
    pub fn new(env: Env<'a>) -> Self {
        Self {
            env,
            map: rustler::types::map::map_new(env),
        }
    }

    /// Put a key-value pair into the map.
    ///
    /// The value must implement `Encoder`. Key can be an Atom or any encodable term.
    /// Returns `self` for method chaining.
    pub fn put<K: Encoder, T: Encoder>(mut self, key: K, value: T) -> NifResult<Self> {
        self.map = self
            .map
            .map_put(key.encode(self.env), value.encode(self.env))?;
        Ok(self)
    }

    /// Put a key-value pair only if the condition is true.
    ///
    /// Useful for optional fields that need encoding.
    pub fn put_if<K: Encoder, T: Encoder>(
        mut self,
        condition: bool,
        key: K,
        value: T,
    ) -> NifResult<Self> {
        if condition {
            self.map = self
                .map
                .map_put(key.encode(self.env), value.encode(self.env))?;
        }
        Ok(self)
    }

    /// Put a usize value as a u64 to avoid overflow.
    ///
    /// Automatically casts usize to u64 for encoding, preventing integer overflow
    /// issues when converting Rust usize to Elixir integers.
    pub fn put_usize<K: Encoder>(mut self, key: K, value: usize) -> NifResult<Self> {
        self.map = self.map.map_put(
            key.encode(self.env),
            (value as u64).encode(self.env),
        )?;
        Ok(self)
    }

    /// Consume the builder and return the final map term.
    pub fn build(self) -> NifResult<Term<'a>> {
        Ok(self.map)
    }
}

// =============================================================================
// COLLECTION CONVERSION HELPERS
// =============================================================================

/// Convert a HashMap to an Elixir map term.
///
/// Iterates through key-value pairs and builds a map, handling encoding.
/// Returns error if map building fails.
///
/// # Arguments
///
/// * `env` - Rustler environment for term allocation
/// * `map` - HashMap with String keys and values to convert
///
/// # Returns
///
/// Term representing an Elixir map or an error
#[allow(dead_code)]
pub fn hashmap_to_term<'a>(
    env: Env<'a>,
    map: &HashMap<String, String>,
) -> NifResult<Term<'a>> {
    map.iter().try_fold(rustler::types::map::map_new(env), |m, (k, v)| {
        m.map_put(k.encode(env), v.encode(env))
    })
}

/// Convert a BTreeMap to an Elixir map term.
///
/// Iterates through key-value pairs and builds a map, handling encoding.
/// Returns error if map building fails.
///
/// BTreeMap is often used for ordered string-to-string mappings (e.g., metadata).
///
/// # Arguments
///
/// * `env` - Rustler environment for term allocation
/// * `map` - BTreeMap with String keys and values to convert
///
/// # Returns
///
/// Term representing an Elixir map or an error
#[allow(dead_code)]
pub fn btreemap_to_term<'a>(
    env: Env<'a>,
    map: &BTreeMap<String, String>,
) -> NifResult<Term<'a>> {
    map.iter().try_fold(rustler::types::map::map_new(env), |m, (k, v)| {
        m.map_put(k.encode(env), v.encode(env))
    })
}

// =============================================================================
// ENCODING HELPERS
// =============================================================================

/// Encode a usize value as a Term, converting to u64 to prevent overflow.
///
/// This is a convenience wrapper that handles the cast and encoding in one step.
/// Useful in contexts where `Encoder` trait cannot be directly applied to usize.
///
/// # Arguments
///
/// * `env` - Rustler environment for term encoding
/// * `value` - usize value to encode
///
/// # Returns
///
/// Term representing the encoded u64 value
#[allow(dead_code)]
pub fn encode_usize<'a>(env: Env<'a>, value: usize) -> Term<'a> {
    (value as u64).encode(env)
}

// =============================================================================
// DEPTH-AWARE CONVERSION FUNCTIONS (Future use)
// =============================================================================

/// Convert an ExtractionResult to an Elixir map term with depth tracking.
///
/// Tracks recursion depth to prevent stack overflow from deeply nested OCR results.
/// Returns an error if MAX_RECURSION_DEPTH is exceeded.
///
/// # Note
///
/// This function is designed for future use when implementing direct field-by-field
/// conversions for ExtractionResult. Currently, conversion happens via JSON
/// serialization in lib.rs.
///
/// # Arguments
///
/// * `env` - Rustler environment for term allocation
/// * `result` - ExtractionResult to convert
/// * `depth` - Current recursion depth
///
/// # Returns
///
/// Term representing the extraction result or an error
#[allow(dead_code)]
pub fn extraction_result_to_term_with_depth<'a>(
    _env: Env<'a>,
    _result: &kreuzberg::types::ExtractionResult,
    depth: usize,
) -> NifResult<Term<'a>> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(Error::BadArg);
    }
    // Placeholder - actual implementation would go here
    Err(Error::BadArg)
}

/// Convert an ExtractedImage to an Elixir map term with depth tracking.
///
/// Tracks recursion depth to prevent stack overflow from deeply nested OCR results
/// in image analysis.
///
/// # Note
///
/// This function is designed for future use. Currently, image conversion is
/// handled through JSON serialization in lib.rs.
///
/// # Arguments
///
/// * `env` - Rustler environment for term allocation
/// * `image` - ExtractedImage to convert
/// * `depth` - Current recursion depth
///
/// # Returns
///
/// Term representing the extracted image or an error
#[allow(dead_code)]
pub fn extracted_image_to_term_with_depth<'a>(
    _env: Env<'a>,
    _image: &kreuzberg::types::ExtractedImage,
    depth: usize,
) -> NifResult<Term<'a>> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(Error::BadArg);
    }
    // Placeholder - actual implementation would go here
    Err(Error::BadArg)
}

// =============================================================================
// VALIDATION HELPERS
// =============================================================================

/// Validate that a binary is not too large.
///
/// Prevents allocation of excessively large binary objects that could
/// exhaust system memory.
///
/// # Arguments
///
/// * `size` - Size of the binary in bytes
///
/// # Returns
///
/// Ok if valid, Err(BadArg) if too large
#[allow(dead_code)]
pub fn validate_binary_size(size: usize) -> NifResult<()> {
    if size > MAX_BINARY_SIZE {
        Err(Error::BadArg)
    } else {
        Ok(())
    }
}

/// Validate collection sizes before allocation.
///
/// Prevents allocation of collections that exceed resource limits,
/// protecting against DoS attacks via malformed input.
///
/// # Arguments
///
/// * `tables_len` - Number of tables in extraction
/// * `images_len` - Number of images in extraction
/// * `chunks_len` - Number of chunks in extraction
/// * `pages_len` - Number of pages in extraction
///
/// # Returns
///
/// Ok if all sizes valid, Err(BadArg) if any size exceeds limit
#[allow(dead_code)]
pub fn validate_collection_sizes(
    tables_len: usize,
    images_len: usize,
    chunks_len: usize,
    pages_len: usize,
) -> NifResult<()> {
    if tables_len > MAX_TABLES
        || images_len > MAX_IMAGES
        || chunks_len > MAX_CHUNKS
        || pages_len > MAX_PAGES
    {
        Err(Error::BadArg)
    } else {
        Ok(())
    }
}
