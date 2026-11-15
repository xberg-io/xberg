use super::bounds;
use super::ordering::LineRanks;
use crate::types::{
    ContentLayer, OcrElement,
    internal::{ElementKind, InternalDocument, InternalElement},
};
use std::{collections::HashMap, ops::Range};

#[derive(Debug)]
pub(in crate::extractors::pdf::ocr) struct ColumnLanes {
    frame: (u32, u32),
    cuts: Vec<u32>,
    lines: Vec<SourceLine>,
}

#[derive(Debug)]
struct SourceLine {
    text: String,
    bounds: [u32; 4],
    vetoed: bool,
}

struct Destination {
    lines: Vec<usize>,
    lane: usize,
    top: u32,
}

impl ColumnLanes {
    pub(super) fn from_read(frame: (u32, u32), elements: &[OcrElement], ranks: &LineRanks) -> Option<Self> {
        let lines: Vec<_> = elements
            .iter()
            .zip(&ranks.complete)
            .zip(&ranks.vetoes)
            .filter_map(|((element, &complete), &vetoed)| {
                if !complete {
                    return None;
                }
                Some(SourceLine {
                    text: element.text.clone(),
                    bounds: bounds(element)?,
                    vetoed,
                })
            })
            .collect();
        let cuts = gutters(&lines)?;
        let mut lane_ranks = vec![None::<(usize, usize)>; cuts.len() + 1];
        for line in &ranks.lines {
            let Some(lane) = lane(line.bounds, &cuts) else { continue };
            let (first, last) = lane_ranks[lane].get_or_insert((line.rank, line.rank));
            *first = (*first).min(line.rank);
            *last = (*last).max(line.rank);
        }
        if !lane_ranks
            .windows(2)
            .all(|pair| matches!(pair, [Some((_, a)), Some((b, _))] if a < b))
        {
            return None;
        }
        Some(Self { frame, cuts, lines })
    }

    pub(in crate::extractors::pdf::ocr) fn frame(&self) -> (u32, u32) {
        self.frame
    }

    pub(in crate::extractors::pdf::ocr) fn plan(
        &self,
        doc: &InternalDocument,
        scale: (f32, f32),
    ) -> Option<Vec<usize>> {
        if !doc.relationships.is_empty()
            || !scale.0.is_finite()
            || !scale.1.is_finite()
            || scale.0 <= 0.0
            || scale.1 <= 0.0
        {
            return None;
        }
        let mut literals: HashMap<&str, Vec<(usize, Vec<&str>)>> = HashMap::new();
        for (i, line) in self.lines.iter().enumerate() {
            let tokens: Vec<_> = line.text.split_whitespace().collect();
            if let Some(first) = tokens.first() {
                literals.entry(first).or_default().push((i, tokens));
            }
        }
        let mut occurrences = vec![0usize; self.lines.len()];
        let destinations: Vec<_> = doc
            .elements
            .iter()
            .map(|element| self.destination(element, scale, &literals, &mut occurrences))
            .collect();
        let mut order: Vec<_> = (0..destinations.len()).collect();
        let mut start = 0;
        for end in 0..=destinations.len() {
            if destinations
                .get(end)
                .and_then(Option::as_ref)
                .is_some_and(|dest| dest.lines.iter().all(|&line| occurrences[line] == 1))
            {
                continue;
            }
            reorder_run(start..end, &destinations, &mut order);
            start = end + 1;
        }
        order.iter().copied().ne(0..order.len()).then_some(order)
    }

    fn destination(
        &self,
        element: &InternalElement,
        scale: (f32, f32),
        literals: &HashMap<&str, Vec<(usize, Vec<&str>)>>,
        occurrences: &mut [usize],
    ) -> Option<Destination> {
        let tokens: Vec<_> = element.text.split_whitespace().collect();
        let mut edges = vec![None; tokens.len()];
        for (start, token) in tokens.iter().enumerate() {
            let Some(matches) = literals.get(token) else { continue };
            let mut unique = None;
            for (line, literal) in matches {
                let end = start + literal.len();
                if tokens.get(start..end) != Some(literal.as_slice()) {
                    continue;
                }
                occurrences[*line] += 1;
                unique = if unique.is_none() {
                    Some((end, *line))
                } else {
                    Some((0, 0))
                };
            }
            edges[start] = unique.filter(|&(end, _)| end > start);
        }
        if element.kind != ElementKind::Paragraph
            || element.layer != ContentLayer::Body
            || element.depth != 0
            || !element.annotations.is_empty()
            || element.anchor.is_some()
        {
            return None;
        }
        let mut lines = Vec::new();
        let mut cursor = 0;
        while cursor < tokens.len() {
            let (end, line) = edges[cursor]?;
            if self.lines[line].vetoed {
                return None;
            }
            lines.push(line);
            cursor = end;
        }
        let mut bounds = self.lines[*lines.first()?].bounds;
        for &line in &lines[1..] {
            let [x0, y0, x1, y1] = self.lines[line].bounds;
            bounds = [
                bounds[0].min(x0),
                bounds[1].min(y0),
                bounds[2].max(x1),
                bounds[3].max(y1),
            ];
        }
        let lane = lane(bounds, &self.cuts)?;
        let [left, top, right, bottom] = bounds;
        let expected = [
            left as f32 * scale.0,
            self.frame.1.checked_sub(bottom)? as f32 * scale.1,
            right as f32 * scale.0,
            self.frame.1.checked_sub(top)? as f32 * scale.1,
        ];
        let bbox = element.bbox?;
        [bbox.x0, bbox.y0, bbox.x1, bbox.y1]
            .into_iter()
            .zip(expected)
            .all(|(a, b)| (a - f64::from(b)).abs() <= 1.0)
            .then_some(Destination { lines, lane, top })
    }
}

fn reorder_run(run: Range<usize>, destinations: &[Option<Destination>], order: &mut [usize]) {
    if run.len() < 3 {
        return;
    }
    let mut last_rows = HashMap::new();
    for i in run.clone() {
        let Some(dest) = &destinations[i] else { return };
        if last_rows.insert(dest.lane, dest.top).is_some_and(|top| top > dest.top) {
            return;
        }
    }
    if last_rows.len() > 1 {
        order[run].sort_by_key(|&i| destinations[i].as_ref().map(|dest| dest.lane));
    }
}

fn lane([left, _, right, _]: [u32; 4], cuts: &[u32]) -> Option<usize> {
    let index = cuts.partition_point(|&x| x <= left);
    (index == cuts.partition_point(|&x| x < right)).then_some(index)
}

fn gutters(lines: &[SourceLine]) -> Option<Vec<u32>> {
    let mut heights: Vec<_> = lines.iter().map(|l| l.bounds[3] - l.bounds[1]).collect();
    heights.sort_unstable();
    let middle = heights.len() / 2;
    let height = (f64::from(*heights.get(middle)?) + f64::from(heights[(heights.len() - 1) / 2])) / 2.0;
    let wide: Vec<_> = lines
        .iter()
        .map(|l| l.bounds)
        .filter(|b| f64::from(b[2] - b[0]) >= 6.0 * height)
        .collect();
    if wide.len() < 20 {
        return None;
    }
    let mut events: Vec<_> = wide.iter().flat_map(|b| [(b[0], 1i32), (b[2], -1)]).collect();
    events.sort_unstable_by_key(|e| e.0);
    let mut runs = Vec::<(u32, u32)>::new();
    let mut coverage = 0;
    for pair in events.windows(2) {
        let [(left, change), (right, _)] = [pair[0], pair[1]];
        coverage += change;
        if left == right || coverage as usize * 100 > wide.len() * 2 {
            continue;
        }
        if let Some(run) = runs.last_mut().filter(|run| run.1 == left) {
            run.1 = right;
        } else {
            runs.push((left, right));
        }
    }
    let cuts: Vec<_> = runs
        .into_iter()
        .filter(|&(a, b)| {
            f64::from(b - a) >= height / 2.0
                && wide.iter().filter(|r| r[2] <= a).count() >= 10
                && wide.iter().filter(|r| r[0] >= b).count() >= 10
        })
        .map(|(a, b)| a + (b - a) / 2)
        .collect();
    (!cuts.is_empty()).then_some(cuts)
}

#[cfg(test)]
mod tests {
    use super::{ColumnLanes, ElementKind, InternalDocument, InternalElement, SourceLine};
    use crate::types::{BoundingBox, ContentLayer};

    fn interleaved() -> (ColumnLanes, InternalDocument) {
        let lines = [
            ("Left one", [0, 0, 20, 8]),
            ("Right one", [50, 0, 70, 8]),
            ("Left two", [0, 12, 20, 20]),
            ("Right two", [50, 12, 70, 20]),
        ];
        let witness = ColumnLanes {
            frame: (80, 40),
            cuts: vec![35],
            lines: lines
                .iter()
                .map(|(text, bounds)| SourceLine {
                    text: (*text).into(),
                    bounds: *bounds,
                    vetoed: false,
                })
                .collect(),
        };
        let doc = InternalDocument {
            elements: lines
                .into_iter()
                .map(|(text, [x0, y0, x1, y1])| {
                    let mut element = InternalElement::text(ElementKind::Paragraph, text, 0);
                    element.bbox = Some(BoundingBox {
                        x0: f64::from(x0),
                        y0: f64::from(40 - y1),
                        x1: f64::from(x1),
                        y1: f64::from(40 - y0),
                    });
                    element
                })
                .collect(),
            ..Default::default()
        };
        (witness, doc)
    }

    #[test]
    fn keeps_same_column_lines_together_without_replacing_any_element() {
        let (witness, doc) = interleaved();
        assert_eq!(witness.plan(&doc, (1., 1.)), Some(vec![0, 2, 1, 3]));
    }

    #[test]
    fn geometry_mismatch_and_furniture_remain_fixed_barriers() {
        let (witness, mut doc) = interleaved();
        doc.elements[1].layer = ContentLayer::Header;
        assert_eq!(witness.plan(&doc, (1., 1.)), None);
        doc.elements[1].layer = ContentLayer::Body;
        doc.elements[1].bbox.as_mut().unwrap().x0 = 25.;
        assert_eq!(witness.plan(&doc, (1., 1.)), None);
    }

    #[test]
    fn competing_destination_text_removes_the_line_from_the_plan() {
        let (witness, mut doc) = interleaved();
        let mut duplicate = doc.elements[1].clone();
        duplicate.layer = ContentLayer::Header;
        doc.elements.push(duplicate);
        assert_eq!(witness.plan(&doc, (1., 1.)), None);
    }
}
