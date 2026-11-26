//! Pages set as two side-by-side copies of one table: a fund's schedule of investments prints
//! "Security Par (000) Value" over each half of the page and runs its rows down the left half,
//! then down the right half.
//!
//! The page is portrait and both halves share one row grid, so `two_up_gutter` rejects it, and
//! the gap between a half's labels and its amounts is wider than the gutter between the halves,
//! so the XY-cut splits at the label gaps and reads one half's amounts beside the other half's
//! labels. The repeated header row is what tells the two halves apart from one wide table.

use xberg_native_pdf::layout::TextSpan;

use super::super::text::widest_gap_midpoint;
use super::{
    MIN_COLUMN_GUTTER_PTS, MIN_TWO_COLUMN_CONTENT_WIDTH_PTS, PROSE_LINE_Y_TOLERANCE_PTS, content_bounds,
    group_spans_into_lines, has_balanced_vertical_support, is_usable_span, side_support,
};

/// Fewest alphabetic words each half of the header row repeats.
const MIN_REPEATED_HEADER_WORDS: usize = 2;
/// Largest distance of the gutter from the middle of the content, as a fraction of its width.
const MAX_GUTTER_OFFSET_FRACTION: f32 = 0.1;

/// Reorder `spans` into the text above the repeated header row, then the left half, then the
/// right half, each keeping its incoming order, returning whether the page is such a layout.
pub(super) fn read_repeated_header_halves(spans: &mut Vec<TextSpan>) -> bool {
    let Some((header_top, gutter_x)) = repeated_header_gutter(spans) else {
        return false;
    };
    let mut above = Vec::new();
    let mut left = Vec::new();
    let mut right = Vec::new();
    for span in spans.drain(..) {
        if span.bbox.y > header_top + PROSE_LINE_Y_TOLERANCE_PTS {
            above.push(span);
        } else if span.bbox.x + span.bbox.width * 0.5 < gutter_x {
            left.push(span);
        } else {
            right.push(span);
        }
    }
    spans.extend(above);
    spans.extend(left);
    spans.extend(right);
    true
}

/// The top of the header row and the gutter x of a page whose header row repeats its left
/// half's text in its right half, with an empty corridor between the halves that no span on
/// or below that row crosses, near the middle of the content, and running text on both sides.
/// The running-text test keeps out one wide table with repeated column groups ("2 Years ...
/// Life | 2 Years ... Life"), whose right half holds only amounts.
fn repeated_header_gutter(spans: &[TextSpan]) -> Option<(f32, f32)> {
    let usable: Vec<&TextSpan> = spans.iter().filter(|span| is_usable_span(span)).collect();
    let (content_min, content_max) = content_bounds(&usable)?;
    let content_width = content_max - content_min;
    if content_width < MIN_TWO_COLUMN_CONTENT_WIDTH_PTS {
        return None;
    }
    let content_mid = (content_min + content_max) * 0.5;
    group_spans_into_lines(&usable).into_iter().find_map(|line| {
        let (corridor_left, corridor_right) = repeated_header_corridor(&line)?;
        let header_top = line.iter().map(|span| span.bbox.y).fold(f32::NEG_INFINITY, f32::max);
        let body: Vec<&TextSpan> = usable
            .iter()
            .copied()
            .filter(|span| span.bbox.y <= header_top + PROSE_LINE_Y_TOLERANCE_PTS)
            .collect();
        let gutter_x = widest_empty_corridor_mid(&body, corridor_left, corridor_right)?;
        if (gutter_x - content_mid).abs() > content_width * MAX_GUTTER_OFFSET_FRACTION {
            return None;
        }
        let (left, right): (Vec<&TextSpan>, Vec<&TextSpan>) = body
            .into_iter()
            .partition(|span| span.bbox.x + span.bbox.width <= gutter_x);
        has_balanced_vertical_support(&side_support(left), &side_support(right)).then_some((header_top, gutter_x))
    })
}

/// The empty x-range between the halves of `line` (left to right) when its words, split in
/// half between two spans, repeat the left half in the right half.
fn repeated_header_corridor(line: &[&TextSpan]) -> Option<(f32, f32)> {
    let words: Vec<&str> = line.iter().flat_map(|span| span.text.split_whitespace()).collect();
    let half = words.len() / 2;
    let alphabetic_words = words[..half]
        .iter()
        .filter(|word| word.chars().any(char::is_alphabetic))
        .count();
    if !words.len().is_multiple_of(2) || alphabetic_words < MIN_REPEATED_HEADER_WORDS || words[..half] != words[half..]
    {
        return None;
    }
    let mut words_before = 0;
    let split = line.iter().position(|span| {
        let starts_right_half = words_before == half;
        words_before += span.text.split_whitespace().count();
        starts_right_half
    })?;
    let (left, right) = line.split_at(split);
    let corridor_left = left
        .iter()
        .map(|span| span.bbox.x + span.bbox.width)
        .fold(f32::NEG_INFINITY, f32::max);
    let corridor_right = right[0].bbox.x;
    (corridor_right > corridor_left).then_some((corridor_left, corridor_right))
}

/// The middle of the widest x-range inside `[start, end]` that no span of `spans` covers, if it
/// is at least `MIN_COLUMN_GUTTER_PTS` wide.
fn widest_empty_corridor_mid(spans: &[&TextSpan], start: f32, end: f32) -> Option<f32> {
    let mut extents: Vec<(f32, f32)> = spans
        .iter()
        .map(|span| (span.bbox.x, span.bbox.x + span.bbox.width))
        .filter(|&(left, right)| right > start && left < end)
        .collect();
    extents.sort_by(|left, right| left.0.total_cmp(&right.0));
    // Zero-width edges at `start` and `end` make the corridor's own ends gap boundaries. ~keep
    let edges = std::iter::once((start, start))
        .chain(extents)
        .chain(std::iter::once((end, end)));
    widest_gap_midpoint(edges, MIN_COLUMN_GUTTER_PTS, f32::INFINITY)
}

#[cfg(test)]
mod tests {
    use xberg_native_pdf::geometry::Rect;

    use super::*;

    const ROW_LEADING: f32 = 9.8;
    const HEADER_Y: f32 = 700.0;
    /// Label, par and value columns of one half; the right half sits 291pt further right.
    const HALF_COLUMNS: [f32; 3] = [18.0, 200.0, 253.0];
    const RIGHT_HALF_OFFSET: f32 = 291.0;

    fn span(text: &str, x: f32, y: f32, width: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            bbox: Rect::new(x, y, width, 9.0),
            font_size: 9.0,
            ..TextSpan::default()
        }
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

    /// One half of a schedule of investments: its header row, then holdings whose name wraps
    /// over two lines with the par amount and value on the second.
    fn schedule_half(offset: f32, label: &str, header: [&str; 3]) -> Vec<TextSpan> {
        let mut spans = vec![
            span(header[0], HALF_COLUMNS[0] + offset, HEADER_Y, 24.0),
            span(header[1], HALF_COLUMNS[1] + offset, HEADER_Y, 27.0),
            span(header[2], HALF_COLUMNS[2] + 16.0 + offset, HEADER_Y, 16.0),
        ];
        for holding in 0..8 {
            let y = HEADER_Y - (2 * holding + 1) as f32 * ROW_LEADING;
            spans.push(span(
                &format!("{label} holding {holding} issuer name, Series A,"),
                HALF_COLUMNS[0] + offset,
                y,
                110.0,
            ));
            let y = y - ROW_LEADING;
            spans.push(span(
                &format!("Class D floating rate note {label}{holding}"),
                HALF_COLUMNS[0] + 9.0 + offset,
                y,
                100.0,
            ));
            spans.push(span(&(1000 + holding).to_string(), HALF_COLUMNS[1] + offset, y, 17.0));
            spans.push(span(
                &format!("{},000", 2000 + holding),
                HALF_COLUMNS[2] + offset,
                y,
                29.0,
            ));
        }
        spans
    }

    fn texts(spans: &[TextSpan]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn halves_under_a_repeated_header_are_read_one_after_the_other() {
        let header = ["Security", "Par (000)", "Value"];
        let left = schedule_half(0.0, "left", header);
        let right = schedule_half(RIGHT_HALF_OFFSET, "right", header);
        let title = span(
            "Consolidated Schedule of Investments (continued)",
            18.0,
            HEADER_Y + 30.0,
            280.0,
        );
        let expected: Vec<String> = std::iter::once(&title)
            .chain(rows(left.clone()).iter())
            .chain(rows(right.clone()).iter())
            .map(|span| span.text.clone())
            .collect();
        let mut spans = rows(std::iter::once(title).chain(left).chain(right).collect());

        assert!(read_repeated_header_halves(&mut spans));

        assert_eq!(texts(&spans), expected);
    }

    /// Only the plain-text flow reads the halves one after the other; the structure and table
    /// passes keep the page order the table detector reads the side-by-side halves from.
    #[test]
    fn only_the_text_flow_reads_the_halves_one_after_the_other() {
        const PAGE_WIDTH: f32 = 594.0;
        const PAGE_HEIGHT: f32 = 774.0;
        let header = ["Security", "Par (000)", "Value"];
        let left = schedule_half(0.0, "left", header);
        let right = schedule_half(RIGHT_HALF_OFFSET, "right", header);
        let page = rows(left.clone().into_iter().chain(right.clone()).collect());
        let halves: Vec<String> = rows(left)
            .iter()
            .chain(rows(right).iter())
            .map(|span| span.text.clone())
            .collect();

        let mut text_flow = page.clone();
        super::super::reorder_text_flow_reading_order(&mut text_flow, PAGE_WIDTH, PAGE_HEIGHT, 0);
        let mut structure = page;
        super::super::reorder_page_reading_order(&mut structure, PAGE_WIDTH, PAGE_HEIGHT, 0);

        assert_eq!(texts(&text_flow), halves);
        assert_ne!(texts(&structure), halves);
    }

    #[test]
    fn a_wide_table_with_repeated_column_groups_is_left_alone() {
        let header = ["2 Years", "3 Years", "Life"];
        let mut spans = Vec::new();
        for (offset, x) in [(0.0, 60.0), (RIGHT_HALF_OFFSET, 60.0)] {
            for (column, text) in header.iter().enumerate() {
                spans.push(span(text, x + offset + column as f32 * 60.0, HEADER_Y, 30.0));
            }
            for row in 0..12 {
                let y = HEADER_Y - (row + 1) as f32 * ROW_LEADING;
                for column in 0..header.len() {
                    let amount = format!("{}.00", 13 + row + column);
                    spans.push(span(&amount, x + offset + column as f32 * 60.0, y, 22.0));
                }
            }
        }
        let mut spans = rows(spans);
        let before: Vec<String> = spans.iter().map(|span| span.text.clone()).collect();

        assert!(!read_repeated_header_halves(&mut spans));

        assert_eq!(texts(&spans), before);
    }

    #[test]
    fn halves_whose_headers_differ_are_left_alone() {
        let left = schedule_half(0.0, "left", ["Security", "Par (000)", "Value"]);
        let right = schedule_half(RIGHT_HALF_OFFSET, "right", ["Security", "Shares", "Value"]);
        let mut spans = rows(left.into_iter().chain(right).collect());

        assert!(!read_repeated_header_halves(&mut spans));
    }

    #[test]
    fn a_line_crossing_the_gutter_below_the_header_keeps_the_page_whole() {
        let header = ["Security", "Par (000)", "Value"];
        let mut spans: Vec<TextSpan> = schedule_half(0.0, "left", header)
            .into_iter()
            .chain(schedule_half(RIGHT_HALF_OFFSET, "right", header))
            .collect();
        spans.push(span(
            "A note set across the whole width of the page below both halves",
            18.0,
            HEADER_Y - 20.0 * ROW_LEADING,
            560.0,
        ));
        let mut spans = rows(spans);

        assert!(!read_repeated_header_halves(&mut spans));
    }
}
