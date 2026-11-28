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

mod evidence;
mod glyphs;
mod layout;
mod reliability;
pub(crate) mod sentinels;
#[cfg(test)]
mod tests;

use evidence::{
    has_math_evidence, has_structure, is_trailing_note_mark, missing_closers, only_operator_glyphs,
    reads_as_numeric_row,
};
use glyphs::{MathFont, accent_command, is_mark, is_operator, is_radical_glyph, is_relation, is_unmapped, math_font};
use layout::group_tex;
use reliability::tex_is_reliable;
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
/// Largest distance, in ems, between the centres of the numerator and denominator of a fraction.
const NESTED_CENTRE_EM: f32 = 0.3;
/// Share of the wider part's width that the narrower part of a nested fraction has at least.
const NESTED_WIDTH_SHARE: f32 = 0.6;
/// Smallest baseline distance, in ems, between a nested fraction's numerator and denominator:
/// the bar's clearance. Ordinary leading of 10 pt type on 12 pt lines is 1.2 em, so the margin
/// keeps two stacked lines of text from reading as parts of a fraction.
const NESTED_BAR_EM: f32 = 1.3;
/// Share of the narrower of two axis glyphs they overlap by when one is drawn over the other.
const COLLISION_SHARE: f32 = 0.5;
/// Distance, in ems, within which a glyph run with the same text is an overdraw of another.
const OVERDRAW_EM: f32 = 0.1;

/// The text a radical sign of a TeX extension font is read as.
const RADICAL: &str = "√";

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

/// Rewrite the math of one page as LaTeX delimited by `delimiters`, or `None` when nothing
/// changes.
pub(crate) fn rewrite_math_spans_with(spans: &[TextSpan], delimiters: Delimiters) -> Option<Vec<TextSpan>> {
    rewrite_with_baselines(spans, delimiters, &RawBaselines::default()).map(|(rewritten, _)| rewritten)
}

/// [`rewrite_math_spans_with`] for spans read with flattened script geometry, which snaps a
/// script's baseline onto its base: the raw baselines the page's read recorded put each
/// script back where it was drawn, without reading the page again.
pub(crate) fn rewrite_page_math(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    spans: &[TextSpan],
) -> Option<Vec<TextSpan>> {
    rewrite_page_math_indexed(doc, page_index, spans).map(|(rewritten, _)| rewritten)
}

/// Rewritten spans and, for each source span, its index among them; `None` for a span a
/// formula absorbed. A formula takes its anchor span's index.
pub(crate) type IndexedRewrite = (Vec<TextSpan>, Vec<Option<usize>>);

/// [`rewrite_page_math`] with the source-to-rewritten index map, for a caller that holds
/// an order over the source spans.
pub(crate) fn rewrite_page_math_indexed(
    doc: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    spans: &[TextSpan],
) -> Option<IndexedRewrite> {
    if !has_math_font(spans) {
        return None;
    }
    let baselines = RawBaselines::new(&doc.flattened_script_baselines(page_index));
    rewrite_with_baselines(spans, Delimiters::Dollars, &baselines)
}

/// [`rewrite_math_spans_with`] `$`-delimited, with the source-to-rewritten index map.
#[cfg(all(test, feature = "layout-detection"))]
pub(crate) fn rewrite_math_spans_indexed(spans: &[TextSpan]) -> Option<IndexedRewrite> {
    rewrite_with_baselines(spans, Delimiters::Dollars, &RawBaselines::default())
}

fn has_math_font(spans: &[TextSpan]) -> bool {
    spans.iter().any(|span| math_font(&span.font_name).is_some())
}

/// The pieces with each overdrawn copy removed (the same text drawn again at the same place,
/// as fake bold does), and the removed copies as (copy span, original span). A formula takes
/// its copies with it; a copy outside every formula keeps its text.
fn without_overdraws(pieces: Vec<Piece<'_>>) -> (Vec<Piece<'_>>, Vec<(usize, usize)>) {
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by(|&a, &b| {
        pieces[a]
            .base
            .total_cmp(&pieces[b].base)
            .then(pieces[a].x0.total_cmp(&pieces[b].x0))
    });
    let mut copy_of: Vec<Option<usize>> = vec![None; pieces.len()];
    for (position, &i) in order.iter().enumerate() {
        if copy_of[i].is_some() {
            continue;
        }
        let reach = pieces[i].size * OVERDRAW_EM;
        for &j in &order[position + 1..] {
            if pieces[j].base - pieces[i].base > reach {
                break;
            }
            let same = pieces[j].text.trim() == pieces[i].text.trim()
                && pieces[j].size == pieces[i].size
                && (pieces[j].x0 - pieces[i].x0).abs() <= reach;
            if same && copy_of[j].is_none() {
                copy_of[j] = Some(pieces[i].span);
            }
        }
    }
    let overdraws = pieces
        .iter()
        .zip(&copy_of)
        .filter_map(|(piece, original)| original.map(|original| (piece.span, original)))
        .collect();
    let kept = pieces
        .into_iter()
        .zip(copy_of)
        .filter_map(|(piece, original)| original.is_none().then_some(piece))
        .collect();
    (kept, overdraws)
}

/// Raw baselines of snapped script spans.
#[derive(Default)]
struct RawBaselines(std::collections::HashMap<(u32, u32, u32), f32>);

impl RawBaselines {
    /// Keyed by origin x, snapped baseline and size: aligned equations stacked on a page
    /// share script x positions, so x and size alone would mix their rows.
    fn new(recorded: &[(f32, f32, f32, f32)]) -> Self {
        let mut baselines = std::collections::HashMap::with_capacity(recorded.len());
        for &(x, size, snapped, raw) in recorded {
            baselines
                .entry((x.to_bits(), snapped.to_bits(), size.to_bits()))
                .or_insert(raw);
        }
        Self(baselines)
    }

    fn of(&self, span: &TextSpan) -> f32 {
        if self.0.is_empty() {
            return span.bbox.y;
        }
        self.0
            .get(&(span.bbox.x.to_bits(), span.bbox.y.to_bits(), span.font_size.to_bits()))
            .copied()
            .unwrap_or(span.bbox.y)
    }
}

fn rewrite_with_baselines(
    spans: &[TextSpan],
    delimiters: Delimiters,
    baselines: &RawBaselines,
) -> Option<IndexedRewrite> {
    if !has_math_font(spans) {
        return None;
    }
    let all: Vec<Piece<'_>> = spans
        .iter()
        .enumerate()
        .filter_map(|(i, span)| Piece::new(i, span, baselines.of(span)))
        .collect();
    let (mut pieces, overdraws) = without_overdraws(all);
    mark_crowded(&mut pieces);
    let (mut formulas, refused) = find_formulas(&pieces);
    if formulas.is_empty() && refused.is_empty() {
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
        for &(copy, original) in &overdraws {
            if formula.members.iter().any(|piece| piece.span == original) {
                removed[copy] = true;
            }
        }
        replacement[formula.anchor_span] = Some(formula.to_span(&spans[formula.anchor_span], delimiters));
    }
    for &index in &refused {
        if replacement[index].is_none()
            && !removed[index]
            && let Some(piece) = pieces.iter().find(|piece| piece.span == index)
        {
            replacement[index] = set_apart(&spans[index], piece, &pieces);
        }
    }
    if formulas.is_empty() && replacement.iter().all(Option::is_none) {
        return None;
    }
    let mut rewritten = Vec::with_capacity(spans.len());
    let mut index_map = Vec::with_capacity(spans.len());
    for (i, span) in spans.iter().enumerate() {
        let kept = match replacement[i].take() {
            Some(merged) => Some(merged),
            None if removed[i] => None,
            None => Some(span.clone()),
        };
        index_map.push(kept.map(|kept| {
            rewritten.push(kept);
            rewritten.len() - 1
        }));
    }
    Some((rewritten, index_map))
}

/// `span` with a space on each side where prose reaches it, so a math glyph kept as source
/// text is never read as part of the word beside it; `None` when no side needs one. Prose
/// runs under the glyph on its line: a span that covers it, as a line of prose does a sum sign
/// drawn between two lines.
fn set_apart(span: &TextSpan, piece: &Piece<'_>, pieces: &[Piece<'_>]) -> Option<TextSpan> {
    let same_line =
        |other: &&Piece<'_>| other.class == Class::Prose && (other.base - piece.base).abs() <= piece.size * SAME_ROW_EM;
    let prose_before = pieces
        .iter()
        .filter(same_line)
        .any(|other| other.x0 < piece.x0 && other.x1 > piece.x0);
    let prose_after = pieces
        .iter()
        .filter(same_line)
        .any(|other| other.x1 > piece.x1 && other.x0 < piece.x1);
    let lead = prose_before && !span.text.starts_with(char::is_whitespace);
    let trail = prose_after && !span.text.ends_with(char::is_whitespace);
    if !lead && !trail {
        return None;
    }
    let mut apart = span.clone();
    apart.text = format!(
        "{}{}{}",
        if lead { " " } else { "" },
        span.text,
        if trail { " " } else { "" }
    );
    apart.char_widths.clear();
    apart.char_x_offsets.clear();
    Some(apart)
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
    /// A radical sign of a TeX extension font, drawn off the axis of the radicand it covers.
    radical: bool,
    /// Another piece of the page shares this one's baseline within a size of it: it is part of
    /// a line of text, which the parts of a nested fraction are not.
    crowded: bool,
}

impl<'a> Piece<'a> {
    fn new(span: usize, source: &'a TextSpan, baseline: f32) -> Option<Self> {
        let upright = source.rotation_degrees == 0.0 && !source.mirrored && source.wmode == 0;
        let radical = is_radical_glyph(&source.font_name, &source.text);
        let text = if radical { RADICAL } else { source.text.as_str() };
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
            radical,
            crowded: false,
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

    fn is_radical(&self) -> bool {
        self.radical
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

/// The formulas rebuilt from `pieces`, and the spans of the math glyphs of each group whose
/// rebuild was refused: they stay as the source's plain reading, set apart from the prose
/// around them.
fn find_formulas<'a>(pieces: &[Piece<'a>]) -> (Vec<Formula<'a>>, Vec<usize>) {
    let mut prose: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class == Class::Prose).collect();
    prose.sort_by(|a, b| a.base.total_cmp(&b.base));
    let candidates: Vec<&Piece<'a>> = pieces.iter().filter(|piece| piece.class != Class::Prose).collect();
    let mut formulas = Vec::new();
    let mut refused = Vec::new();
    for component in connected_components(&candidates, &prose) {
        for unit in split_rows(component) {
            let math_spans: Vec<usize> = unit
                .iter()
                .filter(|piece| piece.is_math())
                .map(|piece| piece.span)
                .collect();
            match build_formula(unit, &prose) {
                Built::Formula(formula) if tex_is_reliable(&formula.tex) => formulas.push(*formula),
                Built::Formula(_) | Built::Refused => refused.extend(math_spans),
                Built::NotMath => {}
            }
        }
    }
    (formulas, refused)
}

/// What a group of pieces came to.
enum Built<'a> {
    Formula(Box<Formula<'a>>),
    /// A structured group whose geometry cannot be spelled as one formula.
    Refused,
    NotMath,
}

/// Group candidate pieces that sit on one formula row or stack into one formula. A component
/// of more than [`MAX_COMPONENT_PIECES`] is left out.
fn connected_components<'p, 'a>(candidates: &[&'p Piece<'a>], prose: &[&Piece<'a>]) -> Vec<Vec<&'p Piece<'a>>> {
    let adjacent: Vec<bool> = candidates.iter().map(|piece| is_prose_adjacent(piece, prose)).collect();
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| candidates[a].base.total_cmp(&candidates[b].base));
    // Two pieces link only within LIMIT_EM of the larger one's size, so a piece's scan ends
    // where the rise passes that reach for every piece still ahead: the largest size from
    // there on. A page's largest glyph then widens only the scans that can still reach it.
    let mut largest_ahead = vec![0.0_f32; order.len() + 1];
    for position in (0..order.len()).rev() {
        largest_ahead[position] = largest_ahead[position + 1].max(candidates[order[position]].size);
    }
    for (position, &i) in order.iter().enumerate() {
        for (offset, &j) in order[position + 1..].iter().enumerate() {
            let reach = candidates[i].size.max(largest_ahead[position + 1 + offset]) * LIMIT_EM;
            if candidates[j].base - candidates[i].base > reach {
                break;
            }
            if linked(
                candidates[i],
                candidates[j],
                adjacent[i] || adjacent[j],
                candidates[i].crowded || candidates[j].crowded,
                prose,
            ) {
                let (root_i, root_j) = (find(&mut parent, i), find(&mut parent, j));
                parent[root_i] = root_j;
            }
        }
    }
    let roots: Vec<usize> = (0..candidates.len()).map(|i| find(&mut parent, i)).collect();
    let mut sizes = vec![0_usize; candidates.len()];
    for &root in &roots {
        sizes[root] += 1;
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<&'p Piece<'a>>> = std::collections::BTreeMap::new();
    for (&piece, &root) in candidates.iter().zip(&roots) {
        if sizes[root] <= MAX_COMPONENT_PIECES {
            groups.entry(root).or_default().push(piece);
        }
    }
    groups
        .into_values()
        .filter(|group| group.iter().any(|piece| piece.is_math()))
        .collect()
}

/// Set [`Piece::crowded`] on every piece that has another on its baseline within a size of it.
fn mark_crowded(pieces: &mut [Piece<'_>]) {
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by(|&a, &b| pieces[a].base.total_cmp(&pieces[b].base));
    for (position, &i) in order.iter().enumerate() {
        for &j in &order[position + 1..] {
            if pieces[j].base - pieces[i].base > pieces[i].size * ROW_TOLERANCE_EM {
                break;
            }
            if pieces[i].gap(&pieces[j]) <= pieces[i].size.max(pieces[j].size) {
                pieces[i].crowded = true;
                pieces[j].crowded = true;
            }
        }
    }
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

fn linked(a: &Piece<'_>, b: &Piece<'_>, prose_adjacent: bool, crowded: bool, prose: &[&Piece<'_>]) -> bool {
    let size = a.size.max(b.size);
    let rise = (a.base - b.base).abs();
    let gap = a.gap(b);
    if rise <= size * SAME_ROW_EM {
        gap <= size * ROW_GAP_EM && !prose_between(a, b, prose)
    } else if rise <= size * STACK_EM {
        gap <= size * ROW_GAP_EM && !prose_adjacent
    } else if rise <= size * LIMIT_EM && a.size.min(b.size) <= size * SCRIPT_SIZE_RATIO {
        gap <= size * STACK_GAP_EM && !prose_adjacent
    } else if rise <= size * LIMIT_EM {
        !prose_adjacent && !crowded && are_nested_fraction_lines(a, b)
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
    if unit.iter().any(|member| stacks_on_part(piece, member, base, size)) {
        return true;
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

/// Whether `piece` is a full-size line stacked on `part`, a full-size line of the same side of
/// the axis: the inner numerator over the inner denominator of a nested fraction.
fn stacks_on_part(piece: &Piece<'_>, part: &Piece<'_>, base: f32, size: f32) -> bool {
    let side = |p: &Piece<'_>| (p.base - base).signum();
    !part.is_blank()
        && (part.base - base).abs() > size * FRACTION_OFFSET_EM
        && side(piece) == side(part)
        && are_nested_fraction_lines(piece, part)
}

/// Whether two full-size pieces are the numerator and denominator of a fraction nested in
/// another: centred on each other, as wide as each other and a bar apart, which sets them
/// further from each other than two lines of an aligned equation, whose denominators and
/// numerators merely overlap in x.
fn are_nested_fraction_lines(a: &Piece<'_>, b: &Piece<'_>) -> bool {
    let size = a.size.max(b.size);
    let centred = ((a.x0 + a.x1) - (b.x0 + b.x1)).abs() / 2.0 <= size * NESTED_CENTRE_EM;
    let (narrow, wide) = (a.width().min(b.width()), a.width().max(b.width()));
    let rise = (a.base - b.base).abs();
    a.size.min(b.size) >= size * SCRIPT_SIZE_RATIO
        && centred
        && narrow >= wide * NESTED_WIDTH_SHARE
        && rise >= size * NESTED_BAR_EM
        && rise <= size * LIMIT_EM
}

/// The row a formula is built around: one holding a relation, else the widest.
fn axis_row<'p, 'a>(pieces: &[&'p Piece<'a>]) -> Option<Vec<&'p Piece<'a>>> {
    let on_axis = |piece: &Piece<'_>| !piece.is_blank() && !piece.is_radical();
    let size = pieces
        .iter()
        .filter(|piece| on_axis(piece))
        .map(|piece| piece.size)
        .fold(0.0_f32, f32::max);
    let full: Vec<&'p Piece<'a>> = pieces
        .iter()
        .copied()
        .filter(|piece| on_axis(piece) && piece.size >= size * SCRIPT_SIZE_RATIO)
        .collect();
    let lines = rows(&full, size);
    let ranks: Vec<(bool, bool, f32)> = (0..lines.len()).map(|index| row_rank(&lines, index, size)).collect();
    let best =
        (0..lines.len()).max_by(|&a, &b| ranks[a].partial_cmp(&ranks[b]).unwrap_or(std::cmp::Ordering::Equal))?;
    lines.into_iter().nth(best)
}

/// How well a row anchors a formula: whether it holds a relation, whether it is the line of
/// operators between two lines of fractions, and its width.
fn row_rank(lines: &[Vec<&Piece<'_>>], index: usize, size: f32) -> (bool, bool, f32) {
    let row = &lines[index];
    let relation = row
        .iter()
        .any(|piece| piece.accent().is_none() && piece.text.chars().any(is_relation));
    let (x0, x1) = (
        row.iter().map(|piece| piece.x0).fold(f32::INFINITY, f32::min),
        row.iter().map(|piece| piece.x1).fold(f32::NEG_INFINITY, f32::max),
    );
    let neighbour = |other: Option<&Vec<&Piece<'_>>>| {
        other.is_some_and(|other| {
            (other[0].base - row[0].base).abs() <= size * LIMIT_EM
                && other.iter().any(|piece| piece.overlap(x0, x1) > 0.0)
        })
    };
    let joins_fractions = row
        .iter()
        .any(|piece| piece.is_math() && piece.text.trim().chars().all(|c| matches!(c, '×' | '÷')))
        && neighbour(index.checked_sub(1).and_then(|below| lines.get(below)))
        && neighbour(lines.get(index + 1));
    (relation, joins_fractions, row.iter().map(|piece| piece.width()).sum())
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

fn build_formula<'a>(mut members: Vec<&Piece<'a>>, prose: &[&Piece<'a>]) -> Built<'a> {
    match build_formula_inner(&mut members, prose) {
        Ok(formula) => formula.map_or(Built::NotMath, |formula| Built::Formula(Box::new(formula))),
        Err(()) => Built::Refused,
    }
}

fn build_formula_inner<'a>(members: &mut Vec<&Piece<'a>>, prose: &[&Piece<'a>]) -> Result<Option<Formula<'a>>, ()> {
    members.sort_by(|a, b| a.x0.total_cmp(&b.x0));
    while members
        .last()
        .is_some_and(|piece| piece.class == Class::Neutral && is_equation_number(piece.text))
    {
        members.pop();
    }
    while is_trailing_note_mark(members) {
        members.pop();
    }
    if !has_math_evidence(members) {
        return Ok(None);
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
    let structured = has_structure(members, size);
    let related = members.iter().any(|piece| piece.text.chars().any(is_relation));
    let operated = related || members.iter().any(|piece| piece.text.chars().any(is_operator));
    if !structured && !(display && operated) {
        return Ok(None);
    }
    if !structured && !related && (only_operator_glyphs(members) || reads_as_numeric_row(members)) {
        return Ok(None);
    }
    let (Some(axis), tex) = (axis_row(members), group_tex(members, 0)) else {
        return Ok(None);
    };
    let Some(tex) = tex else {
        return Err(());
    };
    let Some(anchor) = axis.iter().min_by(|a, b| a.x0.total_cmp(&b.x0)) else {
        return Ok(None);
    };
    if tex.is_empty() {
        return Ok(None);
    }
    let closers = (!display)
        .then(|| missing_closers(members, prose, x1, anchor.base, size))
        .flatten();
    Ok(Some(Formula {
        closers,
        anchor_span: anchor.span,
        base: anchor.base,
        members: members.iter().map(|piece| **piece).collect(),
        display,
        tex,
        x0,
        x1,
        size,
    }))
}
