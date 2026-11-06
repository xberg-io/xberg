//! Private source-strip ownership checks for ordered text.

use crate::types::{OcrBoundingGeometry, OcrElement, OcrElementLevel};
use std::ops::Range;

const MAX_ELEMENTS: usize = 1024;
const MAX_TEXT_BYTES: usize = 128 * 1024;
// Two numbered rows do not provide enough evidence to distinguish a list from a label/value pair.
pub(super) const MIN_ORDERED_ENTRIES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BoxPx {
    pub(super) x0: u32,
    pub(super) y0: u32,
    pub(super) x1: u32,
    pub(super) y1: u32,
}

impl BoxPx {
    pub(super) fn read(element: &OcrElement, frame: (u32, u32)) -> Option<Self> {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = element.geometry
        else {
            return None;
        };
        let x1 = left.checked_add(width)?;
        let y1 = top.checked_add(height)?;
        (left < x1 && top < y1 && x1 <= frame.0 && y1 <= frame.1).then_some(Self {
            x0: left,
            y0: top,
            x1,
            y1,
        })
    }

    fn contains(self, word: Self) -> bool {
        self.x0 <= word.x0 && word.x1 <= self.x1 && self.y0 <= word.y0 && word.y1 <= self.y1
    }
}

pub(super) struct SourceLine {
    pub(super) rows: Range<u32>,
    pub(super) prefix: Option<BoxPx>,
}

pub(super) struct SourceColumn {
    pub(super) separator: Range<u32>,
    pub(super) lines: Vec<SourceLine>,
}

fn source_matches(lines: &[&Line<'_>], source: &[SourceLine]) -> bool {
    if lines.len() != source.len() {
        return false;
    }
    for (i, (line, band)) in lines.iter().zip(source).enumerate() {
        let overlaps = |rows: &Range<u32>| line.bounds.y0 < rows.end && rows.start < line.bounds.y1;
        if !overlaps(&band.rows)
            || i.checked_sub(1)
                .is_some_and(|previous| overlaps(&source[previous].rows))
            || source.get(i + 1).is_some_and(|next| overlaps(&next.rows))
        {
            return false;
        }
        let Some((word, text)) = line.words.first() else {
            return false;
        };
        match (band.prefix, ordinal(text)) {
            (Some(prefix), Some(_))
                if word.x0 < prefix.x1 && prefix.x0 < word.x1 && word.y0 < prefix.y1 && prefix.y0 < word.y1 => {}
            (None, None) => {}
            _ => return false,
        }
    }
    true
}

struct Line<'a> {
    bounds: BoxPx,
    text: &'a str,
    words: Vec<(BoxPx, &'a str)>,
}

fn owned_lines(elements: &[OcrElement], frame: (u32, u32)) -> Option<Vec<Line<'_>>> {
    if elements.len() > MAX_ELEMENTS
        || elements
            .iter()
            .try_fold(0usize, |sum, element| sum.checked_add(element.text.len()))?
            > MAX_TEXT_BYTES
    {
        return None;
    }
    let mut lines = Vec::new();
    for element in elements.iter().filter(|element| element.level == OcrElementLevel::Line) {
        lines.push(Line {
            bounds: BoxPx::read(element, frame)?,
            text: &element.text,
            words: Vec::new(),
        });
    }
    for element in elements.iter().filter(|element| element.level == OcrElementLevel::Word) {
        let bounds = BoxPx::read(element, frame)?;
        let mut owners = lines.iter_mut().filter(|line| line.bounds.contains(bounds));
        let owner = owners.next()?;
        if owners.next().is_some() {
            return None;
        }
        owner.words.push((bounds, &element.text));
    }
    lines
        .iter()
        .all(|line| {
            !line.words.is_empty()
                && line
                    .words
                    .iter()
                    .map(|(_, text)| *text)
                    .eq(line.text.split_whitespace())
        })
        .then_some(lines)
}

struct Entry {
    ordinal: u32,
    marker: BoxPx,
    body_x: Range<u32>,
    continuations: usize,
}

fn ordinal(text: &str) -> Option<u32> {
    let digits = text.strip_suffix(['.', ',']).unwrap_or(text);
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse::<u32>().ok())
        .flatten()
}

fn starts_prose(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_alphabetic)
}

fn append_entry(lines: &[&Line<'_>], cursor: &mut usize) -> Option<Entry> {
    let line = lines.get(*cursor)?;
    let [(marker, number), (body, first), _, ..] = line.words.as_slice() else {
        return None;
    };
    if !starts_prose(first) {
        return None;
    }
    let mut entry = Entry {
        ordinal: ordinal(number)?,
        marker: *marker,
        body_x: body.x0..body.x1,
        continuations: 0,
    };
    *cursor += 1;
    while let Some(line) = lines.get(*cursor) {
        let (first, text) = line.words.first()?;
        if ordinal(text).is_some() {
            break;
        }
        if !starts_prose(text) || line.words.len() < 2 || first.x0 < marker.x1 {
            return None;
        }
        entry.body_x.start = entry.body_x.start.max(first.x0);
        entry.body_x.end = entry.body_x.end.min(first.x1);
        if entry.body_x.is_empty() {
            return None;
        }
        entry.continuations += 1;
        *cursor += 1;
    }
    (entry.continuations > 0).then_some(entry)
}

fn complete_ordered_group(lines: &[&Line<'_>]) -> Option<()> {
    let mut cursor = 0;
    let first = append_entry(lines, &mut cursor)?;
    let mut marker_x = first.marker.x0..first.marker.x1;
    let mut last = first.ordinal;
    let mut count = 1;
    while cursor < lines.len() {
        let next = append_entry(lines, &mut cursor)?;
        marker_x.start = marker_x.start.max(next.marker.x0);
        marker_x.end = marker_x.end.min(next.marker.x1);
        if marker_x.is_empty() || last.checked_add(1)? != next.ordinal {
            return None;
        }
        count += 1;
        last = next.ordinal;
    }
    (count >= MIN_ORDERED_ENTRIES).then_some(())
}

pub(super) fn right_column<'a>(
    elements: &'a [OcrElement],
    frame: (u32, u32),
    source: &SourceColumn,
) -> Option<Vec<(&'a str, BoxPx)>> {
    let separator = &source.separator;
    if separator.is_empty() || separator.end > frame.0 {
        return None;
    }
    let lines = owned_lines(elements, frame)?;
    let mut left = 0;
    let mut right = Vec::new();
    for line in &lines {
        if line.bounds.x1 <= separator.start {
            left += 1;
        } else if line.bounds.x0 >= separator.end {
            if !line.words.windows(2).all(|pair| pair[0].0.x1 <= pair[1].0.x0) {
                return None;
            }
            right.push(line);
        } else {
            return None;
        }
    }
    if left == 0 || right.windows(2).any(|pair| pair[0].bounds.y1 > pair[1].bounds.y0) {
        return None;
    }
    if !source_matches(&right, &source.lines) {
        return None;
    }
    complete_ordered_group(&right)?;
    Some(right.into_iter().map(|line| (line.text, line.bounds)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(text: &str, level: OcrElementLevel, x: u32, y: u32, width: u32) -> OcrElement {
        OcrElement {
            text: text.into(),
            level,
            geometry: OcrBoundingGeometry::Rectangle {
                left: x,
                top: y,
                width,
                height: 8,
            },
            ..Default::default()
        }
    }

    fn line(out: &mut Vec<OcrElement>, y: u32, words: &[(u32, &str)]) {
        let width = |text: &str| text.len() as u32 * 3;
        let first = words[0].0;
        let (last, tail) = words[words.len() - 1];
        out.push(element(
            &words.iter().map(|(_, text)| *text).collect::<Vec<_>>().join(" "),
            OcrElementLevel::Line,
            first,
            y,
            last + width(tail) - first,
        ));
        out.extend(
            words
                .iter()
                .map(|(x, text)| element(text, OcrElementLevel::Word, *x, y, width(text))),
        );
    }

    fn specimen(ordinals: &[&str], continuation: bool) -> Vec<OcrElement> {
        let mut out = Vec::new();
        line(&mut out, 10, &[(5, "caption")]);
        for (i, marker) in ordinals.iter().enumerate() {
            let y = i as u32 * 20 + 10;
            line(&mut out, y, &[(75, marker), (90, "Source"), (120, "record")]);
            if continuation {
                line(&mut out, y + 10, &[(91, "source"), (120, "continuation")]);
            }
        }
        out
    }

    fn painted_column(entries: usize) -> SourceColumn {
        SourceColumn {
            separator: 40..65,
            lines: (0..entries as u32 * 2)
                .map(|i| {
                    let y = i * 10 + 10;
                    SourceLine {
                        rows: y..y + 8,
                        prefix: (i % 2 == 0).then_some(BoxPx {
                            x0: 75,
                            y0: y,
                            x1: 78,
                            y1: y + 8,
                        }),
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn refuses_complete_ocr_group_that_omits_a_painted_fourth_entry() {
        let recognized = specimen(&["7", "8", "9"], true);
        assert!(right_column(&recognized, (200, 100), &painted_column(3)).is_some());
        assert!(right_column(&recognized, (200, 100), &painted_column(4)).is_none());
    }

    #[test]
    fn refuses_ordinal_box_outside_its_painted_prefix() {
        let recognized = specimen(&["7", "8", "9"], true);
        let mut source = painted_column(3);
        source.lines[2].prefix.as_mut().unwrap().x0 += 5;
        source.lines[2].prefix.as_mut().unwrap().x1 += 5;
        assert!(right_column(&recognized, (200, 100), &source).is_none());
    }

    #[test]
    fn retains_each_owned_line_of_complete_ordered_runs() {
        let specimen = specimen(&["7", "8.", "9,"], true);
        let lines = right_column(&specimen, (200, 100), &painted_column(3)).unwrap();
        assert_eq!(
            lines.iter().map(|(text, _)| *text).collect::<Vec<_>>(),
            [
                "7 Source record",
                "source continuation",
                "8. Source record",
                "source continuation",
                "9, Source record",
                "source continuation"
            ]
        );
    }

    #[test]
    fn refuses_an_incomplete_ordered_run_or_numeric_rows_without_continuations() {
        for (numbers, continuation) in [
            (&["7", "8"][..], true),
            (&["7", "8", "10"][..], true),
            (&["7", "8", "9"][..], false),
        ] {
            assert!(right_column(&specimen(numbers, continuation), (200, 100), &painted_column(3)).is_none());
        }
    }

    #[test]
    fn refuses_unowned_or_cross_separator_word_occurrences() {
        let mut sample = specimen(&["7", "8", "9"], true);
        for x in [43, 170] {
            sample.push(element("orphan", OcrElementLevel::Word, x, 11, 12));
            assert!(right_column(&sample, (200, 100), &painted_column(3)).is_none());
            sample.pop();
        }
    }
}
