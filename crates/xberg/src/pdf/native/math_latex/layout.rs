//! Spelling a formula's pieces as LaTeX from their layout: the axis row, scripts hung on
//! their bases, numerators over denominators, limits, accents and radicals.

use super::glyphs::{MathFont, spell_glyph};
use super::{
    ACCENT_SHARE, AXIS_OVERLAP_EM, BASE_REACH_EM, CLUSTER_GAP_EM, COLLISION_SHARE, FRACTION_LEFT_GAP_EM,
    FRACTION_OFFSET_EM, LIMIT_OFFSET_EM, LIMIT_SHARE, MAX_DEPTH, OFF_AXIS_EM, Piece, SAME_ROW_EM, SCRIPT_REACH_EM,
    SCRIPT_SIZE_RATIO, STACK_SHARE, STRUCTURE_OFFSET_EM, are_nested_fraction_lines, axis_row, contains, glyphs, rows,
};

/// LaTeX writer that separates a command word from a following letter.
#[derive(Default)]
struct Tex {
    out: String,
    after_command: bool,
}

impl Tex {
    fn push(&mut self, tex: &str, command: bool) {
        if tex.is_empty() {
            return;
        }
        if self.after_command && tex.starts_with(|c: char| c.is_ascii_alphabetic()) {
            self.out.push(' ');
        }
        self.out.push_str(tex);
        self.after_command = command;
    }

    fn raw(&mut self, tex: &str) {
        self.out.push_str(tex);
        self.after_command = false;
    }
}

/// Spell a span's text; upright operator names become their commands.
fn spell_text(tex: &mut Tex, text: &str, font: Option<MathFont>) {
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if font.is_none() && c.is_ascii_alphabetic() {
            let mut end = start + c.len_utf8();
            while let Some(&(next, letter)) = chars.peek() {
                if !letter.is_ascii_alphabetic() {
                    break;
                }
                end = next + letter.len_utf8();
                chars.next();
            }
            let word = &text[start..end];
            if glyphs::OPERATOR_NAMES.contains(&word) {
                tex.push(&format!("\\{word}"), true);
            } else if word.len() > 1 {
                tex.push(&format!("\\mathrm{{{word}}}"), false);
            } else {
                tex.push(word, false);
            }
            continue;
        }
        let spelling = spell_glyph(c, font);
        tex.push(&spelling.tex, spelling.command);
    }
}

fn spell_piece(piece: &Piece<'_>) -> (String, bool) {
    let mut tex = Tex::default();
    spell_text(&mut tex, piece.text, piece.font());
    (tex.out, tex.after_command)
}

enum ItemKind<'p, 'a> {
    Base(&'p Piece<'a>),
    Fraction(Vec<&'p Piece<'a>>, Vec<&'p Piece<'a>>),
    Limits(Vec<&'p Piece<'a>>, Vec<&'p Piece<'a>>),
    /// The empty base `{}` of a script cluster no glyph of the row is near enough to carry.
    Orphan,
}

/// One element of a formula row with the scripts it carries.
struct Item<'p, 'a> {
    x0: f32,
    x1: f32,
    kind: ItemKind<'p, 'a>,
    accents: Vec<&'static str>,
    subscripts: Vec<&'p Piece<'a>>,
    superscripts: Vec<&'p Piece<'a>>,
}

impl<'p, 'a> Item<'p, 'a> {
    fn new(x0: f32, x1: f32, kind: ItemKind<'p, 'a>) -> Self {
        Self {
            x0,
            x1,
            kind,
            accents: Vec::new(),
            subscripts: Vec::new(),
            superscripts: Vec::new(),
        }
    }

    fn is_big_operator(&self) -> bool {
        matches!(self.kind, ItemKind::Base(piece) if piece.text.trim_end().ends_with(['∑', '∏', '∫', '⋃', '⋂', '∐', '∮']))
    }
}

/// Spell a group of pieces as one LaTeX expression, or `None` when axis glyphs are drawn
/// over each other: two rows mixed into one, which the caller keeps as source text.
///
/// A script cluster with no base in reach is written on an empty base (`{}^{2}`) where it
/// stands, and a fraction part that spells to nothing stays an empty group: keeping the
/// source text for those left whole display formulas (arXiv 1602.03238's, whose integral
/// signs and limits sit off the axis with no base) as a scatter of loose glyphs. ~keep
pub(super) fn group_tex(pieces: &[&Piece<'_>], depth: usize) -> Option<String> {
    group_tex_in(pieces, depth, false)
}

/// [`group_tex`] for a group that is `script`-size content (a subscript or superscript, or a
/// part of a fraction in one): its pieces are small only against the formula, so two of them
/// over each other are a script pair, never a nested fraction.
fn group_tex_in(pieces: &[&Piece<'_>], depth: usize, script: bool) -> Option<String> {
    let visible: Vec<&Piece<'_>> = pieces.iter().copied().filter(|piece| !piece.is_blank()).collect();
    if visible.is_empty() {
        return Some(String::new());
    }
    if depth >= MAX_DEPTH {
        return Some(flat_tex(&visible));
    }
    let size = visible.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
    if depth > 0
        && !script
        && let Some(fraction) = stacked_fraction(&visible, size, depth)
    {
        return fraction;
    }
    let (accents, glyphs) = split_accents(&visible, size);
    let Some(axis) = axis_row(&glyphs) else {
        return Some(flat_tex(&visible));
    };
    if axis_glyphs_collide(&axis) {
        return None;
    }
    let base = axis[0].base;
    let rest: Vec<&Piece<'_>> = glyphs.iter().copied().filter(|piece| !contains(&axis, piece)).collect();
    let on_axis = |piece: &Piece<'_>| {
        (piece.base - base).abs() <= size * OFF_AXIS_EM
            || (piece.is_radical() && covers_axis_radicand(piece, &axis, size))
    };
    let mut items: Vec<Item<'_, '_>> = axis
        .iter()
        .copied()
        .chain(rest.iter().copied().filter(|piece| on_axis(piece)))
        .map(|piece| Item::new(piece.x0, piece.x1, ItemKind::Base(piece)))
        .collect();
    for item in &mut items {
        if let ItemKind::Base(piece) = item.kind {
            item.accents = accents
                .iter()
                .filter(|(target, _)| std::ptr::eq(*target, piece))
                .map(|(_, command)| *command)
                .collect();
        }
    }
    let above = merge_stacked_lines(
        script,
        clusters(
            rest.iter()
                .copied()
                .filter(|piece| !on_axis(piece) && piece.base > base + size * OFF_AXIS_EM),
            size,
        ),
        size,
    );
    let below = merge_stacked_lines(
        script,
        clusters(
            rest.iter()
                .copied()
                .filter(|piece| !on_axis(piece) && piece.base < base - size * OFF_AXIS_EM),
            size,
        ),
        size,
    );
    let (above, below) = pair_stacks(&mut items, above, below, &axis, base, size);
    attach_scripts(&mut items, above, below, size, script)?;
    write_items(&items, depth, script)
}

/// Whether the axis row carries the radicand of a radical sign drawn off the axis: a glyph of
/// the row starts right after it. A radical over a fraction part has its radicand in that part.
fn covers_axis_radicand(radical: &Piece<'_>, axis: &[&Piece<'_>], size: f32) -> bool {
    axis.iter()
        .any(|glyph| glyph.x0 >= radical.x1 - size * AXIS_OVERLAP_EM && glyph.x0 - radical.x1 <= size * CLUSTER_GAP_EM)
}

/// A nested fraction's lines: two full-size lines over each other with nothing else, an inner
/// numerator over an inner denominator, written as `\frac`. `None` for any other group.
fn stacked_fraction(visible: &[&Piece<'_>], size: f32, depth: usize) -> Option<Option<String>> {
    let lines = rows(visible, size);
    let [lower, upper] = lines.as_slice() else {
        return None;
    };
    let full = |line: &[&Piece<'_>]| line.iter().all(|piece| piece.size >= size * SCRIPT_SIZE_RATIO);
    let stacked = full(lower)
        && full(upper)
        && upper
            .iter()
            .any(|top| lower.iter().any(|bottom| are_nested_fraction_lines(top, bottom)));
    stacked.then(|| {
        let (numerator, denominator) = (group_tex(upper, depth + 1)?, group_tex(lower, depth + 1)?);
        Some(format!("\\frac{{{numerator}}}{{{denominator}}}"))
    })
}

/// Join the lines of one side of the axis that stand over each other at full size: the parts
/// of a fraction nested in a numerator or a denominator, which a script never is.
fn merge_stacked_lines<'p, 'a>(script: bool, lines: Clusters<'p, 'a>, size: f32) -> Clusters<'p, 'a> {
    if script {
        return lines;
    }
    let stacked = |a: &[&Piece<'_>], b: &[&Piece<'_>]| {
        a.iter().all(|piece| piece.size >= size * SCRIPT_SIZE_RATIO)
            && b.iter().all(|piece| piece.size >= size * SCRIPT_SIZE_RATIO)
            && a.iter()
                .any(|upper| b.iter().any(|lower| are_nested_fraction_lines(upper, lower)))
    };
    let mut merged: Clusters<'p, 'a> = Vec::new();
    for line in lines {
        match merged.iter_mut().find(|other| stacked(other, &line)) {
            Some(other) => other.extend(line),
            None => merged.push(line),
        }
    }
    merged
}

/// Two axis glyphs sharing most of their width: one row drawn over another, not a formula.
fn axis_glyphs_collide(axis: &[&Piece<'_>]) -> bool {
    axis.windows(2).any(|pair| {
        let narrower = pair[0].width().min(pair[1].width());
        narrower > 0.0 && pair[0].overlap(pair[1].x0, pair[1].x1) > narrower * COLLISION_SHARE
    })
}

fn flat_tex(pieces: &[&Piece<'_>]) -> String {
    let mut sorted = pieces.to_vec();
    sorted.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    let mut tex = Tex::default();
    for piece in sorted {
        spell_text(&mut tex, piece.text, piece.font());
    }
    tex.out
}

/// Accent glyphs drawn over a base glyph, as (base, command), and the remaining glyphs.
fn split_accents<'p, 'a>(
    pieces: &[&'p Piece<'a>],
    size: f32,
) -> (Vec<(&'p Piece<'a>, &'static str)>, Vec<&'p Piece<'a>>) {
    let mut accents = Vec::new();
    let mut glyphs = Vec::new();
    for &piece in pieces {
        let target = piece.accent().and_then(|command| {
            pieces
                .iter()
                .copied()
                .filter(|other| {
                    other.accent().is_none()
                        && other.overlap(piece.x0, piece.x1) >= piece.width() * ACCENT_SHARE
                        && piece.base >= other.base - size * STRUCTURE_OFFSET_EM
                        && piece.base <= other.base + size
                })
                .min_by(|a, b| (piece.base - a.base).total_cmp(&(piece.base - b.base)))
                .map(|other| (other, command))
        });
        match target {
            Some(accent) => accents.push(accent),
            None => glyphs.push(piece),
        }
    }
    (accents, glyphs)
}

/// Group off-axis pieces into runs of one baseline (a script, a numerator, a limit).
fn clusters<'p, 'a>(pieces: impl Iterator<Item = &'p Piece<'a>>, size: f32) -> Vec<Vec<&'p Piece<'a>>>
where
    'a: 'p,
{
    let mut sorted: Vec<&'p Piece<'a>> = pieces.collect();
    sorted.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    let mut clusters: Vec<Vec<&'p Piece<'a>>> = Vec::new();
    for piece in sorted {
        match clusters.last_mut() {
            Some(cluster)
                if piece.x0 - extent(cluster).1 <= size * CLUSTER_GAP_EM
                    && (piece.base - cluster[0].base).abs() <= size * SAME_ROW_EM =>
            {
                cluster.push(piece);
            }
            _ => clusters.push(vec![piece]),
        }
    }
    clusters
}

fn extent(pieces: &[&Piece<'_>]) -> (f32, f32) {
    pieces
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(x0, x1), piece| {
            (x0.min(piece.x0), x1.max(piece.x1))
        })
}

type Clusters<'p, 'a> = Vec<Vec<&'p Piece<'a>>>;

/// Pair numerators with denominators (and upper with lower limits) that stand in a gap of
/// the axis; returns the clusters left unpaired.
fn pair_stacks<'p, 'a>(
    items: &mut Vec<Item<'p, 'a>>,
    above: Clusters<'p, 'a>,
    mut below: Clusters<'p, 'a>,
    axis: &[&Piece<'_>],
    base: f32,
    size: f32,
) -> (Clusters<'p, 'a>, Clusters<'p, 'a>) {
    let mut unpaired_above = Vec::new();
    for upper in above {
        let lowest = upper.iter().map(|piece| piece.base).fold(f32::INFINITY, f32::min);
        let partner = (lowest >= base + size * FRACTION_OFFSET_EM)
            .then(|| {
                below
                    .iter()
                    .position(|lower| stacks_with(&upper, lower, axis, items, base, size))
            })
            .flatten();
        let Some(index) = partner else {
            unpaired_above.push(upper);
            continue;
        };
        let lower = below.remove(index);
        let highest = lower.iter().map(|piece| piece.base).fold(f32::NEG_INFINITY, f32::max);
        let small = upper
            .iter()
            .chain(&lower)
            .all(|piece| piece.size <= size * SCRIPT_SIZE_RATIO);
        let far = lowest > base + size * LIMIT_OFFSET_EM && highest < base - size * LIMIT_OFFSET_EM;
        let (x0, x1) = (
            extent(&upper).0.min(extent(&lower).0),
            extent(&upper).1.max(extent(&lower).1),
        );
        let kind = if small && far {
            ItemKind::Limits(upper, lower)
        } else {
            ItemKind::Fraction(upper, lower)
        };
        items.push(Item::new(x0, x1, kind));
    }
    items.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    (unpaired_above, below)
}

fn stacks_with(
    upper: &[&Piece<'_>],
    lower: &[&Piece<'_>],
    axis: &[&Piece<'_>],
    items: &[Item<'_, '_>],
    base: f32,
    size: f32,
) -> bool {
    let highest = lower.iter().map(|piece| piece.base).fold(f32::NEG_INFINITY, f32::max);
    if highest > base - size * FRACTION_OFFSET_EM {
        return false;
    }
    let (upper_x0, upper_x1) = extent(upper);
    let (lower_x0, lower_x1) = extent(lower);
    let shared = upper_x1.min(lower_x1) - upper_x0.max(lower_x0);
    if shared <= STACK_SHARE * (upper_x1 - upper_x0).min(lower_x1 - lower_x0) {
        return false;
    }
    let (x0, x1) = (upper_x0.min(lower_x0), upper_x1.max(lower_x1));
    if axis.iter().any(|glyph| glyph.overlap(x0, x1) > size * AXIS_OVERLAP_EM) {
        return false;
    }
    // A script stack hugs its base; a fraction keeps the null delimiter space from it. ~keep
    let left_edge = items
        .iter()
        .filter(|item| matches!(item.kind, ItemKind::Base(_)) && item.x1 <= x0 + size * AXIS_OVERLAP_EM)
        .map(|item| item.x1)
        .fold(f32::NEG_INFINITY, f32::max);
    x0 - left_edge >= size * FRACTION_LEFT_GAP_EM
}

/// Hang each unpaired cluster on the item it scripts; a cluster with no base in reach gets an
/// [`ItemKind::Orphan`] base at its own position. `None` when a cluster is a full-size line
/// (a problem number over an expression, a line of a stack that is no fraction): no script
/// is as large as its base, so the group is not one formula. A script group is sized against
/// its own largest glyph, so its lines are not compared.
fn attach_scripts<'p, 'a>(
    items: &mut Vec<Item<'p, 'a>>,
    above: Clusters<'p, 'a>,
    below: Clusters<'p, 'a>,
    size: f32,
    script: bool,
) -> Option<()> {
    let is_line = |cluster: &[&Piece<'_>]| {
        cluster.iter().any(|piece| piece.size > size * SCRIPT_SIZE_RATIO)
            && !cluster
                .iter()
                .all(|piece| piece.text.trim().chars().all(|c| matches!(c, '\'' | '′' | '″')))
    };
    if !script && above.iter().chain(&below).any(|cluster| is_line(cluster)) {
        return None;
    }
    let mut orphans = Vec::new();
    let sides = above
        .into_iter()
        .map(|cluster| (cluster, true))
        .chain(below.into_iter().map(|c| (c, false)));
    for (cluster, superscript) in sides {
        let (x0, x1) = extent(&cluster);
        let operator = items
            .iter()
            .position(|item| item.is_big_operator() && x1.min(item.x1) - x0.max(item.x0) > LIMIT_SHARE * (x1 - x0));
        let target = operator.or_else(|| items.iter().rposition(|item| item.x0 <= x0 + size * BASE_REACH_EM));
        match target {
            Some(index) if x0 - items[index].x1 <= size * SCRIPT_REACH_EM => {
                let item = &mut items[index];
                let side = if superscript {
                    &mut item.superscripts
                } else {
                    &mut item.subscripts
                };
                side.extend(cluster);
            }
            _ => {
                let mut orphan = Item::new(x0, x1, ItemKind::Orphan);
                if superscript {
                    orphan.superscripts = cluster;
                } else {
                    orphan.subscripts = cluster;
                }
                orphans.push(orphan);
            }
        }
    }
    if !orphans.is_empty() {
        items.extend(orphans);
        items.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    }
    Some(())
}

fn write_items(items: &[Item<'_, '_>], depth: usize, script: bool) -> Option<String> {
    let mut tex = Tex::default();
    let mut open_radical = false;
    for item in items {
        let subscript = group_tex_in(&item.subscripts, depth + 1, true)?;
        let superscript = group_tex_in(&item.superscripts, depth + 1, true)?;
        match &item.kind {
            ItemKind::Base(piece) => {
                let (mut spelled, command) = spell_piece(piece);
                for accent in &item.accents {
                    spelled = format!("\\{accent}{{{spelled}}}");
                }
                let radical = spelled.ends_with("\\sqrt") && item.accents.is_empty();
                if radical && has_degree(item) {
                    if !subscript.is_empty() {
                        return None;
                    }
                    tex.push(&format!("{spelled}[{superscript}]"), false);
                    open_radical = true;
                    continue;
                }
                if radical && !(subscript.is_empty() && superscript.is_empty()) {
                    tex.push(&format!("{spelled}{{{subscript}{superscript}}}"), false);
                    continue;
                }
                if open_radical {
                    tex.raw(&format!("{{{spelled}}}"));
                } else {
                    tex.push(&spelled, command && item.accents.is_empty());
                }
                open_radical = radical;
            }
            ItemKind::Fraction(upper, lower) => {
                let (numerator, denominator) = (
                    group_tex_in(upper, depth + 1, script)?,
                    group_tex_in(lower, depth + 1, script)?,
                );
                tex.raw(&format!("\\frac{{{numerator}}}{{{denominator}}}"));
            }
            ItemKind::Limits(upper, lower) => {
                let (upper, lower) = (
                    group_tex_in(upper, depth + 1, true)?,
                    group_tex_in(lower, depth + 1, true)?,
                );
                tex.raw(&format!("{{}}_{{{lower}}}^{{{upper}}}"));
            }
            ItemKind::Orphan => tex.raw("{}"),
        }
        write_scripts(&mut tex, &subscript, &superscript);
    }
    if open_radical {
        tex.raw("{}");
    }
    Some(tex.out)
}

/// Whether a radical's superscript starts over the radical sign: its degree, where a script
/// drawn after the sign is the radicand.
fn has_degree(item: &Item<'_, '_>) -> bool {
    !item.superscripts.is_empty() && extent(&item.superscripts).0 < (item.x0 + item.x1) / 2.0
}

fn write_scripts(tex: &mut Tex, subscript: &str, superscript: &str) {
    if !subscript.is_empty() {
        tex.raw(&format!("_{{{subscript}}}"));
    }
    if superscript.is_empty() {
        return;
    }
    if superscript.chars().all(|c| c == '\'') {
        tex.raw(superscript);
    } else {
        tex.raw(&format!("^{{{superscript}}}"));
    }
}
