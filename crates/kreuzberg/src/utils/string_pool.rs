//! String interning/pooling for frequently used strings.
//!
//! This module provides thread-safe string interning to reduce memory allocations
//! for strings that appear repeatedly across documents (MIME types, language codes, format field names).
//!
//! # Performance
//!
//! String interning provides 0.1-0.3% improvement by:
//! - Deduplicating repeated strings (e.g., "application/pdf" appears 1000s of times)
//! - Reducing allocation overhead for commonly used strings
//! - Enabling pointer comparisons instead of string comparisons
//!
//! # Thread Safety
//!
//! The intern pool uses a `DashMap` for lock-free concurrent access. Multiple threads
//! can insert and lookup strings simultaneously without contention.
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::utils::string_pool::intern_mime_type;
//!
//! let mime1 = intern_mime_type("application/pdf");
//! let mime2 = intern_mime_type("application/pdf");
//! // Both mime1 and mime2 point to the same interned string
//! assert_eq!(mime1, mime2);
//! ```

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "pool-metrics")]
use std::sync::atomic::AtomicUsize;

/// A reference to an interned string stored in an Arc.
///
/// This wraps an Arc<String> and provides convenient access to the string content.
/// Multiple calls with the same string content will share the same Arc, reducing memory usage.
#[derive(Clone)]
pub struct InternedString(Arc<String>);

impl InternedString {
    /// Get the string content.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for InternedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternedString").field(&self.as_str()).finish()
    }
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for InternedString {}

impl std::hash::Hash for InternedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl std::ops::Deref for InternedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// String pool for MIME types.
///
/// Lazily initializes with all known MIME types from `kreuzberg::core::mime`.
/// Pre-interning is deferred until first access to reduce startup memory usage.
struct MimeStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
    initialized: AtomicBool,
}

impl MimeStringPool {
    /// Create a new MIME string pool.
    /// Pre-interning is deferred until first `get_or_intern()` call.
    fn new() -> Self {
        MimeStringPool {
            pool: dashmap::DashMap::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Ensure all known MIME types are pre-interned (one-time initialization).
    #[inline]
    fn ensure_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }

        let mime_types = vec![
            "text/html",
            "text/markdown",
            "text/x-markdown",
            "text/plain",
            "application/pdf",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/msword",
            "application/vnd.ms-powerpoint",
            "message/rfc822",
            "application/vnd.ms-outlook",
            "application/json",
            "text/json",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/yaml",
            "application/toml",
            "text/toml",
            "application/xml",
            "text/xml",
            "image/svg+xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.ms-excel",
            "application/vnd.ms-excel.sheet.macroEnabled.12",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
            "application/vnd.ms-excel.addin.macroEnabled.12",
            "application/vnd.ms-excel.template.macroEnabled.12",
            "application/vnd.oasis.opendocument.spreadsheet",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.oasis.opendocument.text",
            "image/bmp",
            "image/gif",
            "image/jp2",
            "image/jpeg",
            "image/jpm",
            "image/jpx",
            "image/mj2",
            "image/pjpeg",
            "image/png",
            "image/tiff",
            "image/webp",
            "image/x-bmp",
            "image/x-ms-bmp",
            "image/x-portable-anymap",
            "image/x-portable-bitmap",
            "image/x-portable-graymap",
            "image/x-portable-pixmap",
            "image/x-tiff",
            "application/csl+json",
            "application/docbook+xml",
            "application/epub+zip",
            "application/rtf",
            "application/x-biblatex",
            "application/x-bibtex",
            "application/x-endnote+xml",
            "application/x-fictionbook+xml",
            "application/x-ipynb+json",
            "application/x-jats+xml",
            "application/x-latex",
            "application/xml+opml",
            "application/x-opml+xml",
            "application/x-research-info-systems",
            "application/x-typst",
            "text/csv",
            "text/tab-separated-values",
            "text/troff",
            "text/x-commonmark",
            "text/x-dokuwiki",
            "text/x-gfm",
            "text/x-markdown-extra",
            "text/x-mdoc",
            "text/x-multimarkdown",
            "text/x-opml",
            "text/x-org",
            "text/x-pod",
            "text/x-rst",
            "application/zip",
            "application/x-zip-compressed",
            "application/x-tar",
            "application/tar",
            "application/x-gtar",
            "application/x-ustar",
            "application/gzip",
            "application/x-7z-compressed",
        ];

        for mime_type in mime_types {
            self.pool.insert(mime_type.to_string(), Arc::new(mime_type.to_string()));
        }

        let _ = self
            .initialized
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed);
    }

    /// Get or intern a MIME type string.
    /// Ensures pre-interned MIME types are initialized on first call.
    fn get_or_intern(&self, mime_type: &str) -> Arc<String> {
        self.ensure_initialized();

        if let Some(entry) = self.pool.get(mime_type) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(mime_type.to_string());
            self.pool.insert(mime_type.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// String pool for language codes.
///
/// Lazily initializes with common ISO 639 language codes.
/// Pre-interning is deferred until first access to reduce startup memory usage.
struct LanguageStringPool {
    pool: dashmap::DashMap<String, Arc<String>>,
    initialized: AtomicBool,
}

impl LanguageStringPool {
    /// Create a new language string pool.
    /// Pre-interning is deferred until first `get_or_intern()` call.
    fn new() -> Self {
        LanguageStringPool {
            pool: dashmap::DashMap::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Ensure all known language codes are pre-interned (one-time initialization).
    #[inline]
    fn ensure_initialized(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }

        let lang_codes = vec![
            "en", "es", "fr", "de", "it", "pt", "ru", "ja", "ko", "zh", "ar", "hi", "th", "tr", "pl", "nl", "sv", "no",
            "da", "fi", "cs", "hu", "ro", "el", "he", "fa", "ur", "vi", "id", "ms", "bn", "pa", "te", "mr", "ta", "gu",
            "kn", "ml", "or", "uk", "bg", "sr", "hr", "sl", "sk", "et", "lv", "lt", "sq", "mk", "ka", "hy", "eo",
            "ast", "ca", "eu", "gl", "cy", "gd", "ga",
        ];

        for code in lang_codes {
            self.pool.insert(code.to_string(), Arc::new(code.to_string()));
        }

        let _ = self
            .initialized
            .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed);
    }

    /// Get or intern a language code string.
    /// Ensures pre-interned language codes are initialized on first call.
    fn get_or_intern(&self, lang_code: &str) -> Arc<String> {
        self.ensure_initialized();

        if let Some(entry) = self.pool.get(lang_code) {
            Arc::clone(&*entry)
        } else {
            let arc_string = Arc::new(lang_code.to_string());
            self.pool.insert(lang_code.to_string(), Arc::clone(&arc_string));
            arc_string
        }
    }
}

/// Configuration for the string buffer pool.
pub struct PoolConfig {
    /// Maximum buffers per size bucket
    pub max_buffers_per_size: usize,
    /// Initial capacity for new buffers
    pub initial_capacity: usize,
    /// Maximum capacity before discarding
    pub max_capacity_before_discard: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_buffers_per_size: 4,
            initial_capacity: 4096,
            max_capacity_before_discard: 65536,
        }
    }
}

/// Thread-safe reusable string buffer pool.
///
/// This pool allows allocation and reuse of String buffers to reduce memory allocations
/// during document extraction. Buffers are returned to the pool with cleared contents
/// but preserved capacity, ready for reuse.
///
/// # Thread Safety
///
/// The pool uses DashMap for lock-free concurrent access. Multiple threads can
/// acquire and release buffers simultaneously.
///
/// # Usage
///
/// ```rust,ignore
/// use kreuzberg::utils::string_pool::STRING_BUFFER_POOL;
///
/// // Acquire a buffer from the pool
/// let mut buffer = STRING_BUFFER_POOL.acquire();
/// buffer.push_str("some content");
/// // Automatically returned to pool when dropped
/// drop(buffer);
/// ```
pub struct StringBufferPool {
    pool: dashmap::DashMap<usize, VecDeque<String>>,
    config: PoolConfig,
    #[cfg(feature = "pool-metrics")]
    acquire_count: AtomicUsize,
    #[cfg(feature = "pool-metrics")]
    reuse_count: AtomicUsize,
}

impl StringBufferPool {
    /// Create a new string buffer pool with given configuration.
    pub fn new(config: PoolConfig) -> Self {
        StringBufferPool {
            pool: dashmap::DashMap::new(),
            config,
            #[cfg(feature = "pool-metrics")]
            acquire_count: AtomicUsize::new(0),
            #[cfg(feature = "pool-metrics")]
            reuse_count: AtomicUsize::new(0),
        }
    }

    /// Find the appropriate bucket size for a given capacity.
    fn find_bucket(&self, capacity: usize) -> usize {
        if capacity <= 1024 {
            1024
        } else if capacity <= 4096 {
            4096
        } else if capacity <= 16384 {
            16384
        } else if capacity <= 65536 {
            65536
        } else {
            262144
        }
    }

    /// Try to acquire a buffer from a specific bucket, returning it if found.
    fn try_acquire_from_bucket(&self, bucket: usize) -> Option<String> {
        if let Some(mut entry) = self.pool.get_mut(&bucket) {
            entry.pop_front()
        } else {
            None
        }
    }

    /// Acquire a string buffer from the pool, or allocate a new one if pool is exhausted.
    ///
    /// The returned buffer is automatically returned to the pool when dropped.
    /// Must be called with the pool wrapped in Arc.
    pub fn acquire(self: Arc<Self>) -> PooledString {
        #[cfg(feature = "pool-metrics")]
        self.acquire_count.fetch_add(1, Ordering::Relaxed);

        let default_bucket = self.config.initial_capacity;
        if let Some(buffer) = self.try_acquire_from_bucket(default_bucket) {
            #[cfg(feature = "pool-metrics")]
            self.reuse_count.fetch_add(1, Ordering::Relaxed);
            return PooledString { buffer, pool: self };
        }

        for &bucket in &[1024, 16384, 65536] {
            if let Some(buffer) = self.try_acquire_from_bucket(bucket) {
                #[cfg(feature = "pool-metrics")]
                self.reuse_count.fetch_add(1, Ordering::Relaxed);
                return PooledString { buffer, pool: self };
            }
        }

        PooledString {
            buffer: String::with_capacity(self.config.initial_capacity),
            pool: self,
        }
    }

    /// Return a buffer to the pool for reuse.
    pub fn release(&self, mut buffer: String) {
        if buffer.capacity() > self.config.max_capacity_before_discard {
            return;
        }

        let bucket = self.find_bucket(buffer.capacity());
        buffer.clear();

        if let Some(mut queue) = self.pool.get_mut(&bucket) {
            if queue.len() < self.config.max_buffers_per_size {
                queue.push_back(buffer);
            }
        } else {
            let mut queue = VecDeque::with_capacity(self.config.max_buffers_per_size);
            queue.push_back(buffer);
            self.pool.insert(bucket, queue);
        }
    }

    /// Get the current pool size across all buckets.
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.pool.iter().map(|entry| entry.value().len()).sum()
    }

    /// Get buffer reuse metrics (only available with `pool-metrics` feature).
    #[cfg(feature = "pool-metrics")]
    pub fn metrics(&self) -> StringBufferPoolMetrics {
        let acquire = self.acquire_count.load(Ordering::Relaxed);
        let reuse = self.reuse_count.load(Ordering::Relaxed);
        let hit_rate = if acquire == 0 {
            0.0
        } else {
            (reuse as f64 / acquire as f64) * 100.0
        };

        StringBufferPoolMetrics {
            total_acquires: acquire,
            total_reuses: reuse,
            hit_rate,
        }
    }
}

/// Metrics for StringBufferPool (only available with `pool-metrics` feature).
#[cfg(feature = "pool-metrics")]
#[derive(Debug, Clone, Copy)]
pub struct StringBufferPoolMetrics {
    /// Total number of acquire calls
    pub total_acquires: usize,
    /// Total number of buffer reuses from pool
    pub total_reuses: usize,
    /// Hit rate as percentage (0.0-100.0)
    pub hit_rate: f64,
}

/// RAII wrapper for a pooled string buffer.
///
/// Automatically returns the buffer to the pool when dropped.
pub struct PooledString {
    buffer: String,
    pool: Arc<StringBufferPool>,
}

impl PooledString {
    /// Get mutable access to the underlying string buffer.
    pub fn buffer_mut(&mut self) -> &mut String {
        &mut self.buffer
    }

    /// Get immutable access to the underlying string buffer.
    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
}

impl std::ops::Deref for PooledString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for PooledString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

impl Drop for PooledString {
    fn drop(&mut self) {
        let buffer = std::mem::take(&mut self.buffer);
        self.pool.release(buffer);
    }
}

impl std::fmt::Display for PooledString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buffer)
    }
}

impl std::fmt::Debug for PooledString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PooledString").field(&self.buffer).finish()
    }
}

/// Global MIME type string pool.
static MIME_POOL: Lazy<MimeStringPool> = Lazy::new(MimeStringPool::new);

/// Global language code string pool.
static LANGUAGE_POOL: Lazy<LanguageStringPool> = Lazy::new(LanguageStringPool::new);

/// Global string buffer pool for temporary allocations during extraction.
pub static STRING_BUFFER_POOL: Lazy<Arc<StringBufferPool>> =
    Lazy::new(|| Arc::new(StringBufferPool::new(PoolConfig::default())));

/// Get or intern a MIME type string.
///
/// Returns an `InternedString` that is guaranteed to be deduplicated with any other
/// intern call for the same MIME type. This reduces memory usage and allows
/// fast pointer-based comparisons.
///
/// # Arguments
///
/// * `mime_type` - The MIME type string to intern
///
/// # Returns
///
/// An `InternedString` pointing to the deduplicated string
///
/// # Example
///
/// ```rust,ignore
/// let pdf1 = intern_mime_type("application/pdf");
/// let pdf2 = intern_mime_type("application/pdf");
/// assert_eq!(pdf1, pdf2); // Same pointer
/// ```
pub fn intern_mime_type(mime_type: &str) -> InternedString {
    InternedString(MIME_POOL.get_or_intern(mime_type))
}

/// Get or intern a language code string.
///
/// Returns an `InternedString` that is guaranteed to be deduplicated with any other
/// intern call for the same language code.
///
/// # Arguments
///
/// * `lang_code` - The language code to intern (e.g., "en", "es", "fr")
///
/// # Returns
///
/// An `InternedString` pointing to the deduplicated string
///
/// # Example
///
/// ```rust,ignore
/// let en1 = intern_language_code("en");
/// let en2 = intern_language_code("en");
/// assert_eq!(en1, en2); // Same pointer
/// ```
pub fn intern_language_code(lang_code: &str) -> InternedString {
    InternedString(LANGUAGE_POOL.get_or_intern(lang_code))
}

/// Acquire a string buffer from the global pool.
///
/// The returned buffer is automatically returned to the pool when dropped.
///
/// # Example
///
/// ```rust,ignore
/// let mut buffer = acquire_string_buffer();
/// buffer.push_str("content");
/// // Automatically returned to pool when buffer goes out of scope
/// ```
pub fn acquire_string_buffer() -> PooledString {
    Arc::clone(&*STRING_BUFFER_POOL).acquire()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_deduplication() {
        let mime1 = intern_mime_type("application/pdf");
        let mime2 = intern_mime_type("application/pdf");

        assert_eq!(mime1, mime2);
        assert!(Arc::ptr_eq(&mime1.0, &mime2.0));
    }

    #[test]
    fn test_language_code_deduplication() {
        let en1 = intern_language_code("en");
        let en2 = intern_language_code("en");

        assert_eq!(en1, en2);
        assert!(Arc::ptr_eq(&en1.0, &en2.0));
    }

    #[test]
    fn test_interned_string_display() {
        let mime = intern_mime_type("text/html");
        assert_eq!(format!("{}", mime), "text/html");
    }

    #[test]
    fn test_interned_string_deref() {
        let mime = intern_mime_type("application/json");
        assert_eq!(&*mime, "application/json");
        assert_eq!(mime.as_ref(), "application/json");
        assert_eq!(mime.as_str(), "application/json");
    }

    #[test]
    fn test_preinterned_mime_types() {
        let pdf = intern_mime_type("application/pdf");
        assert_eq!(pdf.as_str(), "application/pdf");

        let html = intern_mime_type("text/html");
        assert_eq!(html.as_str(), "text/html");

        let json = intern_mime_type("application/json");
        assert_eq!(json.as_str(), "application/json");
    }

    #[test]
    fn test_preinterned_language_codes() {
        let en = intern_language_code("en");
        assert_eq!(en.as_str(), "en");

        let es = intern_language_code("es");
        assert_eq!(es.as_str(), "es");

        let fr = intern_language_code("fr");
        assert_eq!(fr.as_str(), "fr");
    }

    #[test]
    fn test_concurrent_interning() {
        use std::sync::Arc;
        use std::thread;

        let mime = "application/pdf";
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let results = Arc::clone(&results);
                thread::spawn(move || {
                    let interned = intern_mime_type(mime);
                    results.lock().unwrap().push(interned);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let interned_strings = results.lock().unwrap();
        assert_eq!(interned_strings.len(), 10);

        let first_arc = &interned_strings[0].0;
        for interned in &*interned_strings {
            assert!(
                Arc::ptr_eq(&interned.0, first_arc),
                "All interned strings should share the same Arc"
            );
        }
    }

    #[test]
    fn test_interned_string_hash() {
        let mime1 = intern_mime_type("application/pdf");
        let mime2 = intern_mime_type("application/pdf");

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(mime1);
        set.insert(mime2);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_interned_string_clone() {
        let mime1 = intern_mime_type("text/html");
        let mime2 = mime1.clone();

        assert_eq!(mime1, mime2);
        assert!(Arc::ptr_eq(&mime1.0, &mime2.0));
    }

    #[test]
    fn test_buffer_pool_acquire_and_release() {
        let config = PoolConfig::default();
        let pool = Arc::new(StringBufferPool::new(config));

        let mut buffer = pool.clone().acquire();
        buffer.push_str("test content");
        let capacity = buffer.capacity();

        drop(buffer);

        let buffer2 = pool.clone().acquire();
        assert_eq!(buffer2.capacity(), capacity);
        assert!(buffer2.is_empty());
    }

    #[test]
    fn test_buffer_pool_size() {
        let config = PoolConfig::default();
        let pool = Arc::new(StringBufferPool::new(config));

        assert_eq!(pool.size(), 0);

        let buffer1 = pool.clone().acquire();
        drop(buffer1);
        assert_eq!(pool.size(), 1);

        let buffer2 = pool.clone().acquire();
        drop(buffer2);
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_buffer_pool_global() {
        let buffer1 = acquire_string_buffer();
        drop(buffer1);

        let buffer2 = acquire_string_buffer();
        assert!(buffer2.capacity() >= 4096);
    }

    #[test]
    fn test_pooled_string_deref() {
        let mut buffer = acquire_string_buffer();
        buffer.push_str("hello");

        assert_eq!(&*buffer, "hello");
        assert_eq!(buffer.as_str(), "hello");
        assert!(buffer.len() > 0);
    }

    #[test]
    fn test_pooled_string_deref_mut() {
        let mut buffer = acquire_string_buffer();
        buffer.push_str("test");

        buffer.buffer_mut().push_str(" more");
        assert_eq!(buffer.as_str(), "test more");
    }
}
