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
mod layout;
pub(crate) mod sentinels;
#[cfg(test)]
mod tests;

use glyphs::{MathFont, accent_command, is_mark, is_operator, is_relation, is_unmapped, math_font};
use layout::group_tex;
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
/// Baseline distance, in ems, within which a prose span can share a formula's band.
const PROSE_BAND_EM: f32 = 3.0;
/// Components larger than this are left as they are: no formula has this many runs, and the
/// row split is cubic in the worst case.
const MAX_COMPONENT_PIECES: usize = 256;
/// More than this many purely numeric text runs with no relation reads as a table row.
const MAX_NUMERIC_RUNS: usize = 3;
/// Nesting depth past which a group is spelled flat.
const MAX_DEPTH: usize = 6;

/// How a rewritten formula is delimited in its span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delimiters {
    /// `$...$` and `$$...$$`, for text written as is.
    Dollars,
    /// The [`sentinels`] pairs, for text that structure assembly turns into math annotations.
    Sentinels,
}

impl Delimiters {
    fn wrap(self, tex: &str, display: bool) -> String {
        use sentinels::{DISPLAY_CLOSE, DISPLAY_OPEN, INLINE_CLOSE, INLINE_OPEN};
        let (open, close) = match (self, display) {
            (Self::Dollars, false) => ('$'.to_string(), '$'.to_string()),
            (Self::Dollars, true) => ("$$".to_owned(), "$$".to_owned()),
            (Self::Sentinels, false) => (INLINE_OPEN.to_string(), INLINE_CLOSE.to_string()),
            (Self::Sentinels, true) => (DISPLAY_OPEN.to_string(), DISPLAY_CLOSE.to_string()),
        };
        format!("{open}{tex}{close}")
    }
}

/// Rewrite the math of one page as `$`-delimited LaTeX, or `None` when nothing changes.
#[cfg(test)]
pub(crate) fn rewrite_math_spans(spans: &[TextSpan]) -> Option<Vec<TextSpan>> {
    rewrite_math_spans_with(spans, Delimiters::Dollars)
}

/// Rewrite the math of one page as LaTeX delimited by `delimiters`, or `None` when nothing
/// changes.
pub(crate) fn rewrite_math_spans_with(spans: &[TextSpan], delimiters: Delimiters) -> Option<Vec<TextSpan>> {
    rewrite_with_baselines(spans, delimiters, &RawBaselines::default())
}

/// [`rewrite_math_spans`] for spans read with flattened script geometry, which snaps a
/// script's baseline onto its base: the raw baselines the page's read recorded put each
/// script back where it was drawn, without reading the page again.
pub(crate) fn rewrite_page_math(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    spans: &[TextSpan],
) -> Option<Vec<TextSpan>> {
    if !has_math_font(spans) {
        return None;
    }
    let baselines = RawBaselines::new(&doc.flattened_script_baselines(page_index));
    rewrite_with_baselines(spans, Delimiters::Dollars, &baselines)
}

fn has_math_font(spans: &[TextSpan]) -> bool {
    spans.iter().any(|span| math_font(&span.font_name).is_some())
}

/// Raw baselines of snapped script spans, keyed by origin x and size.
#[derive(Default)]
struct RawBaselines(std::collections::HashMap<(u32, u32), f32>);

impl RawBaselines {
    fn new(recorded: &[(f32, f32, f32)]) -> Self {
        let mut baselines = std::collections::HashMap::with_capacity(recorded.len());
        for &(x, size, baseline) in recorded {
            baselines.entry((x.to_bits(), size.to_bits())).or_insert(baseline);
        }
        Self(baselines)
    }

    fn of(&self, span: &TextSpan) -> f32 {
        if self.0.is_empty() {
            return span.bbox.y;
        }
        self.0
            .get(&(span.bbox.x.to_bits(), span.font_size.to_bits()))
            .copied()
            .unwrap_or(span.bbox.y)
    }
}

fn rewrite_with_baselines(
    spans: &[TextSpan],
    delimiters: Delimiters,
    baselines: &RawBaselines,
) -> Option<Vec<TextSpan>> {
    if !has_math_font(spans) {
        return None;
    }
    let pieces: Vec<Piece<'_>> = spans
        .iter()
        .enumerate()
        .filter_map(|(i, span)| Piece::new(i, span, baselines.of(span)))
        .collect();
    let mut formulas = find_formulas(&pieces);
    if formulas.is_empty() {
        return None;
    }
    let mut replacement: Vec<Option<TextSpan>> = vec![None; spans.len()];
    let mut removed = vec![false; spans.len()];
    for formula in &mut formulas {
        if let Some((index, count)) = formula.closers
            && let Some(trimmed) = trim_leading_chars(&spans[index], count)
        {
            formula.tex.extend(spans[index].text.chars().take(count));
            formula.x1 = formula.x1.max(trimmed.bbox.x);
            replacement[index] = (!trimmed.text.trim().is_empty()).then_some(trimmed);
            removed[index] = true;
        }
        for piece in &formula.members {
            removed[piece.span] = true;
        }
        replacement[formula.anchor_span] = Some(formula.to_span(&spans[formula.anchor_span], delimiters));
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

/// `span` without its first `count` characters, its left edge moved to the next glyph;
/// `None` when the glyph positions are unknown.
fn trim_leading_chars(span: &TextSpan, count: usize) -> Option<TextSpan> {
    let chars = span.to_chars();
    let next = chars.get(count)?;
    let bytes = span.text.char_indices().nth(count)?.0;
    let mut trimmed = span.clone();
    trimmed.text = span.text[bytes..].to_owned();
    trimmed.bbox.width = (span.bbox.x + span.bbox.width - next.bbox.x).max(0.0);
    trimmed.bbox.x = next.bbox.x;
    let char_count = span.text.chars().count();
    for positions in [&mut trimmed.char_widths, &mut trimmed.char_x_offsets] {
        if positions.len() == char_count {
            positions.drain(..count);
        } else {
            positions.clear();
        }
    }
    Some(trimmed)
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
    fn new(span: usize, source: &'a TextSpan, baseline: f32) -> Option<Self> {
        let upright = source.rotation_degrees == 0.0 && !source.mirrored && source.wmode == 0;
        let text = source.text.as_str();
        let geometry_ok = source.font_size.is_finite()
            && source.font_size > 0.0
            && source.bbox.x.is_finite()
            && baseline.is_finite()
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
            base: baseline + source.text_rise * source.font_size,
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
    let text = span.text.as_str();
    let leading = text.chars().take_while(|c| c.is_whitespace()).count();
    let trailing = text.chars().rev().take_while(|c| c.is_whitespace()).count();
    if leading == 0 && trailing == 0 {
        return full;
    }
    let count = text.chars().count();
    if span.char_widths.len() != count || leading + trailing >= count {
        return full;
    }
    let start = span.bbox.x + span.char_widths[..leading].iter().sum::<f32>();
    let end = start + span.char_widths[leading..count - trailing].iter().sum::<f32>();
    (start, end)
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
    let inner = match inner.strip_prefix(|c: char| c.is_ascii_uppercase()) {
        Some(rest) => rest.strip_prefix('.').unwrap_or(rest),
        None => inner,
    };
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
    /// A prose span starting with the closing brackets this formula lacks: (span, count).
    closers: Option<(usize, usize)>,
}

impl Formula<'_> {
    fn to_span(&self, anchor: &TextSpan, delimiters: Delimiters) -> TextSpan {
        let mut span = anchor.clone();
        span.text = delimiters.wrap(&self.tex, self.display);
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
    let mut prose: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class == Class::Prose).collect();
    prose.sort_by(|a, b| a.base.total_cmp(&b.base));
    let candidates: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class != Class::Prose).collect();
    let mut formulas = Vec::new();
    for component in connected_components(&candidates, &prose) {
        if component.len() > MAX_COMPONENT_PIECES {
            continue;
        }
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

/// The prose pieces (sorted by baseline) whose baseline lies in `low..=high`.
fn prose_near<'s, 'p, 'a>(prose: &'s [&'p Piece<'a>], low: f32, high: f32) -> &'s [&'p Piece<'a>] {
    let start = prose.partition_point(|piece| piece.base < low);
    let end = prose.partition_point(|piece| piece.base <= high);
    &prose[start..end.max(start)]
}

fn is_prose_adjacent(piece: &Piece<'_>, prose: &[&Piece<'_>]) -> bool {
    let reach = piece.size * SAME_ROW_EM / 2.0;
    prose_near(prose, piece.base - reach, piece.base + reach)
        .iter()
        .any(|other| {
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
    let reach = a.size.max(b.size) * PROSE_BAND_EM;
    prose_near(prose, a.base.min(b.base) - reach, a.base.max(b.base) + reach)
        .iter()
        .any(|other| {
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
        let mut members: std::collections::HashSet<usize> = unit.iter().map(|piece| piece.span).collect();
        while changed {
            changed = false;
            for &piece in &remaining {
                if members.contains(&piece.span) || covers_axis(piece) || !hangs_off(piece, &unit, base, size) {
                    continue;
                }
                members.insert(piece.span);
                unit.push(piece);
                changed = true;
            }
        }
        remaining.retain(|piece| !members.contains(&piece.span));
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
    while is_trailing_note_mark(&members) {
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
    let band = size * PROSE_BAND_EM;
    let display = !prose_near(prose, bottom - band, top + band).iter().any(|other| {
        other.bottom() < top && other.top() > bottom && other.x0.max(x0) - other.x1.min(x1) < size * PROSE_DISTANCE_EM
    });
    let structured = has_structure(&members, size);
    let related = members.iter().any(|piece| piece.text.chars().any(is_relation));
    let operated = related || members.iter().any(|piece| piece.text.chars().any(is_operator));
    if !structured && !(display && operated) {
        return None;
    }
    if !structured && !related && (only_operator_glyphs(&members) || reads_as_numeric_row(&members)) {
        return None;
    }
    let axis = axis_row(&members)?;
    let anchor = axis.iter().min_by(|a, b| a.x0.total_cmp(&b.x0))?;
    let tex = group_tex(&members, 0);
    if tex.is_empty() {
        return None;
    }
    let closers = (!display)
        .then(|| missing_closers(&members, prose, x1, anchor.base, size))
        .flatten();
    Some(Formula {
        closers,
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

/// A raised script-size number right after sentence punctuation: a footnote mark that
/// follows the formula, not its exponent.
fn is_trailing_note_mark(members: &[&Piece<'_>]) -> bool {
    let [.., before, last] = members else {
        return false;
    };
    let size = members.iter().map(|piece| piece.size).fold(0.0_f32, f32::max);
    let digits = last.text.trim();
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && last.size < size * SCRIPT_SIZE_RATIO
        && last.base > before.base + size * STRUCTURE_OFFSET_EM
        && before.text.trim_end().ends_with(['.', ',', ';', ':'])
}

/// The prose span right after an inline formula that opens with the closing brackets the
/// formula is missing, and how many of them it lends: (span, count).
fn missing_closers(
    members: &[&Piece<'_>],
    prose: &[&Piece<'_>],
    x1: f32,
    base: f32,
    size: f32,
) -> Option<(usize, usize)> {
    let balance = |open: char, close: char| {
        members
            .iter()
            .flat_map(|piece| piece.text.chars())
            .fold(0_isize, |depth, c| {
                depth + isize::from(c == open) - isize::from(c == close)
            })
    };
    let (mut parens, mut brackets) = (balance('(', ')'), balance('[', ']'));
    if parens <= 0 && brackets <= 0 {
        return None;
    }
    let reach = size * STRUCTURE_OFFSET_EM;
    let next = prose_near(prose, base - reach, base + reach)
        .iter()
        .filter(|piece| piece.x0 >= x1 - size * AXIS_OVERLAP_EM && piece.x0 - x1 <= size * CLUSTER_GAP_EM)
        .min_by(|a, b| a.x0.total_cmp(&b.x0))?;
    let mut count = 0;
    for c in next.text.chars() {
        match c {
            ')' if parens > 0 => parens -= 1,
            ']' if brackets > 0 => brackets -= 1,
            _ => break,
        }
        count += 1;
    }
    (count > 0).then_some((next.span, count))
}

/// Whether every math-font glyph is a binary operator, as a LaTeX table's `$-$3.4` minus is.
fn only_operator_glyphs(members: &[&Piece<'_>]) -> bool {
    members
        .iter()
        .filter(|piece| piece.is_math())
        .flat_map(|piece| piece.text.chars())
        .filter(|&c| !c.is_whitespace() && !is_unmapped(c))
        .all(is_operator)
}

/// Several text runs that are all signed numbers: table cells, not a formula.
fn reads_as_numeric_row(members: &[&Piece<'_>]) -> bool {
    let numeric = |text: &str| {
        let text = text.trim();
        text.chars().any(|c| c.is_ascii_digit())
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_whitespace() || matches!(c, '.' | ',' | '%' | '−' | '-' | '+'))
    };
    let neutral: Vec<&&Piece<'_>> = members.iter().filter(|piece| piece.class == Class::Neutral).collect();
    neutral.len() > MAX_NUMERIC_RUNS && neutral.iter().all(|piece| numeric(piece.text))
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
