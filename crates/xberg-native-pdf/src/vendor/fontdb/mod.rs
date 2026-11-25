// Derived from fontdb 0.24.0 (https://github.com/RazrFalcon/fontdb),
// Copyright (c) 2020 Yevhenii Reizner, MIT licensed. Modified: the inlined
// ttf_parser-based parser was replaced with skrifa/read-fonts, `log` call
// sites were converted to `tracing`, and the unsafe memory-mapped file path
// (`Source::SharedFile`, `Database::make_shared_face_data`) was removed
// entirely — it was never reachable from this crate and is the only unsafe
// entry point upstream fontdb has. The slotmap-backed `ID` was replaced with
// a plain `Vec` index, since nothing in this crate ever calls
// `Database::remove_face`/`push_face_info` (the only operations that need
// generational-index safety). See `src/vendor/fontdb/LICENSE` for the full
// licence text and `ATTRIBUTIONS.md` for the summary. ~keep

//! Vendored subset of `fontdb` 0.24.0: an in-memory font database with
//! CSS Fonts Level 4 style matching, system-font directory discovery, and
//! (on Linux) fontconfig-driven generic-family + directory configuration.
//!
//! This is not a full port. Only what `crate::rendering::text_rasterizer`
//! actually calls is vendored:
//!
//! - `Database`, `ID`, `Query`, `Family`, `Style`, `Stretch`, `Weight`,
//!   `Source`, `FaceInfo` — same names and shapes as upstream fontdb 0.24,
//!   so the two consumer modules only had to update their `use` paths.
//! - `FaceInfo::families` is `Vec<(String, Option<String>)>` (name, BCP-47
//!   language tag) rather than upstream's `Vec<(String, ttf_parser::
//!   Language)>` — see `parse::parse_families` for why that's a semantic
//!   (not byte-for-byte) equivalent of upstream's algorithm.
//! - `FaceInfo::post_script_name` and `FaceInfo::monospaced` are dropped:
//!   nothing in this crate reads either field.
//! - `Source::SharedFile` and the `unsafe`/`memmap2`-backed sharing API
//!   (`make_shared_face_data`/`make_face_data_unshared`) are dropped: the
//!   memmap path is the only unsafe code upstream fontdb has, and nothing
//!   here calls it (`SharedFile` is only ever constructed behind that path).
//!   Without the map, the system-font scan reads only each file's table
//!   directories and the four tables the face parse needs (`scan`), rather
//!   than the whole file, which is what the map achieves upstream.
//! - Windows system-font discovery is ported (plain `std::fs` directory
//!   walking, no registry access — upstream fontdb doesn't touch the
//!   registry either).

mod matching;
mod parse;
mod scan;
mod system_fonts;

use std::path::PathBuf;
use std::sync::Arc;

/// A unique per-`Database` face ID.
///
/// Upstream fontdb backs this with a `slotmap` generational key so that
/// `Database::remove_face` can free a slot without invalidating other IDs.
/// This crate never removes faces (grepped: zero calls to `remove_face` or
/// `push_face_info` anywhere in the codebase), so a plain index into
/// `Database::faces` is sufficient and avoids the `slotmap`/`tinyvec`
/// dependencies entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ID(usize);

/// A font database.
pub(crate) struct Database {
    faces: Vec<FaceInfo>,
    family_serif: String,
    family_sans_serif: String,
    family_cursive: String,
    family_fantasy: String,
    family_monospace: String,
}

impl Database {
    /// Creates a new, empty `Database` with the same hard-coded generic
    /// family defaults as upstream fontdb (overridden by fontconfig on
    /// Linux — see `system_fonts::load_fontconfig`).
    pub(crate) fn new() -> Self {
        Database {
            faces: Vec::new(),
            family_serif: "Times New Roman".to_string(),
            family_sans_serif: "Arial".to_string(),
            family_cursive: "Comic Sans MS".to_string(),
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            family_fantasy: "Impact".to_string(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            family_fantasy: "Papyrus".to_string(),
            family_monospace: "Courier New".to_string(),
        }
    }

    /// Loads font data (raw TrueType/OpenType/collection bytes) into the
    /// `Database`. Loads every face in case of a font collection.
    ///
    /// Only called from the `cjk-render-fallback` feature's bundled-font
    /// injection path (and its tests) — dead under `--no-default-features`
    /// by design, not an oversight. ~keep
    #[cfg_attr(not(feature = "cjk-render-fallback"), allow(dead_code))]
    pub(crate) fn load_font_data(&mut self, data: Vec<u8>) {
        self.load_font_source(Source::Binary(Arc::new(data)));
    }

    #[cfg_attr(not(feature = "cjk-render-fallback"), allow(dead_code))]
    fn load_font_source(&mut self, source: Source) {
        let source_for_log = source.clone();
        source.with_data(|data| {
            for (index, font_result) in skrifa::raw::FontRef::fonts(data).enumerate() {
                let index = index as u32;
                let parsed = match font_result {
                    Ok(font) => parse::parse_face_info(&font, source_for_log.clone(), index),
                    Err(_) => Err(parse::LoadError::Malformed),
                };
                match parsed {
                    Ok(info) => self.faces.push(info),
                    Err(error) => tracing::warn!(
                        target: "xberg_native_pdf::vendor::fontdb",
                        face_index = index,
                        %error,
                        "failed to load a font face from source; skipping",
                    ),
                }
            }
        });
    }

    /// Returns the generic family name or the `Family::Name` itself.
    fn family_name<'a>(&'a self, family: &'a Family<'a>) -> &'a str {
        match family {
            Family::Name(name) => name,
            Family::Serif => self.family_serif.as_str(),
            Family::SansSerif => self.family_sans_serif.as_str(),
            Family::Cursive => self.family_cursive.as_str(),
            Family::Fantasy => self.family_fantasy.as_str(),
            Family::Monospace => self.family_monospace.as_str(),
        }
    }

    /// Performs a CSS-like query and returns the best matched font face.
    pub(crate) fn query(&self, query: &Query) -> Option<ID> {
        for family in query.families {
            let name = self.family_name(family);
            let candidates: Vec<(ID, &FaceInfo)> = self
                .faces
                .iter()
                .enumerate()
                .filter(|(_, face)| face.families.iter().any(|(family_name, _)| family_name == name))
                .map(|(index, info)| (ID(index), info))
                .collect();

            if !candidates.is_empty()
                && let Some(id) = matching::find_best_match(&candidates, query)
            {
                return Some(id);
            }
        }

        None
    }

    /// Selects a `FaceInfo` by `id`.
    pub(crate) fn face(&self, id: ID) -> Option<&FaceInfo> {
        self.faces.get(id.0)
    }

    /// Returns font face storage and the face index by `ID`.
    fn face_source(&self, id: ID) -> Option<(Source, u32)> {
        self.face(id).map(|info| (info.source.clone(), info.index))
    }

    /// Executes a closure with a font's data. Returns `None` when the
    /// backing file failed to load (`Source::File`) or the ID is unknown.
    pub(crate) fn with_face_data<P, T>(&self, id: ID, p: P) -> Option<T>
    where
        P: FnOnce(&[u8], u32) -> T,
    {
        let (src, face_index) = self.face_source(id)?;
        src.with_data(|data| p(data, face_index))
    }

    /// Sets the family used by `Family::Serif`. Only called from
    /// `system_fonts::load_fontconfig`, hence the matching `cfg`: on other
    /// targets these would otherwise be dead code. ~keep
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn set_serif_family<S: Into<String>>(&mut self, family: S) {
        self.family_serif = family.into();
    }

    /// Sets the family used by `Family::SansSerif`.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn set_sans_serif_family<S: Into<String>>(&mut self, family: S) {
        self.family_sans_serif = family.into();
    }

    /// Sets the family used by `Family::Cursive`.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn set_cursive_family<S: Into<String>>(&mut self, family: S) {
        self.family_cursive = family.into();
    }

    /// Sets the family used by `Family::Fantasy`.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn set_fantasy_family<S: Into<String>>(&mut self, family: S) {
        self.family_fantasy = family.into();
    }

    /// Sets the family used by `Family::Monospace`.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn set_monospace_family<S: Into<String>>(&mut self, family: S) {
        self.family_monospace = family.into();
    }
}

/// A single font face's info. A single item of the `Database`.
#[derive(Debug, Clone)]
pub(crate) struct FaceInfo {
    /// A font source.
    ///
    /// Multiple `FaceInfo` objects reference the same `Source` in case of
    /// font collections.
    pub(crate) source: Source,

    /// A face index in `source`.
    pub(crate) index: u32,

    /// `(name, BCP-47 language tag)` pairs. The first entry is English
    /// (language tag starting `"en"`) when the font has one, matching
    /// upstream fontdb's "English US first" ordering. See the module-level
    /// doc comment for why this isn't byte-for-byte identical to upstream.
    pub(crate) families: Vec<(String, Option<String>)>,

    /// A font face style.
    pub(crate) style: Style,

    /// A font face weight.
    pub(crate) weight: Weight,

    /// A font face stretch.
    pub(crate) stretch: Stretch,
}

/// A font source: either a file path or raw in-memory bytes.
///
/// Upstream fontdb also has `Source::SharedFile`, produced only by the
/// `unsafe`, `memmap`-feature-gated `Database::make_shared_face_data`. This
/// crate never calls that method, so the variant (and the `unsafe` it would
/// require to vendor) is dropped. ~keep
#[derive(Clone)]
pub(crate) enum Source {
    /// A font's path on disk. `load_system_fonts` is the only producer, and
    /// it does no filesystem walking on wasm32 or Android (no target directory
    /// list matches — see its doc comment), so this variant is structurally
    /// unconstructed there. ~keep
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    File(PathBuf),
    /// A font's raw data. Only constructed via `Database::load_font_data`,
    /// which is itself gated on `cjk-render-fallback` — see its doc comment. ~keep
    #[cfg_attr(not(feature = "cjk-render-fallback"), allow(dead_code))]
    Binary(Arc<Vec<u8>>),
}

impl core::fmt::Debug for Source {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Source::File(path) => f.debug_tuple("File").field(path).finish(),
            Source::Binary(data) => f.debug_tuple("Binary").field(&data.len()).finish(),
        }
    }
}

impl Source {
    fn with_data<P, T>(&self, p: P) -> Option<T>
    where
        P: FnOnce(&[u8]) -> T,
    {
        match self {
            Source::File(path) => std::fs::read(path).ok().map(|data| p(&data)),
            Source::Binary(data) => Some(p(data)),
        }
    }
}

/// A database query. Mainly used by `Database::query()`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Query<'a> {
    /// A prioritized list of font family names or generic family names.
    pub(crate) families: &'a [Family<'a>],
    /// The weight of glyphs in the font.
    pub(crate) weight: Weight,
    /// Selects a normal, condensed, or expanded face from a font family.
    pub(crate) stretch: Stretch,
    /// Allows italic or oblique faces to be selected.
    pub(crate) style: Style,
}

/// A [font family](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#propdef-font-family).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// `Cursive`/`Fantasy`/`Monospace` are unconstructed here: this crate only ever asks for a
// concrete `Name` or the two generics it maps PDF base-14 fonts onto. They are part of
// upstream fontdb's public `Family` API and are kept verbatim so this vendored copy stays
// diffable against upstream -- deleting them would diverge the file for no gain. This is a
// deliberate allow, not a stale one. ~keep
#[allow(dead_code)]
pub(crate) enum Family<'a> {
    /// The name of a font family of choice.
    Name(&'a str),
    /// Serif fonts represent the formal text style for a script.
    Serif,
    /// Low-contrast fonts with plain stroke endings.
    SansSerif,
    /// Informal, handwriting-like fonts.
    Cursive,
    /// Primarily decorative or expressive fonts.
    Fantasy,
    /// All glyphs share a single fixed advance width.
    Monospace,
}

/// The weight of glyphs in a font, their degree of blackness or stroke
/// thickness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Weight(pub(crate) u16);

impl Default for Weight {
    fn default() -> Weight {
        Weight::NORMAL
    }
}

impl Weight {
    /// Normal (400).
    pub(crate) const NORMAL: Weight = Weight(400);
    /// Medium weight (500, higher than normal). Never constructed by a
    /// caller directly, but `matching::find_best_match` needs it for the
    /// CSS Fonts Level 4 400/500 tie-break. ~keep
    pub(crate) const MEDIUM: Weight = Weight(500);
    /// Bold weight (700).
    pub(crate) const BOLD: Weight = Weight(700);
}

/// Allows italic or oblique faces to be selected.
///
/// `Oblique` is never constructed by a `Query` in this crate (only
/// `Normal`/`Italic` are), but real installed fonts do parse to `Oblique` —
/// dropping the variant would silently change which face
/// `matching::find_best_match`'s CSS fallback ordering picks whenever a
/// family only has an Oblique face. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Style {
    /// A face that is neither italic nor obliqued.
    #[default]
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A typically-sloped version of the regular face.
    Oblique,
}

/// Selects a normal, condensed, or expanded face from a font family. Mirrors
/// upstream fontdb's re-export of `ttf_parser::Width`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum Stretch {
    /// 50% of normal width.
    UltraCondensed,
    /// 62.5% of normal width.
    ExtraCondensed,
    /// 75% of normal width.
    Condensed,
    /// 87.5% of normal width.
    SemiCondensed,
    /// 100% width.
    #[default]
    Normal,
    /// 112.5% of normal width.
    SemiExpanded,
    /// 125% of normal width.
    Expanded,
    /// 150% of normal width.
    ExtraExpanded,
    /// 200% of normal width.
    UltraExpanded,
}

impl Stretch {
    fn to_number(self) -> u16 {
        match self {
            Stretch::UltraCondensed => 1,
            Stretch::ExtraCondensed => 2,
            Stretch::Condensed => 3,
            Stretch::SemiCondensed => 4,
            Stretch::Normal => 5,
            Stretch::SemiExpanded => 6,
            Stretch::Expanded => 7,
            Stretch::ExtraExpanded => 8,
            Stretch::UltraExpanded => 9,
        }
    }
}
