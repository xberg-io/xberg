//! The two font families a document is set in, and the PDF objects that embed them.
//!
//! DejaVu Sans is embedded as a subset, as a `Type0` font whose codes are assigned one
//! per distinct character in first-use order. A `CIDToGIDMap` sends each code to its
//! glyph and a `ToUnicode` map sends it back to its character, so a character the font
//! has no glyph for still extracts as itself. Code is set in the standard Courier faces,
//! which are not embedded.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::sync::LazyLock;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use xberg_native_pdf::fonts::bundled::DEJAVU_SANS;
use xberg_native_pdf::fonts::encoding::unicode_to_winansi;
use xberg_native_pdf::fonts::{TrueTypeFont, subset_font_bytes};

use super::document_error;
use crate::Result;

/// Every glyph of every Courier face is 600/1000 em wide. The widths are written out
/// anyway: a reader without the standard fonts' metrics otherwise guesses them.
const COURIER_ADVANCE: f32 = 600.0;
const COURIER_FIRST_CHAR: u8 = 0x20;
const COURIER_LAST_CHAR: u8 = 0xFF;
const SANS_BASE_FONT: &str = "DejaVuSans";
const SANS_RESOURCE: &str = "F0";
/// `bfchar` operators may map at most 100 codes each.
const MAX_BFCHAR_ENTRIES: usize = 100;

static SANS: LazyLock<Option<TrueTypeFont<'static>>> = LazyLock::new(|| TrueTypeFont::parse(DEJAVU_SANS).ok());

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Face {
    Sans,
    Courier { bold: bool, italic: bool },
}

impl Face {
    pub(super) fn resource_name(self) -> &'static str {
        match self {
            Face::Sans => SANS_RESOURCE,
            Face::Courier {
                bold: false,
                italic: false,
            } => "F1",
            Face::Courier {
                bold: true,
                italic: false,
            } => "F2",
            Face::Courier {
                bold: false,
                italic: true,
            } => "F3",
            Face::Courier {
                bold: true,
                italic: true,
            } => "F4",
        }
    }

    fn courier_base_font(bold: bool, italic: bool) -> &'static str {
        match (bold, italic) {
            (false, false) => "Courier",
            (true, false) => "Courier-Bold",
            (false, true) => "Courier-Oblique",
            (true, true) => "Courier-BoldOblique",
        }
    }
}

/// Glyph advances, in thousandths of an em.
pub(super) struct Metrics {
    sans: &'static TrueTypeFont<'static>,
}

impl Metrics {
    pub(super) fn load() -> Result<Self> {
        let sans = SANS
            .as_ref()
            .ok_or_else(|| document_error("the bundled DejaVu Sans font could not be parsed"))?;
        Ok(Self { sans })
    }

    pub(super) fn advance(&self, face: Face, c: char) -> f32 {
        match face {
            Face::Courier { .. } => COURIER_ADVANCE,
            Face::Sans => f32::from(self.sans.glyph_width(self.sans.glyph_id(c as u32).unwrap_or(0))),
        }
    }

    #[cfg(test)]
    pub(super) fn has_glyph(&self, c: char) -> bool {
        self.sans.glyph_id(c as u32).is_some_and(|glyph| glyph != 0)
    }

    /// The face `c` is drawn in: a Courier face for code its encoding can hold, DejaVu
    /// Sans otherwise.
    pub(super) fn face_for(c: char, code: bool, bold: bool, italic: bool) -> Face {
        if code && winansi_byte(c).is_some() {
            Face::Courier { bold, italic }
        } else {
            Face::Sans
        }
    }
}

fn winansi_byte(c: char) -> Option<u8> {
    unicode_to_winansi(c as u32).filter(|byte| *byte >= 0x20)
}

/// Encodes text for content streams and records what each font has to embed.
pub(super) struct FontSet {
    sans: &'static TrueTypeFont<'static>,
    codes: HashMap<char, u16>,
    /// The character behind each code; code `n` is `chars[n - 1]`, code 0 is `.notdef`.
    chars: Vec<char>,
    courier: BTreeSet<Face>,
}

impl FontSet {
    pub(super) fn new(metrics: &Metrics) -> Self {
        Self {
            sans: metrics.sans,
            codes: HashMap::new(),
            chars: Vec::new(),
            courier: BTreeSet::new(),
        }
    }

    /// `text` as the bytes of a string operand for `face`.
    pub(super) fn encode(&mut self, face: Face, text: &str) -> Vec<u8> {
        match face {
            Face::Sans => text.chars().flat_map(|c| self.sans_code(c).to_be_bytes()).collect(),
            Face::Courier { .. } => {
                self.courier.insert(face);
                text.chars().map(|c| winansi_byte(c).unwrap_or(b'?')).collect()
            }
        }
    }

    fn sans_code(&mut self, c: char) -> u16 {
        if let Some(code) = self.codes.get(&c) {
            return *code;
        }
        // Two-byte codes run out after 65,535 distinct characters; the rest draw and
        // extract as `.notdef` rather than failing the document.
        let Ok(code) = u16::try_from(self.chars.len() + 1) else {
            return 0;
        };
        self.chars.push(c);
        self.codes.insert(c, code);
        code
    }

    /// Add the font objects to `document` and return the `/Font` resource dictionary.
    pub(super) fn into_resources(self, document: &mut Document) -> Result<Dictionary> {
        let mut fonts = Dictionary::new();
        if !self.chars.is_empty() {
            fonts.set(SANS_RESOURCE, self.embed_sans(document)?);
        }
        for face in &self.courier {
            if let Face::Courier { bold, italic } = *face {
                let widths = (COURIER_FIRST_CHAR..=COURIER_LAST_CHAR)
                    .map(|_| Object::Integer(COURIER_ADVANCE as i64))
                    .collect::<Vec<_>>();
                let font = document.add_object(dictionary! {
                    "Type" => "Font",
                    "Subtype" => "Type1",
                    "BaseFont" => Face::courier_base_font(bold, italic),
                    "Encoding" => "WinAnsiEncoding",
                    "FirstChar" => i64::from(COURIER_FIRST_CHAR),
                    "LastChar" => i64::from(COURIER_LAST_CHAR),
                    "Widths" => widths,
                });
                fonts.set(face.resource_name(), font);
            }
        }
        Ok(fonts)
    }

    fn embed_sans(&self, document: &mut Document) -> Result<ObjectId> {
        let sans = self.sans;
        let glyphs: Vec<u16> = self
            .chars
            .iter()
            .map(|c| sans.glyph_id(*c as u32).unwrap_or(0))
            .collect();
        let used: BTreeSet<u16> = glyphs.iter().copied().filter(|glyph| *glyph != 0).collect();
        let (subset, remapper) = subset_font_bytes(DEJAVU_SANS, 0, &used).map_err(document_error)?;

        let mut cid_to_gid = vec![0u8, 0u8];
        for glyph in &glyphs {
            let new_glyph = remapper.get(*glyph).unwrap_or(0);
            cid_to_gid.extend_from_slice(&new_glyph.to_be_bytes());
        }
        let widths: Vec<Object> = glyphs
            .iter()
            .map(|glyph| Object::Integer(i64::from(sans.glyph_width(*glyph))))
            .collect();

        let base_font = format!("{}+{SANS_BASE_FONT}", subset_tag(&used));
        let scale = |units: i16| i64::from(units) * 1000 / i64::from(sans.units_per_em().max(1));
        let (x_min, y_min, x_max, y_max) = sans.bbox();

        let font_file = document.add_object(Stream::new(dictionary! { "Length1" => subset.len() as i64 }, subset));
        let descriptor = document.add_object(dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => Object::Name(base_font.clone().into_bytes()),
            "Flags" => i64::from(sans.font_flags()),
            "FontBBox" => vec![scale(x_min).into(), scale(y_min).into(), scale(x_max).into(), scale(y_max).into()],
            "ItalicAngle" => 0,
            "Ascent" => scale(sans.ascender()),
            "Descent" => scale(sans.descender()),
            "CapHeight" => scale(sans.cap_height().unwrap_or(sans.ascender())),
            "StemV" => i64::from(sans.stem_v()),
            "FontFile2" => font_file,
        });
        let cid_to_gid_map = document.add_object(Stream::new(Dictionary::new(), cid_to_gid));
        let descendant = document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "CIDFontType2",
            "BaseFont" => Object::Name(base_font.clone().into_bytes()),
            "CIDSystemInfo" => dictionary! {
                "Registry" => Object::string_literal("Adobe"),
                "Ordering" => Object::string_literal("Identity"),
                "Supplement" => 0,
            },
            "FontDescriptor" => descriptor,
            "DW" => i64::from(sans.glyph_width(0)),
            "W" => vec![Object::Integer(1), Object::Array(widths)],
            "CIDToGIDMap" => cid_to_gid_map,
        });
        let to_unicode = document.add_object(Stream::new(Dictionary::new(), self.to_unicode_cmap().into_bytes()));
        Ok(document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type0",
            "BaseFont" => Object::Name(base_font.into_bytes()),
            "Encoding" => "Identity-H",
            "DescendantFonts" => vec![descendant.into()],
            "ToUnicode" => to_unicode,
        }))
    }

    fn to_unicode_cmap(&self) -> String {
        let mut cmap = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
             /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
             /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
             1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
        );
        for (chunk_index, chunk) in self.chars.chunks(MAX_BFCHAR_ENTRIES).enumerate() {
            let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
            for (offset, c) in chunk.iter().enumerate() {
                let code = chunk_index * MAX_BFCHAR_ENTRIES + offset + 1;
                let _ = write!(cmap, "<{code:04X}> <");
                for unit in c.encode_utf16(&mut [0; 2]) {
                    let _ = write!(cmap, "{unit:04X}");
                }
                cmap.push_str(">\n");
            }
            cmap.push_str("endbfchar\n");
        }
        cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        cmap
    }
}

/// A subset font's name carries a six-letter tag unique to its glyph set (ISO 32000-1
/// §9.6.4). FNV-1a keeps the tag the same across builds and platforms.
fn subset_tag(glyphs: &BTreeSet<u16>) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for glyph in glyphs {
        for byte in glyph.to_be_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    (0..6)
        .map(|_| {
            let letter = char::from(b'A' + (hash % 26) as u8);
            hash /= 26;
            letter
        })
        .collect()
}
