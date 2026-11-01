use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use super::{SegmentData, SegmentScriptOwner};
use crate::pdf::native::hierarchy::script_ownership::page_segments::{
    NativeProjectionWitness, NativeSegment, SegmentProjection,
};
use crate::pdf::structure::types::{PdfLine, PdfParagraph};

type Scalars = ([u32; 7], [bool; 3], Option<u8>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct LookupKey {
    scalars: Scalars,
    text_hint: u64,
}

struct Edit {
    before: std::ops::Range<usize>,
    replacement: Vec<u8>,
}

impl Edit {
    fn between(before: &str, after: &str) -> Option<Self> {
        if before == after {
            return None;
        }
        let prefix = before.bytes().zip(after.bytes()).take_while(|(a, b)| a == b).count();
        let suffix = before.as_bytes()[prefix..]
            .iter()
            .rev()
            .zip(after.as_bytes()[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        Some(Self {
            before: prefix..before.len() - suffix,
            replacement: after.as_bytes()[prefix..after.len() - suffix].to_vec(),
        })
    }
}

#[derive(Default)]
struct Entry {
    valid: bool,
    retained: bool,
    observed: bool,
    edit: Option<Edit>,
}

impl Entry {
    fn bytes<'a>(&'a self, original: &'a str) -> impl Iterator<Item = u8> + 'a {
        let (prefix, replacement, suffix) = match &self.edit {
            Some(edit) => (
                &original.as_bytes()[..edit.before.start],
                edit.replacement.as_slice(),
                &original.as_bytes()[edit.before.end..],
            ),
            None => (original.as_bytes(), &[][..], &[][..]),
        };
        prefix.iter().chain(replacement).chain(suffix).copied()
    }

    fn key(&self, source: &SegmentData) -> LookupKey {
        LookupKey {
            scalars: SegmentProjection::from(source).scalars(),
            text_hint: text_hint(self.bytes(&source.text)),
        }
    }

    fn matches(&self, source: &SegmentData, current: &SegmentData) -> bool {
        SegmentProjection::from(source).scalars() == SegmentProjection::from(current).scalars()
            && self.bytes(&source.text).eq(current.text.bytes())
    }
}

fn text_hint(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut hasher = DefaultHasher::new();
    for byte in bytes {
        byte.hash(&mut hasher);
    }
    hasher.finish()
}

fn key(segment: &SegmentData) -> LookupKey {
    LookupKey {
        scalars: SegmentProjection::from(segment).scalars(),
        text_hint: text_hint(segment.text.bytes()),
    }
}

fn insert_unique<K: Eq + Hash>(map: &mut HashMap<K, Option<usize>>, key: K, slot: usize) {
    map.entry(key).and_modify(|value| *value = None).or_insert(Some(slot));
}

pub(super) struct ProjectionStore {
    source: NativeProjectionWitness,
    originals: HashMap<LookupKey, Option<usize>>,
    ordinals: HashMap<usize, Option<usize>>,
    entries: Vec<Entry>,
    repair_candidates: HashMap<Scalars, Vec<usize>>,
}

impl ProjectionStore {
    pub(super) fn new(
        segments: &[SegmentData],
        ordinals: &[usize],
        scripts: &HashMap<usize, SegmentScriptOwner>,
    ) -> Option<Self> {
        let source = scripts.values().next()?.page_projection()?.clone();
        if segments.len() != ordinals.len()
            || scripts
                .values()
                .any(|owner| owner.page_projection().is_none_or(|page| !page.shares_page(&source)))
        {
            return None;
        }
        let mut store = Self::indexed(source);
        for (segment, id) in segments.iter().zip(ordinals) {
            let Some(Some(slot)) = store.ordinals.get(id) else {
                continue;
            };
            let entry = &mut store.entries[*slot];
            if entry.retained
                || SegmentProjection::from(segment) != SegmentProjection::from(&store.source.records()[*slot].segment)
            {
                entry.valid = false;
            }
            entry.retained = true;
        }
        Some(store)
    }

    fn indexed(source: NativeProjectionWitness) -> Self {
        let mut originals = HashMap::new();
        let mut ids = HashMap::new();
        for (slot, record) in source.records().iter().enumerate() {
            insert_unique(&mut originals, key(&record.segment), slot);
            insert_unique(&mut ids, record.original_index, slot);
        }
        let entries: Vec<_> = source
            .records()
            .iter()
            .enumerate()
            .map(|(slot, record)| Entry {
                valid: originals.get(&key(&record.segment)) == Some(&Some(slot))
                    && ids.get(&record.original_index) == Some(&Some(slot)),
                ..Default::default()
            })
            .collect();
        let mut repair_candidates: HashMap<Scalars, Vec<usize>> = HashMap::new();
        for (slot, record) in source.records().iter().enumerate() {
            repair_candidates
                .entry(SegmentProjection::from(&record.segment).scalars())
                .or_default()
                .push(slot);
        }
        Self {
            source,
            originals,
            ordinals: ids,
            entries,
            repair_candidates,
        }
    }

    pub(super) fn observe_repair(&mut self, before: &SegmentData, after: &str) {
        let Some(Some(slot)) = self.originals.get(&key(before)) else {
            if let Some(candidates) = self
                .repair_candidates
                .remove(&SegmentProjection::from(before).scalars())
            {
                for slot in candidates {
                    self.entries[slot].valid = false;
                }
            }
            return;
        };
        let source = &self.source.records()[*slot].segment;
        let entry = &mut self.entries[*slot];
        if !entry.valid
            || !entry.retained
            || entry.observed
            || SegmentProjection::from(source) != SegmentProjection::from(before)
        {
            entry.valid = false;
            return;
        }
        entry.observed = true;
        entry.edit = Edit::between(&before.text, after);
    }

    pub(super) fn view<'a>(&'a self, paragraphs: &[PdfParagraph]) -> ProjectionView<'a> {
        let mut lookup = self.originals.clone();
        for (slot, entry) in self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.valid && entry.retained)
        {
            lookup
                .entry(entry.key(&self.source.records()[slot].segment))
                .and_modify(|value| {
                    if *value != Some(slot) {
                        *value = None;
                    }
                })
                .or_insert(Some(slot));
        }
        let mut view = ProjectionView {
            store: self,
            lookup,
            counts: vec![0; self.entries.len()],
        };
        for segment in paragraphs.iter().flat_map(|p| &p.lines).flat_map(|l| &l.segments) {
            if let Some(slot) = view.lookup(segment) {
                view.counts[slot] += 1;
            }
        }
        view
    }
}

pub(super) struct ProjectionView<'a> {
    store: &'a ProjectionStore,
    lookup: HashMap<LookupKey, Option<usize>>,
    counts: Vec<usize>,
}

impl ProjectionView<'_> {
    fn lookup(&self, segment: &SegmentData) -> Option<usize> {
        let slot = (*self.lookup.get(&key(segment))?)?;
        let entry = &self.store.entries[slot];
        (entry.valid && entry.retained && entry.matches(&self.store.source.records()[slot].segment, segment))
            .then_some(slot)
    }

    pub(super) fn resolve(&self, segment: &SegmentData) -> Option<usize> {
        let slot = self.lookup(segment)?;
        (self.counts[slot] == 1).then_some(slot)
    }

    pub(super) fn text_unchanged(&self, slot: usize) -> bool {
        self.store.entries[slot].edit.is_none()
    }

    pub(super) fn script_slot(&self, owner: &SegmentScriptOwner, line: &PdfLine) -> Option<usize> {
        if !owner.page_projection()?.shares_page(&self.store.source) {
            return None;
        }
        for (index, script) in [(owner.base_index, false), (owner.script_index, true)] {
            let slot = self.original_slot(owner.original_index(script))?;
            if self.resolve(line.segments.get(index)?) != Some(slot) {
                return None;
            }
        }
        self.original_slot(owner.original_index(true))
    }

    pub(super) fn records(&self) -> &[NativeSegment] {
        self.store.source.records()
    }

    pub(super) fn original_slot(&self, ordinal: usize) -> Option<usize> {
        (*self.store.ordinals.get(&ordinal)?).filter(|&slot| {
            self.counts[slot] == 1 && self.store.entries[slot].valid && self.store.entries[slot].retained
        })
    }
}

#[cfg(test)]
#[path = "projection_proof_tests.rs"]
mod tests;
