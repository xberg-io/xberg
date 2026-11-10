//! The text of a ligature glyph, read from the component glyphs that the font's own GSUB ligature
//! substitutions (OpenType LookupType 4) join into it.

use skrifa::raw::tables::gsub::{LigatureSubst, SubstitutionSubtables};
use skrifa::raw::types::Tag;
use skrifa::raw::{FontRef, TableProvider};

/// The GSUB features whose ligatures stand for the letters they join. Shaping features of complex scripts
/// (such as `blwf` or `abvs`) also use ligature lookups, but their components are in glyph order, not in
/// reading order. ~keep
const LETTER_LIGATURE_FEATURES: [Tag; 4] = [
    Tag::new(b"liga"),
    Tag::new(b"clig"),
    Tag::new(b"dlig"),
    Tag::new(b"hlig"),
];

/// The text of glyph `gid` as the concatenated text of its components, from every GSUB ligature rule
/// whose output is `gid`. `component_text` gives the text of one component glyph.
///
/// Returns `None` when no rule outputs `gid`, when a component of a rule has no text, or when two rules
/// give different text, because then the font does not say which letters the glyph stands for.
pub(crate) fn ligature_text(
    font_data: &[u8],
    gid: u16,
    mut component_text: impl FnMut(u16) -> Option<String>,
) -> Option<String> {
    let mut found: Option<String> = None;
    for components in ligature_rules(font_data, gid) {
        let text = components
            .into_iter()
            .map(&mut component_text)
            .collect::<Option<String>>()?;
        match &found {
            Some(previous) if *previous != text => return None,
            Some(_) => {}
            None => found = Some(text),
        }
    }
    found
}

/// The component glyphs of every Format 1 GSUB ligature rule whose output is `gid`, in the lookups of the
/// letter ligature features. A font without a readable GSUB table has none.
fn ligature_rules(font_data: &[u8], gid: u16) -> Vec<Vec<u16>> {
    let Some(gsub) = FontRef::new(font_data).ok().and_then(|font| font.gsub().ok()) else {
        return Vec::new();
    };
    let (Ok(features), Ok(lookup_list)) = (gsub.feature_list(), gsub.lookup_list()) else {
        return Vec::new();
    };
    let mut indices: Vec<u16> = features
        .feature_records()
        .iter()
        .filter(|record| LETTER_LIGATURE_FEATURES.contains(&record.feature_tag()))
        .filter_map(|record| record.feature(features.offset_data()).ok())
        .flat_map(|feature| feature.lookup_list_indices().iter().map(|index| index.get()))
        .collect();
    indices.sort_unstable();
    indices.dedup();
    let lookups = lookup_list.lookups();
    let mut rules = Vec::new();
    for lookup in indices
        .into_iter()
        .filter_map(|index| lookups.get(usize::from(index)).ok())
    {
        let Ok(SubstitutionSubtables::Ligature(subtables)) = lookup.subtables() else {
            continue;
        };
        for subtable in subtables.iter().flatten() {
            let LigatureSubst::Format1(subst) = subtable else {
                continue;
            };
            let Ok(coverage) = subst.coverage() else {
                continue;
            };
            // Zip before dropping unreadable sets, so each set stays paired with its coverage glyph. ~keep
            for (first, set) in coverage.iter().zip(subst.ligature_sets().iter()) {
                let (Ok(set), Ok(first)) = (set, u16::try_from(first.to_u32())) else {
                    continue;
                };
                let ligatures = set.ligatures().iter().flatten();
                for ligature in ligatures.filter(|ligature| ligature.ligature_glyph().to_u16() == gid) {
                    let rest = ligature.component_glyph_ids().iter().map(|g| g.get().to_u16());
                    rules.push(std::iter::once(first).chain(rest).collect());
                }
            }
        }
    }
    rules
}
