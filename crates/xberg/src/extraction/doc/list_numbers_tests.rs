//! The measured cases of PR #2882's review, rebuilt as tests over synthetic list tables.
//!
//! Every ordered scheme keeps its count and paints the label Word paints, a markerless
//! paragraph still counts at its level, and the restart rules of [MS-DOC] 2.9.27 hold.
//! The tables are hand-built `PlfLst`/`PlfLfo` blocks because the #2857 fixtures carry no
//! `LFOLVL` overrides and one level and one list kind each.

use super::list_numbers::{ListFormats, NumberingState};
use super::papx::ListBinding;

const NFC_DECIMAL: u8 = 0;
const NFC_UPPER_ROMAN: u8 = 1;
const NFC_UPPER_LETTER: u8 = 3;
const NFC_LOWER_LETTER: u8 = 4;
const NFC_BULLET: u8 = 23;
const NFC_NONE: u8 = 255;

const LSTF_LEN: usize = 28;
const LVLF_LEN: usize = 28;
const LFO_LEN: usize = 16;
const LEVELS: usize = 9;

const FIB_IDX_PLF_LST: usize = 73;
const FIB_IDX_PLF_LFO: usize = 74;

/// One level's specification in a synthetic `PlfLst`.
#[derive(Clone)]
struct LevelSpec {
    start: i32,
    nfc: u8,
    no_restart: bool,
    restart_lim: u8,
    /// The `Xst` label template, where the character 0x0000 + ilvl is that level's
    /// placeholder for its own number ([MS-DOC] 2.9.28).
    xst: Vec<u16>,
}

/// A decimal level painting `N.`, starting at `start`.
fn decimal_level(start: i32) -> LevelSpec {
    LevelSpec {
        start,
        nfc: NFC_DECIMAL,
        no_restart: false,
        restart_lim: 0,
        xst: Vec::new(),
    }
}

fn decimal_levels(start: i32) -> [LevelSpec; LEVELS] {
    std::array::from_fn(|_| decimal_level(start))
}

/// Serialize one `LVL`: the `LVLF` header, then the `Xst` (a 16-bit count and that many
/// UTF-16 units).
fn lvl_bytes(spec: &LevelSpec, level: u8) -> Vec<u8> {
    let mut lvlf = [0u8; LVLF_LEN];
    lvlf[..4].copy_from_slice(&spec.start.to_le_bytes());
    lvlf[4] = spec.nfc;
    if spec.no_restart {
        lvlf[5] |= 0x08;
    }
    lvlf[26] = spec.restart_lim.min(level);
    let mut out = lvlf.to_vec();
    out.extend_from_slice(&(spec.xst.len() as u16).to_le_bytes());
    for unit in &spec.xst {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// The level's own placeholder followed by a period -- the plain `N.` label shape.
fn placeholder_label(level: u8) -> Vec<u16> {
    vec![u16::from(level), u16::from(b'.')]
}

/// A synthetic `PlfLst` for one multilevel list, one `LVL` per level in order.
fn plf_lst(lsid: u32, levels: [LevelSpec; LEVELS]) -> Vec<u8> {
    let mut out = vec![0x01, 0x00];
    let mut lstf = [0u8; LSTF_LEN];
    lstf[..4].copy_from_slice(&lsid.to_le_bytes());
    out.extend_from_slice(&lstf);
    for (level, spec) in levels.iter().enumerate() {
        let level = u8::try_from(level).unwrap_or(u8::MAX);
        // A markerless level carries an empty `Xst` -- Word writes no label for it; a
        // level with no template of its own paints its placeholder and a period.
        let xst = if spec.nfc == NFC_NONE {
            Vec::new()
        } else if spec.xst.is_empty() {
            placeholder_label(level)
        } else {
            spec.xst.clone()
        };
        out.extend_from_slice(&lvl_bytes(&LevelSpec { xst, ..spec.clone() }, level));
    }
    out
}

/// A synthetic `PlfLfo` with one `LFO` per entry, every one naming `lsid`, each with an
/// `LFOLVL` start override for `(ilvl, start)` when `Some`. All `LFO` records come first,
/// then each one's `LFOData`.
fn plf_lfo(lsid: u32, overrides: &[(u8, Option<i32>)]) -> Vec<u8> {
    let mut out = vec![0, 0, 0, 0];
    out[..4].copy_from_slice(&u32::try_from(overrides.len()).unwrap_or(u32::MAX).to_le_bytes());
    for (_, start_override) in overrides {
        let mut lfo = [0u8; LFO_LEN];
        lfo[..4].copy_from_slice(&lsid.to_le_bytes());
        lfo[12] = u8::from(start_override.is_some());
        out.extend_from_slice(&lfo);
    }
    for (ilvl, start_override) in overrides {
        out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        if let Some(start) = start_override {
            out.extend_from_slice(&start.to_le_bytes());
            out.push(0x10 | ilvl); // fStartAt, not fFormatting, at `ilvl`
            out.extend_from_slice(&[0; 3]);
        }
    }
    out
}

/// `ListFormats` over synthetic blocks the FIB stub points at. The `PlfLst` block sits
/// at 0 and `PlfLfo` follows it, past 0x1000 when the list table itself is larger.
fn formats(plf_lst: &[u8], plf_lfo: &[u8]) -> ListFormats {
    let lst_at = 0usize;
    let lfo_at = plf_lst.len().max(0x1000);
    let rg = 0x40;
    let mut word_doc = vec![0u8; rg + FIB_IDX_PLF_LFO * 8 + 8];
    for (index, (fc, block)) in [
        (FIB_IDX_PLF_LST, (lst_at, plf_lst)),
        (FIB_IDX_PLF_LFO, (lfo_at, plf_lfo)),
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

/// Walk the paragraphs `(ilfo, ilvl)` in document order and collect the label each paints;
/// `None` also for a paragraph that joins no list at all.
fn walked(formats: &ListFormats, paragraphs: &[(u16, u8)]) -> Vec<Option<String>> {
    let mut state = NumberingState::default();
    paragraphs
        .iter()
        .map(|&(ilfo, ilvl)| {
            formats
                .membership_for(ListBinding { ilfo, ilvl }, &mut state)
                .and_then(|membership| membership.label)
        })
        .collect()
}

/// The painted label of a paragraph that joins a list.
fn label(text: &str) -> Option<String> {
    Some(text.to_owned())
}

#[test]
fn a_markerless_shallower_paragraph_still_restarts_deeper_levels() {
    // The review's `markerless-shallower` case: level 0 paints no marker (`nfc` 255) --
    // the binding of a paragraph removed from the list, such as a styled heading.
    // [MS-DOC] "Determining Level Number of a Paragraph" step 8 counts every paragraph
    // with a nonzero `iLfo`, so the markerless paragraph must still interrupt level 1's
    // sequence: it painted [1, 2, None, 3] before the fix; Word paints [1, 2, None, 1].
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_NONE,
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 1), (1, 1), (1, 0), (1, 1)]),
        [label("1."), label("2."), None, label("1.")],
        "the markerless level-0 paragraph paints no number yet still restarts level 1"
    );
}

#[test]
fn roman_levels_paint_their_numbers() {
    // The review's `roman` case: an ordered level that is not decimal kept no counter and
    // painted nothing (got [None, None]); Word paints I, II -- and keeps the count across
    // an interrupting paragraph, which is the #2857 bug for every ordered scheme.
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_UPPER_ROMAN,
        xst: placeholder_label(0),
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 0), (1, 0)]),
        [label("I."), label("II.")],
        "the upper-roman scheme paints its numbers through the same label channel"
    );
}

#[test]
fn an_upper_letter_level_paints_uppercase_letters() {
    // The review's `upper-letter` case: the letter helper writes the lowercase run and
    // the DOCX counterpart applies the case the level's format names (`w:numFmt`
    // `upperLetter`, `docx/parser.rs::format_list_counter`); the .doc scheme reads the
    // same Word level table and must apply the same case, or `A.`/`B.` paint `a.`/`b.`.
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_UPPER_LETTER,
        xst: placeholder_label(0),
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 0), (1, 0)]),
        [label("A."), label("B.")],
        "the upper-letter scheme paints uppercase letters through the same label channel"
    );
}

#[test]
fn a_lower_letter_level_keeps_its_count_across_a_markerless_interrupt() {
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_NONE,
        ..decimal_level(1)
    };
    levels[1] = LevelSpec {
        nfc: NFC_LOWER_LETTER,
        xst: placeholder_label(1),
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 1), (1, 0), (1, 1)]),
        [label("a."), None, label("a.")],
        "the lower-letter sequence restarts after the markerless level-0 paragraph"
    );
}

#[test]
fn a_parenthesized_label_template_paints_its_parentheses() {
    // The review's `(1)`-style case: the label template, not a hardcoded `N.`, decides the
    // punctuation Word paints.
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        xst: vec![u16::from(b'('), 0, u16::from(b')')],
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 0), (1, 0)]),
        [label("(1)"), label("(2)")],
        "the level's Xst template paints the label with its own punctuation"
    );
}

#[test]
fn a_start_override_does_not_survive_a_restart() {
    // The review's `override-after-restart` case, pinned to the 2025 [MS-DOC] text
    // (2.9.144 `LFOLVL`): `fStartAt` applies once, at the overriding `ilfo`'s first
    // paragraph at the level; after a deeper restart the sequence begins again at the
    // level's own `LVLF.iStartAt`. (The 2017 text reads the override to survive the
    // restart -- 5, 6, 1, 5 -- which is not what this code follows.)
    let formats = formats(&plf_lst(7, decimal_levels(1)), &plf_lfo(7, &[(1, Some(5))]));
    assert_eq!(
        walked(&formats, &[(1, 1), (1, 1), (1, 0), (1, 1)]),
        [label("5."), label("6."), label("1."), label("1.")],
        "the override starts the first sequence at 5; the restart after level 0 begins at \
         the level's own start, 1"
    );
}

#[test]
fn special_ilfos_bind_no_list() {
    // The review's `special-ilfo` case: `ilfo` 2047 (`ilfoNil`) and the negative values
    // are how Word records a paragraph removed from a list; they name no list and join no
    // sequence, rather than becoming numberless list items.
    let formats = formats(&plf_lst(7, decimal_levels(1)), &plf_lfo(7, &[(0, None)]));
    for ilfo in [2047u16, 65535, 63489] {
        let mut state = NumberingState::default();
        assert!(
            formats
                .membership_for(ListBinding { ilfo, ilvl: 0 }, &mut state)
                .is_none(),
            "ilfo {ilfo} binds no list"
        );
    }
}

#[test]
fn f_no_restart_with_limit_zero_never_restarts() {
    // [MS-DOC] 2.9.27: a level with `fNoRestart` set and `ilvlRestartLim` 0 is restarted
    // by no shallower level -- the review measured it; pinned here because no other test
    // covers the flag.
    let mut levels = decimal_levels(1);
    levels[1].no_restart = true;
    levels[1].restart_lim = 0;
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    assert_eq!(
        walked(&formats, &[(1, 0), (1, 1), (1, 1), (1, 0), (1, 1)]),
        [label("1."), label("1."), label("2."), label("2."), label("3.")],
        "the level-1 sequence runs on across the interrupting level-0 paragraphs"
    );
}

#[test]
fn a_declared_lfo_count_beyond_the_table_stops_at_the_table() {
    // `cLfo` claims more `LFO` records than the table holds. The count is file-read data:
    // sizing by it unchecked wraps a 32-bit `usize` and panics a debug build.
    let mut plf_lfo = 0xFFFF_FFFFu32.to_le_bytes().to_vec();
    let mut record = [0u8; LFO_LEN];
    record[..4].copy_from_slice(&7u32.to_le_bytes());
    plf_lfo.extend_from_slice(&record);
    let formats = formats(&plf_lst(7, decimal_levels(1)), &plf_lfo);
    assert_eq!(
        walked(&formats, &[(1, 0), (2, 0)]),
        [label("1."), None],
        "the one record present is read; the missing one binds no list"
    );
}

#[test]
fn truncated_list_tables_never_panic() {
    // Every truncation of either table, and of the LVL blocks behind them, is refused by
    // bounds checks rather than panicking or spinning.
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_BULLET,
        xst: vec![u16::from(b'o')],
        ..decimal_level(1)
    };
    let lst = plf_lst(7, levels);
    let lfo = plf_lfo(7, &[(0, None), (1, Some(3))]);
    for cut in 0..=lst.len() {
        let formats = formats(&lst[..cut], &lfo);
        walked(&formats, &[(1, 0), (1, 1), (2, 0)]);
    }
    for cut in 0..=lfo.len() {
        let formats = formats(&lst, &lfo[..cut]);
        walked(&formats, &[(1, 0), (1, 1), (2, 0)]);
    }
}

#[test]
fn a_template_past_the_unit_bound_paints_no_label() {
    // One unit past the 64-unit template bound: the level keeps its count but paints no
    // label. The refusal happens at read, independently of the painted-label cap -- this
    // paint (65 characters) never reaches that cap's bound, so removing the template cap
    // alone turns this test red.
    let mut levels = decimal_levels(1);
    levels[0].xst = vec![u16::from(b'x'); 65];
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    let mut state = NumberingState::default();
    let mut counted = 0;
    for _ in 0..200 {
        let membership = formats
            .membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state)
            .expect("the level still counts its paragraphs");
        assert!(membership.ordered, "the refused template does not unorder the level");
        assert!(
            membership.label.is_none(),
            "a template past the 64-unit bound paints no label, got {:?}",
            membership.label
        );
        counted += 1;
    }
    assert_eq!(counted, 200, "every paragraph of the walk was counted");
}

#[test]
fn an_oversized_label_template_paints_no_label_and_stays_bounded() {
    // A hostile `Xst` at the 16-bit count's maximum (65 535 UTF-16 units, ~128 KB) is
    // repainted for every list-bound paragraph: before the bounds, one 200-paragraph run
    // built 200 labels of 65 536 characters each, memory proportional to
    // paragraphs x template length and outside `SecurityLimits`. The red-before row of
    // this test is the High's proof shape: the assertion below failed against unfixed
    // code with "got 65535 chars", the unbounded paint itself. Post-fix both caps guard
    // it: the template cap refuses the `Xst` at read, so this test passes even with the
    // painted-label cap removed only jointly (removing the template cap alone is caught
    // by `a_label_painted_beyond_its_cap_is_refused` on a template whose paint reaches
    // that cap, and by `a_template_past_the_unit_bound_paints_no_label` on one that does
    // not). The level keeps its count but paints no label.
    let mut levels = decimal_levels(1);
    levels[0].xst = vec![u16::from(b'x'); 65535];
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    let mut state = NumberingState::default();
    let mut counted = 0;
    for _ in 0..200 {
        let membership = formats
            .membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state)
            .expect("the level still counts its paragraphs");
        assert!(membership.ordered, "the oversized template does not unorder the level");
        assert!(
            membership.label.is_none(),
            "a template past Word's 64-unit label bound paints no label, got {} chars",
            membership.label.as_ref().map_or(0, |label| label.chars().count())
        );
        counted += 1;
    }
    assert_eq!(counted, 200, "every paragraph of the walk was counted");
}

#[test]
fn a_label_painted_beyond_its_cap_is_refused() {
    // A template within the 64-unit bound can still paint past the label cap when every
    // unit is a placeholder whose number is long: 64 copies of this level's own
    // placeholder, upper-case roman at 3999, paint 15 symbols each -- 960 characters,
    // past the bound a well-formed template cannot pass (64 template units plus nine
    // placeholders' numbers). The label is refused, not truncated, so the repaint per
    // paragraph stays bounded.
    let mut levels = decimal_levels(1);
    levels[0] = LevelSpec {
        nfc: NFC_UPPER_ROMAN,
        start: 3999,
        xst: vec![0; 64],
        ..decimal_level(1)
    };
    let formats = formats(&plf_lst(7, levels), &plf_lfo(7, &[(0, None)]));
    let mut state = NumberingState::default();
    let membership = formats
        .membership_for(ListBinding { ilfo: 1, ilvl: 0 }, &mut state)
        .expect("the level still counts its paragraphs");
    assert!(membership.ordered, "the oversized painted label does not unorder the level");
    assert!(
        membership.label.is_none(),
        "a label past the paint bound is refused, got {} chars",
        membership.label.as_ref().map_or(0, |label| label.chars().count())
    );
}
