//! Content-stream column order: keep the producer's span order on a page of three or more
//! side-by-side prose columns when that order already reads it column by column.
//!
//! A geometric gutter search models one gutter, and an XY-cut needs a page-wide horizontal
//! valley, which a heading spanning only some of the columns removes. Federal Register pages
//! defeat both and fell back to row order, interleaving three columns line by line. Their
//! producer writes the content stream in reading order, so the stream is read as the order
//! when its geometry proves it: walked in `sequence` order, the spans form runs that each
//! descend one column, the runs cover most of the page, at least three body-text prose runs
//! stand side by side, and no same-size block is written before a block to its left that it
//! stands beside or before a block above it. Shuffled, row-interleaved and bottom-first
//! streams fail these tests and keep the geometric order.

use std::collections::{HashMap, HashSet};

use crate::layout::TextSpan;

/// Fewest side-by-side body-text runs that make the page multi-column for this pass. Two-column
/// pages keep the geometric gutter repairs, which measure better there than stream order.
const MIN_STREAM_COLUMNS: usize = 3;
/// Fewest lines in a run for it to count as a column or a block.
const MIN_RUN_LINES: usize = 3;
/// Share of stream lines that must sit in runs of `MIN_RUN_LINES` or more.
const MIN_RUN_COVERAGE: f32 = 0.7;
/// Baseline difference, as a fraction of the smaller type size, within which two spans share a line.
const LINE_Y_TOLERANCE: f32 = 0.5;
/// Drop, as a fraction of the type size, that makes a line the next line of its run.
const MIN_LINE_DESCENT: f32 = 0.3;
/// Vertical overlap, as a fraction of the shorter run, for two runs to stand side by side.
const MIN_VERTICAL_OVERLAP: f32 = 0.5;
/// Largest relative difference from the page's body size for a run to be a body column.
const BODY_SIZE_TOLERANCE: f32 = 0.2;
/// Narrowest body column as a fraction of the content width; formula and cell stacks are narrower.
const MIN_COLUMN_WIDTH_FRACTION: f32 = 0.12;
/// Size ratio below which one of two runs is secondary type (notes, table headers), whose
/// position in the stream says nothing about the order of the body columns.
const SECONDARY_TYPE_RATIO: f32 = 0.9;
/// Most stream runs a page may break into for this pass. The run comparisons are quadratic, and a
/// prose page reads as a few runs per column; a page past this is fragments, not columns.
const MAX_STREAM_RUNS: usize = 512;
/// Fewest alphabetic words for a line to count as prose.
const MIN_PROSE_LINE_WORDS: usize = 3;
/// Most line breaks in a prose column that leave room for the next line's first word. Prose
/// breaks only where that word does not fit, so early breaks are paragraph ends; a text table
/// written column by column breaks at every cell end. Measured on the Federal Register pages
/// the median column breaks early 8% of the time; ragged text-table columns 36% to 64%.
const MAX_EARLY_BREAK_FRACTION: f32 = 1.0 / 3.0;
/// Percentile of a run's line ends taken as its right margin.
const RIGHT_MARGIN_PERCENTILE: f32 = 0.8;
/// Word space, as a fraction of the type size, assumed before a word that would have fit.
const WORD_SPACE: f32 = 0.25;
/// Slack, as a fraction of the type size, left at the right margin before a break counts as early.
const MARGIN_SLACK: f32 = 0.5;

struct StreamLine {
    left: f32,
    right: f32,
    y: f32,
    size: f32,
    words: usize,
    first_word_width: f32,
}

struct StreamRun {
    lines: Vec<usize>,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    size: f32,
    prose: bool,
}

/// Reorder `spans` into content-stream order when that order reads a page of at least three
/// side-by-side prose columns column by column, returning whether it did.
///
/// Lines follow the stream, except that notes written ahead of the column text above them are
/// read after it (`note_targets`); the spans of each line are then read left to right, so a run the
/// producer wrote late (a bold word, an out-of-order `TJ` piece) does not scramble its line.
/// Artifacts and rotated runs are left out of the analysis but, once the page is accepted, take
/// their stream position like every other span. A page with too few distinct `sequence` values
/// to describe a stream (synthetic spans all left at 0) is never reordered.
pub fn order_by_stream_columns(spans: &mut [TextSpan]) -> bool {
    let mut candidates: Vec<&TextSpan> = spans.iter().filter(|span| is_flow_span(span)).collect();
    candidates.sort_by_key(|span| span.sequence);
    let distinct: HashSet<usize> = candidates.iter().map(|span| span.sequence).collect();
    if distinct.len() < MIN_STREAM_COLUMNS * MIN_RUN_LINES {
        return false;
    }
    let (lines, line_of_candidate) = stream_lines(&candidates);
    let runs = stream_runs(&lines);
    if runs.len() > MAX_STREAM_RUNS || !reads_column_by_column(&lines, &runs) {
        return false;
    }
    let mut run_of_line = vec![0; lines.len()];
    for (run_index, run) in runs.iter().enumerate() {
        for &line in &run.lines {
            run_of_line[line] = run_index;
        }
    }
    let run_of_candidate: Vec<usize> = line_of_candidate.iter().map(|&line| run_of_line[line]).collect();
    spans.sort_by_key(|span| span.sequence);
    defer_notes_written_ahead_of_their_column(spans, &run_of_candidate, &note_targets(&lines, &runs));
    read_stream_lines_left_to_right(spans);
    true
}

/// For each run, the run it is read after instead of at its stream position: a run in secondary
/// type that the producer wrote before a body-size run standing above it in the same column is
/// read after the last such run. Federal Register pages write every column's footnotes ahead of
/// the body (2026-13845), so in stream order they open the page; read where they stand, each
/// column's notes follow that column's text.
///
/// A target is never itself in secondary type, so no target is moved and every note lands after a
/// span that keeps its place.
fn note_targets(lines: &[StreamLine], runs: &[StreamRun]) -> Vec<Option<usize>> {
    let Some(body_size) = body_size(lines) else {
        return vec![None; runs.len()];
    };
    runs.iter()
        .enumerate()
        .map(|(index, note)| {
            if note.size >= SECONDARY_TYPE_RATIO * body_size {
                return None;
            }
            runs.iter()
                .enumerate()
                .skip(index + 1)
                .filter(|(_, run)| {
                    run.size >= SECONDARY_TYPE_RATIO * body_size
                        && run.size - body_size <= BODY_SIZE_TOLERANCE * body_size
                        && run.right.min(note.right) > run.left.max(note.left)
                        && note.top <= run.bottom
                })
                .map(|(target, _)| target)
                .last()
        })
        .collect()
}

/// Move every flow span of a run with a target (`note_targets`) to just after the last flow span
/// of that target, keeping the stream order inside each moved run. `spans` must already be in
/// stream order, whose flow spans are then the `run_of_candidate` candidates in the same order.
fn defer_notes_written_ahead_of_their_column(
    spans: &mut [TextSpan],
    run_of_candidate: &[usize],
    targets: &[Option<usize>],
) {
    if targets.iter().all(Option::is_none) {
        return;
    }
    let flow_positions: Vec<usize> = (0..spans.len()).filter(|&index| is_flow_span(&spans[index])).collect();
    let mut last_position_of_run: Vec<Option<usize>> = vec![None; targets.len()];
    for (candidate, &position) in flow_positions.iter().enumerate() {
        last_position_of_run[run_of_candidate[candidate]] = Some(position);
    }
    let mut deferred_after: Vec<Vec<usize>> = vec![Vec::new(); spans.len()];
    let mut moved = vec![false; spans.len()];
    for (candidate, &position) in flow_positions.iter().enumerate() {
        if let Some(target) = targets[run_of_candidate[candidate]]
            && let Some(after) = last_position_of_run[target]
        {
            deferred_after[after].push(position);
            moved[position] = true;
        }
    }
    let mut order = Vec::with_capacity(spans.len());
    for position in 0..spans.len() {
        if !moved[position] {
            order.push(position);
            order.extend_from_slice(&deferred_after[position]);
        }
    }
    debug_assert_eq!(order.len(), spans.len(), "a deferred run lost its anchor span");
    let mut taken: Vec<TextSpan> = spans.iter_mut().map(std::mem::take).collect();
    for (slot, &position) in spans.iter_mut().zip(&order) {
        *slot = std::mem::take(&mut taken[position]);
    }
}

fn type_size(span: &TextSpan) -> f32 {
    if span.font_size.is_finite() && span.font_size > 0.0 {
        span.font_size
    } else {
        span.bbox.height
    }
}

fn is_flow_span(span: &TextSpan) -> bool {
    span.artifact_type.is_none()
        && span.rotation_degrees == 0.0
        && !span.text.trim().is_empty()
        && span.bbox.x.is_finite()
        && span.bbox.y.is_finite()
        && span.bbox.width.is_finite()
        && span.bbox.width > 0.0
        && span.bbox.height.is_finite()
        && span.bbox.height > 0.0
        && type_size(span).is_finite()
        && type_size(span) > 0.0
}

fn same_line(span: &TextSpan, line_y: f32, line_size: f32) -> bool {
    (span.bbox.y - line_y).abs() <= LINE_Y_TOLERANCE * type_size(span).min(line_size)
}

fn alphabetic_words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count()
}

/// Width of the span's first word, its share of the span's characters.
fn first_word_width(span: &TextSpan) -> f32 {
    let text = span.text.trim();
    let first = text.split_whitespace().next().unwrap_or_default();
    span.bbox.width * first.chars().count() as f32 / text.chars().count().max(1) as f32
}

/// Group consecutive stream spans on one baseline into lines, also returning each span's line.
fn stream_lines(spans: &[&TextSpan]) -> (Vec<StreamLine>, Vec<usize>) {
    let mut lines: Vec<StreamLine> = Vec::new();
    let mut line_of_span = Vec::with_capacity(spans.len());
    for span in spans {
        let (left, right) = (span.bbox.x, span.bbox.x + span.bbox.width);
        let words = alphabetic_words(&span.text);
        if let Some(line) = lines.last_mut()
            && same_line(span, line.y, line.size)
        {
            if left < line.left {
                line.left = left;
                line.first_word_width = first_word_width(span);
            }
            line.right = line.right.max(right);
            line.words += words;
            line_of_span.push(lines.len() - 1);
            continue;
        }
        line_of_span.push(lines.len());
        lines.push(StreamLine {
            left,
            right,
            y: span.bbox.y,
            size: type_size(span),
            words,
            first_word_width: first_word_width(span),
        });
    }
    (lines, line_of_span)
}

/// Share of the run's line breaks after which the next line's first word would still have fit
/// before the run's right margin.
fn early_break_fraction(lines: &[StreamLine], group: &[usize]) -> f32 {
    let mut ends: Vec<f32> = group.iter().map(|&index| lines[index].right).collect();
    ends.sort_by(f32::total_cmp);
    let margin = ends[((ends.len() - 1) as f32 * RIGHT_MARGIN_PERCENTILE) as usize];
    let breaks = group.len() - 1;
    let early = group
        .windows(2)
        .filter(|pair| {
            let (line, next) = (&lines[pair[0]], &lines[pair[1]]);
            line.right + WORD_SPACE * line.size + next.first_word_width < margin - MARGIN_SLACK * line.size
        })
        .count();
    early as f32 / breaks.max(1) as f32
}

/// Group consecutive lines into runs, each line below the previous one and overlapping it
/// horizontally. A run's extent and size are medians, so a spanning heading at its top or a
/// wide block at its foot does not widen it across a gutter.
///
/// A line opening a column's width or more left of the previous line starts a new run, and so
/// does a body-size line after a run set mostly in note type: a column descends at one left
/// edge, so a jump left is a full-width block (a table, its notes) or the next column's foot,
/// and notes do not continue into body text. Federal Register pages write a table spanning the
/// columns between the top of the last column and the foot of the first (2026-14578 page 9) or
/// write the table's notes straight before the first column's text (page 7); merged into one
/// run with the column text, neither column counted as prose and the page fell back to row
/// order.
fn stream_runs(lines: &[StreamLine]) -> Vec<StreamRun> {
    let content_left = lines.iter().map(|line| line.left).fold(f32::INFINITY, f32::min);
    let content_right = lines.iter().map(|line| line.right).fold(f32::NEG_INFINITY, f32::max);
    let max_left_jump = MIN_COLUMN_WIDTH_FRACTION * (content_right - content_left);
    let body = body_size(lines);
    let is_note_type = |size: f32| body.is_some_and(|body| size < (1.0 - BODY_SIZE_TOLERANCE) * body);
    let is_body_type = |size: f32| body.is_some_and(|body| (size - body).abs() <= BODY_SIZE_TOLERANCE * body);
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut note_lines_in_last_group = 0;
    for (index, line) in lines.iter().enumerate() {
        if let Some(group) = groups.last_mut() {
            let previous = &lines[*group.last().expect("groups are never empty")];
            let descends = line.y < previous.y - MIN_LINE_DESCENT * line.size;
            let overlaps = line.right.min(previous.right) > line.left.max(previous.left);
            let jumps_left = line.left < previous.left - max_left_jump;
            let leaves_notes = is_body_type(line.size) && note_lines_in_last_group > group.len() / 2;
            if descends && overlaps && !jumps_left && !leaves_notes {
                group.push(index);
                note_lines_in_last_group += usize::from(is_note_type(line.size));
                continue;
            }
        }
        groups.push(vec![index]);
        note_lines_in_last_group = usize::from(is_note_type(line.size));
    }
    groups
        .into_iter()
        .map(|group| {
            let median = |value: fn(&StreamLine) -> f32| {
                let mut values: Vec<f32> = group.iter().map(|&index| value(&lines[index])).collect();
                values.sort_by(f32::total_cmp);
                values[values.len() / 2]
            };
            let first = &lines[group[0]];
            let last = &lines[*group.last().expect("groups are never empty")];
            let prose_lines = group
                .iter()
                .filter(|&&index| lines[index].words >= MIN_PROSE_LINE_WORDS)
                .count();
            let prose =
                prose_lines * 2 >= group.len() && early_break_fraction(lines, &group) <= MAX_EARLY_BREAK_FRACTION;
            StreamRun {
                left: median(|line| line.left),
                right: median(|line| line.right),
                top: first.y + first.size,
                bottom: last.y,
                size: median(|line| line.size),
                prose,
                lines: group,
            }
        })
        .collect()
}

fn side_by_side(one: &StreamRun, other: &StreamRun) -> bool {
    let overlap = one.top.min(other.top) - one.bottom.max(other.bottom);
    let shorter = (one.top - one.bottom).min(other.top - other.bottom);
    shorter > 0.0 && overlap / shorter >= MIN_VERTICAL_OVERLAP
}

fn reads_column_by_column(lines: &[StreamLine], runs: &[StreamRun]) -> bool {
    let blocks: Vec<&StreamRun> = runs.iter().filter(|run| run.lines.len() >= MIN_RUN_LINES).collect();
    let covered: usize = blocks.iter().map(|run| run.lines.len()).sum();
    if (covered as f32) < MIN_RUN_COVERAGE * lines.len() as f32 {
        return false;
    }
    column_count(lines, &blocks) >= MIN_STREAM_COLUMNS && !writes_a_block_out_of_order(&blocks)
}

/// The type size most stream lines are set in.
fn body_size(lines: &[StreamLine]) -> Option<f32> {
    let mut size_counts: HashMap<i32, usize> = HashMap::new();
    for line in lines {
        *size_counts.entry((line.size * 10.0).round() as i32).or_default() += 1;
    }
    size_counts
        .into_iter()
        .max_by_key(|&(size, count)| (count, std::cmp::Reverse(size)))
        .map(|(size, _)| size as f32 / 10.0)
}

/// The most body-text runs that stand side by side, disjoint left to right.
fn column_count(lines: &[StreamLine], blocks: &[&StreamRun]) -> usize {
    let Some(body_size) = body_size(lines) else {
        return 0;
    };
    let content_left = lines.iter().map(|line| line.left).fold(f32::INFINITY, f32::min);
    let content_right = lines.iter().map(|line| line.right).fold(f32::NEG_INFINITY, f32::max);
    let min_width = MIN_COLUMN_WIDTH_FRACTION * (content_right - content_left);
    let columns: Vec<&StreamRun> = blocks
        .iter()
        .copied()
        .filter(|run| {
            run.prose
                && (run.size - body_size).abs() <= BODY_SIZE_TOLERANCE * body_size
                && run.right - run.left >= min_width
        })
        .collect();
    columns
        .iter()
        .map(|&anchor| {
            let mut peers: Vec<&StreamRun> = columns
                .iter()
                .copied()
                .filter(|&peer| side_by_side(anchor, peer))
                .collect();
            peers.sort_by(|left, right| left.left.total_cmp(&right.left));
            let mut edge = f32::NEG_INFINITY;
            peers
                .iter()
                .filter(|peer| {
                    let disjoint = peer.left >= edge;
                    if disjoint {
                        edge = peer.right;
                    }
                    disjoint
                })
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// True when a block is written before a same-size block that stands beside it on its left, or
/// before a same-size block above it that it overlaps horizontally (a lower story written first).
fn writes_a_block_out_of_order(blocks: &[&StreamRun]) -> bool {
    blocks.iter().enumerate().any(|(later_index, later)| {
        blocks[..later_index].iter().any(|earlier| {
            let comparable = earlier.size.min(later.size) >= SECONDARY_TYPE_RATIO * earlier.size.max(later.size);
            let right_of_later = earlier.left >= later.right && side_by_side(earlier, later);
            let overlaps_horizontally = earlier.right.min(later.right) > earlier.left.max(later.left);
            let below_later = overlaps_horizontally && earlier.top <= later.bottom;
            comparable && (right_of_later || below_later)
        })
    })
}

/// Sort each run of consecutive same-baseline flow spans left to right, keeping the line order.
fn read_stream_lines_left_to_right(spans: &mut [TextSpan]) {
    let mut start = 0;
    while start < spans.len() {
        let mut end = start + 1;
        if is_flow_span(&spans[start]) {
            let (line_y, line_size) = (spans[start].bbox.y, type_size(&spans[start]));
            while end < spans.len() && is_flow_span(&spans[end]) && same_line(&spans[end], line_y, line_size) {
                end += 1;
            }
            spans[start..end].sort_by(|left, right| left.bbox.x.total_cmp(&right.bbox.x));
        }
        start = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    const LEADING: f32 = 11.0;
    const BODY_SIZE: f32 = 9.0;
    const TOP_Y: f32 = 720.0;
    const COLUMN_LEFTS: [f32; 3] = [45.0, 222.0, 399.0];
    const COLUMN_WIDTH: f32 = 156.0;

    fn span(text: &str, x: f32, y: f32, width: f32, size: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, width, size),
            font_size: size,
            ..TextSpan::default()
        }
    }

    /// `lines` prose lines down the column at `left`, starting `skip` lines below the top.
    fn column(label: &str, left: f32, skip: usize, lines: usize) -> Vec<TextSpan> {
        (0..lines)
            .map(|line| {
                let y = TOP_Y - (skip + line) as f32 * LEADING;
                span(
                    &format!("{label} line {line} of prose"),
                    left,
                    y,
                    COLUMN_WIDTH,
                    BODY_SIZE,
                )
            })
            .collect()
    }

    /// Number `spans` in the order given, the order a producer wrote them.
    fn written(spans: Vec<TextSpan>) -> Vec<TextSpan> {
        spans
            .into_iter()
            .enumerate()
            .map(|(sequence, span)| TextSpan { sequence, ..span })
            .collect()
    }

    /// Top to bottom, then left to right: the order the extractor hands over.
    fn rows(mut spans: Vec<TextSpan>) -> Vec<TextSpan> {
        spans.sort_by(|left, right| {
            right
                .bbox
                .y
                .total_cmp(&left.bbox.y)
                .then(left.bbox.x.total_cmp(&right.bbox.x))
        });
        spans
    }

    fn texts(spans: &[TextSpan]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    /// Column one runs the full height; a heading spans columns two and three above them, the
    /// Federal Register layout that leaves no page-wide horizontal valley for an XY-cut.
    fn three_columns_under_spanning_heading() -> Vec<TextSpan> {
        let mut spans = column("first", COLUMN_LEFTS[0], 0, 8);
        spans.push(span(
            "Spanning Heading Over Two Columns",
            COLUMN_LEFTS[1],
            TOP_Y,
            COLUMN_LEFTS[2] + COLUMN_WIDTH - COLUMN_LEFTS[1],
            BODY_SIZE,
        ));
        spans.extend(column("second", COLUMN_LEFTS[1], 2, 6));
        spans.extend(column("third", COLUMN_LEFTS[2], 2, 6));
        spans
    }

    #[test]
    fn three_column_stream_with_spanning_heading_reads_column_major() {
        let reading_order = three_columns_under_spanning_heading();
        let expected: Vec<String> = reading_order.iter().map(|span| span.text.clone()).collect();
        let mut spans = rows(written(reading_order));
        assert_ne!(texts(&spans), expected, "the fixture must arrive row-interleaved");

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    #[test]
    fn split_runs_sharing_a_sequence_still_read_column_major() {
        let mut expected = Vec::new();
        let mut spans = Vec::new();
        for (sequence, whole) in three_columns_under_spanning_heading().into_iter().enumerate() {
            let half = whole.bbox.width / 2.0;
            for (piece, offset) in [("head", 0.0), ("tail", half)] {
                let text = format!("{} {piece}", whole.text);
                expected.push(text.clone());
                spans.push(TextSpan {
                    text,
                    bbox: Rect::new(whole.bbox.x + offset, whole.bbox.y, half, whole.bbox.height),
                    sequence,
                    ..whole.clone()
                });
            }
        }
        let mut spans = rows(spans);

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    #[test]
    fn words_written_out_of_order_on_a_line_read_left_to_right() {
        let mut expected = Vec::new();
        let mut written_order = Vec::new();
        for whole in three_columns_under_spanning_heading() {
            let half = whole.bbox.width / 2.0;
            let piece = |name: &str, offset: f32| TextSpan {
                text: format!("{} {name}", whole.text),
                bbox: Rect::new(whole.bbox.x + offset, whole.bbox.y, half, whole.bbox.height),
                ..whole.clone()
            };
            let (head, tail) = (piece("head", 0.0), piece("tail", half));
            expected.extend([head.text.clone(), tail.text.clone()]);
            written_order.extend([tail, head]);
        }
        let mut spans = rows(written(written_order));

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    #[test]
    fn lower_story_written_before_the_story_above_keeps_geometric_order() {
        let mut written_order = Vec::new();
        for (label, &left) in ["lower first", "lower second", "lower third"].iter().zip(&COLUMN_LEFTS) {
            written_order.extend(column(label, left, 10, 8));
        }
        for (label, &left) in ["upper first", "upper second", "upper third"].iter().zip(&COLUMN_LEFTS) {
            written_order.extend(column(label, left, 0, 8));
        }
        let mut spans = rows(written(written_order));
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        assert!(!order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), before);
    }

    /// Three columns of text cells, each cell three or more words, written column by column.
    /// Every column passes the word count, so only the early-break test tells the cells from
    /// prose: most cells end with room left for the next cell's first word.
    #[test]
    fn text_table_written_column_by_column_keeps_geometric_order() {
        const CELL_WIDTHS: [f32; 8] = [60.0, 150.0, 90.0, 120.0, 70.0, 140.0, 80.0, 156.0];
        let mut written_order = Vec::new();
        for &left in &COLUMN_LEFTS {
            for (row, &width) in CELL_WIDTHS.iter().enumerate() {
                let y = TOP_Y - row as f32 * LEADING;
                let text = format!("cell text in row {row} at {left}");
                written_order.push(span(&text, left, y, width, BODY_SIZE));
            }
        }
        let mut spans = rows(written(written_order));
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        assert!(!order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), before);
    }

    #[test]
    fn row_interleaved_stream_keeps_geometric_order() {
        let mut spans = rows(written(rows(three_columns_under_spanning_heading())));
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        assert!(!order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), before);
    }

    #[test]
    fn two_column_stream_is_left_to_the_gutter_repairs() {
        let mut reading_order = column("first", COLUMN_LEFTS[0], 0, 8);
        reading_order.extend(column("second", COLUMN_LEFTS[1], 0, 8));
        let mut spans = rows(written(reading_order));

        assert!(!order_by_stream_columns(&mut spans));
    }

    #[test]
    fn single_column_stream_keeps_geometric_order() {
        let mut spans = rows(written(column("only", COLUMN_LEFTS[0], 0, 20)));

        assert!(!order_by_stream_columns(&mut spans));
    }

    #[test]
    fn spans_without_stream_sequence_are_never_reordered() {
        let mut spans = rows(three_columns_under_spanning_heading());

        assert!(!order_by_stream_columns(&mut spans));
    }

    #[test]
    fn column_written_before_its_left_neighbour_keeps_geometric_order() {
        let mut reading_order = column("second", COLUMN_LEFTS[1], 0, 8);
        reading_order.extend(column("first", COLUMN_LEFTS[0], 0, 8));
        reading_order.extend(column("third", COLUMN_LEFTS[2], 0, 8));
        let mut spans = rows(written(reading_order));

        assert!(!order_by_stream_columns(&mut spans));
    }

    /// Footnotes under columns two and three, written before the body (2026-13845). Column one
    /// runs down beside them, so the notes stand right of a run written after them.
    fn notes_written_ahead_of_the_body(note_size: f32) -> Vec<TextSpan> {
        let mut reading_order = Vec::new();
        for (column_index, &left) in COLUMN_LEFTS[1..].iter().enumerate() {
            for line in 0..3 {
                let y = TOP_Y - 16.0 * LEADING - line as f32 * 8.0;
                let text = format!("footnote {column_index} line {line} in smaller type");
                reading_order.push(span(&text, left, y, COLUMN_WIDTH, note_size));
            }
        }
        reading_order.extend(column("first", COLUMN_LEFTS[0], 0, 20));
        reading_order.extend(column("second", COLUMN_LEFTS[1], 0, 14));
        reading_order.extend(column("third", COLUMN_LEFTS[2], 0, 14));
        reading_order
    }

    #[test]
    fn notes_written_ahead_of_the_body_are_read_after_their_column() {
        let mut spans = rows(written(notes_written_ahead_of_the_body(7.0)));

        assert!(order_by_stream_columns(&mut spans));

        let mut expected = Vec::new();
        for (label, lines, notes_column) in [("first", 20, None), ("second", 14, Some(0)), ("third", 14, Some(1))] {
            expected.extend((0..lines).map(|line| format!("{label} line {line} of prose")));
            if let Some(column_index) = notes_column {
                expected.extend((0..3).map(|line| format!("footnote {column_index} line {line} in smaller type")));
            }
        }
        assert_eq!(texts(&spans), expected);
    }

    #[test]
    fn notes_written_after_the_body_keep_their_stream_position() {
        let mut reading_order = notes_written_ahead_of_the_body(7.0);
        let notes: Vec<TextSpan> = reading_order.drain(..6).collect();
        reading_order.extend(notes);
        let expected: Vec<String> = reading_order.iter().map(|span| span.text.clone()).collect();
        let mut spans = rows(written(reading_order));

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    fn lines(label: &str, left: f32, width: f32, size: f32, ys: &[f32]) -> Vec<TextSpan> {
        ys.iter()
            .enumerate()
            .map(|(line, &y)| span(&format!("{label} line {line} of prose"), left, y, width, size))
            .collect()
    }

    /// A note under column two whose last target stands in type between 0.8 and 0.9 of the body
    /// size: once both a note and a target, that run was moved after its own target, and the
    /// first note, anchored on it, was dropped from the page.
    #[test]
    fn note_whose_target_is_itself_a_note_keeps_its_spans() {
        let note = lines("note", COLUMN_LEFTS[1], 60.0, 6.0, &[500.0, 492.0, 484.0]);
        let middle = lines("middle", COLUMN_LEFTS[1], COLUMN_WIDTH, 7.5, &[540.0, 532.0, 524.0]);
        let right_part = lines(
            "aside",
            300.0,
            COLUMN_LEFTS[1] + COLUMN_WIDTH - 300.0,
            BODY_SIZE,
            &[560.0, 549.0],
        );
        let mut reading_order = note.clone();
        reading_order.extend(column("first", COLUMN_LEFTS[0], 0, 20));
        reading_order.extend(column("second", COLUMN_LEFTS[1], 0, 14));
        reading_order.extend(column("third", COLUMN_LEFTS[2], 0, 14));
        reading_order.extend(middle.clone());
        reading_order.extend(right_part.clone());
        let mut spans = rows(written(reading_order));

        assert!(order_by_stream_columns(&mut spans));

        let expected: Vec<String> = column("first", COLUMN_LEFTS[0], 0, 20)
            .into_iter()
            .chain(column("second", COLUMN_LEFTS[1], 0, 14))
            .chain(note)
            .chain(column("third", COLUMN_LEFTS[2], 0, 14))
            .chain(right_part)
            .chain(middle)
            .map(|span| span.text)
            .collect();
        assert_eq!(texts(&spans), expected);
    }

    #[test]
    fn body_size_block_written_ahead_of_its_left_neighbour_keeps_geometric_order() {
        let mut spans = rows(written(notes_written_ahead_of_the_body(BODY_SIZE)));

        assert!(!order_by_stream_columns(&mut spans));
    }

    /// A block spanning all three columns, `count` lines starting `skip` lines below the top.
    fn spanning_block(label: &str, skip: usize, count: usize, size: f32) -> Vec<TextSpan> {
        let width = COLUMN_LEFTS[2] + COLUMN_WIDTH - COLUMN_LEFTS[0];
        (0..count)
            .map(|row| {
                let y = TOP_Y - (skip + row) as f32 * LEADING;
                span(
                    &format!("{label} row {row} across the page"),
                    COLUMN_LEFTS[0],
                    y,
                    width,
                    size,
                )
            })
            .collect()
    }

    /// 2026-14578 page 9: the upper story's three columns, then a table spanning the page written
    /// right after the top of the third column, then the lower story's three columns.
    #[test]
    fn table_written_between_two_stories_reads_in_stream_order() {
        let mut reading_order = Vec::new();
        for (label, &left) in ["upper first", "upper second", "upper third"].iter().zip(&COLUMN_LEFTS) {
            reading_order.extend(column(label, left, 0, 8));
        }
        reading_order.extend(spanning_block("table", 9, 2, BODY_SIZE));
        for (label, &left) in ["lower first", "lower second", "lower third"].iter().zip(&COLUMN_LEFTS) {
            reading_order.extend(column(label, left, 12, 16));
        }
        let expected: Vec<String> = reading_order.iter().map(|span| span.text.clone()).collect();
        let mut spans = rows(written(reading_order));

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    /// 2026-14578 page 7: a table spanning the page, its notes in smaller type, then the three
    /// columns below, written in that order.
    #[test]
    fn table_notes_written_before_the_columns_read_in_stream_order() {
        let mut reading_order = spanning_block("table", 0, 2, BODY_SIZE);
        reading_order.extend((0..6).map(|line| {
            let y = TOP_Y - 2.0 * LEADING - line as f32 * 8.0;
            let width = COLUMN_LEFTS[2] + COLUMN_WIDTH - COLUMN_LEFTS[0];
            span(
                &format!("table note line {line} in smaller type"),
                COLUMN_LEFTS[0],
                y,
                width,
                7.0,
            )
        }));
        for (label, &left) in ["first", "second", "third"].iter().zip(&COLUMN_LEFTS) {
            reading_order.extend(column(label, left, 7, 14));
        }
        let expected: Vec<String> = reading_order.iter().map(|span| span.text.clone()).collect();
        let mut spans = rows(written(reading_order));

        assert!(order_by_stream_columns(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    /// Three long prose columns followed by more one-line fragments in small type than the
    /// quadratic run passes are allowed to compare. The columns keep the body size and the line
    /// coverage, so only the run cap turns the page down.
    #[test]
    fn page_of_too_many_stream_runs_keeps_geometric_order() {
        const COLUMN_LINES: usize = 420;
        let mut reading_order = Vec::new();
        for (label, &left) in ["first", "second", "third"].iter().zip(&COLUMN_LEFTS) {
            reading_order.extend(column(label, left, 0, COLUMN_LINES));
        }
        let mut control = rows(written(reading_order.clone()));
        assert!(
            order_by_stream_columns(&mut control),
            "the columns alone read column-major"
        );
        for fragment in 0..=MAX_STREAM_RUNS {
            let (x, y) = if fragment % 2 == 0 {
                (COLUMN_LEFTS[0], 10.0)
            } else {
                (COLUMN_LEFTS[2], 30.0)
            };
            reading_order.push(span(&format!("fragment {fragment}"), x, y, 20.0, 4.0));
        }
        let mut spans = rows(written(reading_order));

        assert!(!order_by_stream_columns(&mut spans));
    }

    #[test]
    fn table_written_column_by_column_keeps_geometric_order() {
        let mut reading_order = Vec::new();
        for &left in &COLUMN_LEFTS {
            for row in 0..8 {
                let y = TOP_Y - row as f32 * LEADING;
                reading_order.push(span(&(100 + row).to_string(), left, y, COLUMN_WIDTH, BODY_SIZE));
            }
        }
        let mut spans = rows(written(reading_order));

        assert!(!order_by_stream_columns(&mut spans));
    }
}
