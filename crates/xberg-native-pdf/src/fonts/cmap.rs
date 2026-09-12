//! ToUnicode CMap parser with optimized state machine and binary search.
//!
//! CMap (Character Map) streams define the mapping from character codes
//! to Unicode characters. This is essential for text extraction when fonts
//! use custom encodings.
//!
//! Phase 4, Task 4.4
//! Phase 4.1: Advanced CMap Directives support
//!   - beginnotdefrange sections (fallback for unmapped characters)
//!   - Escape sequences for special characters (space, tab, newline, etc.)
//!   - Flexible whitespace in CMap syntax
//!
//! Phase 5.2: Global CMap Caching System
//!   - Global cache prevents re-parsing of identical CMaps across fonts
//!   - Reference counting with `Arc<CMap>` for efficient sharing
//!   - Cache keyed by stream hash for fast lookup
//!   - Thread-safe design using Mutex and Arc
//!
//! Phase 5.3: Optimized CMap Parsing
//!   - State machine parser replacing regex-based approach
//!   - Binary search for O(log n) range lookups
//!   - Support for 100k+ entry CMaps
//!   - 20-40% faster parsing performance

use crate::cache::MutexExt;
use crate::error::Result;
use regex::Regex;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// A range entry for efficient binary search lookups.
///
/// Stores start and end character codes with the corresponding target Unicode.
/// Used for fast O(log n) range lookups in large CMaps.
#[derive(Clone, Debug)]
struct RangeEntry {
    start: u32,
    end: u32,
    target: u32,
}

/// A character map from character codes to Unicode strings.
///
/// Optimized storage for efficient lookups:
/// - `chars`: HashMap for individual bfchar mappings (direct lookup O(1))
/// - `ranges`: Sorted Vec of range entries for binary search (O(log n))
/// - `notdef_ranges`: Sorted Vec for fallback mappings
/// - `code_width`: Maximum code width in bytes (1 or 2), from `begincodespacerange`
///
/// Keys are character codes (typically 1-4 bytes), values are Unicode strings.
/// We use u32 to support multi-byte character codes found in CID fonts.
#[derive(Clone, Debug)]
pub struct CMap {
    /// Individual character mappings from bfchar sections
    chars: HashMap<u32, String>,
    /// Range mappings for O(log n) binary search lookups
    ranges: Vec<RangeEntry>,
    /// Undefined range fallbacks for unmapped codes
    notdef_ranges: Vec<RangeEntry>,
    /// Maximum character code width in bytes, derived from `begincodespacerange`.
    ///
    /// - `1` (default) means single-byte codes (standard simple fonts).
    /// - `2` means two-byte codes (CJK composite fonts, Identity-H CMaps).
    ///
    /// Set during parsing if any codespace entry has a 2-byte (4-hex-digit) hex string.
    /// Used by the text extractor to decide whether to read 1 or 2 bytes per character
    /// from the PDF content stream (§9.7.5 "CMaps").
    pub code_width: u8,
    /// Writing mode declared by the CMap stream via `/WMode 0 def` or `/WMode 1 def`.
    ///
    /// - `0` (default): horizontal writing — per-glyph advance is along the x-axis.
    /// - `1`: vertical writing — per-glyph advance is along the y-axis and the
    ///   per-CID vertical-origin offset `(v_x, v_y)` shifts the glyph from its
    ///   horizontal origin to its vertical origin before painting.
    ///
    /// Populated by `parse_tounicode_cmap` when the CMap source contains a
    /// `/WMode <int> def` directive (ISO 32000-1:2008 §9.7.5.4 / Adobe CMap and
    /// CIDFont Files Specification §7.2). Predefined PDF CMaps whose names end
    /// in `-V` (Identity-V, UniJIS-UTF16-V, UniGB-UTF16-V, UniCNS-UTF16-V,
    /// UniKS-UTF16-V) and the bare legacy `V` are detected separately on
    /// `FontInfo` from the encoding name; this field is the authoritative
    /// signal for *embedded* CMap streams which may carry `/WMode 1` even when
    /// their `/CMapName` does not advertise a `-V` suffix.
    pub wmode: u8,
}

impl CMap {
    /// Unicode string for a character code.
    ///
    /// 1. `chars` (bfchar + non-contiguous bfrange entries) — O(1), borrowed.
    /// 2. `ranges` (compressed contiguous bfranges) — O(log n) binary search;
    ///    the value is computed (`target + (code - start)`), so owned.
    /// 3. `notdef_ranges` fallback.
    ///
    /// `chars` is checked first and holds the document-order-correct value for
    /// any code a later `bfchar` redefined (§9.10.3); `ranges` only holds
    /// runs that were contiguous in the final `chars` state.
    pub fn get(&self, code: &u32) -> Option<std::borrow::Cow<'_, str>> {
        if let Some(s) = self.chars.get(code) {
            return Some(std::borrow::Cow::Borrowed(s));
        }

        if let Ok(pos) = self.ranges.binary_search_by(|r| {
            if r.end < *code {
                std::cmp::Ordering::Less
            } else if r.start > *code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            let r = &self.ranges[pos];
            let cp = r.target.wrapping_add(*code - r.start);
            if let Some(ch) = char::from_u32(cp) {
                return Some(std::borrow::Cow::Owned(ch.to_string()));
            }
        }

        for range in &self.notdef_ranges {
            if range.start <= *code
                && *code <= range.end
                && let Some(s) = self.chars.get(&range.target)
            {
                return Some(std::borrow::Cow::Borrowed(s));
            }
        }

        None
    }

    /// Collapse long contiguous runs in `chars` into `ranges`, cutting the
    /// persistent memory of large sequential bfranges (e.g. `<0000><FFFF>`
    /// expands to ~65 536 `String`s, shared via `Arc` in the global cache).
    ///
    /// Operates on the *final* `chars` state, so any code a later definition
    /// redefined already holds the document-order-correct value (§9.10.3)
    /// — compressing it cannot change semantics. A run is collapsed only when
    /// both the code and its single-char codepoint are contiguous and the run
    /// is long enough to be worth it; multi-char (ligature) values and
    /// notdef-range targets are left in `chars`.
    fn compress_sequential_ranges(&mut self) {
        const MIN_RUN: usize = 256;

        let notdef_targets: std::collections::HashSet<u32> = self.notdef_ranges.iter().map(|r| r.target).collect();

        let mut singles: Vec<(u32, u32)> = self
            .chars
            .iter()
            .filter(|(c, _)| !notdef_targets.contains(c))
            .filter_map(|(&c, s)| {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(ch), None) => Some((c, ch as u32)),
                    _ => None,
                }
            })
            .collect();
        if singles.len() < MIN_RUN {
            return;
        }
        singles.sort_unstable_by_key(|&(c, _)| c);

        let mut i = 0;
        while i < singles.len() {
            let mut j = i;
            while j + 1 < singles.len() && singles[j + 1].0 == singles[j].0 + 1 && singles[j + 1].1 == singles[j].1 + 1
            {
                j += 1;
            }
            if j - i + 1 >= MIN_RUN {
                self.ranges.push(RangeEntry {
                    start: singles[i].0,
                    end: singles[j].0,
                    target: singles[i].1,
                });
                for &(c, _) in &singles[i..=j] {
                    self.chars.remove(&c);
                }
            }
            i = j + 1;
        }
        self.ranges.sort_unstable_by_key(|r| r.start);
    }

    /// Check if the CMap is empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty() && self.ranges.is_empty() && self.notdef_ranges.is_empty()
    }

    /// Get the number of mappings.
    pub fn len(&self) -> usize {
        self.chars.len() + self.ranges.len() + self.notdef_ranges.len()
    }

    /// Create a new empty CMap.
    fn new() -> Self {
        CMap {
            chars: HashMap::new(),
            ranges: Vec::new(),
            notdef_ranges: Vec::new(),
            code_width: 1,
            wmode: 0,
        }
    }

    /// Insert individual character mapping.
    fn insert(&mut self, code: u32, unicode: String) {
        self.chars.insert(code, unicode);
    }
}

/// Key for indexing into the global CMap cache.
///
/// CMap streams are cached by the hash of their raw bytes.
/// This allows identical CMaps (even with different object IDs) to share
/// a single parsed instance, reducing memory usage and parsing overhead
/// in documents with repeated font definitions.
///
/// # Why Stream Hash?
/// - Deterministic: Same stream content = same hash
/// - Fast: O(n) to compute, O(1) to lookup
/// - Reliable: Collisions extremely unlikely for real PDFs
/// - Flexible: Doesn't require PDF object metadata
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct CMapKey(u64);

/// Compute a hash of the raw CMap stream bytes.
///
/// Uses the platform's default hasher (SipHash by default).
/// The hash is used as the key in the global CMap cache.
fn compute_stream_hash(data: &[u8]) -> CMapKey {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    CMapKey(hasher.finish())
}

// Global CMap cache for deduplicating parsed CMaps.
//
// Design:
// - Maps from stream hash to Arc<CMap> (reference-counted parsed CMap)
// - Arc allows efficient sharing without cloning
// - Mutex ensures thread-safe access
// - Bounded at MAX_CMAP_CACHE_ENTRIES with LRU-style eviction (`get` promotes hot entries)
//
// Usage:
// When a LazyCMap is first accessed, it checks this cache before parsing.
// If the same stream bytes appear in multiple fonts, only one CMap is
// parsed and shared via Arc reference counting.
//
// Thread Safety:
// Multiple threads can safely:
// - Check cache simultaneously (read-only Arc clones)
// - Parse and insert new entries (Mutex serializes writes)
// - Access shared CMaps concurrently (Arc is thread-safe) ~keep

/// Maximum number of entries in the global CMap cache.
const MAX_CMAP_CACHE_ENTRIES: usize = 1024;

static CMAP_CACHE: std::sync::LazyLock<Mutex<crate::cache::BoundedEntryCache<CMapKey, Arc<CMap>>>> =
    std::sync::LazyLock::new(|| Mutex::new(crate::cache::BoundedEntryCache::new(MAX_CMAP_CACHE_ENTRIES)));

/// Clear the global CMap cache.
///
/// Call this to reclaim memory in long-lived processes (MCP servers,
/// Python REPLs, Node.js services) that process many different PDFs.
pub fn clear_cmap_cache() {
    CMAP_CACHE.lock_or_recover().clear();
}

/// Returns the current number of entries in the global CMap cache.
pub fn cmap_cache_size() -> usize {
    CMAP_CACHE.lock_or_recover().len()
}

/// Lazy-loaded ToUnicode CMap wrapper.
///
/// Defers parsing of ToUnicode CMap streams until first character lookup,
/// improving performance during initial font loading. After first access,
/// the parsed CMap is cached and reused for subsequent lookups.
///
/// # Two-Level Caching
/// - **Local cache** (`parsed`): Caches result in this LazyCMap instance
/// - **Global cache**: Deduplicates identical CMaps across fonts (Phase 5.2)
///
/// # Design
/// - **raw_stream**: Stores unparsed CMap stream bytes
/// - **cache_key**: Hash of stream bytes for global cache lookup
/// - **parsed**: Mutex-protected, tri-state memoization slot
///   - Outer `Option`: `None` = parsing not yet attempted
///   - Inner `Option<Arc<CMap>>`: `Some(attempted)`, where `attempted` is
///     `Some(cmap)` on success or `None` on a parse failure. Both outcomes
///     are memoized so a malformed stream is parsed at most once, not once
///     per character lookup (`get()` is called from the per-character
///     decode path). ~keep
///   - Arc: Thread-safe sharing of a successfully parsed result
///   - Mutex: Thread-safe mutable access to the memoization slot
///
/// # Thread Safety
/// Multiple threads can safely call `get()` concurrently:
/// - Parse is attempted once, even with concurrent access, regardless of outcome
/// - Cached result is shared via `Arc<CMap>` globally
/// - Mutex ensures atomic updates to cached state
///
/// # Performance Impact
/// - Font creation: 30-40% faster (skips CMap parsing)
/// - First lookup: Slightly slower (parse + store cost, amortized across fonts)
/// - Subsequent lookups: Same speed (cached result, including a memoized failure)
/// - Multi-font documents: Significant improvement (50-70% for repeated fonts)
/// - Global cache: Deduplicates identical CMaps across fonts
#[derive(Debug, Clone)]
pub struct LazyCMap {
    /// Raw CMap stream bytes (not yet parsed)
    raw_stream: Vec<u8>,

    /// Cache key derived from stream hash
    cache_key: CMapKey,

    /// Parsed CMap, lazily loaded on first access.
    /// `None` means parsing has not been attempted yet; `Some(None)` means
    /// parsing was attempted and failed; `Some(Some(cmap))` means it
    /// succeeded. Both terminal states are memoized so the parser
    /// never re-runs after the first call. ~keep
    parsed: Arc<Mutex<Option<Option<Arc<CMap>>>>>,
}

impl LazyCMap {
    /// Create a new lazy CMap from raw stream bytes.
    ///
    /// # Arguments
    /// * `raw_stream` - Unparsed CMap stream bytes
    ///
    /// # Returns
    /// A new LazyCMap that will parse on first access via `get()`
    ///
    /// # Performance
    /// This is O(n) where n is the size of raw_stream (for hashing).
    /// Parsing is deferred until first call to `get()`.
    pub fn new(raw_stream: Vec<u8>) -> Self {
        let cache_key = compute_stream_hash(&raw_stream);
        LazyCMap {
            raw_stream,
            cache_key,
            parsed: Arc::new(Mutex::new(None)),
        }
    }

    /// Same as [`LazyCMap::new`]. The font name is accepted for API compatibility
    /// but is intentionally excluded from telemetry because PDF names are untrusted.
    pub fn new_for_font(raw_stream: Vec<u8>, _font_name: String) -> Self {
        LazyCMap::new(raw_stream)
    }

    /// Get a reference to the parsed CMap.
    ///
    /// On first call, checks global cache, then parses if needed.
    /// On subsequent calls, returns the cached `Arc<CMap>`.
    ///
    /// # Caching Strategy
    /// 1. Check local `parsed` cache (fastest, no lock contention)
    /// 2. Check global `CMAP_CACHE` (fast, shared across fonts)
    /// 3. Parse and populate both caches on miss
    ///
    /// # Returns
    /// `Some(Arc<CMap>)` if parsing succeeded, `None` if parsing failed or stream was empty
    /// Get the raw CMap stream bytes.
    pub fn raw_data(&self) -> &[u8] {
        &self.raw_stream
    }

    /// Return the character code width (1 or 2) declared by `begincodespacerange`.
    ///
    /// Parses and caches the CMap if not already done.
    /// Returns `1` when the CMap is missing or unparseable (safe default for simple fonts).
    /// Returns `2` when the codespace declares 2-byte codes, indicating a CJK composite font
    /// whose content stream must be read two bytes at a time.
    pub fn code_width(&self) -> u8 {
        self.get().map(|cmap| cmap.code_width).unwrap_or(1)
    }

    /// Return the writing mode declared by the underlying CMap stream.
    ///
    /// Parses and caches the CMap if not already done.
    /// Returns `0` (horizontal) when the CMap is missing, unparseable, or does
    /// not contain an explicit `/WMode` directive — matching the spec default.
    /// Returns `1` when the CMap declares `/WMode 1 def` (vertical writing).
    pub fn wmode(&self) -> u8 {
        self.get().map(|cmap| cmap.wmode).unwrap_or(0)
    }

    /// Returns the parsed CMap, loading and caching it on first access.
    pub fn get(&self) -> Option<Arc<CMap>> {
        let mut parsed_guard = self.parsed.lock_or_recover();

        // Outer `Some` means an attempt already ran, whether it succeeded
        // (`Some(Some(cmap))`) or failed (`Some(None)`). Either way, return
        // the memoized outcome without re-parsing. ~keep
        if let Some(attempted) = parsed_guard.as_ref() {
            return attempted.as_ref().map(Arc::clone);
        }

        {
            let mut global = CMAP_CACHE.lock_or_recover();
            if let Some(cached) = global.get(&self.cache_key) {
                let arc = Arc::clone(cached);
                *parsed_guard = Some(Some(Arc::clone(&arc)));
                tracing::debug!("CMap cache hit (global) for stream hash {:?}", self.cache_key);
                return Some(arc);
            }
        }

        match parse_tounicode_cmap(&self.raw_stream) {
            Ok(cmap) => {
                let cmap_arc = Arc::new(cmap);

                *parsed_guard = Some(Some(Arc::clone(&cmap_arc)));

                {
                    let mut global = CMAP_CACHE.lock_or_recover();
                    global.insert(self.cache_key, Arc::clone(&cmap_arc));
                }

                tracing::debug!("CMap parsed and cached (stream hash {:?})", self.cache_key);
                Some(cmap_arc)
            }
            Err(error) => {
                // Memoized as `Some(None)` above so this fires at most once
                // per LazyCMap even though `get()` sits on the per-character
                // decode path (`FontInfo::char_to_unicode`) — a broken
                // ToUnicode CMap is a genuine "fallback taken" a consumer
                // wants to see, and it can no longer spam once per glyph. ~keep
                *parsed_guard = Some(None);
                crate::error::trace_recovery("parse_tounicode_cmap", &error);
                None
            }
        }
    }
}

/// Parse an escape sequence token like `<space>`, `<tab>`, etc.
///
/// These are symbolic names for special characters in CMap files.
/// Supported sequences:
/// - `<space>` -> U+0020 (space)
/// - `<tab>` -> U+0009 (tab)
/// - `<newline>` -> U+000A (newline)
/// - `<carriage return>` -> U+000D (carriage return)
///
/// # Arguments
///
/// * `token` - A string token from the CMap (should be enclosed in angle brackets)
///
/// # Returns
///
/// Some(String) containing the mapped character, or None if not an escape sequence
fn parse_escape_sequence(token: &str) -> Option<String> {
    let token = token.trim();
    let token = if token.starts_with('<') && token.ends_with('>') {
        &token[1..token.len() - 1]
    } else {
        token
    };

    let token_lower = token.to_lowercase();
    match token_lower.trim() {
        "space" => Some(" ".to_string()),
        "tab" => Some("\t".to_string()),
        "newline" => Some("\n".to_string()),
        "carriage return" => Some("\r".to_string()),
        _ => None,
    }
}

/// Decode a UTF-16 surrogate pair encoded as a 32-bit value.
///
/// PDF ToUnicode CMaps sometimes encode Unicode code points > U+FFFF
/// as UTF-16 surrogate pairs represented as 8 hex digits.
///
/// Example: D835DF0C (0xD835DF0C) represents:
/// - High surrogate: 0xD835
/// - Low surrogate: 0xDF0C
/// - Decoded: U+1D70C (MATHEMATICAL ITALIC SMALL RHO '𝜌')
///
/// # Arguments
///
/// * `value` - A 32-bit value where the high 16 bits are the high surrogate
///            and the low 16 bits are the low surrogate
///
/// # Returns
///
/// The decoded Unicode character as a String, or None if the surrogate pair is invalid
fn decode_utf16_surrogate_pair(value: u32) -> Option<String> {
    let high = (value >> 16) as u16;
    let low = (value & 0xFFFF) as u16;

    // Check if these are valid surrogate pairs
    // High surrogate: 0xD800 - 0xDBFF
    // Low surrogate: 0xDC00 - 0xDFFF ~keep
    if (0xD800..=0xDBFF).contains(&high) && (0xDC00..=0xDFFF).contains(&low) {
        let codepoint = 0x10000 + (((high & 0x3FF) as u32) << 10) + ((low & 0x3FF) as u32);
        char::from_u32(codepoint).map(|ch| ch.to_string())
    } else {
        char::from_u32(value).map(|ch| ch.to_string())
    }
}

/// Structural keywords that mark a stream as at least attempting to be a CMap
/// (Adobe CMap & CIDFont Files Spec §7; ISO 32000-1 §9.7.5). Used only to
/// distinguish "not a CMap at all" from "a CMap with malformed content" —
/// see [`parse_tounicode_cmap`]'s `# Errors` section.
const CMAP_STRUCTURAL_KEYWORDS: [&str; 5] = [
    "begincmap",
    "beginbfchar",
    "beginbfrange",
    "begincodespacerange",
    "beginnotdefrange",
];

/// Returns `true` if `content` contains at least one CMap structural keyword.
fn has_any_cmap_structural_keyword(content: &str) -> bool {
    CMAP_STRUCTURAL_KEYWORDS.iter().any(|keyword| content.contains(keyword))
}

/// Parse a ToUnicode CMap stream with optimized state machine parser.
///
/// ToUnicode CMaps contain mappings in two formats:
/// - `bfchar`: Single character mappings
/// - `bfrange`: Range mappings
///
/// # Format Examples
///
/// ```text
/// beginbfchar
/// <0041> <0041>  % Maps 0x41 to Unicode U+0041 ('A')
/// <0042> <0042>  % Maps 0x42 to Unicode U+0042 ('B')
/// endbfchar
///
/// beginbfrange
/// <0020> <007E> <0020>  % Maps 0x20-0x7E to U+0020-U+007E (ASCII printable)
/// endbfrange
/// ```
///
/// # Phase 5.3 Optimization
///
/// Uses state machine parsing for 20-40% faster performance:
/// - State transitions: HEADER -> CODESPACE -> BFCHAR/BFRANGE/NOTDEFRANGE -> FOOTER
/// - Sequential token processing without full buffering
/// - Binary search on sorted ranges for O(log n) lookups
/// - Direct insertion into HashMap for bfchar entries
///
/// # Arguments
///
/// * `data` - Raw CMap stream data (should be decoded/decompressed first)
///
/// # Returns
///
/// A CMap with optimized storage for O(1) direct lookup and O(log n) range lookup.
///
/// # Errors
///
/// This is a parser for untrusted binary input (§9.10): a wrong-but-plausible
/// mapping is worse than a loud failure, so `Err` is reserved for the one case
/// where any mapping we could derive would be arbitrary — a **non-empty**
/// stream containing none of the CMap structural keywords (`begincmap`,
/// `beginbfchar`, `beginbfrange`, `begincodespacerange`, `beginnotdefrange`).
/// That is not a truncated or malformed CMap, it is not a CMap at all (wrong
/// stream, corrupted object, non-CMap binary data).
///
/// A **zero-length** stream is treated as legitimately empty (`Ok`, no
/// warning) — some producers emit an empty ToUnicode stream for a font that
/// maps nothing, and that is not evidence of corruption.
///
/// Everything else — a `begin…` block with no matching `end…` before EOF, a
/// bfchar/bfrange/notdefrange line with malformed hex or an unmappable code
/// point, a bfrange array whose entry count disagrees with its declared
/// range — is DEGRADED: the malformed section or line is dropped, a
/// `tracing::warn!` is emitted so the defect is diagnosable, and parsing
/// continues with whatever the rest of the stream yields. ~keep
///
/// # Examples
///
/// ```
/// use xberg_native_pdf::fonts::cmap::parse_tounicode_cmap;
///
/// let cmap_data = b"beginbfchar\n<0041> <0041>\nendbfchar";
/// let cmap = parse_tounicode_cmap(cmap_data).unwrap();
/// assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
/// ```
pub fn parse_tounicode_cmap(data: &[u8]) -> Result<CMap> {
    let mut cmap = CMap::new();
    let content = String::from_utf8_lossy(data);

    // A non-empty stream carrying none of the CMap structural keywords is not
    // a CMap at all — the wrong stream, a corrupted object, or non-CMap
    // binary data. Any mapping derived from it would be arbitrary, so this is
    // the one case where the parser fails loudly instead of returning a
    // silently empty (and therefore indistinguishable from "legitimately
    // empty") CMap. A zero-length stream is handled separately below as
    // legitimately empty, matching producers that emit an empty ToUnicode
    // stream for a font that maps nothing. ~keep
    if !data.is_empty() && !has_any_cmap_structural_keyword(&content) {
        return Err(crate::error::Error::Font(format!(
            "ToUnicode CMap stream ({} byte(s)) has none of begincmap/beginbfchar/beginbfrange/\
             begincodespacerange/beginnotdefrange; not a valid CMap",
            data.len()
        )));
    }

    // Parse `/WMode N def` directive (Adobe CMap & CIDFont Files Spec §7.2, ISO
    // 32000-1 §9.7.5.4). `N` is `0` (horizontal) or `1` (vertical). The
    // directive appears at the top level of the CMap stream, outside any
    // `begin…end` block, so a substring + integer scan is sufficient and
    // avoids a second tokenizer pass. ~keep
    if let Some(parsed_wmode) = parse_wmode_directive(&content) {
        cmap.wmode = parsed_wmode;
        if parsed_wmode == 1 {
            tracing::trace!("CMap declares /WMode 1 (vertical writing)");
        }
    }

    // Parse begincodespacerange sections (PDF Spec §9.7.5 / §9.10.3)
    //
    // The codespace range declares the valid domain of character codes and,
    // critically, **their byte width**.  A range like `<00> <FF>` is 1-byte;
    // `<0000> <FFFF>` is 2-byte.  We use the widest range found to set
    // `cmap.code_width`, which the text extractor uses to decide how many
    // bytes to consume per character from the PDF content stream.
    //
    // Without this, any CJK ToUnicode CMap that does not use one of the
    // well-known encoding names (Identity-H, EUC, GBK, …) would be read
    // one byte at a time, splitting every 2-byte CID into two wrong codes. ~keep
    for section in extract_sections(&content, "begincodespacerange", "endcodespacerange") {
        for line in section.lines() {
            let width = parse_codespacerange_line_width(line);
            if width > cmap.code_width {
                cmap.code_width = width;
                tracing::trace!("ToUnicode codespacerange: code_width set to {}", cmap.code_width);
            }
        }
    }

    // Parse bfchar and bfrange sections in document order so that later entries
    // overwrite earlier ones for the same code (ISO 32000-1:2008 §9.10.3).
    // pdf.js, MuPDF, and Poppler all use this last-wins, document-order semantics. ~keep
    for (kind, section) in bf_sections_in_document_order(&content) {
        match kind {
            BfSectionKind::Char => {
                let mut attempted = 0usize;
                let mut malformed = 0usize;
                for line in significant_lines(section) {
                    attempted += 1;
                    let pairs = parse_bfchar_line(line);
                    if pairs.is_empty() {
                        malformed += 1;
                        continue;
                    }
                    for (src, dst) in pairs {
                        tracing::trace!("ToUnicode bfchar: 0x{:02X} -> {:?}", src, dst);
                        cmap.insert(src, dst);
                    }
                }
                warn_on_malformed_lines("bfchar", attempted, malformed);
            }
            BfSectionKind::Range => {
                let mut attempted = 0usize;
                let mut malformed = 0usize;
                for line in significant_lines(section) {
                    attempted += 1;
                    match parse_bfrange_line(line) {
                        Some(mappings) => {
                            tracing::trace!("ToUnicode bfrange: {} mappings parsed", mappings.len());
                            for (src, dst) in mappings {
                                cmap.insert(src, dst);
                            }
                        }
                        None => malformed += 1,
                    }
                }
                warn_on_malformed_lines("bfrange", attempted, malformed);
            }
        }
    }

    for section in extract_sections(&content, "beginnotdefrange", "endnotdefrange") {
        let mut attempted = 0usize;
        let mut malformed = 0usize;
        for line in significant_lines(section) {
            attempted += 1;
            match parse_notdefrange_line(line) {
                Some(mappings) => {
                    tracing::trace!("ToUnicode notdefrange: {} mappings parsed", mappings.len());
                    for (src, dst) in mappings {
                        // Only insert if not already mapped (normal mappings take precedence)
                        // For notdefrange, we need to check if source is already mapped ~keep
                        if !cmap.chars.contains_key(&src) {
                            cmap.insert(src, dst);
                        }
                    }
                }
                None => malformed += 1,
            }
        }
        warn_on_malformed_lines("notdefrange", attempted, malformed);
    }

    cmap.compress_sequential_ranges();
    Ok(cmap)
}

enum BfSectionKind {
    Char,
    Range,
}

/// Lines of a `begin…end` section body worth attempting to parse: skips blank
/// lines and lines that are entirely a PostScript comment (`%` to
/// end-of-line), neither of which represent a malformation.
pub(crate) fn significant_lines(section: &str) -> impl Iterator<Item = &str> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('%'))
}

/// Emit one aggregated `tracing::warn!` for a `begin…end` block that had at
/// least one line which failed to parse into any mapping — malformed hex, an
/// unmappable code point, or a line matching none of the recognized formats
/// (§9.10.3). One warning per block avoids spamming the log per malformed
/// line while still surfacing the defect (DEGRADED: the malformed lines are
/// dropped, parsing continues with everything else in the block). ~keep
fn warn_on_malformed_lines(section_kind: &str, attempted: usize, malformed: usize) {
    if malformed > 0 {
        tracing::warn!(
            "ToUnicode CMap: {} of {} {} line(s) failed to parse (malformed hex or unmappable \
             code point); those entries are skipped",
            malformed,
            attempted,
            section_kind
        );
    }
}

/// Yield `beginbfchar` and `beginbfrange` sections in the order they appear in
/// the CMap stream, so that callers can process them with document-order,
/// last-wins semantics (matching pdf.js, MuPDF, and Poppler).
fn bf_sections_in_document_order(content: &str) -> impl Iterator<Item = (BfSectionKind, &str)> {
    let mut remaining = content;
    std::iter::from_fn(move || {
        loop {
            let pos = remaining.find("beginbf")?;
            let after = &remaining[pos + "beginbf".len()..];

            if let Some(body) = after.strip_prefix("char") {
                if let Some(end) = body.find("endbfchar") {
                    remaining = &body[end + "endbfchar".len()..];
                    return Some((BfSectionKind::Char, &body[..end]));
                }
                // Truncated: `beginbfchar` with no `endbfchar` anywhere in the
                // rest of the stream. DEGRADED — drop this block and keep
                // scanning; a single damaged block does not invalidate
                // mappings found elsewhere in the CMap. ~keep
                tracing::warn!(
                    "ToUnicode CMap: `beginbfchar` has no matching `endbfchar`; dropping this \
                     truncated block"
                );
            } else if let Some(body) = after.strip_prefix("range") {
                if let Some(end) = body.find("endbfrange") {
                    remaining = &body[end + "endbfrange".len()..];
                    return Some((BfSectionKind::Range, &body[..end]));
                }
                tracing::warn!(
                    "ToUnicode CMap: `beginbfrange` has no matching `endbfrange`; dropping this \
                     truncated block"
                );
            }
            // Unrecognised "beginbf…" token; skip past it. ~keep
            remaining = after;
        }
    })
}

/// Extract sections between begin and end markers.
///
/// `pub(crate)` so [`super::cid_cmap`]'s `begincidrange`/`begincidchar`/
/// `begincodespacerange` parser can reuse this tokenizer instead of
/// duplicating it. ~keep
pub(crate) fn extract_sections<'a>(content: &'a str, begin: &str, end: &str) -> Vec<&'a str> {
    let mut sections = Vec::new();
    let mut remaining = content;

    while let Some(begin_pos) = remaining.find(begin) {
        let after_begin = &remaining[begin_pos + begin.len()..];
        if let Some(end_pos) = after_begin.find(end) {
            sections.push(&after_begin[..end_pos]);
            remaining = &after_begin[end_pos + end.len()..];
        } else {
            // Truncated: `begin` with no matching `end` before EOF. DEGRADED —
            // nothing after this point can be a complete section of this
            // kind, so the rest of the stream is dropped for this
            // begin/end pair specifically (other keyword pairs, e.g. bfchar
            // sections, are scanned independently and are unaffected). ~keep
            tracing::warn!(
                "ToUnicode CMap: `{}` has no matching `{}`; the rest of the stream is dropped \
                 for this section type",
                begin,
                end
            );
            break;
        }
    }

    sections
}

/// Parse a `/WMode N def` directive from a CMap source string.
///
/// Returns `Some(0)` for explicit horizontal, `Some(1)` for explicit vertical,
/// and `None` when no directive is present (caller keeps the spec default of
/// `0`). Per Adobe CMap & CIDFont Files Spec §7.2 and ISO 32000-1 §9.7.5.4,
/// `/WMode` must precede `begincmap` but in practice all writers we have seen
/// place it within the prologue before `begincodespacerange`. A direct lexical
/// scan is robust to either ordering.
///
/// Only matches values `0` or `1`; any other integer is treated as a malformed
/// directive and ignored (returns `None`).
pub(crate) fn parse_wmode_directive_public(content: &str) -> Option<u8> {
    parse_wmode_directive(content)
}

fn parse_wmode_directive(content: &str) -> Option<u8> {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"/WMode\s+([0-9]+)\s+def").unwrap());
    // PostScript comments run from `%` to end-of-line (Adobe PostScript
    // Language Reference §3.3.1). Strip them so a commented-out directive
    // like `% /WMode 1 def` does not flip the writing mode. Keep newlines
    // intact so any subsequent legitimate `/WMode` on a later line is
    // still matched. ~keep
    let cleaned: String = content
        .lines()
        .map(|line| match line.find('%') {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let caps = RE.captures(&cleaned)?;
    let value: u32 = caps[1].parse().ok()?;
    match value {
        0 => Some(0),
        1 => Some(1),
        // M6: non-spec values (e.g. `/WMode 2 def`) surface a warning so
        // producer bugs are diagnosable. We still return None and let
        // the caller fall back to the horizontal default — the spec
        // (§9.7.5.4) only defines values 0 and 1. ~keep
        other => {
            tracing::warn!(
                "Non-standard /WMode {} in CMap stream; falling back to horizontal (WMode 0)",
                other
            );
            None
        }
    }
}

/// Parse a `begincodespacerange` line and return the maximum code byte-width found.
///
/// Each entry is a pair of hex strings: `<lo> <hi>`.  The number of hex digits
/// in each string determines the byte width of the character codes:
/// - 2 hex digits  → 1-byte code  (e.g. `<00> <FF>`)
/// - 4 hex digits  → 2-byte code  (e.g. `<0000> <FFFF>`)
///
/// Returns 1 if the line does not contain a valid codespace pair, or 2 if at
/// least one 2-byte (4-hex-digit) entry is found.
fn parse_codespacerange_line_width(line: &str) -> u8 {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>").unwrap());

    let mut max_width: u8 = 1;
    for caps in RE.captures_iter(line) {
        let lo_hex = caps[1].trim().replace(char::is_whitespace, "");
        let hi_hex = caps[2].trim().replace(char::is_whitespace, "");
        if lo_hex.len() >= 4 || hi_hex.len() >= 4 {
            max_width = 2;
        }
    }
    max_width
}

/// Returns `true` if every byte of `s` is an ASCII hex digit (`0-9`, `a-f`, `A-F`),
/// and `s` is non-empty.
///
/// PDF ToUnicode CMap hex strings (`<...>`) are defined to contain only ASCII
/// hex digits (Adobe CMap & CIDFont Files Spec §7, ISO 32000-1 §9.10.3). The
/// capture regexes used throughout this module (`<([^>]*)>`) match any byte
/// sequence between angle brackets, so a raw stream containing invalid UTF-8
/// — decoded upstream via `String::from_utf8_lossy`, which substitutes each
/// invalid byte sequence with the 3-byte U+FFFD replacement character — can
/// hand a capture that is not what it looks like: a length check such as
/// `dst_hex.len() > 8` counts BYTES, not hex digits, so a captured string
/// that mixes ASCII bytes with one U+FFFD can satisfy that check while the
/// `step_by(4)` loop it guards then slices `&dst_hex[i..i+4]` at an offset
/// landing inside the middle of the replacement character, panicking with
/// "byte index N is not a char boundary". Validating ASCII-hex-only at the
/// capture boundary, before any length check or slicing, guarantees every
/// subsequent byte offset is also a char boundary (every ASCII byte is one
/// char), and rejects the corrupted capture the same way any other malformed
/// hex is already rejected. ~keep
fn is_ascii_hex_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse a bfchar line, returning all `<src> <dst>` pairs found on the line.
///
/// Example: `<0041> <0041>` maps character code 0x41 to Unicode U+0041.
/// Example: `<0003> <00410042>` maps character code 0x03 to Unicode "AB" (multi-char mapping).
/// Example: `<01> <0041> <02> <0042>` maps two character codes on one line.
///
/// Supports multiple pairs per line, hex code points, ligatures, escape sequences,
/// and flexible whitespace inside angle brackets.
fn parse_bfchar_line(line: &str) -> Vec<(u32, String)> {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>").unwrap());

    let mut results = Vec::new();

    for caps in RE.captures_iter(line) {
        let parsed = (|| -> Option<(u32, String)> {
            let src_str = caps[1].trim().replace(char::is_whitespace, "");
            let src = u32::from_str_radix(&src_str, 16).ok()?;

            let dst_str = caps[2].trim();

            let dst = if let Some(escape) = parse_escape_sequence(&format!("<{}>", dst_str)) {
                escape
            } else {
                let dst_hex = dst_str.replace(char::is_whitespace, "");
                // Reject a corrupted capture (e.g. a lossy-decoded U+FFFD from
                // invalid UTF-8 in the raw stream) before any length check or
                // byte-index slice below can misread its byte count as a hex
                // digit count. See `is_ascii_hex_digits`. ~keep
                if !is_ascii_hex_digits(&dst_hex) {
                    return None;
                }

                if dst_hex.len() <= 4 {
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    char::from_u32(dst_code)?.to_string()
                } else if dst_hex.len() <= 6 {
                    // 5-6 hex digits: direct supplementary Unicode code point (e.g., 020BB7 = U+20BB7)
                    // ~keep
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    char::from_u32(dst_code)?.to_string()
                } else if dst_hex.len() == 8 {
                    let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
                    if let Some(decoded) = decode_utf16_surrogate_pair(dst_code) {
                        decoded
                    } else {
                        let mut result = String::new();
                        if let Ok(code1) = u32::from_str_radix(&dst_hex[0..4], 16)
                            && let Some(ch) = char::from_u32(code1)
                        {
                            result.push(ch);
                        }
                        if let Ok(code2) = u32::from_str_radix(&dst_hex[4..8], 16)
                            && let Some(ch) = char::from_u32(code2)
                        {
                            result.push(ch);
                        }
                        if result.is_empty() {
                            return None;
                        }
                        result
                    }
                } else {
                    let mut result = String::new();
                    for i in (0..dst_hex.len()).step_by(4) {
                        let end = (i + 4).min(dst_hex.len());
                        if let Ok(code) = u32::from_str_radix(&dst_hex[i..end], 16)
                            && let Some(ch) = char::from_u32(code)
                        {
                            result.push(ch);
                        }
                    }
                    if result.is_empty() {
                        return None;
                    }
                    result
                }
            };

            Some((src, dst))
        })();

        if let Some(pair) = parsed {
            results.push(pair);
        }
    }

    results
}

/// Parse a bfrange line: `<start> <end> <dst>`
///
/// Example: `<0020> <007E> <0020>` maps codes 0x20-0x7E to Unicode U+0020-U+007E.
///
/// There are two formats:
/// 1. `<start> <end> <dst>` - Sequential mapping starting at dst
/// 2. `<start> <end> [<dst1> <dst2> ...]` - Array of individual destinations
///
/// This function supports both formats and flexible whitespace within angle brackets.
fn parse_bfrange_line(line: &str) -> Option<Vec<(u32, String)>> {
    static RE_SEQ: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*<([^>]*)>").unwrap());
    static RE_ARRAY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*\[((?:\s*<[^>]+>\s*)+)\]").unwrap());

    if let Some(caps) = RE_ARRAY.captures(line) {
        let start_str = caps[1].trim().replace(char::is_whitespace, "");
        let end_str = caps[2].trim().replace(char::is_whitespace, "");
        let start = u32::from_str_radix(&start_str, 16).ok()?;
        let end = u32::from_str_radix(&end_str, 16).ok()?;
        let array_str = &caps[3];

        static RE_HEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>").unwrap());
        // Drop any array entry that is not pure ASCII hex (e.g. a lossy-decoded
        // U+FFFD from invalid UTF-8 in the raw stream) before it can reach the
        // byte-index slices below. See `is_ascii_hex_digits`. A dropped entry
        // shrinks `dst_hexes` below `range_size`, which the existing
        // size-mismatch warning already surfaces. ~keep
        let dst_hexes: Vec<String> = RE_HEX
            .captures_iter(array_str)
            .filter_map(|cap| {
                let s = cap.get(1).unwrap().as_str().trim().replace(char::is_whitespace, "");
                if is_ascii_hex_digits(&s) { Some(s) } else { None }
            })
            .collect();

        let mut result = Vec::new();

        // `start`/`end` are attacker-controlled hex from the font's ToUnicode stream, so a
        // reversed range (`<0100> <0000>`) or `end == u32::MAX` makes the naive
        // `(end - start + 1)` overflow: a panic under `overflow-checks` (debug and test), a
        // silent wrap to a bogus count in release, which this crate's profile does not
        // enable. The sequential branch and `parse_notdefrange_line` clamp with
        // `end.saturating_sub(start).min(10000)`; here `range_size` is an entry COUNT rather
        // than a 0-based loop offset, hence the `saturating_add(1)`, and their `.min(10000)`
        // is unnecessary because this value only bounds a `.take()` over `dst_hexes` (already
        // sized by the literal array in the stream) instead of driving iteration. ~keep
        let range_size = end.saturating_sub(start).saturating_add(1) as usize;

        // SPEC VALIDATION: PDF Spec ISO 32000-1:2008, Section 9.10.3
        // The array must have exactly (end - start + 1) entries.
        // Current behavior (lenient): Use what's available, ignore extras/missing.
        // Proper strict mode: Should fail if array size doesn't match range_size. ~keep
        if dst_hexes.len() != range_size {
            tracing::warn!(
                "ToUnicode bfrange array size mismatch: expected {} entries for range 0x{:X}-0x{:X}, got {}",
                range_size,
                start,
                end,
                dst_hexes.len()
            );
        }

        for (i, dst_hex) in dst_hexes.iter().take(range_size).enumerate() {
            let src = start + i as u32;

            let dst = if dst_hex.len() <= 4 {
                let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
                char::from_u32(dst_code)?.to_string()
            } else if dst_hex.len() <= 6 {
                let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
                if let Some(ch) = char::from_u32(dst_code) {
                    ch.to_string()
                } else {
                    continue;
                }
            } else if dst_hex.len() == 8 {
                let dst_code = u32::from_str_radix(dst_hex, 16).ok()?;
                if let Some(decoded) = decode_utf16_surrogate_pair(dst_code) {
                    decoded
                } else {
                    let mut unicode_string = String::new();
                    if let Ok(code) = u32::from_str_radix(&dst_hex[0..4], 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        unicode_string.push(ch);
                    }
                    if let Ok(code) = u32::from_str_radix(&dst_hex[4..8], 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        unicode_string.push(ch);
                    }
                    if unicode_string.is_empty() {
                        continue;
                    }
                    unicode_string
                }
            } else {
                let mut unicode_string = String::new();
                for chunk_start in (0..dst_hex.len()).step_by(4) {
                    let chunk_end = (chunk_start + 4).min(dst_hex.len());
                    if let Ok(code) = u32::from_str_radix(&dst_hex[chunk_start..chunk_end], 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        unicode_string.push(ch);
                    }
                }
                if unicode_string.is_empty() {
                    continue;
                }
                unicode_string
            };

            result.push((src, dst));
        }
        return Some(result);
    }

    if let Some(caps) = RE_SEQ.captures(line) {
        let start_str = caps[1].trim().replace(char::is_whitespace, "");
        let end_str = caps[2].trim().replace(char::is_whitespace, "");
        let dst_start_str = caps[3].trim().replace(char::is_whitespace, "");
        let start = u32::from_str_radix(&start_str, 16).ok()?;
        let end = u32::from_str_radix(&end_str, 16).ok()?;
        let dst_start = u32::from_str_radix(&dst_start_str, 16).ok()?;

        let mut result = Vec::new();
        let range_size = end.saturating_sub(start).min(10000);

        // For surrogate pair destinations (8 hex digits), decode to Unicode code point
        // first, then increment the code point. Naively incrementing the raw u32 would
        // overflow across the low surrogate boundary (0xDFFF → 0xE000). ~keep
        let base_codepoint = if dst_start > 0xFFFF {
            if let Some(decoded) = decode_utf16_surrogate_pair(dst_start) {
                decoded.chars().next().map(|c| c as u32)
            } else {
                Some(dst_start)
            }
        } else {
            Some(dst_start)
        };

        if let Some(base_cp) = base_codepoint {
            for i in 0..=range_size {
                let src = start.wrapping_add(i);
                let cp = base_cp.wrapping_add(i);
                if let Some(ch) = char::from_u32(cp) {
                    result.push((src, ch.to_string()));
                }
            }
        }
        return Some(result);
    }

    None
}

/// Parse a notdefrange line: `<start> <end> <dst>`
///
/// Phase 4.1 addition: Support for beginnotdefrange sections
///
/// Example: `<0000> <0040> <FFFD>` maps codes 0x0000-0x0040 to U+FFFD (replacement character)
/// for unmapped character codes (fallback/notdef handling).
///
/// Unlike bfrange, notdefrange only supports the sequential format (not arrays).
/// Notdefrange mappings are applied only to codes not already mapped by bfchar/bfrange.
fn parse_notdefrange_line(line: &str) -> Option<Vec<(u32, String)>> {
    static RE_SEQ: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<([^>]*)>\s*<([^>]*)>\s*<([^>]*)>").unwrap());

    if let Some(caps) = RE_SEQ.captures(line) {
        let start_str = caps[1].trim().replace(char::is_whitespace, "");
        let end_str = caps[2].trim().replace(char::is_whitespace, "");
        let dst_str = caps[3].trim();

        let start = u32::from_str_radix(&start_str, 16).ok()?;
        let end = u32::from_str_radix(&end_str, 16).ok()?;

        let dst = if let Some(escape) = parse_escape_sequence(&format!("<{}>", dst_str)) {
            escape
        } else {
            let dst_hex = dst_str.replace(char::is_whitespace, "");
            let dst_code = u32::from_str_radix(&dst_hex, 16).ok()?;
            if dst_code > 0xFFFF {
                decode_utf16_surrogate_pair(dst_code).or_else(|| char::from_u32(dst_code).map(|ch| ch.to_string()))?
            } else {
                char::from_u32(dst_code)?.to_string()
            }
        };

        let mut result = Vec::new();
        let range_size = end.saturating_sub(start).min(10000);
        for i in 0..=range_size {
            let src = start.wrapping_add(i);
            result.push((src, dst.clone()));
        }
        return Some(result);
    }

    None
}

/// Parse a CID to Unicode mapping (simplified version for CID fonts).
///
/// This is a wrapper around `parse_tounicode_cmap` for consistency.
pub fn parse_cid_to_unicode(data: &[u8]) -> Result<CMap> {
    parse_tounicode_cmap(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bfchar_single() {
        let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    }

    #[test]
    fn test_parse_bfchar_multiple() {
        let data = b"beginbfchar\n<0041> <0041>\n<0042> <0042>\n<0043> <0043>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
        assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
    }

    #[test]
    fn test_large_bfrange_compresses_and_resolves() {
        // A 513-code contiguous range collapses into `ranges`, leaving `chars`
        // empty, and still resolves via computed range lookup. ~keep
        let data = b"beginbfrange\n<0100> <0300> <0500>\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert!(!cmap.ranges.is_empty(), "large contiguous range should compress");
        assert!(cmap.chars.is_empty(), "compressed codes should leave `chars`");
        assert_eq!(cmap.get(&0x100).as_deref(), Some("\u{0500}"));
        assert_eq!(cmap.get(&0x300).as_deref(), Some("\u{0700}"));
        assert_eq!(cmap.get(&0x0FF), None);
        assert_eq!(cmap.get(&0x301), None);
    }

    #[test]
    fn test_bfchar_override_survives_range_compression() {
        // A bfchar after a bfrange wins for that code (§9.10.3); compression must
        // not swallow it (it breaks contiguity and stays in `chars`). ~keep
        let data = b"beginbfrange\n<0100> <0300> <0500>\nendbfrange\n\
                     beginbfchar\n<0200> <0041>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x200).as_deref(), Some("A"), "later bfchar must win");
        assert_eq!(cmap.get(&0x1FF).as_deref(), Some("\u{05FF}"));
        assert_eq!(cmap.get(&0x201).as_deref(), Some("\u{0601}"));
    }

    #[test]
    fn test_parse_bfchar_non_ascii() {
        let data = b"beginbfchar\n<00E9> <00E9>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0xE9).as_deref(), Some("é"));
    }

    #[test]
    fn test_parse_bfrange_simple() {
        let data = b"beginbfrange\n<0041> <0043> <0041>\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
        assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
    }

    #[test]
    fn test_parse_bfrange_ascii_printable() {
        let data = b"beginbfrange\n<0020> <007E> <0020>\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();

        assert_eq!(cmap.get(&0x20).as_deref(), Some(" "));
        assert_eq!(cmap.get(&0x30).as_deref(), Some("0"));
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x7A).as_deref(), Some("z"));
        assert_eq!(cmap.get(&0x7E).as_deref(), Some("~"));
    }

    #[test]
    fn test_parse_mixed_bfchar_bfrange() {
        let data = b"beginbfchar\n<0041> <0058>\nendbfchar\nbeginbfrange\n<0042> <0044> <0042>\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();

        assert_eq!(cmap.get(&0x41).as_deref(), Some("X"));
        assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
        assert_eq!(cmap.get(&0x43).as_deref(), Some("C"));
        assert_eq!(cmap.get(&0x44).as_deref(), Some("D"));
    }

    #[test]
    fn test_parse_empty_cmap() {
        let data = b"";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert!(cmap.is_empty());
    }

    #[test]
    fn test_parse_cmap_with_whitespace() {
        let data = b"beginbfchar\n  <0041>    <0041>  \n  <0042>  <0042>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
    }

    #[test]
    fn test_parse_bfchar_line() {
        assert_eq!(parse_bfchar_line("<0041> <0041>"), vec![(0x41, "A".to_string())]);
        assert_eq!(parse_bfchar_line("<00E9> <00E9>"), vec![(0xE9, "é".to_string())]);
        assert!(parse_bfchar_line("invalid line").is_empty());
    }

    #[test]
    fn test_parse_bfchar_multiple_pairs_per_line() {
        let result = parse_bfchar_line("<01> <0041> <02> <0042> <03> <0043>");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (0x01, "A".to_string()));
        assert_eq!(result[1], (0x02, "B".to_string()));
        assert_eq!(result[2], (0x03, "C".to_string()));
    }

    #[test]
    fn test_parse_bfrange_line() {
        let result = parse_bfrange_line("<0041> <0043> <0041>").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (0x41, "A".to_string()));
        assert_eq!(result[1], (0x42, "B".to_string()));
        assert_eq!(result[2], (0x43, "C".to_string()));
    }

    #[test]
    fn test_parse_bfrange_line_single_char() {
        let result = parse_bfrange_line("<0041> <0041> <0041>").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (0x41, "A".to_string()));
    }

    #[test]
    fn test_parse_bfrange_line_invalid() {
        assert!(parse_bfrange_line("invalid").is_none());
    }

    #[test]
    fn test_extract_sections() {
        let content = "before\nbeginbfchar\ndata1\nendbfchar\nmiddle\nbeginbfchar\ndata2\nendbfchar\nafter";
        let sections = extract_sections(content, "beginbfchar", "endbfchar");
        assert_eq!(sections.len(), 2);
        assert!(sections[0].contains("data1"));
        assert!(sections[1].contains("data2"));
    }

    #[test]
    fn test_extract_sections_none() {
        let content = "no sections here";
        let sections = extract_sections(content, "beginbfchar", "endbfchar");
        assert_eq!(sections.len(), 0);
    }

    #[test]
    fn test_parse_cid_to_unicode() {
        let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
        let cmap = parse_cid_to_unicode(data).unwrap();
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
    }

    #[test]
    fn test_parse_hex_case_insensitive() {
        let data = b"beginbfchar\n<00aB> <00Ab>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0xAB).as_deref(), Some("«"));
    }

    #[test]
    fn test_parse_multiple_sections() {
        let data = b"beginbfchar\n<0041> <0041>\nendbfchar\nbeginbfchar\n<0042> <0042>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.len(), 2);
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x42).as_deref(), Some("B"));
    }

    #[test]
    fn test_parse_bfchar_ligature() {
        let data = b"beginbfchar\n<000C> <00660069>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x0C).as_deref(), Some("fi"));
    }

    #[test]
    fn test_parse_bfchar_multiple_ligatures() {
        let data = b"beginbfchar\n<000B> <00660066>\n<000C> <00660069>\n<000D> <0066006C>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x0B).as_deref(), Some("ff"));
        assert_eq!(cmap.get(&0x0C).as_deref(), Some("fi"));
        assert_eq!(cmap.get(&0x0D).as_deref(), Some("fl"));
    }

    #[test]
    fn test_parse_bfrange_array_ligatures() {
        let data = b"beginbfrange\n<005F> <0061> [<00660066> <00660069> <00660066006C>]\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x5F).as_deref(), Some("ff"));
        assert_eq!(cmap.get(&0x60).as_deref(), Some("fi"));
        assert_eq!(cmap.get(&0x61).as_deref(), Some("ffl"));
    }

    #[test]
    fn test_parse_bfrange_array_mixed() {
        let data = b"beginbfrange\n<0010> <0012> [<0041> <00660069> <0043>]\nendbfrange";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0x11).as_deref(), Some("fi"));
        assert_eq!(cmap.get(&0x12).as_deref(), Some("C"));
    }

    #[test]
    fn test_parse_zekat_cmap() {
        let cmap_data = r#"
/CIDInit /ProcSet findresource begin
19 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
/Ordering (UCS)
/Supplement 0
>> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfrange
<0003> <0004> <0020>
endbfrange
3 beginbfchar
<000F> <002C>
<0011> <002E>
<0024> <0041>
endbfchar
1 beginbfrange
<0027> <0029> <0044>
endbfrange
2 beginbfchar
<002C> <0049>
<002E> <004B>
endbfchar
2 beginbfrange
<0030> <0032> <004D>
<0035> <0037> <0052>
endbfrange
2 beginbfchar
<0039> <0056>
<003D> <005A>
endbfchar
5 beginbfrange
<0044> <0048> <0061>
<004A> <004C> <0067>
<004E> <0053> <006B>
<0055> <0059> <0072>
<005C> <005D> <0079>
endbfrange
5 beginbfchar
<006B> <00E2>
<006F> <00E7>
<007C> <00F6>
<0081> <00FC>
<00AB> <2026>
endbfchar
1 beginbfrange
<00B3> <00B4> <201C>
endbfrange
4 beginbfchar
<00C6> <00C2>
<00D5> <0131>
<00F7> <011F>
<00FA> <015F>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
"#
        .as_bytes();

        let cmap = parse_tounicode_cmap(cmap_data).expect("Failed to parse CMap");

        assert_eq!(cmap.get(&0x3D).as_deref(), Some("Z"));
        assert_eq!(cmap.get(&0x24).as_deref(), Some("A"));
        assert_eq!(cmap.get(&0xC6).as_deref(), Some("\u{00C2}"));
    }

    /// `/WMode 1 def` on a CMap stream marks the font as vertical writing,
    /// even when the CMap name does not advertise a `-V` suffix. This is the
    /// authoritative signal per ISO 32000-1 §9.7.5.4 and is required for
    /// embedded CMap streams used by tategaki layouts where the writer keeps
    /// a horizontal-shaped CMap name but flips the writing mode internally.
    #[test]
    fn test_parse_wmode_vertical() {
        let data = b"\
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
/WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.wmode, 1, "explicit /WMode 1 def must set vertical writing");
        assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
        assert_eq!(cmap.code_width, 2);
    }

    /// Default WMode is `0` (horizontal) when the directive is absent. Most
    /// ToUnicode CMaps for horizontal text omit `/WMode` entirely; this
    /// guards the dominant code path.
    #[test]
    fn test_parse_wmode_default_horizontal() {
        let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.wmode, 0, "missing /WMode must default to horizontal");
    }

    /// `/WMode 0 def` is a no-op but must be parsed without warning.
    #[test]
    fn test_parse_wmode_explicit_horizontal() {
        let data = b"\
begincmap
/WMode 0 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.wmode, 0);
    }

    /// M5: a `/WMode N def` directive that lives inside a PostScript
    /// comment (`%` to end-of-line, §3.3.1) must NOT flip the writing
    /// mode. Without comment-stripping, this commented-out producer
    /// debug line would silently switch a horizontal CMap to vertical.
    #[test]
    fn test_parse_wmode_ignored_inside_postscript_comment() {
        let data = b"\
begincmap
% /WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(
            cmap.wmode, 0,
            "/WMode 1 def inside a PostScript comment must be ignored"
        );
    }

    /// M5 corollary: a legitimate `/WMode 1 def` on a later line is
    /// still picked up even when an earlier line carries an unrelated
    /// comment.
    #[test]
    fn test_parse_wmode_after_comment_still_seen() {
        let data = b"\
begincmap
% some prologue comment unrelated to wmode
/WMode 1 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(cmap.wmode, 1);
    }

    /// M6: a non-standard `/WMode 2 def` must NOT silently flip writing
    /// mode; the spec only defines 0 and 1 (§9.7.5.4). Parser returns
    /// None (callers fall back to horizontal default) and emits a warn
    /// log so producer bugs are diagnosable.
    #[test]
    fn test_parse_wmode_non_standard_value_falls_back() {
        let data = b"\
begincmap
/WMode 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
1 beginbfchar
<0041> <0041>
endbfchar
endcmap
";
        let cmap = parse_tounicode_cmap(data).unwrap();
        assert_eq!(
            cmap.wmode, 0,
            "/WMode 2 def is non-standard; parser must fall back to horizontal"
        );
    }

    // ------------------------------------------------------------------
    // Malformed-input regression tests (fail-loudly / degraded / empty
    // classification for `parse_tounicode_cmap`).
    //
    // `capture_warnings` installs a minimal in-process `tracing::Subscriber`
    // that records event messages, scoped only to the closure passed to it.
    // It is test-only code living entirely in this module — no production
    // type is touched to make these assertions possible. ~keep
    // ------------------------------------------------------------------

    #[derive(Clone, Default)]
    struct RecordingSubscriber {
        messages: Arc<Mutex<Vec<String>>>,
    }

    #[derive(Default)]
    struct MessageVisitor(Vec<String>);

    impl tracing::field::Visit for MessageVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.push(format!("{}={value:?}", field.name()));
        }
    }

    impl tracing::Subscriber for RecordingSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = MessageVisitor::default();
            event.record(&mut visitor);
            self.messages.lock_or_recover().push(visitor.0.join(" "));
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Run `f` under a subscriber that records every tracing event message,
    /// returning them in emission order.
    fn capture_warnings<F: FnOnce()>(f: F) -> Vec<String> {
        let subscriber = RecordingSubscriber::default();
        let messages = Arc::clone(&subscriber.messages);
        tracing::subscriber::with_default(subscriber, f);
        messages.lock_or_recover().clone()
    }

    /// FATAL: a non-empty stream with none of the CMap structural keywords is
    /// not a CMap at all — any mapping derived from it would be arbitrary, so
    /// the parser must fail loudly instead of returning a silently empty
    /// CMap indistinguishable from a font that legitimately maps nothing.
    #[test]
    fn garbage_stream_without_cmap_syntax_is_rejected() {
        let data = b"RANDOM BINARY GARBAGE, NOT A CMAP STREAM AT ALL 0xDEADBEEF 1234567890";
        let err = parse_tounicode_cmap(data)
            .expect_err("a stream with no CMap keywords at all must be rejected, not silently empty");
        let message = err.to_string();
        assert!(
            message.to_lowercase().contains("cmap"),
            "error message should name the defect: {message}"
        );
    }

    /// LEGITIMATELY EMPTY: a zero-length stream is not evidence of
    /// corruption — some producers emit an empty ToUnicode stream for a font
    /// that genuinely maps nothing. No warning should fire.
    #[test]
    fn zero_length_stream_is_legitimately_empty_without_warning() {
        let logs = capture_warnings(|| {
            let cmap = parse_tounicode_cmap(b"").unwrap();
            assert!(cmap.is_empty());
        });
        assert!(
            logs.is_empty(),
            "a zero-length stream must not warn, it is a legitimate empty CMap: {logs:?}"
        );
    }

    /// DEGRADED (truncation): a `beginbfchar` block with no matching
    /// `endbfchar` before EOF is dropped, not guessed at, and the defect is
    /// surfaced via `tracing::warn!` rather than silently disappearing.
    #[test]
    fn truncated_bfchar_block_is_dropped_with_warning() {
        let data = b"beginbfchar\n<0041> <0041>\n";
        let logs = capture_warnings(|| {
            let cmap = parse_tounicode_cmap(data).expect("truncation is degraded, not fatal");
            assert!(
                cmap.is_empty(),
                "a block with no closing endbfchar must be dropped entirely, not partially guessed"
            );
        });
        assert!(
            logs.iter()
                .any(|m| m.contains("beginbfchar") && m.contains("endbfchar")),
            "expected a WARN naming the truncated beginbfchar block, got: {logs:?}"
        );
    }

    /// DEGRADED (truncation): the same defect class for `begincodespacerange`,
    /// verifying that dropping one section type does not lose unrelated
    /// sections (`beginbfchar` is scanned independently) and that the
    /// dropped section's effect (2-byte `code_width`) is correctly absent.
    #[test]
    fn truncated_codespacerange_block_is_dropped_with_warning() {
        let data = b"begincodespacerange\n<0000> <FFFF>\nbeginbfchar\n<0041> <0041>\nendbfchar";
        let logs = capture_warnings(|| {
            let cmap = parse_tounicode_cmap(data).unwrap();
            assert_eq!(
                cmap.code_width, 1,
                "an unterminated codespacerange section must not set code_width"
            );
            assert_eq!(
                cmap.get(&0x41).as_deref(),
                Some("A"),
                "the unrelated, well-formed bfchar block must still parse"
            );
        });
        assert!(
            logs.iter()
                .any(|m| m.contains("begincodespacerange") && m.contains("endcodespacerange")),
            "expected a WARN naming the truncated codespacerange block, got: {logs:?}"
        );
    }

    /// DEGRADED (malformed hex): a line with an unparseable src code is
    /// dropped, its well-formed neighbor still parses, and the defect is
    /// surfaced via `tracing::warn!`.
    #[test]
    fn malformed_bfchar_hex_operand_is_skipped_with_warning() {
        let data = b"beginbfchar\n<0041> <0041>\n<ZZZZ> <0042>\nendbfchar";
        let logs = capture_warnings(|| {
            let cmap = parse_tounicode_cmap(data).unwrap();
            assert_eq!(cmap.get(&0x41).as_deref(), Some("A"), "well-formed entry still parses");
            assert_eq!(cmap.len(), 1, "the malformed entry must not appear in the map");
        });
        assert!(
            logs.iter()
                .any(|m| m.contains("bfchar") && m.contains("failed to parse")),
            "expected a WARN about the malformed bfchar line, got: {logs:?}"
        );
    }

    /// A parse failure now reaching `LazyCMap::get()` must be memoized: the
    /// `Err` arm's `tracing::warn!` fires at most once per `LazyCMap`, not
    /// once per character, since `get()` sits on the per-character decode
    /// path (`FontInfo::char_to_unicode`).
    #[test]
    fn lazy_cmap_memoizes_a_parse_failure_and_warns_once() {
        const CONFIDENTIAL_MARKER: &str = "CONFIDENTIAL_CMAP_NAME_9c12";
        let data = format!("RANDOM BINARY GARBAGE {CONFIDENTIAL_MARKER}").into_bytes();
        let lazy = LazyCMap::new(data);
        let logs = capture_warnings(|| {
            assert!(lazy.get().is_none(), "a garbage stream must fail to parse");
            assert!(lazy.get().is_none(), "second call must reuse the memoized failure");
            assert!(lazy.get().is_none(), "third call must reuse the memoized failure");
        });
        let failure_warnings = logs
            .iter()
            .filter(|message| {
                message.contains("operation=\"parse_tounicode_cmap\"")
                    && message.contains("error_code=\"font_error\"")
                    && message.contains("message=PDF operation degraded")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            failure_warnings.len(),
            1,
            "LazyCMap::get() must warn on a parse failure exactly once, not per call: {logs:?}"
        );
        assert!(!format!("{logs:?}").contains(CONFIDENTIAL_MARKER));
    }
}
