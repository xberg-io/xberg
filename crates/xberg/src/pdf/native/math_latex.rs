//! LaTeX for the math of a native PDF page.
//!
//! TeX sets math in dedicated font families (CMMI, CMSY, CMEX, AMS MSBM, ...), so glyphs in
//! those fonts, with the digits, brackets and operators of the text font between them, form
//! a formula. Assembled as prose, a formula's scripts and fraction parts scatter into
//! single-letter words on separate lines. This pass rebuilds each formula from glyph
//! geometry (script baselines and sizes, numerators stacked over denominators) and replaces
//! its spans with one span holding LaTeX: `$$...$$` for a display line, `$...$` inline.
//!
//! The pass returns `None`, leaving the caller's spans untouched, unless the page has a
//! math-font span and at least one formula is rewritten; text fonts alone (CMR included)
//! never start a formula.

mod glyphs;
#[cfg(test)]
mod tests;

use glyphs::{MathFont, accent_command, is_mark, is_operator, is_relation, is_unmapped, math_font, spell_glyph};
use xberg_native_pdf::layout::TextSpan;

/// A glyph below this fraction of the formula's largest size is a script or limit.
const SCRIPT_SIZE_RATIO: f32 = 0.85;
/// Baseline offset, in ems, within which two pieces share a row or one scripts the other.
const SAME_ROW_EM: f32 = 0.6;
/// Largest baseline offset of a fraction part from its axis, in ems.
const STACK_EM: f32 = 1.1;
/// Largest baseline offset of a big operator's limit from the axis, in ems.
const LIMIT_EM: f32 = 1.6;
/// Largest horizontal gap linking two pieces of one formula row, in ems.
const ROW_GAP_EM: f32 = 1.2;
/// Largest horizontal gap linking a limit to the formula, in ems.
const STACK_GAP_EM: f32 = 0.6;
/// A prose span nearer than this, in ems, on the formula's line makes the formula inline.
const PROSE_DISTANCE_EM: f32 = 1.5;
/// A piece this close to prose on its own row, in ems, belongs to that line of prose.
const PROSE_ADJACENT_EM: f32 = 0.5;
/// Baseline tolerance of one row, in ems.
const ROW_TOLERANCE_EM: f32 = 0.12;
/// Horizontal overlap, in ems, at which a piece covers an axis glyph (another row's glyph).
const AXIS_OVERLAP_EM: f32 = 0.1;
/// Baseline offset, in ems, within which a formula member counts as on the axis.
const AXIS_MEMBER_EM: f32 = 0.2;
/// Baseline offset, in ems, past which a piece sits above or below the axis.
const OFF_AXIS_EM: f32 = 0.05;
/// Smallest baseline offset of a numerator or denominator, in ems.
const FRACTION_OFFSET_EM: f32 = 0.3;
/// TeX's null delimiter space keeps a fraction this far, in ems, from the glyph before it.
const FRACTION_LEFT_GAP_EM: f32 = 0.08;
/// Smallest baseline offset of a big operator's limits, in ems.
const LIMIT_OFFSET_EM: f32 = 0.8;
/// Largest gap, in ems, between the pieces of one script or fraction part.
const CLUSTER_GAP_EM: f32 = 0.35;
/// Largest gap, in ems, between a script and its base.
const SCRIPT_REACH_EM: f32 = 0.6;
/// How far, in ems, a script may start left of its base's left edge.
const BASE_REACH_EM: f32 = 0.15;
/// Baseline offset, in ems, that counts as a script or stacked part.
const STRUCTURE_OFFSET_EM: f32 = 0.1;
/// Ascender and descender of a glyph box, in ems, for vertical overlap tests.
const ASCENT_EM: f32 = 0.75;
const DESCENT_EM: f32 = 0.25;
/// Share of the narrower part a numerator and denominator overlap by.
const STACK_SHARE: f32 = 0.5;
/// Share of an accent's width that lies over its base.
const ACCENT_SHARE: f32 = 0.5;
/// Share of a limit's width that lies under or over its big operator.
const LIMIT_SHARE: f32 = 0.3;
/// Nesting depth past which a group is spelled flat.
const MAX_DEPTH: usize = 6;

/// Rewrite the math of one page as LaTeX, or `None` when nothing changes.
pub(crate) fn rewrite_math_spans(spans: &[TextSpan]) -> Option<Vec<TextSpan>> {
    if !spans.iter().any(|span| math_font(&span.font_name).is_some()) {
        return None;
    }
    let pieces: Vec<Piece<'_>> = spans
        .iter()
        .enumerate()
        .filter_map(|(i, span)| Piece::new(i, span))
        .collect();
    let formulas = find_formulas(&pieces);
    if formulas.is_empty() {
        return None;
    }
    let mut replacement: Vec<Option<TextSpan>> = vec![None; spans.len()];
    let mut removed = vec![false; spans.len()];
    for formula in &formulas {
        for piece in &formula.members {
            removed[piece.span] = true;
        }
        replacement[formula.anchor_span] = Some(formula.to_span(&spans[formula.anchor_span]));
    }
    let rewritten = spans
        .iter()
        .enumerate()
        .filter_map(|(i, span)| match replacement[i].take() {
            Some(merged) => Some(merged),
            None if removed[i] => None,
            None => Some(span.clone()),
        })
        .collect();
    Some(rewritten)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Math(MathFont),
    /// Text-font glyphs a formula may hold: digits, brackets, operators, operator names.
    Neutral,
    Prose,
}

/// One upright span with its ink extent and baseline in page space (y grows upward).
#[derive(Debug, Clone, Copy)]
struct Piece<'a> {
    span: usize,
    text: &'a str,
    x0: f32,
    x1: f32,
    base: f32,
    size: f32,
    class: Class,
}

impl<'a> Piece<'a> {
    fn new(span: usize, source: &'a TextSpan) -> Option<Self> {
        let upright = source.rotation_degrees == 0.0 && !source.mirrored && source.wmode == 0;
        let text = source.text.as_str();
        let geometry_ok = source.font_size.is_finite()
            && source.font_size > 0.0
            && source.bbox.x.is_finite()
            && source.bbox.y.is_finite()
            && source.bbox.width.is_finite();
        if !upright || source.rtl_draw_logical || !geometry_ok || text.trim().is_empty() {
            return None;
        }
        let (x0, x1) = ink_extent(source);
        Some(Self {
            span,
            text,
            x0,
            x1,
            base: source.bbox.y + source.text_rise * source.font_size,
            size: source.font_size,
            class: classify(text, &source.font_name),
        })
    }

    fn is_math(&self) -> bool {
        matches!(self.class, Class::Math(_))
    }

    fn font(&self) -> Option<MathFont> {
        match self.class {
            Class::Math(font) => Some(font),
            _ => None,
        }
    }

    /// A glyph that decoded to nothing (an unmapped big delimiter or operator).
    fn is_blank(&self) -> bool {
        self.text.trim().chars().all(is_unmapped)
    }

    fn accent(&self) -> Option<&'static str> {
        let mut chars = self.text.trim().chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => accent_command(c),
            _ => None,
        }
    }

    fn top(&self) -> f32 {
        self.base + self.size * ASCENT_EM
    }

    fn bottom(&self) -> f32 {
        self.base - self.size * DESCENT_EM
    }

    fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Horizontal gap to `other`; negative when they overlap.
    fn gap(&self, other: &Piece<'_>) -> f32 {
        self.x0.max(other.x0) - self.x1.min(other.x1)
    }

    fn overlap(&self, x0: f32, x1: f32) -> f32 {
        self.x1.min(x1) - self.x0.max(x0)
    }
}

/// The horizontal extent of a span's visible glyphs, without edge whitespace.
fn ink_extent(span: &TextSpan) -> (f32, f32) {
    let full = (span.bbox.x, span.bbox.x + span.bbox.width);
    if span.text.trim().len() == span.text.len() {
        return full;
    }
    let chars = span.to_chars();
    let mut ink = chars.iter().filter(|c| !c.char.is_whitespace());
    match (ink.next(), ink.last()) {
        (Some(first), Some(last)) => (first.bbox.x, last.bbox.x + last.bbox.width),
        (Some(only), None) => (only.bbox.x, only.bbox.x + only.bbox.width),
        _ => full,
    }
}

fn classify(text: &str, font_name: &str) -> Class {
    if let Some(font) = math_font(font_name) {
        return Class::Math(font);
    }
    if is_neutral(text) { Class::Neutral } else { Class::Prose }
}

/// Text-font content that reads as math: no word except an operator name.
fn is_neutral(text: &str) -> bool {
    const PUNCTUATION: &str = "()[]{}+−-=<>,.;:!/|'′″*·×±∓≤≥…⋯⟨⟩∗";
    let allowed = |c: char| {
        c.is_ascii_digit()
            || c.is_whitespace()
            || c.is_ascii_alphabetic()
            || PUNCTUATION.contains(c)
            || glyphs::greek_command(c).is_some()
    };
    if !text.chars().all(allowed) {
        return false;
    }
    text.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() > 1)
        .all(|word| glyphs::OPERATOR_NAMES.contains(&word) || glyphs::ROMAN_MATH_WORDS.contains(&word))
}

/// `(4)`, `(2.13)`, `(A.1)`, `(3b)`: a display equation's number, kept outside the math.
fn is_equation_number(text: &str) -> bool {
    let Some(inner) = text.trim().strip_prefix('(').and_then(|rest| rest.strip_suffix(')')) else {
        return false;
    };
    let inner = inner.trim();
    let inner = inner.strip_prefix(|c: char| c.is_ascii_uppercase()).unwrap_or(inner);
    let inner = inner.strip_suffix(|c: char| c.is_ascii_lowercase()).unwrap_or(inner);
    !inner.is_empty()
        && inner
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// One rewritten formula: the spans it replaces and the span it is written in place of.
struct Formula<'a> {
    members: Vec<Piece<'a>>,
    anchor_span: usize,
    display: bool,
    tex: String,
    x0: f32,
    x1: f32,
    base: f32,
    size: f32,
}

impl Formula<'_> {
    fn to_span(&self, anchor: &TextSpan) -> TextSpan {
        let delimiter = if self.display { "$$" } else { "$" };
        let mut span = anchor.clone();
        span.text = format!("{delimiter}{}{delimiter}", self.tex);
        span.bbox.x = self.x0;
        span.bbox.y = self.base;
        span.bbox.width = self.x1 - self.x0;
        span.bbox.height = span.bbox.height.max(self.size);
        span.font_size = self.size;
        span.text_rise = 0.0;
        span.char_widths.clear();
        span.char_x_offsets.clear();
        span.ligature_glyph = false;
        span
    }
}

fn find_formulas<'a>(pieces: &[Piece<'a>]) -> Vec<Formula<'a>> {
    let prose: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class == Class::Prose).collect();
    let candidates: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class != Class::Prose).collect();
    let mut formulas = Vec::new();
    for component in connected_components(&candidates, &prose) {
        for unit in split_rows(component) {
            if let Some(formula) = build_formula(unit, &prose) {
                formulas.push(formula);
            }
        }
    }
    formulas
}

/// Group candidate pieces that sit on one formula row or stack into one formula.
fn connected_components<'p, 'a>(candidates: &[&'p Piece<'a>], prose: &[&Piece<'a>]) -> Vec<Vec<&'p Piece<'a>>> {
    let adjacent: Vec<bool> = candidates.iter().map(|piece| is_prose_adjacent(piece, prose)).collect();
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| candidates[a].base.total_cmp(&candidates[b].base));
    let reach = candidates.iter().map(|piece| piece.size).fold(0.0_f32, f32::max) * LIMIT_EM;
    for (position, &i) in order.iter().enumerate() {
        for &j in &order[position + 1..] {
            if candidates[j].base - candidates[i].base > reach {
                break;
            }
            if linked(candidates[i], candidates[j], adjacent[i] || adjacent[j], prose) {
                let (root_i, root_j) = (find(&mut parent, i), find(&mut parent, j));
                parent[root_i] = root_j;
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<&'p Piece<'a>>> = std::collections::BTreeMap::new();
    for (i, &piece) in candidates.iter().enumerate() {
        let root = find(&mut parent, i);
        groups.entry(root).or_default().push(piece);
    }
    groups
        .into_values()
        .filter(|group| group.iter().any(|piece| piece.is_math()))
        .collect()
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn is_prose_adjacent(piece: &Piece<'_>, prose: &[&Piece<'_>]) -> bool {
    prose.iter().any(|other| {
        (other.base - piece.base).abs() <= piece.size * SAME_ROW_EM / 2.0
            && piece.gap(other) <= piece.size * PROSE_ADJACENT_EM
    })
}

fn linked(a: &Piece<'_>, b: &Piece<'_>, prose_adjacent: bool, prose: &[&Piece<'_>]) -> bool {
    let size = a.size.max(b.size);
    let rise = (a.base - b.base).abs();
    let gap = a.gap(b);
    if rise <= size * SAME_ROW_EM {
        gap <= size * ROW_GAP_EM && !prose_between(a, b, prose)
    } else if rise <= size * STACK_EM {
        gap <= size * ROW_GAP_EM && !prose_adjacent
    } else if rise <= size * LIMIT_EM && a.size.min(b.size) <= size * SCRIPT_SIZE_RATIO {
        gap <= size * STACK_GAP_EM && !prose_adjacent
    } else {
        false
    }
}

/// Whether a prose span lies in the horizontal gap between two pieces of one row.
fn prose_between(a: &Piece<'_>, b: &Piece<'_>, prose: &[&Piece<'_>]) -> bool {
    if a.gap(b) <= 0.0 {
        return false;
    }
    let (left, right) = (a.x1.min(b.x1), a.x0.max(b.x0));
    prose.iter().any(|other| {
        other.overlap(left, right) > 0.0
            && a.bottom().max(b.bottom()) < other.top()
            && a.top().min(b.top()) > other.bottom()
    })
}

/// Split a component into formulas: an axis row and the scripts and stacked parts that hang
/// off it. A piece covering an axis glyph belongs to another row and starts its own formula.
fn split_rows<'p, 'a>(component: Vec<&'p Piece<'a>>) -> Vec<Vec<&'p Piece<'a>>> {
    let mut units = Vec::new();
    let mut remaining = component;
    while remaining.iter().any(|piece| piece.is_math() && !piece.is_blank()) {
        let Some(axis) = axis_row(&remaining) else { break };
        let size = axis.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
        let base = axis[0].base;
        let mut unit = axis.clone();
        let covers_axis = |piece: &Piece<'_>| {
            piece.accent().is_none()
                && axis
                    .iter()
                    .any(|glyph| piece.overlap(glyph.x0, glyph.x1) > size * AXIS_OVERLAP_EM)
        };
        let mut changed = true;
        while changed {
            changed = false;
            for &piece in &remaining {
                if contains(&unit, piece) || covers_axis(piece) || !hangs_off(piece, &unit, base, size) {
                    continue;
                }
                unit.push(piece);
                changed = true;
            }
        }
        remaining.retain(|piece| !contains(&unit, piece));
        units.push(unit);
    }
    units
}

fn contains(group: &[&Piece<'_>], piece: &Piece<'_>) -> bool {
    group.iter().any(|member| std::ptr::eq(*member, piece))
}

/// Whether `piece` attaches to a formula whose axis is at `base`.
fn hangs_off(piece: &Piece<'_>, unit: &[&Piece<'_>], base: f32, size: f32) -> bool {
    let rise = (piece.base - base).abs();
    let near = |reach: f32| unit.iter().any(|member| piece.gap(member) <= size * reach);
    if piece.is_blank() {
        return rise <= size * LIMIT_EM && near(LIMIT_EM);
    }
    if rise <= size * STACK_EM {
        return near(LIMIT_EM);
    }
    let scripts_member = unit.iter().any(|member| {
        !member.is_blank()
            && (piece.base - member.base).abs() <= member.size * SAME_ROW_EM
            && piece.gap(member) <= size * SCRIPT_REACH_EM
    });
    if scripts_member {
        return true;
    }
    let is_limit = piece.size <= size * SCRIPT_SIZE_RATIO && rise <= size * LIMIT_EM;
    is_limit
        && unit.iter().any(|member| {
            (member.base - base).abs() <= size * AXIS_MEMBER_EM && piece.gap(member) <= size * STACK_GAP_EM
        })
}

/// The row a formula is built around: one holding a relation, else the widest.
fn axis_row<'p, 'a>(pieces: &[&'p Piece<'a>]) -> Option<Vec<&'p Piece<'a>>> {
    let size = pieces
        .iter()
        .filter(|piece| !piece.is_blank())
        .map(|piece| piece.size)
        .fold(0.0_f32, f32::max);
    let full: Vec<&'p Piece<'a>> = pieces
        .iter()
        .copied()
        .filter(|piece| !piece.is_blank() && piece.size >= size * SCRIPT_SIZE_RATIO)
        .collect();
    rows(&full, size).into_iter().max_by(|a, b| {
        row_rank(a)
            .partial_cmp(&row_rank(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn row_rank(row: &[&Piece<'_>]) -> (bool, f32) {
    let relation = row
        .iter()
        .any(|piece| piece.accent().is_none() && piece.text.chars().any(is_relation));
    (relation, row.iter().map(|piece| piece.width()).sum())
}

/// Group pieces into rows of one baseline, bottom to top.
fn rows<'p, 'a>(pieces: &[&'p Piece<'a>], size: f32) -> Vec<Vec<&'p Piece<'a>>> {
    let mut sorted = pieces.to_vec();
    sorted.sort_by(|a, b| a.base.total_cmp(&b.base));
    let mut rows: Vec<Vec<&'p Piece<'a>>> = Vec::new();
    for piece in sorted {
        match rows.last_mut() {
            Some(row) if (row[0].base - piece.base).abs() <= size * ROW_TOLERANCE_EM => row.push(piece),
            _ => rows.push(vec![piece]),
        }
    }
    for row in &mut rows {
        row.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    }
    rows
}

fn build_formula<'a>(mut members: Vec<&Piece<'a>>, prose: &[&Piece<'a>]) -> Option<Formula<'a>> {
    members.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    while members
        .last()
        .is_some_and(|piece| piece.class == Class::Neutral && is_equation_number(piece.text))
    {
        members.pop();
    }
    if !has_math_evidence(&members) {
        return None;
    }
    let size = members.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
    let x0 = members.iter().map(|piece| piece.x0).fold(f32::INFINITY, f32::min);
    let x1 = members.iter().map(|piece| piece.x1).fold(f32::NEG_INFINITY, f32::max);
    let top = members
        .iter()
        .map(|piece| piece.top())
        .fold(f32::NEG_INFINITY, f32::max);
    let bottom = members.iter().map(|piece| piece.bottom()).fold(f32::INFINITY, f32::min);
    let display = !prose.iter().any(|other| {
        other.bottom() < top && other.top() > bottom && other.x0.max(x0) - other.x1.min(x1) < size * PROSE_DISTANCE_EM
    });
    let structured = has_structure(&members, size);
    let operated = members
        .iter()
        .any(|piece| piece.text.chars().any(|c| is_relation(c) || is_operator(c)));
    if !structured && !(display && operated) {
        return None;
    }
    let axis = axis_row(&members)?;
    let anchor = axis.iter().min_by(|a, b| a.x0.total_cmp(&b.x0))?;
    let tex = group_tex(&members, 0);
    if tex.is_empty() {
        return None;
    }
    Some(Formula {
        anchor_span: anchor.span,
        base: anchor.base,
        members: members.into_iter().copied().collect(),
        display,
        tex,
        x0,
        x1,
        size,
    })
}

/// A math-font glyph that is more than a footnote or list mark.
fn has_math_evidence(members: &[&Piece<'_>]) -> bool {
    members.iter().filter(|piece| piece.is_math()).any(|piece| {
        piece.text.chars().any(|c| {
            !c.is_whitespace()
                && !is_unmapped(c)
                && !is_mark(c)
                && (c.is_alphanumeric()
                    || glyphs::greek_command(c).is_some()
                    || glyphs::symbol_command(c).is_some()
                    || is_relation(c)
                    || is_operator(c))
        })
    })
}

/// Scripts, stacked parts or accents: what prose assembly breaks apart.
fn has_structure(members: &[&Piece<'_>], size: f32) -> bool {
    let first = members[0].base;
    members.iter().any(|piece| {
        (piece.base - first).abs() > size * STRUCTURE_OFFSET_EM
            || piece.size < size * SCRIPT_SIZE_RATIO
            || piece.accent().is_some()
    })
}

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

/// Spell a group of pieces as one LaTeX expression.
fn group_tex(pieces: &[&Piece<'_>], depth: usize) -> String {
    let visible: Vec<&Piece<'_>> = pieces.iter().copied().filter(|piece| !piece.is_blank()).collect();
    if visible.is_empty() {
        return String::new();
    }
    if depth >= MAX_DEPTH {
        return flat_tex(&visible);
    }
    let size = visible.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
    let (accents, glyphs) = split_accents(&visible, size);
    let Some(axis) = axis_row(&glyphs) else {
        return flat_tex(&visible);
    };
    let base = axis[0].base;
    let rest: Vec<&Piece<'_>> = glyphs.iter().copied().filter(|piece| !contains(&axis, piece)).collect();
    let mut items: Vec<Item<'_, '_>> = axis
        .iter()
        .copied()
        .chain(
            rest.iter()
                .copied()
                .filter(|piece| (piece.base - base).abs() <= size * OFF_AXIS_EM),
        )
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
    let above = clusters(
        rest.iter()
            .copied()
            .filter(|piece| piece.base > base + size * OFF_AXIS_EM),
        size,
    );
    let below = clusters(
        rest.iter()
            .copied()
            .filter(|piece| piece.base < base - size * OFF_AXIS_EM),
        size,
    );
    let (above, below) = pair_stacks(&mut items, above, below, &axis, base, size);
    let leftovers = attach_scripts(&mut items, above, below, size);
    write_items(&items, &leftovers, depth)
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

/// Hang each unpaired cluster on the item it scripts; returns clusters with no base.
fn attach_scripts<'p, 'a>(
    items: &mut [Item<'p, 'a>],
    above: Clusters<'p, 'a>,
    below: Clusters<'p, 'a>,
    size: f32,
) -> Clusters<'p, 'a> {
    let mut leftovers = Vec::new();
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
            _ => leftovers.push(cluster),
        }
    }
    leftovers
}

fn write_items(items: &[Item<'_, '_>], leftovers: &[Vec<&Piece<'_>>], depth: usize) -> String {
    let mut tex = Tex::default();
    let mut open_radical = false;
    for item in items {
        let subscript = group_tex(&item.subscripts, depth + 1);
        let superscript = group_tex(&item.superscripts, depth + 1);
        match &item.kind {
            ItemKind::Base(piece) => {
                let (mut spelled, command) = spell_piece(piece);
                for accent in &item.accents {
                    spelled = format!("\\{accent}{{{spelled}}}");
                }
                let radical = spelled.ends_with("\\sqrt") && item.accents.is_empty();
                if radical && !(subscript.is_empty() && superscript.is_empty()) {
                    tex.push(&format!("{spelled}{{{subscript}{superscript}}}"), false);
                    continue;
                }
                if open_radical {
                    tex.raw(&format!("{{{spelled}}}"));
                    open_radical = false;
                } else {
                    tex.push(&spelled, command && item.accents.is_empty());
                }
                open_radical = radical;
            }
            ItemKind::Fraction(upper, lower) => {
                let (numerator, denominator) = (group_tex(upper, depth + 1), group_tex(lower, depth + 1));
                if numerator.is_empty() || denominator.is_empty() {
                    tex.raw(&format!("{numerator}{denominator}"));
                } else {
                    tex.raw(&format!("\\frac{{{numerator}}}{{{denominator}}}"));
                }
            }
            ItemKind::Limits(upper, lower) => {
                let (upper, lower) = (group_tex(upper, depth + 1), group_tex(lower, depth + 1));
                tex.raw(&format!("{{}}_{{{lower}}}^{{{upper}}}"));
            }
        }
        write_scripts(&mut tex, &subscript, &superscript);
    }
    for cluster in leftovers {
        tex.raw(&group_tex(cluster, depth + 1));
    }
    if open_radical {
        tex.raw("{}");
    }
    tex.out
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
