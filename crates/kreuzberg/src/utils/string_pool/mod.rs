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
//! Interned strings are wrapped in `Arc<str>` for cheap cloning and shared ownership
//! across threads.
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

mod interned;

// Re-export public types and functions
pub use interned::InternedString;
