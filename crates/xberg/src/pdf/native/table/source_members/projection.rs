use super::{BoundingBox, Edges, Run, SegmentData, SegmentKey, Table, TableSources, table_bounds};
use ahash::{AHashMap, AHashSet};

#[derive(Default)]
pub(crate) struct CertifiedRuns {
    pub(crate) contained: Vec<Vec<SegmentKey>>,
    pub(crate) painted_overhangs: Vec<Vec<SegmentKey>>,
}

const MAX_PAGE_TABLES: usize = super::MAX_CAPTURE_TABLES;
pub(crate) const MAX_PAGE_SEGMENTS: usize = 16_384;
const MAX_RUN_SEGMENTS: usize = 64;
pub(crate) const MAX_PROOF_WORK: usize = 131_072;

#[derive(Default)]
struct Work(usize);

impl Work {
    fn charge(&mut self, amount: usize) -> Option<()> {
        self.0 = self.0.checked_add(amount)?;
        (self.0 <= MAX_PROOF_WORK).then_some(())
    }
}

struct SourceIndex<'a> {
    segments: Vec<&'a SegmentData>,
    max_height: f32,
}

fn segment_bounds(segment: &SegmentData) -> Option<Edges> {
    Edges::new(segment.x, segment.y, segment.width, segment.height)
}

impl<'a> SourceIndex<'a> {
    fn new(segments: &'a [SegmentData], work: &mut Work) -> Option<Self> {
        if segments.len() > MAX_PAGE_SEGMENTS {
            return None;
        }
        let mut selected = Vec::new();
        let mut max_height = 0.0f32;
        for segment in segments {
            work.charge(segment.text.len().checked_add(1)?)?;
            if segment.text.is_empty() {
                continue;
            }
            segment_bounds(segment)?;
            if ![segment.font_size, segment.baseline_y, segment.rotation_degrees]
                .iter()
                .all(|value| value.is_finite())
            {
                return None;
            }
            max_height = max_height.max(segment.height);
            selected.push(segment);
        }
        selected.sort_by(|left, right| left.y.total_cmp(&right.y).then(left.x.total_cmp(&right.x)));
        Some(Self {
            segments: selected,
            max_height,
        })
    }

    fn run(&self, run: &Run, work: &mut Work) -> Option<Option<Vec<SegmentKey>>> {
        work.charge(run.text.len().checked_add(1)?)?;
        let start = self
            .segments
            .partition_point(|segment| segment.y < run.bounds.bottom - self.max_height);
        let end = self.segments.partition_point(|segment| segment.y <= run.bounds.top);
        let mut selected = Vec::new();
        for segment in &self.segments[start..end] {
            work.charge(1)?;
            let bounds = segment_bounds(segment)?;
            if !run.bounds.intersects(bounds) {
                continue;
            }
            if !run.bounds.contains(bounds) || !run.matches(segment) || selected.len() >= MAX_RUN_SEGMENTS {
                return Some(None);
            }
            work.charge(segment.text.len())?;
            selected.push(*segment);
        }
        Some(exact_run(run, selected))
    }
}

fn exact_run(run: &Run, mut selected: Vec<&SegmentData>) -> Option<Vec<SegmentKey>> {
    selected.sort_by(|left, right| left.x.total_cmp(&right.x));
    let mut bounds = segment_bounds(selected.first()?)?;
    let mut text = String::new();
    let mut keys = Vec::new();
    let mut unique = AHashSet::new();
    for segment in selected {
        bounds = bounds.union(segment_bounds(segment)?);
        text.push_str(&segment.text);
        let key = SegmentKey::from(segment);
        if !unique.insert(key.clone()) {
            return None;
        }
        keys.push(key);
    }
    // ~keep Synthetic heights are rounded; require the exact computed edge union instead of equal stored heights.
    (bounds.exact(run.bounds) && text == run.text).then_some(keys)
}

fn cell(
    index: &SourceIndex<'_>,
    runs: &[Run],
    table: BoundingBox,
    work: &mut Work,
) -> Option<Option<Vec<Vec<SegmentKey>>>> {
    work.charge(runs.len())?;
    if !runs.iter().all(|run| within_table(run, table)) {
        return Some(None);
    }
    let mut groups = Vec::new();
    let mut unique = AHashSet::new();
    for run in runs {
        let Some(members) = index.run(run, work)? else {
            return Some(None);
        };
        if !members.iter().all(|key| unique.insert(key.clone())) {
            return Some(None);
        }
        groups.push(members);
    }
    Some(Some(groups))
}

fn within_table(run: &Run, table: BoundingBox) -> bool {
    let bounds = run.bounds;
    table.x0 <= f64::from(bounds.left)
        && table.y0 <= f64::from(bounds.bottom)
        && table.x1 >= f64::from(bounds.right)
        && table.y1 >= f64::from(bounds.top)
}

fn matches(source: &TableSources, table: &Table, work: &mut Work) -> Option<bool> {
    if source.page != table.page_number || table_bounds(table.bounding_box) != Some(source.bbox) {
        return Some(false);
    }
    work.charge(source.identity_work)?;
    Some(source.cells == table.cells && source.markdown == table.markdown)
}

fn page(
    segments: &[SegmentData],
    sources: &[&TableSources],
    emitted: &[(usize, &Table)],
) -> Option<AHashMap<usize, CertifiedRuns>> {
    if sources.len() > MAX_PAGE_TABLES || emitted.len() > MAX_PAGE_TABLES {
        return None;
    }
    let mut work = Work::default();
    let index = SourceIndex::new(segments, &mut work)?;
    let mut owned = AHashMap::<usize, CertifiedRuns>::new();
    let mut all_keys = AHashSet::new();
    for source in sources {
        let mut owner = None;
        for &(table_index, table) in emitted {
            if matches(source, table, &mut work)? && owner.replace((table_index, table.bounding_box?)).is_some() {
                return None;
            }
        }
        let Some((owner, bounds)) = owner else {
            continue;
        };
        for runs in &source.members {
            if let Some(groups) = cell(&index, runs, bounds, &mut work)? {
                if !groups.iter().flatten().all(|key| all_keys.insert(key.clone())) {
                    return None;
                }
                owned.entry(owner).or_default().contained.extend(groups);
            }
        }
        for run in &source.paint_overhangs {
            if within_table(run, bounds)
                && let Some(keys) = index.run(run, &mut work)?
            {
                if !keys.iter().all(|key| all_keys.insert(key.clone())) {
                    return None;
                }
                owned.entry(owner).or_default().painted_overhangs.push(keys);
            }
        }
    }
    Some(owned)
}

pub(crate) fn project(
    pages: &[Vec<SegmentData>],
    sources: &[&TableSources],
    tables: &[Table],
) -> AHashMap<usize, CertifiedRuns> {
    let mut source_pages = AHashMap::<usize, Vec<&TableSources>>::new();
    for source in sources {
        if let Some(index) = source
            .page
            .checked_sub(1)
            .filter(|index| (*index as usize) < pages.len())
        {
            let entries = source_pages.entry(index as usize).or_default();
            if entries.len() <= MAX_PAGE_TABLES {
                entries.push(source);
            }
        }
    }
    if source_pages.is_empty() {
        return AHashMap::new();
    }
    let mut table_pages = AHashMap::<usize, Vec<(usize, &Table)>>::new();
    for (index, table) in tables.iter().enumerate() {
        if let Some(page) = table
            .page_number
            .checked_sub(1)
            .filter(|page| source_pages.contains_key(&(*page as usize)))
        {
            let entries = table_pages.entry(page as usize).or_default();
            if entries.len() <= MAX_PAGE_TABLES {
                entries.push((index, table));
            }
        }
    }
    let mut result = AHashMap::new();
    for (index, sources) in source_pages {
        if let Some(segments) = pages.get(index)
            && let Some(tables) = table_pages.get(&index)
            && let Some(owned) = page(segments, &sources, tables)
        {
            result.extend(owned);
        }
    }
    result
}
