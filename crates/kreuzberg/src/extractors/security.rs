//! Security utilities for document extractors.
//!
//! This module provides validation and protection mechanisms against common attacks:
//! - ZIP bomb detection (decompression bombs)
//! - XML entity expansion limits
//! - Nesting depth limits
//! - Input size limits
//! - Entity length validation

#[cfg(feature = "archives")]
use std::io::{Read, Seek};

/// Configuration for security limits across extractors.
///
/// All limits are intentionally conservative to prevent DoS attacks
/// while still supporting legitimate documents.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SecurityLimits {
    /// Maximum uncompressed size for archives (500 MB)
    pub max_archive_size: usize,

    /// Maximum compression ratio before flagging as potential bomb (100:1)
    pub max_compression_ratio: usize,

    /// Maximum number of files in archive (10,000)
    pub max_files_in_archive: usize,

    /// Maximum nesting depth for structures (100)
    pub max_nesting_depth: usize,

    /// Maximum length of any single XML entity / attribute / token (1 MiB).
    /// This is a per-token cap, NOT a cumulative cap — billion-laughs class
    /// attacks where a single entity expands to hundreds of MB are caught
    /// here, while normal long text content (a paragraph, a CDATA block) is
    /// caught by `max_content_size` instead.
    pub max_entity_length: usize,

    /// Maximum string growth per document (100 MB)
    pub max_content_size: usize,

    /// Maximum iterations per operation
    pub max_iterations: usize,

    /// Maximum XML depth (100 levels)
    pub max_xml_depth: usize,

    /// Maximum cells per table (100,000)
    pub max_table_cells: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_archive_size: 500 * 1024 * 1024,
            max_compression_ratio: 100,
            max_files_in_archive: 10_000,
            // 1024 levels — generous headroom for legitimate DOCX/PPTX/EPUB
            // documents (deeply nested tables-in-cells, OMath expressions,
            // formatting wrappers) while still catching depth-bomb attacks
            // (those typically have 5 000+ levels).
            max_nesting_depth: 1024,
            // 1 MiB — per-token cap that catches billion-laughs entity
            // expansion (single entities ballooning to hundreds of MB) without
            // false-positiving on legitimate long attributes / CDATA blocks.
            // Cumulative content size is bounded separately by max_content_size.
            max_entity_length: 1024 * 1024,
            max_content_size: 100 * 1024 * 1024,
            max_iterations: 10_000_000,
            max_xml_depth: 1024,
            max_table_cells: 100_000,
        }
    }
}

/// Security validation errors.
#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Potential ZIP bomb detected
    ZipBombDetected {
        compressed_size: u64,
        uncompressed_size: u64,
        ratio: f64,
    },

    /// Archive exceeds maximum size
    ArchiveTooLarge { size: u64, max: usize },

    /// Archive contains too many files
    TooManyFiles { count: usize, max: usize },

    /// Nesting too deep
    NestingTooDeep { depth: usize, max: usize },

    /// Content exceeds maximum size
    ContentTooLarge { size: usize, max: usize },

    /// Entity/string too long
    EntityTooLong { length: usize, max: usize },

    /// Too many iterations
    TooManyIterations { count: usize, max: usize },

    /// XML depth exceeded
    XmlDepthExceeded { depth: usize, max: usize },

    /// Too many table cells
    TooManyCells { cells: usize, max: usize },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SecurityError::ZipBombDetected {
                compressed_size,
                uncompressed_size,
                ratio,
            } => {
                write!(
                    f,
                    "Potential ZIP bomb detected: compressed {}B -> uncompressed {}B (ratio: {:.1}:1)",
                    compressed_size, uncompressed_size, ratio
                )
            }
            SecurityError::ArchiveTooLarge { size, max } => {
                write!(f, "Archive too large: {} bytes (max: {} bytes)", size, max)
            }
            SecurityError::TooManyFiles { count, max } => {
                write!(f, "Archive has too many files: {} (max: {})", count, max)
            }
            SecurityError::NestingTooDeep { depth, max } => {
                write!(f, "Nesting too deep: {} levels (max: {})", depth, max)
            }
            SecurityError::ContentTooLarge { size, max } => {
                write!(f, "Content too large: {} bytes (max: {} bytes)", size, max)
            }
            SecurityError::EntityTooLong { length, max } => {
                write!(f, "Entity too long: {} chars (max: {})", length, max)
            }
            SecurityError::TooManyIterations { count, max } => {
                write!(f, "Too many iterations: {} (max: {})", count, max)
            }
            SecurityError::XmlDepthExceeded { depth, max } => {
                write!(f, "XML depth exceeded: {} (max: {})", depth, max)
            }
            SecurityError::TooManyCells { cells, max } => {
                write!(f, "Too many table cells: {} (max: {})", cells, max)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Helper struct for validating ZIP archives for security issues.
#[cfg(feature = "archives")]
pub struct ZipBombValidator {
    limits: SecurityLimits,
}

#[cfg(feature = "archives")]
impl ZipBombValidator {
    /// Create a new ZIP bomb validator.
    pub(crate) fn new(limits: SecurityLimits) -> Self {
        Self { limits }
    }

    /// Validate a ZIP archive for security issues.
    ///
    /// # Arguments
    /// * `archive` - Mutable ZIP archive to validate
    ///
    /// # Returns
    /// * `Ok(())` if archive is safe
    /// * `Err(SecurityError)` if security limit violated
    pub(crate) fn validate<R: Read + Seek>(&self, archive: &mut zip::ZipArchive<R>) -> Result<(), SecurityError> {
        let file_count = archive.len();

        if file_count > self.limits.max_files_in_archive {
            return Err(SecurityError::TooManyFiles {
                count: file_count,
                max: self.limits.max_files_in_archive,
            });
        }

        let mut total_uncompressed: u64 = 0;
        let mut total_compressed: u64 = 0;

        for i in 0..file_count {
            if let Ok(file) = archive.by_index(i) {
                let compressed_size = file.compressed_size();
                let uncompressed_size = file.size();

                total_uncompressed += uncompressed_size;
                total_compressed += compressed_size;

                if compressed_size > 0 && uncompressed_size > 0 {
                    let ratio = uncompressed_size as f64 / compressed_size as f64;
                    if ratio > self.limits.max_compression_ratio as f64 {
                        return Err(SecurityError::ZipBombDetected {
                            compressed_size,
                            uncompressed_size,
                            ratio,
                        });
                    }
                }
            }
        }

        if total_uncompressed > self.limits.max_archive_size as u64 {
            return Err(SecurityError::ArchiveTooLarge {
                size: total_uncompressed,
                max: self.limits.max_archive_size,
            });
        }

        if total_compressed > 0 {
            let ratio = total_uncompressed as f64 / total_compressed as f64;
            if ratio > self.limits.max_compression_ratio as f64 {
                return Err(SecurityError::ZipBombDetected {
                    compressed_size: total_compressed,
                    uncompressed_size: total_uncompressed,
                    ratio,
                });
            }
        }

        Ok(())
    }
}

/// Helper struct for tracking and validating cumulative string growth during extraction.
///
/// Use this when an extractor accumulates user-controlled content into a `String`
/// or `Vec<u8>`. Call `check_append(len)` *before* pushing each chunk so the producer
/// can stop early on a quadratic-concatenation / billion-laughs-style attack instead
/// of OOMing the process.
///
/// `Send + Sync` because all state is owned and contains only primitives.
#[derive(Debug, Clone)]
pub(crate) struct StringGrowthValidator {
    max_size: usize,
    current_size: usize,
}

impl StringGrowthValidator {
    /// Create a new string growth validator capped at `max_size` bytes.
    pub(crate) fn new(max_size: usize) -> Self {
        Self {
            max_size,
            current_size: 0,
        }
    }

    /// Account for `len` more bytes about to be appended.
    ///
    /// Returns `Err(SecurityError::ContentTooLarge)` when the cumulative total exceeds
    /// `max_size`. Counter is updated using saturating arithmetic so a malicious caller
    /// cannot wrap to zero.
    pub(crate) fn check_append(&mut self, len: usize) -> Result<(), SecurityError> {
        self.current_size = self.current_size.saturating_add(len);
        if self.current_size > self.max_size {
            Err(SecurityError::ContentTooLarge {
                size: self.current_size,
                max: self.max_size,
            })
        } else {
            Ok(())
        }
    }
}

/// Helper struct for capping iteration counts in parser loops.
///
/// Use inside any unbounded loop reading a user-controlled stream
/// (XML token loop, HTML tokenizer, JSON parser) to bail out before a malicious
/// document spins the CPU. Call `check_iteration()` once per loop turn.
#[derive(Debug, Clone)]
pub(crate) struct IterationValidator {
    max_iterations: usize,
    current_count: usize,
}

impl IterationValidator {
    /// Create a new iteration validator capped at `max_iterations`.
    pub(crate) fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            current_count: 0,
        }
    }

    /// Increment the counter and return `Err(SecurityError::TooManyIterations)`
    /// once `max_iterations` is exceeded.
    pub(crate) fn check_iteration(&mut self) -> Result<(), SecurityError> {
        self.current_count = self.current_count.saturating_add(1);
        if self.current_count > self.max_iterations {
            Err(SecurityError::TooManyIterations {
                count: self.current_count,
                max: self.max_iterations,
            })
        } else {
            Ok(())
        }
    }
}

/// Helper struct for capping recursion / nesting depth.
///
/// Use to bound XML element nesting, HTML DOM depth, JSON object nesting, etc.
/// `push()` increments before checking so the *cap* depth itself is allowed
/// (e.g. `max_depth=100` accepts depth 100 and rejects 101). Always pair with
/// `pop()` on the matching close event.
#[derive(Debug, Clone)]
pub(crate) struct DepthValidator {
    max_depth: usize,
    current_depth: usize,
}

impl DepthValidator {
    /// Create a new depth validator capped at `max_depth` levels.
    pub(crate) fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            current_depth: 0,
        }
    }

    /// Enter one level of nesting. Returns `Err(SecurityError::NestingTooDeep)`
    /// once depth exceeds `max_depth`.
    pub(crate) fn push(&mut self) -> Result<(), SecurityError> {
        self.current_depth = self.current_depth.saturating_add(1);
        if self.current_depth > self.max_depth {
            Err(SecurityError::NestingTooDeep {
                depth: self.current_depth,
                max: self.max_depth,
            })
        } else {
            Ok(())
        }
    }

    /// Exit one level of nesting. Saturates at zero so an unbalanced close
    /// event in a malformed document cannot underflow.
    pub(crate) fn pop(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }
}

/// Helper struct for capping individual entity / attribute string length.
///
/// Use against XML entity expansion (billion-laughs class) and any place
/// a single token can grow unboundedly. Stateless — safe to share by reference
/// across an extraction.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityValidator {
    max_length: usize,
}

impl EntityValidator {
    /// Create a new entity validator capped at `max_length` bytes.
    pub(crate) fn new(max_length: usize) -> Self {
        Self { max_length }
    }

    /// Validate that `content` does not exceed `max_length`.
    pub(crate) fn validate(&self, content: &str) -> Result<(), SecurityError> {
        if content.len() > self.max_length {
            Err(SecurityError::EntityTooLong {
                length: content.len(),
                max: self.max_length,
            })
        } else {
            Ok(())
        }
    }

    /// Validate an XML attribute name+value pair. The check is applied to the
    /// value (attribute names are normally short) but the name is included in
    /// the call signature so callers can wire `quick_xml::Reader` attribute
    /// iteration directly.
    #[cfg(any(feature = "xml", feature = "office"))]
    pub(crate) fn check_attr(&self, _name: &str, value: &str) -> Result<(), SecurityError> {
        self.validate(value)
    }
}

/// Helper struct for capping cumulative table-cell counts across a document.
///
/// Use in CSV/XLSX/HTML table extraction to prevent a malicious document
/// from claiming billions of empty cells and exhausting memory. Call
/// `add_cells(n)` once per row (or once per emitted batch); the validator
/// fails when the running total exceeds `max_cells`.
#[derive(Debug, Clone)]
pub(crate) struct TableValidator {
    max_cells: usize,
    current_cells: usize,
}

impl TableValidator {
    /// Create a new table validator capped at `max_cells` total cells.
    pub(crate) fn new(max_cells: usize) -> Self {
        Self {
            max_cells,
            current_cells: 0,
        }
    }

    /// Account for `count` more cells. Returns `Err(SecurityError::TooManyCells)`
    /// once the cumulative total exceeds `max_cells`. Saturating arithmetic.
    pub(crate) fn add_cells(&mut self, count: usize) -> Result<(), SecurityError> {
        self.current_cells = self.current_cells.saturating_add(count);
        if self.current_cells > self.max_cells {
            Err(SecurityError::TooManyCells {
                cells: self.current_cells,
                max: self.max_cells,
            })
        } else {
            Ok(())
        }
    }
}

/// Bundle of the four hostile-input validators tied to a single document
/// extraction. Holds running counters (depth, iteration, content size) plus
/// the stateless entity-length checker, so a single mutable reference threaded
/// into a parser is enough to enforce every limit advertised by `SecurityLimits`.
///
/// The convenience constructors build the bundle from either a borrowed
/// `SecurityLimits` or an `ExtractionConfig` (taking the `security_limits`
/// override, falling back to defaults when `None`).
#[derive(Debug, Clone)]
pub(crate) struct SecurityBudget {
    pub(crate) depth: DepthValidator,
    pub(crate) iteration: IterationValidator,
    pub(crate) entity: EntityValidator,
    pub(crate) growth: StringGrowthValidator,
    /// Cell counter for tabular extraction (CSV, XLSX, HTML tables).
    /// Threaded alongside the per-event budget but only consumed by table-emitting paths.
    pub(crate) table: TableValidator,
}

impl SecurityBudget {
    /// Build a budget from a borrowed `SecurityLimits`.
    pub(crate) fn from_limits(limits: &SecurityLimits) -> Self {
        Self {
            depth: DepthValidator::new(limits.max_xml_depth.max(limits.max_nesting_depth)),
            iteration: IterationValidator::new(limits.max_iterations),
            entity: EntityValidator::new(limits.max_entity_length),
            growth: StringGrowthValidator::new(limits.max_content_size),
            table: TableValidator::new(limits.max_table_cells),
        }
    }

    /// Convenience: build from `ExtractionConfig.security_limits` falling back to defaults.
    pub(crate) fn from_config(config: &crate::core::config::ExtractionConfig) -> Self {
        let owned: SecurityLimits;
        let limits: &SecurityLimits = match config.security_limits.as_ref() {
            Some(l) => l,
            None => {
                owned = SecurityLimits::default();
                &owned
            }
        };
        Self::from_limits(limits)
    }

    /// Build with explicit defaults (no config available, e.g. internal call sites).
    #[cfg(any(feature = "xml", feature = "office"))]
    pub(crate) fn with_defaults() -> Self {
        Self::from_limits(&SecurityLimits::default())
    }

    /// Apply the iteration cap. Call once per parser-loop turn before reading an event.
    pub(crate) fn step(&mut self) -> Result<(), SecurityError> {
        self.iteration.check_iteration()
    }

    /// Apply nesting on a Start event. Call this after `step()` when the parser
    /// reaches an opening element / object / array / table / etc.
    pub(crate) fn enter(&mut self) -> Result<(), SecurityError> {
        self.depth.push()
    }

    /// Apply nesting on an End event. Saturates at zero on unbalanced input.
    pub(crate) fn leave(&mut self) {
        self.depth.pop();
    }

    /// Account for `len` bytes of emitted text. Returns `Err(ContentTooLarge)`
    /// once cumulative output exceeds `max_content_size`.
    pub(crate) fn account_text(&mut self, len: usize) -> Result<(), SecurityError> {
        self.growth.check_append(len)
    }

    /// Validate an XML / HTML attribute value against `max_entity_length`.
    #[cfg(any(feature = "xml", feature = "office"))]
    pub(crate) fn check_attr(&self, name: &str, value: &str) -> Result<(), SecurityError> {
        self.entity.check_attr(name, value)
    }

    /// Validate a single entity / token string against `max_entity_length`.
    pub(crate) fn check_entity(&self, value: &str) -> Result<(), SecurityError> {
        self.entity.validate(value)
    }

    /// Account for `count` more table cells. Returns `Err(TooManyCells)` once
    /// cumulative cells exceed `max_table_cells`.
    pub(crate) fn add_cells(&mut self, count: usize) -> Result<(), SecurityError> {
        self.table.add_cells(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = SecurityLimits::default();
        assert_eq!(limits.max_archive_size, 500 * 1024 * 1024);
        assert_eq!(limits.max_nesting_depth, 1024);
        assert_eq!(limits.max_entity_length, 1024 * 1024);
    }

    #[test]
    fn test_string_growth_validator_basic() {
        let mut v = StringGrowthValidator::new(100);
        assert!(v.check_append(50).is_ok());
        assert_eq!(v.current_size, 50);
        assert!(v.check_append(50).is_ok());
        assert_eq!(v.current_size, 100);
        assert!(matches!(
            v.check_append(1),
            Err(SecurityError::ContentTooLarge { size: 101, max: 100 })
        ));
    }

    #[test]
    fn test_string_growth_validator_saturates_on_overflow() {
        let mut v = StringGrowthValidator::new(usize::MAX - 10);
        assert!(v.check_append(usize::MAX).is_err(), "saturating add cannot wrap");
    }

    #[test]
    fn test_iteration_validator_basic() {
        let mut v = IterationValidator::new(3);
        assert!(v.check_iteration().is_ok());
        assert!(v.check_iteration().is_ok());
        assert!(v.check_iteration().is_ok());
        assert!(matches!(
            v.check_iteration(),
            Err(SecurityError::TooManyIterations { count: 4, max: 3 })
        ));
    }

    #[test]
    fn test_depth_validator_push_pop() {
        let mut v = DepthValidator::new(3);
        assert!(v.push().is_ok());
        assert!(v.push().is_ok());
        assert!(v.push().is_ok());
        assert_eq!(v.current_depth, 3);
        assert!(matches!(
            v.push(),
            Err(SecurityError::NestingTooDeep { depth: 4, max: 3 })
        ));
        v.pop();
        assert_eq!(v.current_depth, 3);
    }

    #[test]
    fn test_depth_validator_pop_saturates_at_zero() {
        let mut v = DepthValidator::new(10);
        v.pop();
        v.pop();
        assert_eq!(v.current_depth, 0, "underflow is impossible");
    }

    #[test]
    fn test_entity_validator() {
        let v = EntityValidator::new(10);
        assert!(v.validate("short").is_ok());
        assert!(v.validate("0123456789").is_ok());
        assert!(matches!(
            v.validate("01234567890"),
            Err(SecurityError::EntityTooLong { length: 11, max: 10 })
        ));
        #[cfg(any(feature = "xml", feature = "office"))]
        {
            assert!(v.check_attr("href", "http://x").is_ok());
            assert!(v.check_attr("data", &"x".repeat(50)).is_err());
        }
    }

    #[test]
    fn test_table_validator() {
        let mut v = TableValidator::new(10);
        assert!(v.add_cells(5).is_ok());
        assert_eq!(v.current_cells, 5);
        assert!(v.add_cells(5).is_ok());
        assert_eq!(v.current_cells, 10);
        assert!(matches!(
            v.add_cells(1),
            Err(SecurityError::TooManyCells { cells: 11, max: 10 })
        ));
    }
}
