//! Font handling and encoding.
//!
//! This module provides font dictionary parsing, encoding handling,
//! and ToUnicode CMap parsing for accurate text extraction.
//!
//! # PDF Creation
//!
//! For PDF creation with embedded fonts, this module also provides:
//! - `truetype_parser` - Parse TTF/OTF fonts for embedding
//! - `font_subsetter` - Subset fonts to reduce file size
//! - `encoding` - Unicode encoding support for CID fonts

mod adobe_glyph_list;
/// CFF font encoding parser for extracting built-in encoding from CFF FontFile data.
pub mod bundled;
pub mod cff_encoding;
pub mod character_mapper;
/// Embedded `/Encoding` CMap stream parser (charcode → CID), distinct from
/// `cmap`'s `/ToUnicode` (charcode → Unicode string) parser.
///
/// `pub` so [`FontInfo::embedded_cid_map`](font_dict::FontInfo::embedded_cid_map)
/// (a `pub` field, matching every other `FontInfo` field) can name
/// [`cid_cmap::CidCMap`] in its type — external crates can hold/clone/compare
/// the value or set it to `None`, but every constructor and lookup method
/// stays `pub(crate)`, so the type is opaque outside this crate in practice.
pub mod cid_cmap;
/// CID to Unicode mappings for predefined Adobe CJK character collections.
pub mod cid_mappings;
pub mod cmap;
pub mod cmap_injector;
pub mod encoding;
pub mod encoding_normalizer;
pub mod font_dict;
pub mod font_subsetter;
pub mod form_fallback;
/// Process-level cross-document font cache for batch processing.
pub mod global_cache;
pub mod non_text_detection;
/// Adobe predefined CIDFont base-name registry for substitution when the PDF
/// references one of the Adobe-Japan1 / Adobe-GB1 / Adobe-CNS1 / Adobe-Korea1
/// faces (Ryumin-Light, GothicBBB-Medium, STSong-Light, …) without embedding
/// glyph outlines. ISO 32000-2 §9.7.5.2 mandates support for these collections.
pub mod predefined_cidfont;
pub mod provenance;
mod radical_forms;
/// TrueType font CMap parsing for glyph-to-character mapping.
pub mod truetype_cmap;
/// TrueType/OpenType font parser for PDF embedding.
pub mod truetype_parser;
/// Type 1 font encoding parser for extracting built-in encoding from FontFile data.
pub mod type1_encoding;
/// The one glyph decoder shared by extraction and rendering.
pub(crate) mod unicode_decode;
/// System Unicode font fallback for office round-trip.
pub mod unicode_fallback;

pub use character_mapper::{CharacterMapper, PredefinedCMapConfig};
pub use cmap::{CMap, LazyCMap, parse_tounicode_cmap};
pub use encoding::UnicodeEncoder;
pub use encoding_normalizer::EncodingNormalizer;
pub use font_dict::{CIDSystemInfo, CIDToGIDMap, Encoding, FontInfo, VerticalMetrics};
pub use font_subsetter::{FontSubsetter, GlyphRemapper, SubsetError, subset_font_bytes};
pub use non_text_detection::{CharacterConfidence, ConfidenceReason, NonTextDetector, NonTextStats};
pub use provenance::MappingProvenance;
pub use truetype_cmap::TrueTypeCMap;
pub use truetype_parser::{FontMetrics, TrueTypeError, TrueTypeFont, TrueTypeResult};
