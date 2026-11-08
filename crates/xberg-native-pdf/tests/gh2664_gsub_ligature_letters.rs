//! GH#2664: the letters of a ligature glyph with no /ToUnicode entry were dropped.
//!
//! The fixture is a Type0 Identity-H font over a hand-built TrueType program. It draws
//! "potential" with one glyph for "ti". The /ToUnicode map covers every glyph except the
//! ligature, the program's `post` table carries no glyph names (format 3) and its `cmap`
//! has no entry for the ligature, as producers of such subsets write them. The font's
//! GSUB ligature table names the component glyphs of the ligature.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

/// Glyph IDs of the font program. The CID of each is the glyph ID plus `Fixture::cid_offset`.
const P: u16 = 1;
const O: u16 = 2;
const T: u16 = 3;
const E: u16 = 4;
const N: u16 = 5;
const I: u16 = 6;
const A: u16 = 7;
const L: u16 = 8;
const LIG_TI: u16 = 9;
const NUM_GLYPHS: u16 = 10;

/// The letters each component glyph stands for, in the font `cmap` and in /ToUnicode.
const LETTERS: [(u16, char); 8] = [
    (P, 'p'),
    (O, 'o'),
    (T, 't'),
    (E, 'e'),
    (N, 'n'),
    (I, 'i'),
    (A, 'a'),
    (L, 'l'),
];

/// "potential", with "ti" drawn as the ligature glyph.
const DRAWN: [u16; 8] = [P, O, T, E, N, LIG_TI, A, L];

/// One variant of the fixture font and its /ToUnicode map.
struct Fixture {
    /// GSUB ligature rules: (component glyphs, ligature glyph). Empty means no GSUB table.
    ligatures: Vec<(Vec<u16>, u16)>,
    /// /ToUnicode entries beyond the letters, as (glyph, text).
    extra_to_unicode: Vec<(u16, &'static str)>,
    /// A letter glyph whose /ToUnicode entry is replaced by this text, or left out for `None`.
    changed_letter: Option<(u16, Option<&'static str>)>,
    /// Added to each glyph ID to make its CID. Nonzero writes an explicit CIDToGIDMap stream.
    cid_offset: u16,
    /// Writes an explicit CIDToGIDMap stream with only this many entries. A CID past its end keeps
    /// its own value as the glyph ID.
    short_map: Option<usize>,
    /// The tag of the GSUB feature that references the ligature lookup.
    feature: &'static [u8; 4],
}

impl Fixture {
    fn standard() -> Self {
        Self {
            ligatures: vec![(vec![T, I], LIG_TI)],
            extra_to_unicode: Vec::new(),
            changed_letter: None,
            cid_offset: 0,
            short_map: None,
            feature: b"liga",
        }
    }
}

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

fn len16(v: usize) -> [u8; 2] {
    be16(u16::try_from(v).expect("fixture table fits u16"))
}

/// One box outline, used for every glyph except .notdef, so the page renders with ink.
fn box_glyph() -> Vec<u8> {
    let mut glyf: Vec<u8> = Vec::new();
    for v in [1u16, 50, 0, 450, 700, 3, 0] {
        glyf.extend(be16(v));
    }
    glyf.extend([0x01, 0x01, 0x01, 0x01]);
    for dx in [50i16, 400, 0, -400] {
        glyf.extend(dx.to_be_bytes());
    }
    for dy in [0i16, 0, 700, 0] {
        glyf.extend(dy.to_be_bytes());
    }
    glyf
}

/// A `cmap` with one format 4 subtable (3, 1) that maps each letter to its glyph.
fn cmap_table() -> Vec<u8> {
    let mut letters: Vec<(u16, u16)> = LETTERS.iter().map(|&(gid, ch)| (ch as u16, gid)).collect();
    letters.sort_unstable();
    let mut ends: Vec<u16> = letters.iter().map(|&(code, _)| code).collect();
    let mut starts = ends.clone();
    let mut deltas: Vec<u16> = letters.iter().map(|&(code, gid)| gid.wrapping_sub(code)).collect();
    ends.push(0xFFFF);
    starts.push(0xFFFF);
    deltas.push(1);
    let seg_count = ends.len();
    let entry_selector = usize::BITS - 1 - seg_count.leading_zeros();
    let search_range = 2usize << entry_selector;
    let mut sub: Vec<u8> = Vec::new();
    sub.extend(be16(4));
    sub.extend(len16(16 + 8 * seg_count));
    sub.extend(be16(0));
    sub.extend(len16(seg_count * 2));
    sub.extend(len16(search_range));
    sub.extend(len16(entry_selector as usize));
    sub.extend(len16(seg_count * 2 - search_range));
    ends.iter().for_each(|&v| sub.extend(be16(v)));
    sub.extend(be16(0));
    starts.iter().for_each(|&v| sub.extend(be16(v)));
    deltas.iter().for_each(|&v| sub.extend(be16(v)));
    (0..seg_count).for_each(|_| sub.extend(be16(0)));
    let mut cmap: Vec<u8> = Vec::new();
    for v in [0u16, 1, 3, 1] {
        cmap.extend(be16(v));
    }
    cmap.extend(be32(12));
    cmap.extend(sub);
    cmap
}

/// A GSUB table with one LookupType 4 lookup holding every rule in one Format 1 subtable, referenced by one
/// feature with the tag `feature`.
fn gsub_table(rules: &[(Vec<u16>, u16)], feature: &[u8; 4]) -> Vec<u8> {
    let mut firsts: Vec<u16> = rules.iter().map(|(comps, _)| comps[0]).collect();
    firsts.sort_unstable();
    firsts.dedup();
    let sets: Vec<Vec<u8>> = firsts
        .iter()
        .map(|&first| {
            let ligs: Vec<Vec<u8>> = rules
                .iter()
                .filter(|(comps, _)| comps[0] == first)
                .map(|(comps, glyph)| {
                    let mut lig = be16(*glyph).to_vec();
                    lig.extend(len16(comps.len()));
                    comps[1..].iter().for_each(|&c| lig.extend(be16(c)));
                    lig
                })
                .collect();
            let mut set = len16(ligs.len()).to_vec();
            let mut at = 2 + 2 * ligs.len();
            for lig in &ligs {
                set.extend(len16(at));
                at += lig.len();
            }
            ligs.iter().for_each(|lig| set.extend(lig));
            set
        })
        .collect();
    let mut coverage = be16(1).to_vec();
    coverage.extend(len16(firsts.len()));
    firsts.iter().for_each(|&g| coverage.extend(be16(g)));

    let mut subst = be16(1).to_vec();
    let header = 6 + 2 * sets.len();
    subst.extend(len16(header));
    subst.extend(len16(sets.len()));
    let mut at = header + coverage.len();
    for set in &sets {
        subst.extend(len16(at));
        at += set.len();
    }
    subst.extend(&coverage);
    sets.iter().for_each(|set| subst.extend(set));

    let mut lookup: Vec<u8> = Vec::new();
    for v in [4u16, 0, 1, 8] {
        lookup.extend(be16(v));
    }
    lookup.extend(&subst);

    // Header (10 bytes), an empty ScriptList at 10, the FeatureList at 12 (one record and its Feature table,
    // 14 bytes), the LookupList at 26 with the lookup 4 bytes past it. ~keep
    let mut gsub: Vec<u8> = Vec::new();
    for v in [1u16, 0, 10, 12, 26, 0, 1] {
        gsub.extend(be16(v));
    }
    gsub.extend(feature);
    for v in [8u16, 0, 1, 0, 1, 4] {
        gsub.extend(be16(v));
    }
    gsub.extend(lookup);
    gsub
}

/// The font program: box glyphs, a letters-only `cmap`, `post` format 3, and the GSUB rules.
fn font_program(ligatures: &[(Vec<u16>, u16)], feature: &[u8; 4]) -> Vec<u8> {
    let outline = box_glyph();
    let mut glyf: Vec<u8> = Vec::new();
    // Short loca (offset / 2): .notdef is empty, then one box per glyph. ~keep
    let mut loca: Vec<u8> = Vec::new();
    loca.extend(be16(0));
    loca.extend(be16(0));
    for _ in 1..NUM_GLYPHS {
        glyf.extend(&outline);
        loca.extend(len16(glyf.len() / 2));
    }

    let mut head: Vec<u8> = Vec::new();
    head.extend(be32(0x0001_0000));
    head.extend(be32(0));
    head.extend(be32(0));
    head.extend(be32(0x5F0F_3CF5));
    head.extend(be16(0));
    head.extend(be16(1000));
    head.extend([0u8; 16]);
    for v in [0u16, 0, 450, 700, 0, 8, 2, 0, 0] {
        head.extend(be16(v));
    }

    let mut hhea: Vec<u8> = Vec::new();
    hhea.extend(be32(0x0001_0000));
    hhea.extend(be16(800));
    hhea.extend((-200i16).to_be_bytes());
    for v in [0u16, 500, 0, 0, 450, 1, 0, 0] {
        hhea.extend(be16(v));
    }
    hhea.extend([0u8; 8]);
    hhea.extend(be16(0));
    hhea.extend(be16(NUM_GLYPHS));

    let mut hmtx: Vec<u8> = Vec::new();
    for _ in 0..NUM_GLYPHS {
        hmtx.extend(be16(500));
        hmtx.extend(be16(0));
    }

    let mut maxp: Vec<u8> = Vec::new();
    maxp.extend(be32(0x0001_0000));
    maxp.extend(be16(NUM_GLYPHS));
    maxp.extend([0u8; 26]);

    let mut post: Vec<u8> = Vec::new();
    post.extend(be32(0x0003_0000));
    post.extend([0u8; 28]);

    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = Vec::new();
    if !ligatures.is_empty() {
        tables.push((b"GSUB", gsub_table(ligatures, feature)));
    }
    tables.extend([
        (b"cmap", cmap_table()),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca),
        (b"maxp", maxp),
        (b"post", post),
    ]);
    common::build_sfnt(&tables)
}

fn utf16_hex(text: &str) -> String {
    text.encode_utf16().map(|u| format!("{u:04X}")).collect()
}

fn to_unicode_cmap(fixture: &Fixture) -> Vec<u8> {
    let letters = LETTERS.iter().filter_map(|&(gid, ch)| match fixture.changed_letter {
        Some((changed, text)) if changed == gid => text.map(|text| (gid, text.to_string())),
        _ => Some((gid, ch.to_string())),
    });
    let extra = fixture
        .extra_to_unicode
        .iter()
        .map(|&(gid, text)| (gid, text.to_string()));
    let entries: Vec<String> = letters
        .chain(extra)
        .map(|(gid, text)| format!("<{:04X}> <{}>", gid + fixture.cid_offset, utf16_hex(&text)))
        .collect();
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
         {} beginbfchar\n{}\nendbfchar\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend",
        entries.len(),
        entries.join("\n")
    )
    .into_bytes()
}

/// Append object `offsets.len()` with `dict`, and with `stream` data when given.
fn push_object(pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, dict: &str, stream: Option<&[u8]>) {
    let id = offsets.len();
    offsets.push(pdf.len());
    match stream {
        None => pdf.extend_from_slice(format!("{id} 0 obj\n{dict}\nendobj\n").as_bytes()),
        Some(data) => {
            pdf.extend_from_slice(format!("{id} 0 obj\n<< /Length {}{dict} >>\nstream\n", data.len()).as_bytes());
            pdf.extend_from_slice(data);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }
    }
}

/// A one-page PDF that draws "potential" in the fixture font.
fn fixture_pdf(fixture: &Fixture) -> Vec<u8> {
    let font = font_program(&fixture.ligatures, fixture.feature);
    let codes: String = DRAWN
        .iter()
        .map(|g| format!("{:04X}", g + fixture.cid_offset))
        .collect();
    let content = format!("BT /F1 24 Tf 20 100 Td <{codes}> Tj ET");
    let to_unicode = to_unicode_cmap(fixture);
    let font_stream_keys = format!(" /Length1 {}", font.len());
    // CID -> GID as big-endian pairs; CIDs below the offset map to .notdef. ~keep
    let mut cid_to_gid: Vec<u8> = (0..fixture.cid_offset)
        .map(|_| 0u16)
        .chain(0..NUM_GLYPHS)
        .flat_map(be16)
        .collect();
    if let Some(entries) = fixture.short_map {
        cid_to_gid.truncate(entries * 2);
    }
    let explicit_map = fixture.cid_offset != 0 || fixture.short_map.is_some();
    let cid_to_gid_ref = if explicit_map { "10 0 R" } else { "/Identity" };
    let cid_font = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /AAAAAA+LigSans \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /FontDescriptor 7 0 R /CIDToGIDMap {cid_to_gid_ref} /DW 500 >>"
    );
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize];
    let objects: [(&str, Option<&[u8]>); 10] = [
        ("<< /Type /Catalog /Pages 2 0 R >>", None),
        ("<< /Type /Pages /Kids [3 0 R] /Count 1 >>", None),
        (
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            None,
        ),
        ("", Some(content.as_bytes())),
        (
            "<< /Type /Font /Subtype /Type0 /BaseFont /AAAAAA+LigSans /Encoding /Identity-H \
             /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>",
            None,
        ),
        (&cid_font, None),
        (
            "<< /Type /FontDescriptor /FontName /AAAAAA+LigSans /Flags 32 /FontBBox [0 -200 500 800] \
             /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 9 0 R >>",
            None,
        ),
        ("", Some(&to_unicode)),
        (&font_stream_keys, Some(&font)),
        ("", Some(&cid_to_gid)),
    ];
    for (dict, stream) in objects {
        push_object(&mut pdf, &mut offsets, dict, stream);
    }
    common::finalize_pdf(pdf, &offsets)
}

fn extract(fixture: &Fixture) -> String {
    let doc = PdfDocument::from_bytes(fixture_pdf(fixture)).expect("parse fixture");
    doc.extract_text(0).expect("extract page 0")
}

#[test]
fn the_fixture_font_paints_its_glyphs() {
    // A fixture whose font program does not parse would decode nothing and prove nothing. ~keep
    let doc = PdfDocument::from_bytes(fixture_pdf(&Fixture::standard())).expect("parse fixture");
    let image = PageRenderer::new(RenderOptions::default())
        .render_page(&doc, 0)
        .expect("render page");
    let luma = image::load_from_memory(&image.data).expect("decode render").to_luma8();
    let inked = luma.pixels().filter(|pixel| pixel.0[0] < 250).count();
    assert!(inked > 0, "the fixture font painted nothing");
}

#[test]
fn the_letters_of_an_unmapped_ligature_come_from_its_gsub_components() {
    let text = extract(&Fixture::standard());
    assert!(
        text.contains("potential"),
        "the ligature glyph lost its letters: {text:?} (expected \"potential\")"
    );
}

#[test]
fn without_a_gsub_table_no_letters_are_invented() {
    let fixture = Fixture {
        ligatures: Vec::new(),
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(text.contains("poten"), "control letters missing: {text:?}");
    assert!(
        !text.contains("ti"),
        "letters appeared with no source for them: {text:?}"
    );
}

#[test]
fn a_component_without_a_to_unicode_entry_gives_no_partial_letters() {
    let fixture = Fixture {
        changed_letter: Some((I, None)),
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(text.contains("poten"), "control letters missing: {text:?}");
    assert!(
        !text.contains("potent"),
        "a ligature with an unmapped component produced part of its letters: {text:?}"
    );
}

#[test]
fn a_to_unicode_entry_for_the_ligature_wins_over_gsub() {
    let fixture = Fixture {
        extra_to_unicode: vec![(LIG_TI, "TI")],
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(
        text.contains("potenTIal"),
        "the /ToUnicode entry of the ligature did not decide its text: {text:?}"
    );
}

#[test]
fn two_rules_that_disagree_on_the_letters_give_none() {
    let fixture = Fixture {
        ligatures: vec![(vec![T, I], LIG_TI), (vec![P, O], LIG_TI)],
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(text.contains("poten"), "control letters missing: {text:?}");
    assert!(
        !text.contains("ti") && !text.contains("enpo"),
        "an ambiguous ligature took the letters of one rule: {text:?}"
    );
}

#[test]
fn a_component_mapped_to_a_placeholder_gives_no_letters() {
    let fixture = Fixture {
        changed_letter: Some((I, Some("\u{FFFD}"))),
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(text.contains("poten"), "control letters missing: {text:?}");
    assert!(
        !text.contains("potent"),
        "a ligature took a placeholder component as a letter: {text:?}"
    );
}

#[test]
fn components_map_through_an_explicit_cid_to_gid_map() {
    let fixture = Fixture {
        cid_offset: 100,
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(
        text.contains("potential"),
        "the ligature lost its letters when CIDs differ from glyph IDs: {text:?}"
    );
}

#[test]
fn components_past_the_end_of_a_short_cid_to_gid_map_keep_their_cid_as_glyph() {
    let fixture = Fixture {
        short_map: Some(usize::from(T)),
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(
        text.contains("potential"),
        "the ligature lost its letters when its components lie past the CIDToGIDMap: {text:?}"
    );
}

#[test]
fn a_shaping_form_ligature_gives_no_letters() {
    let fixture = Fixture {
        feature: b"blwf",
        ..Fixture::standard()
    };
    let text = extract(&fixture);
    assert!(text.contains("poten"), "control letters missing: {text:?}");
    assert!(
        !text.contains("potential"),
        "a ligature from a shaping feature took the letters of its glyph-order components: {text:?}"
    );
}
