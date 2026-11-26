use super::{
    column_selection::{self, Word},
    document, pipeline,
};
use crate::cancellation::CancellationToken;
use crate::core::config::OcrConfig;
use crate::types::{BoundingBox, ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};
use std::{future::Future, ops::Range, sync::Arc, time::Duration};

#[cfg(test)]
#[path = "column_restore.rs"]
mod restoration;

#[cfg(feature = "layout-detection")]
#[path = "column_lanes.rs"]
mod lanes;
#[path = "column_order.rs"]
mod ordering;
#[path = "column_regroup.rs"]
mod regroup;
#[cfg(feature = "layout-detection")]
pub(super) use lanes::ColumnLanes;
pub(super) use ordering::LineOrder;

const MIN_WORDS: usize = 160;
const MAX_WORDS: usize = 6000;
const MIN_GUTTER_JUMPS: usize = 12;
const RETRY_STOP_POLL: Duration = Duration::from_millis(25);

pub(super) async fn retry(
    baseline: ExtractedDocument,
    backend: &Arc<dyn crate::plugins::OcrBackend>,
    raster: Arc<Vec<u8>>,
    config: &OcrConfig,
    cancel: Option<&CancellationToken>,
) -> crate::Result<(ExtractedDocument, Option<LineOrder>)> {
    retry_order_read(baseline, backend.name(), cancel, || async {
        let mut config = config.clone();
        config.tesseract_config.get_or_insert_default().psm = Some(3);
        backend.process_image_owned(raster, &config).await
    })
    .await
}

async fn retry_order_read(
    mut baseline: ExtractedDocument,
    backend: &str,
    cancel: Option<&CancellationToken>,
    read: impl AsyncFnOnce() -> crate::Result<ExtractedDocument>,
) -> crate::Result<(ExtractedDocument, Option<LineOrder>)> {
    if explicitly_cancelled(cancel) {
        return Err(crate::XbergError::Cancelled);
    }
    let frame = eligible_frame(&baseline, backend);
    if explicitly_cancelled(cancel) {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(frame) = frame else {
        return Ok((baseline, None));
    };
    let read = retry_until_stop(cancel, read()).await;
    if explicitly_cancelled(cancel) || matches!(read, Some(Err(crate::XbergError::Cancelled))) {
        return Err(crate::XbergError::Cancelled);
    }
    match read {
        Some(Ok(candidate)) if !stop_retry(cancel) => {
            if regroup::regroup(&mut baseline, &candidate, backend, frame) {
                return Ok((baseline, None));
            }
            if let Some(order) = ordering::plan(&baseline, &candidate, backend, frame, cancel) {
                return Ok((baseline, Some(order)));
            }
        }
        Some(Err(_)) => crate::core::extractor::cacheability::invalidate_current(),
        _ => {}
    }
    note_deadline(&mut baseline.processing_warnings, cancel);
    Ok((baseline, None))
}

#[cfg(test)]
async fn retry_read(
    mut baseline: ExtractedDocument,
    backend: &str,
    cancel: Option<&CancellationToken>,
    read: impl AsyncFnOnce() -> crate::Result<ExtractedDocument>,
) -> crate::Result<ExtractedDocument> {
    if explicitly_cancelled(cancel) {
        return Err(crate::XbergError::Cancelled);
    }
    let frame = eligible_frame(&baseline, backend);
    if explicitly_cancelled(cancel) {
        return Err(crate::XbergError::Cancelled);
    }
    let Some(frame) = frame else {
        return Ok(baseline);
    };
    let selection = retry_until_stop(cancel, async {
        read()
            .await
            .map(|candidate| adopt(&baseline, candidate, backend, frame, || stop_retry(cancel)))
    })
    .await;
    if explicitly_cancelled(cancel) || matches!(selection, Some(Err(crate::XbergError::Cancelled))) {
        return Err(crate::XbergError::Cancelled);
    }
    if let Some(Ok(Some(candidate))) = selection {
        return Ok(candidate);
    }
    note_deadline(&mut baseline.processing_warnings, cancel);
    Ok(baseline)
}

pub(super) fn note_deadline(warnings: &mut Vec<crate::types::ProcessingWarning>, cancel: Option<&CancellationToken>) {
    if let Some(token) = cancel.filter(|token| token.time_limit_reached()) {
        token.note_stage_cut();
        crate::core::extractor::cacheability::invalidate_current();
        crate::core::diagnostics::push_warning(
            warnings,
            "time_limit",
            "The PDF column OCR retry was cut short by the extraction time limit. The completed primary read was kept.",
        );
    }
}

fn explicitly_cancelled(cancel: Option<&CancellationToken>) -> bool {
    cancel.is_some_and(CancellationToken::is_cancelled)
}

fn eligible_frame(baseline: &ExtractedDocument, backend: &str) -> Option<(u32, u32)> {
    if pipeline::effective_tesseract_psm(&baseline.metadata, backend) != Some(11)
        || !supported(baseline)
        || document::resolved_ocr_correction_degrees(&baseline.metadata).is_some_and(|angle| angle != 0)
    {
        return None;
    }
    let frame = document::processed_ocr_layout_dimensions(&baseline.metadata)?;
    let old = inventory(baseline)?;
    (interleaving(&old.words, frame) >= MIN_GUTTER_JUMPS).then_some(frame)
}

#[cfg(test)]
fn adopt(
    baseline: &ExtractedDocument,
    mut candidate: ExtractedDocument,
    backend: &str,
    frame: (u32, u32),
    mut stopped: impl FnMut() -> bool,
) -> Option<ExtractedDocument> {
    if stopped()
        || !supported(&candidate)
        || pipeline::effective_tesseract_psm(&candidate.metadata, backend) != Some(3)
        || document::processed_ocr_layout_dimensions(&candidate.metadata) != Some(frame)
        || document::resolved_ocr_correction_degrees(&candidate.metadata).is_some_and(|angle| angle != 0)
        || serde_json::to_value(&baseline.metadata.image_preprocessing).ok()?
            != serde_json::to_value(&candidate.metadata.image_preprocessing).ok()?
    {
        return None;
    }
    let old = inventory(baseline)?;
    let new = inventory(&candidate)?;
    let jumps = interleaving(&old.words, frame);
    if new.words.len() * 100 < old.words.len() * 92
        || new.words.len() * 100 > old.words.len() * 112
        || jumps < MIN_GUTTER_JUMPS
        || interleaving(&new.words, frame) * 100 > jumps * 45
        || stopped()
    {
        return None;
    }
    let unmatched = unmatched_occurrences(&old.words, &new.words)?;
    let restored = !unmatched.is_empty();
    let mut edits = plan_structured(&candidate, &old, &new)?;
    if restored {
        if edits.move_line.is_some() {
            return None;
        }
        if let Some(spans) = restoration::plan(baseline, &candidate, &old, &new, &unmatched) {
            edits.restorations = spans;
        } else {
            edits.whole_lines = restoration::plan_whole(baseline, &candidate, &old, &new, &unmatched)?;
        }
    }
    if stopped() {
        return None;
    }
    edits.apply(&mut candidate);
    if restored {
        let final_words = inventory(&candidate)?;
        if interleaving(&final_words.words, frame) * 100 > jumps * 45
            || !unmatched_occurrences(&old.words, &final_words.words)?.is_empty()
        {
            return None;
        }
    }
    if stopped() {
        return None;
    }
    tracing::debug!("Selected automatic segmentation for interleaved PDF OCR columns");
    Some(candidate)
}

fn stop_retry(cancel: Option<&CancellationToken>) -> bool {
    explicitly_cancelled(cancel) || cancel.is_some_and(CancellationToken::time_limit_reached)
}

async fn retry_until_stop<T>(cancel: Option<&CancellationToken>, retry: impl Future<Output = T>) -> Option<T> {
    if stop_retry(cancel) {
        return None;
    }
    let Some(token) = cancel else {
        return Some(retry.await);
    };
    // Native recognition already runs on a blocking worker. Drop its waiter
    // at the stop boundary so the completed primary can reach PDF assembly.
    let stopped = async {
        while !stop_retry(Some(token)) {
            tokio::time::sleep(RETRY_STOP_POLL).await;
        }
    };
    tokio::select! {
        biased;
        () = stopped => None,
        result = retry => (!stop_retry(Some(token))).then_some(result),
    }
}

/// Every source occurrence needs an independently located replacement. A
/// split or join can discharge that obligation only for unchanged nonspace
/// content, so a large adjacent box cannot stand in for a missing word.
#[cfg(test)]
fn source_order_improves(baseline: &[Word<'_>], candidate: &[Word<'_>], frame: (u32, u32)) -> bool {
    if !source_occurrences_preserved(baseline, candidate) {
        return false;
    }
    let jumps = interleaving(baseline, frame);
    jumps >= MIN_GUTTER_JUMPS && interleaving(candidate, frame) * 100 <= jumps * 45
}

#[cfg(test)]
fn source_occurrences_preserved(old: &[Word<'_>], new: &[Word<'_>]) -> bool {
    unmatched_occurrences(old, new).is_some_and(|unmatched| unmatched.is_empty())
}

#[cfg(test)]
fn unmatched_occurrences(old: &[Word<'_>], new: &[Word<'_>]) -> Option<Vec<bool>> {
    const MAX_GROUP_WORDS: usize = 8;
    let mut links = vec![Vec::new(); old.len()];
    let mut reverse = vec![Vec::new(); new.len()];
    for (i, source) in old.iter().enumerate() {
        for (j, candidate) in new.iter().enumerate() {
            let (area, source_area, candidate_area) = overlap(source.bounds, candidate.bounds);
            if area > source_area.min(candidate_area) / 2 {
                if links[i].len() == MAX_GROUP_WORDS || reverse[j].len() == MAX_GROUP_WORDS {
                    return None;
                }
                links[i].push(j);
                reverse[j].push(i);
            }
        }
    }
    let mut unmatched = vec![false; old.len()];
    for (i, counterparts) in links.iter().enumerate() {
        let preserved = match counterparts.as_slice() {
            [] => {
                unmatched[i] = true;
                true
            }
            [j] if reverse[*j].len() == 1 => {
                let (area, a, b) = overlap(old[i].bounds, new[*j].bounds);
                area > a.max(b) / 2
            }
            [j] => {
                let group = &reverse[*j];
                group.iter().all(|&k| links[k].len() == 1) && equivalent_group(old, group, new[*j])
            }
            group => group.iter().all(|&j| reverse[j].len() == 1) && equivalent_group(new, group, old[i]),
        };
        if !preserved {
            return None;
        }
    }
    if unmatched.iter().any(|&missing| missing) {
        return Some(unmatched);
    }
    Some(Vec::new())
}

fn overlap(a: [u32; 4], b: [u32; 4]) -> (u64, u64, u64) {
    let area = |r: [u32; 4]| u64::from(r[2] - r[0]) * u64::from(r[3] - r[1]);
    let width = a[2].min(b[2]).saturating_sub(a[0].max(b[0]));
    let height = a[3].min(b[3]).saturating_sub(a[1].max(b[1]));
    (u64::from(width) * u64::from(height), area(a), area(b))
}

#[cfg(test)]
fn equivalent_group(words: &[Word<'_>], indices: &[usize], whole: Word<'_>) -> bool {
    let mut parts: Vec<_> = indices.iter().map(|&index| words[index]).collect();
    parts.sort_unstable_by_key(|word| word.bounds[0]);
    let literal = |word: Word<'_>| word.text.split_whitespace().collect::<String>();
    let expected = literal(whole);
    !expected.is_empty()
        && parts
            .windows(2)
            .all(|pair| column_selection::same_line(pair[0], pair[1], f64::from(whole.bounds[3] - whole.bounds[1])))
        && parts.into_iter().map(literal).collect::<String>() == expected
}

fn supported(read: &ExtractedDocument) -> bool {
    read.tables.is_empty()
        && read.formulas.is_empty()
        && read.document.is_none()
        && read
            .ocr_internal_document
            .as_ref()
            .is_some_and(|doc| doc.tables.is_empty() && doc.relationships.is_empty())
}

struct Inventory<'a> {
    #[cfg(test)]
    indexes: Vec<usize>,
    words: Vec<Word<'a>>,
}

#[cfg_attr(
    not(test),
    allow(clippy::unused_enumerate_index, reason = "test builds record the index")
)]
fn inventory(read: &ExtractedDocument) -> Option<Inventory<'_>> {
    #[cfg(test)]
    let mut indexes = Vec::new();
    let mut words = Vec::new();
    for (_index, element) in read.ocr_elements.as_deref()?.iter().enumerate() {
        if element.level != OcrElementLevel::Word {
            continue;
        }
        #[cfg(test)]
        indexes.push(_index);
        words.push(Word {
            text: &element.text,
            bounds: bounds(element)?,
            confidence: element.confidence.recognition,
        });
        if words.len() > MAX_WORDS {
            return None;
        }
    }
    (words.len() >= MIN_WORDS).then_some(Inventory {
        #[cfg(test)]
        indexes,
        words,
    })
}

fn bounds(element: &OcrElement) -> Option<[u32; 4]> {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = element.geometry
    else {
        return None;
    };
    (width > 0 && height > 0).then_some([left, top, left.checked_add(width)?, top.checked_add(height)?])
}

fn interleaving(words: &[Word<'_>], (width, height): (u32, u32)) -> usize {
    if width == 0 || height == 0 {
        return 0;
    }
    let line_height = column_selection::median_height(words);
    let width = f64::from(width);
    let spans: Vec<_> = words
        .iter()
        .map(|w| {
            let [left, top, right, bottom] = w.bounds;
            (
                f64::from(left) / width,
                f64::from(right) / width,
                (f64::from(top) + f64::from(bottom)) / 2.0,
            )
        })
        .collect();
    (20..=80)
        .map(|n| {
            let cut = f64::from(n) / 100.0;
            let mut bands = std::collections::BTreeMap::<u32, u8>::new();
            let mut crossing = 0;
            for &(left, right, y) in &spans {
                let band = (y / (3.0 * line_height)) as u32;
                if right <= cut && cut - right <= 0.055 {
                    *bands.entry(band).or_default() |= 1;
                }
                if left >= cut && left - cut <= 0.055 {
                    *bands.entry(band).or_default() |= 2;
                }
                crossing += usize::from(left < cut && cut < right);
            }
            if crossing * 1000 > words.len() * 15 || bands.values().filter(|&&v| v == 3).count() < 9 {
                return 0;
            }
            spans
                .windows(2)
                .filter(|pair| {
                    let [(a, b, y), (c, d, z)] = [pair[0], pair[1]];
                    (y - z).abs() <= 1.15 * line_height
                        && ((b <= cut && cut <= c && c - b >= 0.018) || (d <= cut && cut <= a && a - d >= 0.018))
                })
                .count()
        })
        .max()
        .unwrap_or(0)
}

#[derive(Clone, Debug)]
struct Slot {
    element: usize,
    range: Range<usize>,
}

fn token_ranges(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let offset = text.as_ptr() as usize;
    text.split_whitespace().map(move |token| {
        let start = token.as_ptr() as usize - offset;
        start..start + token.len()
    })
}

fn slots<'a>(blocks: impl Iterator<Item = (usize, &'a str, BoundingBox)>, words: &[Word<'_>]) -> Vec<Option<Slot>> {
    let mut slots = vec![None; words.len()];
    let mut ambiguous = vec![false; words.len()];
    for (element, text, bbox) in blocks {
        let inside: Vec<_> = words
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                let [l, t, r, b] = w.bounds.map(f64::from);
                l >= bbox.x0 && t >= bbox.y0 && r <= bbox.x1 && b <= bbox.y1
            })
            .collect();
        let ranges: Vec<_> = token_ranges(text).collect();
        let mut matches = Vec::new();
        if inside.len() == ranges.len()
            && inside
                .iter()
                .zip(&ranges)
                .all(|((_, w), span)| w.text == &text[span.clone()])
        {
            matches.extend(
                inside
                    .iter()
                    .zip(&ranges)
                    .map(|((index, _), span)| (*index, span.clone())),
            );
        } else {
            let mut text_windows = std::collections::HashMap::<[&str; 3], Option<usize>>::new();
            for (i, tokens) in ranges.windows(3).enumerate() {
                let key = std::array::from_fn(|j| &text[tokens[j].clone()]);
                text_windows
                    .entry(key)
                    .and_modify(|value| *value = None)
                    .or_insert(Some(i + 1));
            }
            let mut source_windows = std::collections::HashMap::<[&str; 3], Option<usize>>::new();
            for source in inside.windows(3) {
                if source[0].0 + 1 == source[1].0 && source[1].0 + 1 == source[2].0 {
                    let key = std::array::from_fn(|j| source[j].1.text);
                    source_windows
                        .entry(key)
                        .and_modify(|value| *value = None)
                        .or_insert(Some(source[1].0));
                }
            }
            for (tokens, word) in source_windows {
                if let (Some(word), Some(Some(offset))) = (word, text_windows.get(&tokens)) {
                    matches.push((word, ranges[*offset].clone()));
                }
            }
        }
        for (index, range) in matches {
            if slots[index].is_some() || ambiguous[index] {
                slots[index] = None;
                ambiguous[index] = true;
            } else {
                slots[index] = Some(Slot { element, range });
            }
        }
    }
    slots
}

fn bbox(bounds: [u32; 4]) -> BoundingBox {
    let [x0, y0, x1, y1] = bounds.map(f64::from);
    BoundingBox { x0, y0, x1, y1 }
}

#[cfg(test)]
fn document_slots(read: &ExtractedDocument, words: &[Word<'_>]) -> Option<Vec<Option<Slot>>> {
    let doc = read.ocr_internal_document.as_ref()?;
    Some(slots(
        doc.elements
            .iter()
            .enumerate()
            .filter_map(|(i, element)| element.bbox.as_ref().map(|bounds| (i, element.text.as_str(), *bounds))),
        words,
    ))
}

fn line_slots(read: &ExtractedDocument, words: &[Word<'_>]) -> Option<Vec<Option<Slot>>> {
    let public = read.ocr_elements.as_ref()?;
    Some(slots(
        public.iter().enumerate().filter_map(|(i, e)| {
            (e.level == OcrElementLevel::Line)
                .then(|| Some((i, e.text.as_str(), bbox(bounds(e)?))))
                .flatten()
        }),
        words,
    ))
}

#[cfg(test)]
#[derive(Default)]
struct Edits {
    blocks: Vec<(Slot, String)>,
    lines: Vec<(Slot, String)>,
    words: Vec<(usize, String, f64)>,
    move_line: Option<LineMove>,
    restorations: Vec<restoration::Span>,
    whole_lines: Vec<restoration::WholeLine>,
}

#[cfg(test)]
impl Edits {
    fn apply(mut self, read: &mut ExtractedDocument) {
        if self.blocks.is_empty()
            && self.move_line.is_none()
            && self.restorations.is_empty()
            && self.whole_lines.is_empty()
        {
            return;
        }
        let doc = read.ocr_internal_document.as_mut().expect("structured OCR selection");
        for span in &self.restorations {
            self.blocks.push(span.block.clone());
            self.lines.push(span.line.clone());
        }
        self.blocks
            .sort_unstable_by_key(|(slot, _)| (slot.element, std::cmp::Reverse(slot.range.start)));
        for (slot, value) in self.blocks {
            doc.elements[slot.element].text.replace_range(slot.range, &value);
        }
        let elements = read.ocr_elements.as_mut().expect("OCR source boxes");
        self.lines
            .sort_unstable_by_key(|(slot, _)| (slot.element, std::cmp::Reverse(slot.range.start)));
        for (slot, value) in self.lines {
            elements[slot.element].text.replace_range(slot.range, &value);
        }
        for (index, value, confidence) in self.words {
            elements[index].text = value;
            elements[index].confidence.recognition = confidence;
        }
        restoration::apply(self.restorations, doc, elements);
        restoration::apply_whole(self.whole_lines, doc, elements);
        if let Some(movement) = self.move_line {
            movement.apply(doc, elements);
        }
        for (index, element) in doc.elements.iter_mut().enumerate() {
            element.id = element.clone().with_index(index as u32).id;
        }
        read.content = doc
            .elements
            .iter()
            .filter(|e| !e.text.is_empty())
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
    }
}

#[cfg(test)]
fn plan_structured(candidate: &ExtractedDocument, old: &Inventory<'_>, new: &Inventory<'_>) -> Option<Edits> {
    let new_slots = document_slots(candidate, &new.words)?;
    let new_doc = candidate.ocr_internal_document.as_ref()?;
    let source_lines = whole_source_lines(&old.words);
    let plan = column_selection::plan(&old.words, &new.words, &source_lines);
    let public = candidate.ocr_elements.as_ref()?;
    if public
        .iter()
        .any(|e| !matches!(e.level, OcrElementLevel::Word | OcrElementLevel::Line))
    {
        return None;
    }
    let line_slots = line_slots(candidate, &new.words)?;
    let mut valid_moves = plan
        .detached_lines
        .into_iter()
        .filter_map(|(head, tail)| LineMove::new(head, tail, candidate, new, &new_slots, &line_slots));
    let move_line = valid_moves.next().filter(|_| valid_moves.next().is_none());
    let mut edits = Edits {
        move_line,
        ..Default::default()
    };
    for (i, j) in plan.substitutions {
        let (Some(block), Some(line)) = (&new_slots[j], &line_slots[j]) else {
            continue;
        };
        if !new_doc.elements[block.element].annotations.is_empty()
            || edits
                .move_line
                .as_ref()
                .is_some_and(|m| m.blocks.contains(block.element) || m.lines.contains(line.element))
        {
            continue;
        }
        let text = old.words[i].text;
        edits.blocks.push((block.clone(), text.into()));
        edits.lines.push((line.clone(), text.into()));
        edits.words.push((new.indexes[j], text.into(), old.words[i].confidence));
    }
    Some(edits)
}

#[cfg(test)]
fn whole_source_lines(words: &[Word<'_>]) -> Vec<Range<usize>> {
    let height = column_selection::median_height(words);
    let mut start = 0;
    let mut lines = Vec::new();
    for end in 1..=words.len() {
        if end == words.len() || !column_selection::same_line(words[end - 1], words[end], height) {
            if end - start >= 4 {
                lines.push(start..end);
            }
            start = end;
        }
    }
    lines
}

#[cfg(test)]
struct LineMove {
    blocks: Transfer,
    lines: Transfer,
    insert_at: usize,
    word_indices: Vec<usize>,
}

#[cfg(test)]
impl LineMove {
    fn new(
        head: Range<usize>,
        tail: Range<usize>,
        candidate: &ExtractedDocument,
        inventory: &Inventory<'_>,
        blocks: &[Option<Slot>],
        lines: &[Option<Slot>],
    ) -> Option<Self> {
        let doc = candidate.ocr_internal_document.as_ref()?;
        let public = candidate.ocr_elements.as_ref()?;
        let block_head = blocks[head.end - 1].as_ref()?.element;
        let block_tail = blocks[tail.start].as_ref()?.element;
        if [block_head, block_tail]
            .into_iter()
            .any(|i| !doc.elements[i].annotations.is_empty() || doc.elements[i].anchor.is_some())
        {
            return None;
        }
        let (first_head, last_head) = paired_span(&head, lines)?;
        let (first_tail, last_tail) = paired_span(&tail, lines)?;
        if [(&head, first_head, last_head), (&tail, first_tail, last_tail)]
            .into_iter()
            .any(|(range, first, last)| {
                let line = &public[first.element].text;
                first.range.start != 0 || last.range.end != line.len() || line.split_whitespace().count() != range.len()
            })
        {
            return None;
        }
        let block_transfer = Transfer::new(&head, &tail, blocks, &inventory.words, |i| &doc.elements[i].text)?;
        let line_transfer = Transfer::new(&head, &tail, lines, &inventory.words, |i| &public[i].text)?;
        Some(Self {
            blocks: block_transfer,
            lines: line_transfer,
            insert_at: inventory.indexes[head.end - 1] + 1,
            word_indices: inventory.indexes[tail].to_vec(),
        })
    }

    fn apply(self, doc: &mut crate::types::internal::InternalDocument, elements: &mut Vec<OcrElement>) {
        for (index, text, bounds) in self.blocks.payloads() {
            let element = &mut doc.elements[index];
            element.text = text;
            element.bbox = bounds.map(bbox);
            element.ocr_geometry = bounds.map(rectangle);
            element.ocr_confidence = None;
            if let Some(attrs) = element.attributes.as_mut() {
                attrs.remove(crate::ocr::hocr_parser::HOCR_LINE_FONT_SIZES_ATTRIBUTE);
                attrs.remove(crate::ocr::hocr_parser::HOCR_LINE_X_HEIGHTS_ATTRIBUTE);
            }
        }
        if doc.elements[self.blocks.tail].text.is_empty() {
            doc.elements.remove(self.blocks.tail);
        }
        for (index, text, bounds) in self.lines.payloads() {
            elements[index].text = text;
            if let Some(bounds) = bounds {
                elements[index].geometry = rectangle(bounds);
            }
        }
        for (offset, index) in self.word_indices.into_iter().enumerate() {
            let word = elements.remove(index);
            elements.insert(self.insert_at + offset, word);
        }
        elements.retain(|element| element.level != OcrElementLevel::Line || !element.text.is_empty());
    }
}

#[cfg(test)]
struct Transfer {
    head: usize,
    tail: usize,
    head_text: String,
    tail_text: String,
    head_box: [u32; 4],
    tail_box: Option<[u32; 4]>,
}

#[cfg(test)]
impl Transfer {
    fn new<'a>(
        head: &Range<usize>,
        tail: &Range<usize>,
        slots: &[Option<Slot>],
        words: &[Word<'_>],
        text: impl Fn(usize) -> &'a str,
    ) -> Option<Self> {
        let (head_start, head_end) = paired_span(head, slots)?;
        let (tail_start, tail_end) = paired_span(tail, slots)?;
        let head_index = head_end.element;
        let tail_index = tail_start.element;
        if head_index == tail_index {
            return None;
        }
        let head_text = text(head_index);
        let tail_text = text(tail_index);
        let head_range = head_start.range.start..head_end.range.end;
        let tail_range = tail_start.range.start..tail_end.range.end;
        if head_text[head_range.clone()].split_whitespace().count() != head.len()
            || tail_text[tail_range.clone()].split_whitespace().count() != tail.len()
            || head_text[head_range].contains('\n')
            || tail_text[tail_range.clone()].contains('\n')
            || !matches!(tail_text.as_bytes().get(tail_range.end), None | Some(b'\n'))
            || !matches!(head_text.as_bytes().get(head_end.range.end), None | Some(b'\n'))
        {
            return None;
        }
        let collect = |index| {
            slots
                .iter()
                .enumerate()
                .filter_map(move |(i, slot)| slot.as_ref().filter(|s| s.element == index).map(|_| i))
        };
        if collect(head_index).count() != head_text.split_whitespace().count()
            || collect(tail_index).count() != tail_text.split_whitespace().count()
        {
            return None;
        }
        let mut head_text = head_text.to_owned();
        head_text.insert_str(head_end.range.end, &format!(" {}", &tail_text[tail_range.clone()]));
        let mut tail_text = tail_text.to_owned();
        let start = tail_range
            .start
            .checked_sub(1)
            .filter(|&i| tail_text.as_bytes()[i] == b' ')
            .unwrap_or(tail_range.start);
        tail_text.replace_range(start..tail_range.end, "");
        let head_box = union(collect(head_index).chain(tail.clone()).map(|i| words[i].bounds))?;
        let tail_box = union(
            collect(tail_index)
                .filter(|i| !tail.contains(i))
                .map(|i| words[i].bounds),
        );
        Some(Self {
            head: head_index,
            tail: tail_index,
            head_text,
            tail_text: tail_text.trim().to_owned(),
            head_box,
            tail_box,
        })
    }

    fn contains(&self, index: usize) -> bool {
        index == self.head || index == self.tail
    }

    fn payloads(&self) -> [(usize, String, Option<[u32; 4]>); 2] {
        [
            (self.head, self.head_text.clone(), Some(self.head_box)),
            (self.tail, self.tail_text.clone(), self.tail_box),
        ]
    }
}

#[cfg(test)]
fn paired_span<'a>(range: &Range<usize>, slots: &'a [Option<Slot>]) -> Option<(&'a Slot, &'a Slot)> {
    let first = slots.get(range.start)?.as_ref()?;
    let last = slots.get(range.end.checked_sub(1)?)?.as_ref()?;
    let mut previous = first.range.start;
    for slot in slots.get(range.clone())? {
        let slot = slot.as_ref()?;
        if slot.element != first.element || slot.range.start < previous {
            return None;
        }
        previous = slot.range.end;
    }
    Some((first, last))
}

fn rectangle([x0, y0, x1, y1]: [u32; 4]) -> OcrBoundingGeometry {
    OcrBoundingGeometry::Rectangle {
        left: x0,
        top: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

#[cfg(test)]
fn union(boxes: impl Iterator<Item = [u32; 4]>) -> Option<[u32; 4]> {
    boxes.reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
}

#[cfg(test)]
#[path = "column_retry_tests.rs"]
mod tests;
