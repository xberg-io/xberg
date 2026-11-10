//! List identity and the number Word paints for a `.doc` paragraph.
//!
//! [MS-DOC] keeps a list's definitions in `PlfLst` (`LSTF`s and their `LVL`
//! levels) and the per-paragraph bindings in `PlfLfo` (`LFO`s and their
//! `LFOData` overrides), which a paragraph's `sprmPIlfo` indexes. This module
//! resolves what #2857 needs to paint numbers: which list (`lsid`) a
//! paragraph's `ilfo` names, whether its level paints numbers or a bullet,
//! and the number itself, counted per [MS-DOC] "Determining Level Number of a
//! Paragraph" -- the walk that keeps a sequence running across the headings
//! between a list's paragraphs.

use ahash::AHashMap;

use super::papx::{ListBinding, read_fc_lcb};

/// `FibRgFcLcb97` pair holding `fcPlfLst` -- the list definition table.
const FIB_FC_LCB_IDX_PLF_LST: usize = 73;

/// `FibRgFcLcb97` pair holding `fcPlfLfo` -- the list format overrides that
/// `ilfo` indexes into.
const FIB_FC_LCB_IDX_PLF_LFO: usize = 74;

/// `LSTF` record size in the `PlfLst` array.
const LSTF_LEN: usize = 28;

/// `LFO` record size in the `PlfLfo` array.
const LFO_LEN: usize = 16;

/// `LVLF` header size, before the two grpprls and the `Xst` that follow it.
const LVLF_LEN: usize = 28;

/// A list level with `nfc` 23 paints a bullet glyph rather than a number;
/// 255 paints nothing. Everything else is an ordered numbering scheme
/// ([MS-DOC] 2.9.131 `MSONFC`). ~keep
const NFC_BULLET: u8 = 23;
const NFC_NONE: u8 = 255;

/// `MSONFC` 0 paints plain decimal digits -- the one numbering scheme whose
/// label this extractor paints, because a renderer can express `n.` exactly.
const NFC_DECIMAL: u8 = 0;

/// [MS-DOC] bounds `LVLF.iStartAt` and `LFOLVL.iStartAt` to `0..=0x7FFF`
/// (2.9.27, 2.9.144); outside that range a start value is malformed and the
/// level paints no number rather than a guessed one.
const START_AT_MAX: i32 = 0x7FFF;

/// Number of `LVL` records a non-simple list carries, one per outline level.
const LVL_COUNT_MULTILEVEL: usize = 9;

/// A paragraph's place in an automatic list, with the number Word paints.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) struct DocListMembership {
    /// Zero-based nesting depth (`ilvl`).
    pub level: u8,
    /// Whether the level paints numbers rather than bullets, resolved from the
    /// list tables' `nfc`.
    pub ordered: bool,
    /// The `lsid` of the list the paragraph belongs to ([MS-DOC] "Determining
    /// List Formatting of a Paragraph"). Paragraphs sharing an `lsid` share a
    /// number sequence however many `ilfo`s name it and whatever paragraphs
    /// sit between them; distinct `lsid`s are independent lists. Default 0:
    /// the identity of lists whose tables could not be read, which paint no
    /// number.
    #[serde(default)]
    pub list_id: u32,
    /// The number Word paints for the paragraph, when its level's label is
    /// the decimal `N.` shape this extractor resolves. Counted per [MS-DOC]
    /// "Determining Level Number of a Paragraph" across the whole main story,
    /// so it runs on across the headings between a list's paragraphs.
    /// `None` for bullets, non-decimal formats, unreadable tables.
    #[serde(default)]
    pub ordinal: Option<u32>,
}

/// One list level's base formatting, from its `LVL` in `PlfLst` ([MS-DOC]
/// 2.9.28 `LVL`, 2.9.27 `LVLF`).
#[derive(Debug, Clone, Copy)]
pub(super) struct LevelFormat {
    /// `LVLF.nfc` -- the numbering scheme the level paints.
    pub nfc: u8,
    /// `LVLF.iStartAt` -- the beginning number of the level's sequence.
    pub start_at: i32,
    /// `LVLF.fNoRestart` -- when set, [`LevelFormat::restart_lim`] decides
    /// which more-significant levels reset this level's sequence; when
    /// clear, any more-significant level does.
    pub no_restart: bool,
    /// `LVLF.ilvlRestartLim`, clamped to this level's index as the spec
    /// requires.
    pub restart_lim: u8,
    /// The level's `Xst` is exactly this level's placeholder followed by a
    /// period -- the one label shape the extractor paints (`N.` decimal).
    pub decimal_label: bool,
}

/// What an `LFO`'s `LFOLVL` overrides for one level ([MS-DOC] 2.9.144).
#[derive(Debug, Clone, Copy, Default)]
struct LfoLevelOverride {
    /// `LFOLVL.iStartAt` when `fStartAt` is set: the start value the first
    /// paragraph bound to that `ilfo` at that level paints.
    start_at: Option<u32>,
    /// `LFOLVL.fFormatting` -- an embedded `LVL` replaces the level's
    /// formatting. The embedded level is parsed to be walked past; its
    /// formatting is not painted, so the level's number falls back to no
    /// label rather than a guessed one.
    formatting: bool,
}

/// The `LFO` an `ilfo` names: the list (`lsid`) it points at and its level
/// overrides ([MS-DOC] 2.9.143).
#[derive(Debug, Default)]
struct LfoEntry {
    lsid: u32,
    levels: AHashMap<u8, LfoLevelOverride>,
}

/// The effective list binding of one paragraph: the `LFO`'s list identity,
/// the base `LVL` format of its level, and the level overrides its `ilfo`
/// carries.
#[derive(Debug, Clone, Copy)]
struct EffectiveLevel {
    /// The `lsid` of the list the paragraph belongs to. Paragraphs sharing
    /// an `lsid` share a number sequence, however many `ilfo`s name it.
    lsid: u32,
    /// The level's base `LVL` format. `None` when the list tables do not
    /// describe the level.
    format: Option<LevelFormat>,
    /// The `ilfo`'s start override for this level, if any.
    start_override: Option<u32>,
    /// The `ilfo` replaces the level's formatting with an embedded `LVL`.
    formatting_override: bool,
}

/// The main story's list counters, advanced once per list-bound paragraph in
/// document order ([MS-DOC] "Determining Level Number of a Paragraph").
///
/// The walk runs in the paragraph walk, not the FKP index build: FKPs are
/// physical pages whose order is not the document's, and a membership lookup
/// must not advance anything -- a dropped or repeated lookup would shift the
/// numbers of every later paragraph.
#[derive(Default)]
pub(super) struct NumberingState {
    /// Per `(lsid, ilvl)`: the number the last paragraph of that list and
    /// level painted. Absent: the sequence has not (re)started, so the next
    /// paragraph paints the level's start value.
    counters: AHashMap<(u32, u8), u32>,
    /// `(ilfo, ilvl)` pairs whose `LFOLVL.fStartAt` override has been
    /// applied. A start override applies once, at the overriding `ilfo`'s
    /// first paragraph at that level; later paragraphs advance from it.
    started: ahash::AHashSet<(u16, u8)>,
}

/// Whether a list level paints numbers or bullets, and what number, resolved
/// through `ilfo` -> `LFO` -> `lsid` -> `LSTF` -> `LVL[ilvl]`.
///
/// `ElementKind::ListItem` requires an `ordered` flag, so resolving `nfc` is
/// not optional detail: emitting every list-bound paragraph as ordered would
/// mislabel every bulleted list. Painting the number itself additionally
/// needs the counter walk [`ListFormats::membership_for`] runs once per
/// paragraph, in document order. ~keep
#[derive(Default)]
pub(super) struct ListFormats {
    /// Per list (`lsid`), each level's base `LVL` format, indexed by `ilvl`.
    levels_by_lsid: AHashMap<u32, Vec<LevelFormat>>,
    /// Per `ilfo` (1-based index into `PlfLfo`): the list it names and its
    /// level overrides.
    overrides_by_ilfo: AHashMap<u16, LfoEntry>,
}

impl ListFormats {
    pub(super) fn build(table_stream: &[u8], word_doc: &[u8], rg_fc_lcb_offset: usize) -> Self {
        let mut formats = Self::default();

        let Some((lst_fc, lst_lcb)) = read_fc_lcb(word_doc, rg_fc_lcb_offset, FIB_FC_LCB_IDX_PLF_LST) else {
            return formats;
        };
        let Some((lfo_fc, lfo_lcb)) = read_fc_lcb(word_doc, rg_fc_lcb_offset, FIB_FC_LCB_IDX_PLF_LFO) else {
            return formats;
        };
        // `lcbPlfLst` covers only the `cLst` count and the `LSTF` array; the
        // variable-length `LVL` blocks follow immediately *after* it in the
        // table stream. Slicing to `lcb` cuts every `LVL` off, which yields an
        // empty format map whose absence the membership default then silently
        // papers over -- a bulleted list would report as ordered. ~keep
        let (Some(plf_lst), Some(plf_lfo)) = (table_stream.get(lst_fc..), table_stream.get(lfo_fc..lfo_fc + lfo_lcb))
        else {
            return formats;
        };

        formats.levels_by_lsid = parse_plf_lst(plf_lst, lst_lcb);
        formats.parse_plf_lfo(plf_lfo);
        formats
    }

    /// Parse `PlfLfo`: `cLfo` as u32, then that many fixed 16-byte `LFO`
    /// records, then one variable-length `LFOData` per record, in order
    /// ([MS-DOC] 2.9.142). `ilfo` is 1-based.
    fn parse_plf_lfo(&mut self, plf_lfo: &[u8]) {
        if plf_lfo.len() < 4 {
            return;
        }
        let c_lfo = u32::from_le_bytes([plf_lfo[0], plf_lfo[1], plf_lfo[2], plf_lfo[3]]) as usize;
        let mut data_at = 4 + c_lfo * LFO_LEN;
        for i in 0..c_lfo {
            let at = 4 + i * LFO_LEN;
            let Some(record) = plf_lfo.get(at..at + LFO_LEN) else {
                break;
            };
            let lsid = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
            // `clfolvl` counts the `LFOLVL`s in this record's `LFOData`; the
            // rest of the `LFO` is ignored fields. ~keep
            let clfolvl = usize::from(record[12]);
            let Ok(ilfo) = u16::try_from(i + 1) else {
                break;
            };
            let mut entry = LfoEntry { lsid, levels: AHashMap::new() };

            // `LFOData` begins with the CP of the list's first paragraph.
            // Some producers pad it with extra 0xFFFFFFFF dwords (a Word
            // quirk LibreOffice works around the same way), so dwords are
            // stepped over only where overrides follow -- never past a
            // record that has none, which would eat the next record's CP.
            let Some(cp_at) = read_u32_at(plf_lfo, data_at).map(|(_, next)| next) else {
                break;
            };
            data_at = cp_at;
            if clfolvl > 0 {
                while read_u32_at(plf_lfo, data_at).is_some_and(|(value, _)| value == 0xFFFF_FFFF) {
                    data_at += 4;
                }
            }
            for _ in 0..clfolvl {
                let Some((i_start_at, next)) = read_i32_at(plf_lfo, data_at) else {
                    break;
                };
                let Some(&bits) = plf_lfo.get(next) else {
                    break;
                };
                // `LFOLVL`: `iStartAt` (4 bytes), then one byte of `iLvl`
                // (low 4 bits), `fStartAt` (0x10) and `fFormatting` (0x20),
                // then 3 ignored bytes ([MS-DOC] 2.9.144).
                let ilvl = bits & 0x0F;
                let f_start_at = bits & 0x10 != 0;
                let f_formatting = bits & 0x20 != 0;
                let mut after = next + 4;
                if f_formatting
                    && let Some((_, embedded)) = read_level(plf_lfo, after, ilvl)
                {
                    after = embedded;
                }
                data_at = after;
                if ilvl > 8 {
                    continue;
                }
                entry.levels.insert(ilvl, LfoLevelOverride {
                    start_at: (f_start_at && (0..=START_AT_MAX).contains(&i_start_at))
                        .then(|| i_start_at as u32),
                    formatting: f_formatting,
                });
            }

            self.overrides_by_ilfo.insert(ilfo, entry);
        }
    }

    /// The base formats of a list's levels, for the reset walk.
    fn levels_of(&self, lsid: u32) -> &[LevelFormat] {
        self.levels_by_lsid.get(&lsid).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Resolve the list a paragraph's `ilfo`/`ilvl` binds to. `None` means
    /// the `ilfo` names no `LFO`; the paragraph then keeps its binding but
    /// paints no number and joins no sequence.
    fn effective_level(&self, ilfo: u16, ilvl: u8) -> Option<EffectiveLevel> {
        let entry = self.overrides_by_ilfo.get(&ilfo)?;
        let format = self.levels_by_lsid.get(&entry.lsid).and_then(|levels| levels.get(usize::from(ilvl))).copied();
        let level_override = entry.levels.get(&ilvl).copied();
        Some(EffectiveLevel {
            lsid: entry.lsid,
            format,
            start_override: level_override.and_then(|lvl| lvl.start_at),
            formatting_override: level_override.is_some_and(|lvl| lvl.formatting),
        })
    }

    /// Membership and the painted number for a paragraph bound to a list,
    /// advancing `state`'s counters.
    ///
    /// [MS-DOC] "Determining Level Number of a Paragraph": a paragraph's
    /// number lives in its list's (`lsid`'s) sequence for its level, not in
    /// the paragraph. The sequence starts at the level's effective start
    /// value -- the `LFOLVL.fStartAt` override on the paragraph's `ilfo`'s
    /// first paragraph at the level, else `LVLF.iStartAt` -- and each further
    /// paragraph of the same list and level advances it. A paragraph of a
    /// more-significant level restarts the deeper sequences its level's
    /// `LVLF` says it restarts. Headings are not levels of any list: the
    /// sequences they sit between simply continue. ~keep
    pub(super) fn membership_for(&self, binding: ListBinding, state: &mut NumberingState) -> Option<DocListMembership> {
        let level = binding.ilvl;
        let Some(effective) = self.effective_level(binding.ilfo, level) else {
            // The `ilfo` names no `LFO`: the file is invalid ([MS-DOC]
            // "Determining List Formatting of a Paragraph" part 1 step 3).
            // The paragraph keeps the membership shape #1550 gave it --
            // ordered by default -- but paints no number.
            return Some(DocListMembership { level, ordered: true, list_id: 0, ordinal: None });
        };

        let format = effective.format;
        // `nfc` 255 is not "an unordered list": it paints no marker at all --
        // it is how Word encodes a paragraph bound to a list level with no
        // number sequence, such as one removed from a list. Such a paragraph
        // is prose (or its styled heading) to the reader, not a list item;
        // a bullet's 23 keeps its membership.
        if format.is_some_and(|format| format.nfc == NFC_NONE) {
            return None;
        }
        let ordered = format.map_or(true, |format| format.nfc != NFC_BULLET);

        // Restart the deeper sequences of the same list that this paragraph's
        // level interrupts: level `S` restarts when this level is more
        // significant than `S`'s restart threshold ([MS-DOC] 2.9.27
        // `fNoRestart`/`ilvlRestartLim`). A shallower paragraph of another
        // list, or a heading, touches nothing.
        for (deeper, deeper_format) in self
            .levels_of(effective.lsid)
            .iter()
            .enumerate()
            .skip(usize::from(level) + 1)
        {
            let threshold = if deeper_format.no_restart { deeper_format.restart_lim } else { deeper as u8 };
            if level < threshold {
                state.counters.remove(&(effective.lsid, u8::try_from(deeper).unwrap_or(u8::MAX)));
            }
        }

        // The painted number: the level's sequence -- started at its
        // effective start value by the first paragraph of the overriding
        // `ilfo`, advanced by every paragraph since the last restart --
        // provided the label is the one shape this extractor paints.
        let ordinal = format
            .filter(|format| {
                format.nfc == NFC_DECIMAL && format.decimal_label && (0..=START_AT_MAX).contains(&format.start_at)
            })
            .filter(|_| !effective.formatting_override)
            .map(|format| {
                let key = (effective.lsid, level);
                let first_use = state.started.insert((binding.ilfo, level));
                let value = match (first_use, effective.start_override) {
                    // The override's first paragraph paints the overridden
                    // start value itself, not an advance from the base.
                    (true, Some(start)) => start,
                    _ => state
                        .counters
                        .get(&key)
                        .copied()
                        .map_or(format.start_at as u32, |count| count.saturating_add(1)),
                };
                state.counters.insert(key, value);
                value
            });

        Some(DocListMembership { level, ordered, list_id: effective.lsid, ordinal })
    }
}

/// Map each list's `lsid` to its per-level formats.
///
/// `PlfLst` is `cLst` as a 16-bit count, then that many fixed-size `LSTF`
/// records, then the variable-length `LVL` blocks for each list in the same
/// order -- one for a simple list, nine otherwise. The `LVL`s must be walked
/// in sequence because each one's length depends on its own header.
fn parse_plf_lst(plf_lst: &[u8], lcb: usize) -> AHashMap<u32, Vec<LevelFormat>> {
    let mut levels_by_lsid = AHashMap::new();
    if plf_lst.len() < 2 {
        return levels_by_lsid;
    }
    let c_lst = usize::from(u16::from_le_bytes([plf_lst[0], plf_lst[1]]));
    // The declared lcb must actually hold the LSTF array it claims to.
    if 2 + c_lst * LSTF_LEN > lcb {
        return levels_by_lsid;
    }

    let mut lists: Vec<(u32, usize)> = Vec::with_capacity(c_lst);
    for i in 0..c_lst {
        let at = 2 + i * LSTF_LEN;
        let Some(lstf) = plf_lst.get(at..at + LSTF_LEN) else {
            return levels_by_lsid;
        };
        let lsid = u32::from_le_bytes([lstf[0], lstf[1], lstf[2], lstf[3]]);
        let f_simple_list = lstf[26] & 0x01 != 0;
        lists.push((lsid, if f_simple_list { 1 } else { LVL_COUNT_MULTILEVEL }));
    }

    let mut pos = 2 + c_lst * LSTF_LEN;
    for (lsid, level_count) in lists {
        let mut levels = Vec::with_capacity(level_count);
        for level in 0..level_count {
            let Some((format, next)) = read_level(plf_lst, pos, u8::try_from(level).unwrap_or(u8::MAX)) else {
                return levels_by_lsid;
            };
            levels.push(format);
            pos = next;
        }
        levels_by_lsid.insert(lsid, levels);
    }

    levels_by_lsid
}

/// Read one variable-length `LVL` record at `pos`: its `LVLF` fixed fields,
/// then the two grpprls the header sizes, then the `Xst` ([MS-DOC] 2.9.28).
///
/// Returns the level's format and the position one past the record. `level`
/// is the record's zero-based index within its list, which clamps
/// `ilvlRestartLim` and identifies the single-placeholder decimal label.
fn read_level(blocks: &[u8], pos: usize, level: u8) -> Option<(LevelFormat, usize)> {
    let lvlf = blocks.get(pos..pos + LVLF_LEN)?;
    let start_at = i32::from_le_bytes([lvlf[0], lvlf[1], lvlf[2], lvlf[3]]);
    let nfc = lvlf[4];
    let no_restart = lvlf[5] & 0x08 != 0;
    let cb_grpprl_chpx = usize::from(lvlf[24]);
    let cb_grpprl_papx = usize::from(lvlf[25]);
    let restart_lim = lvlf[26].min(level);
    let after_grpprls = pos + LVLF_LEN + cb_grpprl_chpx + cb_grpprl_papx;

    // Xst: a 16-bit character count followed by that many UTF-16 units. The
    // decimal label shape is exactly this level's placeholder character
    // (0x0000 + level) followed by a period; every other template -- bullets,
    // composite multilevel labels, other schemes -- paints no number here.
    let cch = read_u16_at(blocks, after_grpprls)?;
    let chars_at = after_grpprls + 2;
    let first = read_u16_at(blocks, chars_at);
    let second = (cch >= 2).then(|| read_u16_at(blocks, chars_at + 2)).flatten();
    let decimal_label = cch == 2 && first == Some(u16::from(level)) && second == Some(u16::from(b'.'));
    Some((LevelFormat { nfc, start_at, no_restart, restart_lim, decimal_label }, chars_at + usize::from(cch) * 2))
}

/// Read a little-endian `u16` at `at`.
fn read_u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let chunk = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([chunk[0], chunk[1]]))
}

/// Read a little-endian `u32` at `at`.
fn read_u32_at(bytes: &[u8], at: usize) -> Option<(u32, usize)> {
    let chunk = bytes.get(at..at + 4)?;
    Some((u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]), at + 4))
}

/// Read a little-endian `i32` at `at`, returning it and the position past it.
fn read_i32_at(bytes: &[u8], at: usize) -> Option<(i32, usize)> {
    let (raw, next) = read_u32_at(bytes, at)?;
    Some((raw as i32, next))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- The number walk, over synthetic list tables (#2857) ---
    //
    // The fixtures for #2857 carry no LFO overrides, so the override walk and
    // the restart rules have no binary to exercise them; these build the
    // tables by hand. The [MS-DOC] layouts they encode: `PlfLst` = count,
    // `LSTF`s, then each list's `LVL`s; `PlfLfo` = count, `LFO`s, then each
    // `LFO`'s `LFOData` (a CP, then its `LFOLVL`s).

    /// One synthetic `LVL`: an `LVLF` with the given start value and `nfc`,
    /// then an `Xst` of the level's placeholder and a period.
    fn synthetic_lvl(start_at: i32, nfc: u8, level: u8) -> Vec<u8> {
        let mut lvlf = [0u8; LVLF_LEN];
        lvlf[..4].copy_from_slice(&start_at.to_le_bytes());
        lvlf[4] = nfc;
        let mut out = lvlf.to_vec();
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&(u16::from(level)).to_le_bytes());
        out.extend_from_slice(&(u16::from(b'.')).to_le_bytes());
        out
    }

    /// A synthetic `PlfLst` for one multilevel list: `LVL_COUNT_MULTILEVEL`
    /// decimal levels that each start at `start_at`.
    fn synthetic_plf_lst(lsid: u32, start_at: i32) -> Vec<u8> {
        let mut out = vec![0x01, 0x00];
        let mut lstf = [0u8; LSTF_LEN];
        lstf[..4].copy_from_slice(&lsid.to_le_bytes());
        out.extend_from_slice(&lstf);
        for level in 0..LVL_COUNT_MULTILEVEL {
            out.extend_from_slice(&synthetic_lvl(start_at, NFC_DECIMAL, u8::try_from(level).unwrap_or(u8::MAX)));
        }
        out
    }

    /// A synthetic `PlfLfo` with one `LFO` per entry, every one naming
    /// `lsid`, each with an `LFOLVL` start override for level 0 when `Some`.
    /// The layout is all `LFO` records first, then each one's `LFOData`.
    fn synthetic_plf_lfo(lsid: u32, overrides: &[Option<i32>]) -> Vec<u8> {
        let mut out = vec![0, 0, 0, 0];
        out[..4].copy_from_slice(&u32::try_from(overrides.len()).unwrap_or(u32::MAX).to_le_bytes());
        for start_override in overrides {
            let mut lfo = [0u8; LFO_LEN];
            lfo[..4].copy_from_slice(&lsid.to_le_bytes());
            lfo[12] = u8::from(start_override.is_some());
            out.extend_from_slice(&lfo);
        }
        for start_override in overrides {
            out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
            if let Some(start) = start_override {
                out.extend_from_slice(&start.to_le_bytes());
                out.push(0x10); // iLvl 0, fStartAt, not fFormatting
                out.extend_from_slice(&[0; 3]);
            }
        }
        out
    }

    /// `ListFormats` over synthetic blocks the FIB stub points at.
    fn synthetic_formats(plf_lst: &[u8], plf_lfo: &[u8]) -> ListFormats {
        let lfo_at = 0x1000;
        let rg = 0x40;
        let mut word_doc = vec![0u8; rg + FIB_FC_LCB_IDX_PLF_LFO * 8 + 8];
        for (index, (fc, block)) in [
            (FIB_FC_LCB_IDX_PLF_LST, (0usize, plf_lst)),
            (FIB_FC_LCB_IDX_PLF_LFO, (lfo_at, plf_lfo)),
        ] {
            let at = rg + index * 8;
            word_doc[at..at + 4].copy_from_slice(&(fc as u32).to_le_bytes());
            word_doc[at + 4..at + 8].copy_from_slice(&(block.len() as u32).to_le_bytes());
        }
        let mut table = vec![0u8; lfo_at + plf_lfo.len()];
        table[..plf_lst.len()].copy_from_slice(plf_lst);
        table[lfo_at..].copy_from_slice(plf_lfo);
        ListFormats::build(&table, &word_doc, rg)
    }

    /// Walk the given paragraphs, all bound to `ilfo` 1 at the given level,
    /// and collect the ordinals they paint.
    fn painted(formats: &ListFormats, paragraphs: &[(u16, u8)]) -> Vec<u32> {
        let mut state = NumberingState::default();
        paragraphs
            .iter()
            .filter_map(|&(ilfo, ilvl)| formats.membership_for(ListBinding { ilfo, ilvl }, &mut state))
            .filter_map(|membership| membership.ordinal)
            .collect()
    }

    #[test]
    fn a_start_override_applies_at_the_overriding_ilfos_first_paragraph() {
        let formats = synthetic_formats(&synthetic_plf_lst(7, 1), &synthetic_plf_lfo(7, &[Some(5)]));
        assert_eq!(
            painted(&formats, &[(1, 0), (1, 0), (1, 0)]),
            vec![5, 6, 7],
            "the override starts the sequence at 5 once; later paragraphs advance it"
        );
    }

    #[test]
    fn two_ilfos_sharing_a_list_continue_its_sequence() {
        let formats = synthetic_formats(&synthetic_plf_lst(7, 1), &synthetic_plf_lfo(7, &[None, None]));
        assert_eq!(
            painted(&formats, &[(1, 0), (2, 0), (1, 0)]),
            vec![1, 2, 3],
            "an lsid's sequence is the list; the ilfo only carries overrides"
        );
    }

    #[test]
    fn a_shallower_paragraph_of_the_same_list_restarts_deeper_levels() {
        let formats = synthetic_formats(&synthetic_plf_lst(7, 1), &synthetic_plf_lfo(7, &[None]));
        assert_eq!(
            painted(&formats, &[(1, 0), (1, 1), (1, 1), (1, 0), (1, 1)]),
            vec![1, 1, 2, 2, 1],
            "the level-1 sequence starts at 1, advances, and restarts after the deeper \
             interrupting level-0 paragraph"
        );
    }

    #[test]
    fn a_level_with_no_number_sequence_is_not_a_list_item() {
        // `nfc` 255 and an empty `Xst`: Word paints no marker for the level.
        let mut out = vec![0x01, 0x00];
        let mut lstf = [0u8; LSTF_LEN];
        lstf[..4].copy_from_slice(&7u32.to_le_bytes());
        out.extend_from_slice(&lstf);
        let mut lvlf = [0u8; LVLF_LEN];
        lvlf[..4].copy_from_slice(&1i32.to_le_bytes());
        lvlf[4] = NFC_NONE;
        out.extend_from_slice(&lvlf);
        out.extend_from_slice(&0u16.to_le_bytes());

        let formats = synthetic_formats(&out, &synthetic_plf_lfo(7, &[None]));
        let mut state = NumberingState::default();
        assert!(
            formats.membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state).is_none(),
            "a binding whose level paints no marker is prose (or its styled heading), not a list item"
        );
    }

    #[test]
    fn an_ilfo_past_the_lfo_table_keeps_membership_but_paints_no_number() {
        let formats = synthetic_formats(&synthetic_plf_lst(7, 1), &[0, 0, 0, 0]);
        let mut state = NumberingState::default();
        let membership = formats
            .membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state)
            .expect("an invalid ilfo still leaves the paragraph list-bound");
        assert!(membership.ordered, "no tables read for the ilfo: ordered by default, as before #2857");
        assert_eq!(membership.ordinal, None, "no LFO: no sequence, no painted number");
    }

    #[test]
    fn a_formatting_override_is_walked_past_and_later_overrides_still_apply() {
        // LFOLVL[0] replaces level 0's formatting with an embedded LVL;
        // LFOLVL[1] starts level 1 at 9. A walk that misreads the embedded
        // LVL's variable length would read LFOLVL[1] from the wrong bytes.
        let mut plf_lst = vec![0x01, 0x00];
        let mut lstf = [0u8; LSTF_LEN];
        lstf[..4].copy_from_slice(&7u32.to_le_bytes());
        plf_lst.extend_from_slice(&lstf);
        for level in 0..LVL_COUNT_MULTILEVEL {
            plf_lst.extend_from_slice(&synthetic_lvl(1, NFC_DECIMAL, u8::try_from(level).unwrap_or(u8::MAX)));
        }

        let mut plf_lfo = vec![1, 0, 0, 0];
        let mut lfo = [0u8; LFO_LEN];
        lfo[..4].copy_from_slice(&7u32.to_le_bytes());
        lfo[12] = 2; // clfolvl
        plf_lfo.extend_from_slice(&lfo);
        plf_lfo.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        // LFOLVL[0]: fFormatting, embedded LVL follows.
        plf_lfo.extend_from_slice(&0i32.to_le_bytes());
        plf_lfo.push(0x20);
        plf_lfo.extend_from_slice(&[0; 3]);
        plf_lfo.extend_from_slice(&synthetic_lvl(3, NFC_DECIMAL, 0));
        // LFOLVL[1]: fStartAt 9 for level 1.
        plf_lfo.extend_from_slice(&9i32.to_le_bytes());
        plf_lfo.push(0x11); // iLvl 1, fStartAt
        plf_lfo.extend_from_slice(&[0; 3]);

        let formats = synthetic_formats(&plf_lst, &plf_lfo);
        assert_eq!(
            painted(&formats, &[(1, 1), (1, 1)]),
            vec![9, 10],
            "the formatting override leaves its level unpainted, and the next LFOLVL's \
             start override still applies"
        );
        let mut state = NumberingState::default();
        let level_zero = formats
            .membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state)
            .expect("level 0 stays a list item");
        assert!(
            level_zero.ordered,
            "the base format's ordered flag stands when only formatting is overridden"
        );
        assert_eq!(level_zero.ordinal, None, "an overridden format paints no confidently-resolved number");
    }
}
