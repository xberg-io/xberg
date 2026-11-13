#[cfg(test)]
use super::flat_ocr_page_document;
use super::{
    LayerFirst, LineAt, LineBox, MergedOcrAssets, OcrRowIndex, processed_ocr_layout_dimensions,
    reindex_structured_ocr_page, resolved_ocr_correction_degrees,
};

/// Exact literals whose complete set of native occurrences has a one-to-one OCR match.
pub(in crate::extractors::pdf::ocr) struct MatchedLayerOccurrences(
    ahash::AHashMap<String, Vec<OutputOccurrence>>,
    Vec<SplitLiteral>,
    Vec<Vec<IdentifierRow>>,
    Vec<RepeatedLine>,
);

struct SplitLiteral {
    native: String,
    split: String,
    row: String,
    bounds: LineBox,
}

struct IdentifierRow {
    native: String,
    split: String,
    row: String,
    anchor: String,
    bounds: LineBox,
}

struct RepeatedLine {
    text: String,
    boxes: Vec<LineBox>,
    source_lines: Vec<String>,
    anchors: Vec<Option<LineBox>>,
    word_boxes: Vec<Vec<LineBox>>,
}

impl MatchedLayerOccurrences {
    pub(in crate::extractors::pdf::ocr) fn capture(
        pdf: &xberg_native_pdf::PdfDocument,
        page_index: usize,
        layer: &str,
        read: &crate::types::ExtractedDocument,
    ) -> Option<Self> {
        use super::super::native_word_coverage::{native_box, upright_media_box};

        if read.raw_ocr_source_frame.is_some() || resolved_ocr_correction_degrees(&read.metadata).unwrap_or(0) != 0 {
            return None;
        }
        let media = upright_media_box(pdf, page_index)?;
        let raster = processed_ocr_layout_dimensions(&read.metadata)?;
        let (words, ocr) = iterator_words(read, raster, media)?;
        let lines = pdf.extract_text_lines(page_index).ok()?;
        let mut native: ahash::AHashMap<&str, Vec<LineBox>> = ahash::AHashMap::new();
        for word in lines.iter().flat_map(|line| &line.words) {
            native.entry(&word.text).or_default().push(native_box(word.bbox));
        }
        let rows = OcrRowIndex::new(&words);
        let tables = table_rows(&read.tables, |bbox| {
            crate::pdf::render::pixel_bbox_to_pdf_points(bbox, raster.0, raster.1, media.width, media.height)
        });
        let raw = OutputLiterals::new(&read.content, &tables);
        let matched = literal_counts(layer)
            .into_iter()
            .filter_map(|(token, count)| {
                let (source, read) = (native.get(token)?, ocr.get(token)?);
                if source.len() != count || read.len() != count {
                    return None;
                }
                let read_boxes: Vec<_> = read.iter().map(|&index| words[index].bbox).collect();
                if !uniquely_paired(source, &read_boxes) {
                    return None;
                }
                let occurrences = read
                    .iter()
                    .map(|&index| spatial_row(index, &words, &rows))
                    .collect::<Option<Vec<_>>>()?;
                Some((token.to_owned(), raw.bind(token, &occurrences, &words, &rows)?))
            })
            .collect::<ahash::AHashMap<_, _>>();
        let split = capture_split_literals(layer, &native, &ocr, &words, &rows);
        let repeated = capture_repeated_identifiers(layer, &native, &words, &read.tables);
        let repeated_lines = capture_repeated_lines(layer, &native, &ocr, &words, &rows, &read.content);
        (!matched.is_empty() || !split.is_empty() || !repeated.is_empty() || !repeated_lines.is_empty())
            .then_some(Self(matched, split, repeated, repeated_lines))
    }

    pub(in crate::extractors::pdf::ocr) fn replace_split_literals(
        &self,
        first: &mut LayerFirst,
        flat: &mut String,
        doc: &mut crate::types::internal::InternalDocument,
    ) {
        for proof in &self.1 {
            if literal_counts(&first.lines).get(proof.native.as_str()) != Some(&1)
                || literal_counts(flat).contains_key(proof.native.as_str())
            {
                continue;
            }
            if proof.replace(flat, doc) {
                trim_certified_prefix(first, &mut ahash::AHashMap::from([(proof.native.as_str(), 1)]));
            }
        }
    }

    pub(in crate::extractors::pdf::ocr) fn replace_repeated_identifiers(
        &self,
        first: &mut LayerFirst,
        flat: &mut String,
        doc: &mut crate::types::internal::InternalDocument,
    ) {
        for group in &self.2 {
            let token = group[0].native.as_str();
            if literal_counts(&first.lines).get(token) != Some(&group.len()) || literal_counts(flat).contains_key(token)
            {
                continue;
            }
            if replace_identifier_group(group, flat, doc) {
                trim_certified_prefix(first, &mut ahash::AHashMap::from([(token, group.len())]));
            }
        }
    }

    /// Both final views must account for every matched OCR occurrence. If a pass drops or
    /// repeats one, keep the native occurrences of that literal.
    pub(in crate::extractors::pdf::ocr) fn trim_prepend(
        &self,
        first: &mut LayerFirst,
        flat: &str,
        structured: &crate::types::internal::InternalDocument,
        page: u32,
    ) {
        let flat_text = flat;
        let flat = OutputLiterals::new(flat, &table_rows(&structured.tables, |bbox| bbox));
        let mut assets = MergedOcrAssets::new(0, 0);
        let (elements, _) = reindex_structured_ocr_page(structured, page, &mut assets);
        let projection = crate::types::internal::InternalDocument {
            elements,
            tables: assets.tables,
            images: assets.images,
            ..Default::default()
        };
        let positioned = positioned_rows(&projection);
        let rendering = crate::rendering::render_plain_with_spans(&projection);
        let structured_text = rendering.text;
        let structured = OutputLiterals::new(&structured_text, &Default::default());
        let prepended_counts = literal_counts(&first.lines);
        let mut available: ahash::AHashMap<_, _> = self
            .0
            .iter()
            .filter(|(token, occurrences)| {
                prepended_counts
                    .get(token.as_str())
                    .is_some_and(|prepended| *prepended <= occurrences.len())
                    && flat.contains(token, occurrences)
                    && structured.contains(token, occurrences)
                    && occurrences.iter().all(|word| has_positioned_owner(&positioned, word))
            })
            .map(|(token, occurrences)| (token.as_str(), occurrences.len()))
            .collect();
        trim_certified_prefix(first, &mut available);
        for proof in &self.3 {
            let mut line_owners = positioned.clone();
            if let Some(owners) = proof.rendered_line_owners(&projection, &structured_text, &rendering.spans.by_element)
            {
                line_owners.insert(proof.text.clone(), owners);
            }
            proof.trim(first, flat_text, &structured_text, &line_owners);
        }
    }
}

impl RepeatedLine {
    fn rendered_line_owners(
        &self,
        doc: &crate::types::internal::InternalDocument,
        rendered: &str,
        spans: &[Option<(usize, usize)>],
    ) -> Option<Vec<(LineBox, Option<usize>)>> {
        let mut owners = Vec::new();
        let mut used = ahash::AHashSet::new();
        for (element, span) in doc.elements.iter().zip(spans) {
            let Some((start, end)) = *span else { continue };
            let written = rendered.get(start..end.min(rendered.len()))?;
            let lines: Vec<_> = written.lines().map(row_key).filter(|line| !line.is_empty()).collect();
            if !lines.contains(&self.text) {
                continue;
            }
            let bounds = line_box(element.bbox?);
            let mut runs = self
                .source_lines
                .windows(lines.len())
                .enumerate()
                .filter(|(_, run)| *run == lines);
            let (source_start, _) = runs.next()?;
            if runs.next().is_some() {
                return None;
            }
            for (index, _) in lines.iter().enumerate().filter(|(_, line)| **line == self.text) {
                let source_index = source_start + index;
                let anchor = self.anchors.get(source_index.checked_sub(1)?)?.as_ref()?;
                let mut candidates = self
                    .boxes
                    .iter()
                    .enumerate()
                    .filter(|(_, bbox)| follows_positioned_line(anchor, bbox));
                let (occurrence, &bbox) = candidates.next()?;
                if candidates.next().is_some()
                    || !self.word_boxes.get(occurrence)?.iter().all(|word| {
                        bounds.owns(word)
                            && word.left >= bounds.left - 1.0
                            && word.right <= bounds.right + 1.0
                            && word.top >= bounds.top - 1.0
                            && word.bottom <= bounds.bottom + 1.0
                    })
                    || !used.insert(occurrence)
                {
                    return None;
                }
                owners.push((bbox, None));
            }
        }
        (owners.len() == self.boxes.len()).then_some(owners)
    }

    fn trim(&self, first: &mut LayerFirst, flat: &str, structured: &str, positioned: &PositionedRows) {
        if exact_line_count(flat, &self.text) != self.boxes.len()
            || exact_line_count(structured, &self.text) != self.boxes.len()
            || !positioned.get(&self.text).is_some_and(|owners| {
                owners.len() == self.boxes.len()
                    && owners.iter().all(|(_, table)| table.is_none())
                    && uniquely_paired(&owners.iter().map(|(bbox, _)| *bbox).collect::<Vec<_>>(), &self.boxes)
            })
        {
            return;
        }
        let count = exact_line_count(&first.lines, &self.text);
        if count == 0 || count > self.boxes.len() {
            return;
        }
        let mut remaining = first
            .lines
            .lines()
            .filter(|line| row_key(line) != self.text)
            .collect::<Vec<_>>()
            .join("\n");
        if first.lines.ends_with('\n') && !remaining.is_empty() {
            remaining.push('\n');
        }
        retain_surviving_misreads(first, remaining);
    }
}

fn follows_positioned_line(anchor: &LineBox, line: &LineBox) -> bool {
    let height = line.bottom - line.top;
    height > 0.0
        && anchor.top > line.top
        && anchor.top - line.bottom <= height
        && line.left >= anchor.left - height
        && line.right <= anchor.right + height
}

fn positioned_source_lines(words: &[LineAt], rows: &OcrRowIndex) -> ahash::AHashMap<String, Option<LineBox>> {
    let mut positioned = ahash::AHashMap::new();
    for word in words {
        let Some(line) = words_on_row(&word.bbox, words, rows) else {
            continue;
        };
        if !line.first().is_some_and(|first| std::ptr::eq(*first, word)) {
            continue;
        }
        let text = line.iter().map(|word| word.key.as_str()).collect::<Vec<_>>().join(" ");
        let bbox = line.iter().fold(word.bbox, |mut bounds, word| {
            bounds.left = bounds.left.min(word.bbox.left);
            bounds.right = bounds.right.max(word.bbox.right);
            bounds.top = bounds.top.min(word.bbox.top);
            bounds.bottom = bounds.bottom.max(word.bbox.bottom);
            bounds
        });
        positioned
            .entry(text)
            .and_modify(|owner| *owner = None)
            .or_insert(Some(bbox));
    }
    positioned
}

fn exact_line_count(text: &str, wanted: &str) -> usize {
    text.lines().filter(|line| row_key(line) == wanted).count()
}

fn capture_repeated_lines(
    layer: &str,
    native: &ahash::AHashMap<&str, Vec<LineBox>>,
    ocr: &WordIndices<'_>,
    words: &[LineAt],
    rows: &OcrRowIndex,
    flat: &str,
) -> Vec<RepeatedLine> {
    let source_lines: Vec<_> = flat.lines().map(row_key).filter(|line| !line.is_empty()).collect();
    let mut positioned = None;
    let mut counts = ahash::AHashMap::<String, usize>::new();
    for line in layer.lines().map(row_key) {
        *counts.entry(line).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(text, count)| {
            let tokens: Vec<_> = text.split_whitespace().collect();
            if count < 2 || tokens.len() < 3 || exact_line_count(flat, &text) != count {
                return None;
            }
            let starts = ocr.get(tokens[0])?;
            if starts.len() != count {
                return None;
            }
            let mut matched = Vec::new();
            let mut word_boxes = Vec::new();
            let mut used = ahash::AHashSet::new();
            for &index in starts {
                let row = words_on_row(&words[index].bbox, words, rows)?;
                if !row.iter().map(|word| word.key.as_str()).eq(tokens.iter().copied()) {
                    return None;
                }
                word_boxes.push(row.iter().map(|word| word.bbox).collect());
                for word in row {
                    let mut owners = native
                        .get(word.key.as_str())?
                        .iter()
                        .enumerate()
                        .filter(|(_, bounds)| bounds.owns(&word.bbox));
                    let (owner, _) = owners.next()?;
                    if owners.next().is_some() || !used.insert((word.key.as_str(), owner)) {
                        return None;
                    }
                }
                matched.push(words[index].bbox);
            }
            Some(RepeatedLine {
                text,
                boxes: matched,
                word_boxes,
                anchors: {
                    let positioned = positioned.get_or_insert_with(|| positioned_source_lines(words, rows));
                    source_lines
                        .iter()
                        .map(|line| positioned.get(line).copied().flatten())
                        .collect()
                },
                source_lines: source_lines.clone(),
            })
        })
        .collect()
}

fn trim_certified_prefix(first: &mut LayerFirst, available: &mut ahash::AHashMap<&str, usize>) {
    retain_surviving_misreads(first, trim_rows(&first.lines, available));
}

fn retain_surviving_misreads(first: &mut LayerFirst, trimmed: String) {
    if trimmed == first.lines {
        return;
    }
    let inventory = |text: &str| {
        let mut counts = ahash::AHashMap::<_, usize>::new();
        for key in text.lines().map(super::line_key) {
            *counts.entry(key).or_default() += 1;
        }
        counts
    };
    let before = inventory(&first.lines);
    let after = inventory(&trimmed);
    // A key alone cannot tell which identical native occurrence survived a partial trim. ~keep
    first.misread_ocr_rows.retain(|row| {
        after
            .get(&row.native_key)
            .is_some_and(|count| before.get(&row.native_key) == Some(count))
    });
    first.lines = trimmed;
}

fn has_unique_rendered_table(doc: &crate::types::internal::InternalDocument, index: usize) -> bool {
    use crate::types::internal::ElementKind;
    let mut references = doc
        .elements
        .iter()
        .filter(|element| matches!(element.kind, ElementKind::Table { table_index } if table_index as usize == index));
    // The merger synthesizes exactly one table element when there is no explicit reference. ~keep
    references.next().is_none_or(|element| element.text.is_empty()) && references.next().is_none()
}

fn replace_identifier_group(
    group: &[IdentifierRow],
    flat: &mut String,
    doc: &mut crate::types::internal::InternalDocument,
) -> bool {
    if !has_unique_rendered_table(doc, 0) {
        return false;
    }
    let [table] = doc.tables.as_mut_slice() else {
        return false;
    };
    if !table
        .bounding_box
        .map(line_box)
        .is_some_and(|bounds| group.iter().all(|proof| bounds.owns(&proof.bounds)))
    {
        return false;
    }
    let render = crate::rendering::common::render_table_markdown;
    let before = render(&table.cells);
    if flat.matches(before.trim_end()).count() != 1 {
        return false;
    }
    let mut corrected = table.cells.clone();
    let key = identifier_key(&group[0].native);
    let mut used = ahash::AHashSet::new();
    for index in 1..corrected.len() {
        let row = &corrected[index];
        if !row
            .iter()
            .flat_map(|cell| cell.split_whitespace())
            .any(|token| identifier_key(token) == key)
        {
            continue;
        }
        let anchor = register_anchor(&corrected[index - 1]);
        let mut owners = group
            .iter()
            .enumerate()
            .filter(|(_, proof)| anchor == Some(proof.anchor.as_str()) && cell_row_key(row) == proof.row);
        let Some((owner, proof)) = owners.next() else {
            return false;
        };
        if owners.next().is_some() || !used.insert(owner) {
            return false;
        }
        let Some(cells) = replace_cell_slice(row, &proof.split, &proof.native) else {
            return false;
        };
        corrected[index] = cells;
    }
    if used.len() != group.len()
        || corrected.first().is_some_and(|row| {
            row.iter()
                .flat_map(|cell| cell.split_whitespace())
                .any(|word| identifier_key(word) == key)
        })
    {
        return false;
    }
    let after = render(&corrected);
    *flat = flat.replacen(before.trim_end(), after.trim_end(), 1);
    table.cells = corrected;
    table.markdown = after;
    true
}

fn identifier_core(text: &str) -> &str {
    text.trim_start_matches('&').trim_end_matches('=')
}

fn identifier_key(text: &str) -> Option<String> {
    let (name, subscript) = identifier_core(text).split_once('[')?;
    let subscript = subscript.strip_suffix(']')?;
    let name = name.trim_end_matches(['.', ',']);
    if !name.contains('_')
        || name.len() < 8
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || subscript.is_empty()
        || !subscript.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let name = name
        .strip_prefix('1')
        .map_or_else(|| name.to_owned(), |rest| format!("I{rest}"));
    Some(format!("{name}[{subscript}]"))
}

fn capture_repeated_identifiers(
    layer: &str,
    native: &ahash::AHashMap<&str, Vec<LineBox>>,
    words: &[LineAt],
    tables: &[crate::types::Table],
) -> Vec<Vec<IdentifierRow>> {
    let counts = literal_counts(layer);
    let mut reads = ahash::AHashMap::<String, Vec<&LineAt>>::new();
    for word in words {
        if let Some(key) = identifier_key(&word.key) {
            reads.entry(key).or_default().push(word);
        }
    }
    let mut groups: Vec<Vec<IdentifierRow>> = native
        .iter()
        .filter_map(|(&token, boxes)| {
            if boxes.len() < 2 || counts.get(token) != Some(&boxes.len()) {
                return None;
            }
            let read = reads.get(&identifier_key(token)?)?;
            let read_boxes: Vec<_> = read.iter().map(|word| word.bbox).collect();
            if !uniquely_paired(boxes, &read_boxes)
                || read_boxes.iter().any(|read| {
                    native
                        .values()
                        .flatten()
                        .filter(|source| owns_identifier_center(source, read))
                        .count()
                        != 1
                })
            {
                return None;
            }
            let bound = bind_identifier_rows(token, read, tables, words)?;
            bound
                .iter()
                .all(|proof| {
                    let Some([anchor]) = native.get(proof.anchor.as_str()).map(Vec::as_slice) else {
                        return false;
                    };
                    let mut owners = boxes
                        .iter()
                        .filter(|source| owns_identifier_center(source, &proof.bounds));
                    let Some(owner) = owners.next() else { return false };
                    let height = owner.bottom - owner.top;
                    owners.next().is_none()
                        && anchor.right < owner.left
                        && anchor.top > owner.top
                        && anchor.top - owner.top <= height * 2.0
                })
                .then_some(bound)
        })
        .collect();
    groups.sort_by(|left, right| left[0].native.cmp(&right[0].native));
    groups
}

fn owns_identifier_center(source: &LineBox, read: &LineBox) -> bool {
    let center = (read.left + read.right) * 0.5;
    source.owns(read) && (source.left..=source.right).contains(&center)
}

fn register_anchor(row: &[String]) -> Option<&str> {
    let token = row.first()?.split_whitespace().next()?;
    is_register_anchor(token).then_some(token)
}

fn is_register_anchor(token: &str) -> bool {
    token
        .strip_suffix('H')
        .is_some_and(|hex| (3..=8).contains(&hex.len()) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn bind_identifier_rows(
    token: &str,
    reads: &[&LineAt],
    tables: &[crate::types::Table],
    words: &[LineAt],
) -> Option<Vec<IdentifierRow>> {
    let [table] = tables else { return None };
    let key = identifier_key(token);
    let candidates: Vec<_> = table
        .cells
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            row.iter()
                .flat_map(|cell| cell.split_whitespace())
                .any(|word| identifier_key(word) == key)
        })
        .collect();
    if candidates.len() != reads.len() {
        return None;
    }
    let mut used = ahash::AHashSet::new();
    reads
        .iter()
        .map(|word| {
            let height = word.bbox.bottom - word.bbox.top;
            let mut anchors = words.iter().filter(|other| {
                is_register_anchor(&other.key)
                    && other.bbox.right < word.bbox.left
                    && other.bbox.top > word.bbox.top
                    && other.bbox.top - word.bbox.top <= height * 2.0
            });
            let anchor = anchors.next()?;
            if anchors.next().is_some() || !used.insert(anchor.key.as_str()) {
                return None;
            }
            let mut rows = candidates.iter().filter(|(index, _)| {
                index
                    .checked_sub(1)
                    .and_then(|previous| register_anchor(&table.cells[previous]))
                    == Some(anchor.key.as_str())
            });
            let (_, row) = rows.next()?;
            if rows.next().is_some() {
                return None;
            }
            let core = identifier_core(&word.key);
            replace_cell_slice(row, core, token)?;
            Some(IdentifierRow {
                native: token.into(),
                split: core.into(),
                row: cell_row_key(row),
                anchor: anchor.key.clone(),
                bounds: word.bbox,
            })
        })
        .collect()
}

fn capture_split_literals(
    layer: &str,
    native: &ahash::AHashMap<&str, Vec<LineBox>>,
    ocr: &WordIndices<'_>,
    words: &[LineAt],
    rows: &OcrRowIndex,
) -> Vec<SplitLiteral> {
    let counts = literal_counts(layer);
    words
        .windows(2)
        .filter_map(|pair| {
            let [left, right] = pair else { return None };
            let token = format!("{}{}", left.key, right.key);
            if token.len() < 8
                || !token.contains('_')
                || counts.get(token.as_str()) != Some(&1)
                || ocr.contains_key(token.as_str())
            {
                return None;
            }
            let [bounds] = native.get(token.as_str())?.as_slice() else {
                return None;
            };
            let visual = words_on_row(bounds, words, rows)?;
            if !visual
                .windows(2)
                .any(|pair| std::ptr::eq(pair[0], left) && std::ptr::eq(pair[1], right))
                || !split_words_owned(*bounds, left, right, native)
            {
                return None;
            }
            Some(SplitLiteral {
                native: token,
                split: format!("{} {}", left.key, right.key),
                row: visual
                    .iter()
                    .map(|word| word.key.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                bounds: *bounds,
            })
        })
        .collect()
}

fn split_words_owned(
    bounds: LineBox,
    left: &LineAt,
    right: &LineAt,
    native: &ahash::AHashMap<&str, Vec<LineBox>>,
) -> bool {
    let tolerance = (bounds.bottom - bounds.top) * 0.25;
    left.bbox.right <= right.bbox.left + tolerance
        && [left, right].iter().all(|word| {
            let bbox = word.bbox;
            bounds.owns(&bbox)
                && bbox.left >= bounds.left - tolerance
                && bbox.right <= bounds.right + tolerance
                && bbox.top >= bounds.top - tolerance
                && bbox.bottom <= bounds.bottom + tolerance
                && native.values().flatten().filter(|other| other.owns(&bbox)).count() == 1
        })
}

impl SplitLiteral {
    fn replace(&self, flat: &mut String, doc: &mut crate::types::internal::InternalDocument) -> bool {
        let mut matches = doc.tables.iter().enumerate().flat_map(|(table_index, table)| {
            table.cells.iter().enumerate().filter_map(move |(row_index, row)| {
                (table
                    .bounding_box
                    .map(line_box)
                    .is_some_and(|bbox| bbox.owns(&self.bounds))
                    && cell_row_key(row) == self.row)
                    .then_some((table_index, row_index))
            })
        });
        let Some((table, row)) = matches.next() else {
            return false;
        };
        if matches.next().is_some() || !has_unique_rendered_table(doc, table) {
            return false;
        }
        let original = &doc.tables[table];
        let Some(cells) = replace_cell_slice(&original.cells[row], &self.split, &self.native) else {
            return false;
        };
        let render = crate::rendering::common::render_table_markdown;
        let before = render(&original.cells);
        if flat.matches(&before).count() != 1 {
            return false;
        }
        let mut corrected = original.cells.clone();
        corrected[row] = cells;
        let after = render(&corrected);
        *flat = flat.replacen(&before, &after, 1);
        doc.tables[table].cells = corrected;
        doc.tables[table].markdown = after;
        true
    }
}

fn replace_cell_slice(row: &[String], split: &str, native: &str) -> Option<Vec<String>> {
    let mut cells = row.to_vec();
    let mut matches = cells.iter_mut().flat_map(|cell| {
        let ranges: Vec<_> = cell
            .match_indices(split)
            .filter_map(|(start, text)| {
                let end = start + text.len();
                let boundary = |character: Option<char>| character.is_none_or(char::is_whitespace);
                ((boundary(cell[..start].chars().next_back()) || cell[..start].ends_with("&&"))
                    && (boundary(cell[end..].chars().next()) || cell[end..].starts_with('=')))
                .then_some(start..end)
            })
            .collect();
        (!ranges.is_empty()).then_some((cell, ranges))
    });
    let (cell, ranges) = matches.next()?;
    let [range] = ranges.as_slice() else { return None };
    if matches.next().is_some() {
        return None;
    }
    cell.replace_range(range.clone(), native);
    Some(cells)
}

type WordIndices<'a> = ahash::AHashMap<&'a str, Vec<usize>>;

fn iterator_words(
    read: &crate::types::ExtractedDocument,
    raster: (u32, u32),
    media: xberg_native_pdf::geometry::Rect,
) -> Option<(Vec<LineAt>, WordIndices<'_>)> {
    use super::super::native_word_coverage::{is_iterator_word, ocr_word_in_page_points};

    let read_words = read
        .ocr_elements
        .as_ref()?
        .iter()
        .filter(|word| word.level == crate::types::OcrElementLevel::Word);
    let mut words = Vec::new();
    let mut ocr = WordIndices::new();
    for word in read_words {
        if !is_iterator_word(word) {
            return None;
        }
        let key = word.text.trim();
        ocr.entry(key).or_default().push(words.len());
        words.push(LineAt {
            key: key.to_owned(),
            bbox: ocr_word_in_page_points(word, raster, media)?,
        });
    }

    Some((words, ocr))
}

fn trim_rows(lines: &str, available: &mut ahash::AHashMap<&str, usize>) -> String {
    lines
        .split('\n')
        .filter_map(|line| {
            let mut remaining = String::with_capacity(line.len());
            let mut removed = false;
            for part in line.split_inclusive(char::is_whitespace) {
                let token = part.trim_end_matches(char::is_whitespace);
                if let Some(count) = available.get_mut(token).filter(|count| **count > 0) {
                    *count -= 1;
                    removed = true;
                } else if !token.is_empty() || !removed {
                    remaining.push_str(part);
                    removed = false;
                }
            }
            (!remaining.trim().is_empty() || line.trim().is_empty()).then_some(remaining)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_box(bbox: crate::types::BoundingBox) -> LineBox {
    LineBox {
        left: bbox.x0 as f32,
        top: bbox.y0 as f32,
        right: bbox.x1 as f32,
        bottom: bbox.y1 as f32,
    }
}

fn row_key(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

type PositionedRows = ahash::AHashMap<String, Vec<(LineBox, Option<usize>)>>;

fn has_positioned_owner(rows: &PositionedRows, word: &OutputOccurrence) -> bool {
    let Some(owners) = rows.get(word.row.as_ref()) else {
        return false;
    };
    owners
        .iter()
        .filter(|(bbox, index)| {
            bbox.owns(&word.bbox) && word.table == index.map(|index| TableRowPosition { index, bbox: *bbox })
        })
        .count()
        == 1
}

fn positioned_rows(doc: &crate::types::internal::InternalDocument) -> PositionedRows {
    use crate::types::internal::ElementKind;

    let mut rows = PositionedRows::new();
    for element in &doc.elements {
        match element.kind {
            ElementKind::Image { .. } => {}
            ElementKind::Table { table_index } => {
                let Some(table) = doc.tables.get(table_index as usize).filter(|_| element.text.is_empty()) else {
                    continue;
                };
                let Some(bbox) = table.bounding_box.map(line_box) else {
                    continue;
                };
                for (index, cells) in table.cells.iter().enumerate() {
                    rows.entry(cell_row_key(cells)).or_default().push((bbox, Some(index)));
                }
            }
            _ => {
                let Some(bbox) = element.bbox.map(line_box) else {
                    continue;
                };
                for line in element.text.lines() {
                    rows.entry(row_key(line)).or_default().push((bbox, None));
                }
            }
        }
    }
    rows
}

fn words_on_row<'a>(bbox: &LineBox, words: &'a [LineAt], rows: &OcrRowIndex) -> Option<Vec<&'a LineAt>> {
    let mut run: Vec<_> = rows
        .band(bbox)
        .filter_map(|index| {
            let word = &words[index];
            let aligned = LineBox {
                left: bbox.left,
                right: bbox.right,
                ..word.bbox
            };
            bbox.owns(&aligned).then_some(word)
        })
        .collect();
    run.sort_unstable_by(|a, b| a.bbox.left.total_cmp(&b.bbox.left));
    run.windows(2)
        .all(|pair| pair[0].bbox.left < pair[1].bbox.left)
        .then_some(run)
}

fn spatial_row(index: usize, words: &[LineAt], rows: &OcrRowIndex) -> Option<OutputOccurrence> {
    let word = &words[index];
    let mut tokens = Vec::new();
    let mut column = None;
    for current in words_on_row(&word.bbox, words, rows)? {
        if std::ptr::eq(current, word) {
            column = Some(tokens.len());
        }
        tokens.extend(current.key.split_whitespace());
    }
    Some(OutputOccurrence {
        row: tokens.join(" ").into(),
        column: column?,
        bbox: word.bbox,
        table: None,
    })
}

fn competing_row(output: &[OutputRow], table: TableRowPosition, suffix: &str) -> bool {
    output.iter().any(|other| {
        other
            .table
            .is_some_and(|owner| owner.bbox == table.bbox && owner.index != table.index)
            && (other.text.as_ref() == suffix
                || other
                    .text
                    .strip_prefix(suffix)
                    .is_some_and(|tail| tail.starts_with(' ')))
    })
}

/// A continuation must be the next visual line and stay in the preceding row's last cell.
fn wrapped_cell_continues(
    word: &OutputOccurrence,
    row: &OutputRow,
    table: TableRowPosition,
    words: &[LineAt],
    rows: &OcrRowIndex,
    output: &[OutputRow],
) -> Option<()> {
    let suffix = row.text.strip_prefix(word.row.as_ref())?.strip_prefix(' ')?;
    if competing_row(output, table, suffix) {
        return None;
    }
    let first_column = row.last_cell_start?;
    let current = words_on_row(&word.bbox, words, rows)?;
    let top = current.iter().map(|word| word.bbox.top).min_by(f32::total_cmp)?;
    let bottom = current.iter().map(|word| word.bbox.bottom).max_by(f32::total_cmp)?;
    let height = bottom - top;
    let mut columns = 0;
    let cell_left = current.iter().find_map(|entry| {
        let first = columns == first_column;
        columns += entry.key.split_whitespace().count();
        first.then_some(entry.bbox.left)
    })?;
    let below = LineBox {
        top: top - height,
        bottom: top,
        ..table.bbox
    };
    let next = rows
        .band(&below)
        .map(|index| &words[index])
        .filter(|next| table.bbox.owns(&next.bbox) && next.bbox.bottom <= top && top - next.bbox.bottom <= height)
        .max_by(|a, b| a.bbox.bottom.total_cmp(&b.bbox.bottom))?;
    let continuation = words_on_row(&next.bbox, words, rows)?;
    (continuation
        .iter()
        .all(|next| table.bbox.owns(&next.bbox) && next.bbox.left >= cell_left - height)
        && continuation
            .iter()
            .flat_map(|next| next.key.split_whitespace())
            .eq(suffix.split_whitespace()))
    .then_some(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TableRowPosition {
    index: usize,
    bbox: LineBox,
}

struct OutputRow {
    text: std::sync::Arc<str>,
    table: Option<TableRowPosition>,
    last_cell_start: Option<usize>,
}

fn cell_row_key(cells: &[String]) -> String {
    cells
        .iter()
        .flat_map(|cell| cell.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Match the table's serialized rows so that cell pipes and structural separators stay distinct.
fn table_rows(
    tables: &[crate::types::Table],
    in_points: impl Fn(crate::types::BoundingBox) -> crate::types::BoundingBox,
) -> ahash::AHashMap<String, Option<OutputRow>> {
    let mut rows = ahash::AHashMap::new();
    for table in tables {
        let bbox = table.bounding_box.map(&in_points).map(line_box);
        let markdown = crate::rendering::common::render_table_markdown(&table.cells);
        for (index, (row, cells)) in markdown
            .lines()
            .enumerate()
            .filter_map(|(line, row)| (line != 1).then_some(row))
            .zip(&table.cells)
            .enumerate()
        {
            if cells.iter().any(|cell| cell.contains(['\n', '\r'])) {
                continue;
            }
            rows.entry(row_key(row))
                .and_modify(|row: &mut Option<OutputRow>| *row = None)
                .or_insert_with(|| {
                    Some(OutputRow {
                        text: cell_row_key(cells).into(),
                        table: bbox.map(|bbox| TableRowPosition { index, bbox }),
                        last_cell_start: cells
                            .iter()
                            .rposition(|cell| !cell.trim().is_empty())
                            .map(|last| cells[..last].iter().map(|cell| cell.split_whitespace().count()).sum()),
                    })
                });
        }
    }
    rows
}

#[derive(Debug)]
struct OutputOccurrence {
    row: std::sync::Arc<str>,
    column: usize,
    bbox: LineBox,
    table: Option<TableRowPosition>,
}

struct OutputLiterals<'a> {
    original: ahash::AHashMap<&'a str, usize>,
    rows: Vec<OutputRow>,
    positions: ahash::AHashMap<String, Vec<(usize, usize)>>,
}

impl<'a> OutputLiterals<'a> {
    fn new(text: &'a str, table_rows: &ahash::AHashMap<String, Option<OutputRow>>) -> Self {
        let mut rows = Vec::new();
        let mut positions: ahash::AHashMap<String, Vec<(usize, usize)>> = ahash::AHashMap::new();
        for (row, line) in text.lines().enumerate() {
            let key = row_key(line);
            let output = table_rows.get(&key).and_then(Option::as_ref).map_or_else(
                || OutputRow {
                    text: key.clone().into(),
                    table: None,
                    last_cell_start: None,
                },
                |row| OutputRow {
                    text: row.text.clone(),
                    table: row.table,
                    last_cell_start: row.last_cell_start,
                },
            );
            for (column, token) in output.text.split_whitespace().enumerate() {
                positions.entry(token.to_owned()).or_default().push((row, column));
            }
            rows.push(output);
        }
        Self {
            original: literal_counts(text),
            rows,
            positions,
        }
    }

    fn bind(
        &self,
        token: &str,
        occurrences: &[OutputOccurrence],
        words: &[LineAt],
        rows: &OcrRowIndex,
    ) -> Option<Vec<OutputOccurrence>> {
        let mut bound = Vec::with_capacity(occurrences.len());
        self.match_occurrences(
            token,
            occurrences,
            |word, row| {
                if let Some(table) = row.table {
                    if !table.bbox.owns(&word.bbox) {
                        return false;
                    }
                    return row.text == word.row
                        || wrapped_cell_continues(word, row, table, words, rows, &self.rows).is_some();
                }
                row.text == word.row
            },
            |word, row| {
                bound.push(OutputOccurrence {
                    row: row.text.clone(),
                    column: word.column,
                    bbox: word.bbox,
                    table: row.table,
                });
            },
        )
        .then_some(bound)
    }

    fn contains(&self, token: &str, occurrences: &[OutputOccurrence]) -> bool {
        self.match_occurrences(token, occurrences, |word, row| word.row == row.text, |_, _| {})
    }

    fn match_occurrences(
        &self,
        token: &str,
        occurrences: &[OutputOccurrence],
        matches: impl Fn(&OutputOccurrence, &OutputRow) -> bool,
        mut found: impl FnMut(&OutputOccurrence, &OutputRow),
    ) -> bool {
        if self.original.get(token) != Some(&occurrences.len()) {
            return false;
        }
        let Some(positions) = self
            .positions
            .get(token)
            .filter(|positions| positions.len() == occurrences.len())
        else {
            return false;
        };
        let mut used = ahash::AHashSet::new();
        occurrences.iter().all(|word| {
            let mut candidates = positions
                .iter()
                .filter(|&&(row, column)| word.column == column && matches(word, &self.rows[row]));
            let Some(position) = candidates.next() else {
                return false;
            };
            if candidates.next().is_some() || !used.insert(position) {
                return false;
            }
            found(word, &self.rows[position.0]);
            true
        })
    }
}

fn literal_counts(text: &str) -> ahash::AHashMap<&str, usize> {
    let mut counts = ahash::AHashMap::new();
    for token in text.split_whitespace() {
        *counts.entry(token).or_default() += 1;
    }
    counts
}

fn uniquely_paired(native: &[LineBox], ocr: &[LineBox]) -> bool {
    // Bound the work for extremely repetitive literals, leaving those occurrences in the layer. ~keep
    const MAX_REPEATS: usize = 64;
    if native.is_empty() || native.len() != ocr.len() || native.len() > MAX_REPEATS {
        return false;
    }
    let mut used = 0u64;
    for source in native {
        let mut candidates = ocr.iter().enumerate().filter(|(_, read)| source.owns(read));
        let Some((index, _)) = candidates.next() else {
            return false;
        };
        let bit = 1u64 << index;
        if used & bit != 0 || candidates.next().is_some() {
            return false;
        }
        used |= bit;
    }
    true
}

#[cfg(test)]
mod matched_layer_occurrences_tests {
    use super::{
        LayerFirst, LineBox, MatchedLayerOccurrences, OutputOccurrence, TableRowPosition, flat_ocr_page_document,
        uniquely_paired,
    };
    use crate::types::{BoundingBox, Table};

    fn occurrence(row: &str, column: usize) -> OutputOccurrence {
        OutputOccurrence {
            row: row.into(),
            column,
            bbox: at(0.0),
            table: None,
        }
    }

    fn at(left: f32) -> LineBox {
        LineBox {
            left,
            top: 0.0,
            right: left + 10.0,
            bottom: 10.0,
        }
    }

    #[test]
    fn split_table_literal_replacement_keeps_distinct_occurrences_and_ocr_only_cells() {
        use super::{LayerFirst, LineAt, LineBox, OcrRowIndex, capture_split_literals};
        use crate::types::internal::{ElementKind, InternalElement};

        const NATIVE: &str = "_mm_example(__m128i";
        let position = |left, right, top| LineBox {
            left,
            right,
            top,
            bottom: top + 10.0,
        };
        for (distinct, copies) in [(false, 1), (true, 1), (false, 2)] {
            let bounds = position(10.0, 55.0, 20.0);
            let words = vec![
                LineAt {
                    key: "Label".into(),
                    bbox: position(0.0, 8.0, 20.0),
                },
                LineAt {
                    key: "_mm_example(__m1".into(),
                    bbox: position(10.0, 45.0, 20.0),
                },
                LineAt {
                    key: "28i".into(),
                    bbox: position(46.0, 55.0, if distinct { 55.0 } else { 20.0 }),
                },
                LineAt {
                    key: "Ovf_PMC1".into(),
                    bbox: position(60.0, 85.0, 20.0),
                },
            ];
            let mut native = vec![bounds];
            if copies > 1 {
                native.push(position(10.0, 55.0, 60.0));
            }
            let layer = if copies == 1 {
                NATIVE.to_owned()
            } else {
                format!("{NATIVE}\n{NATIVE}")
            };
            let proofs = capture_split_literals(
                &layer,
                &ahash::AHashMap::from([(NATIVE, native)]),
                &Default::default(),
                &words,
                &OcrRowIndex::new(&words),
            );
            let proof = MatchedLayerOccurrences(Default::default(), proofs, Vec::new(), Vec::new());
            let cells = vec![vec!["Label".into(), "_mm_example(__m1 28i".into(), "Ovf_PMC1".into()]];
            let mut doc = flat_ocr_page_document("");
            doc.tables.push(Table {
                cells: cells.clone(),
                bounding_box: Some(BoundingBox {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 100.0,
                }),
                ..Default::default()
            });
            doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0));
            let mut flat = crate::rendering::common::render_table_markdown(&cells);
            let mut first = LayerFirst {
                lines: layer.clone(),
                lost_glyphs: 0,
                misread_ocr_rows: Vec::new(),
            };
            proof.replace_split_literals(&mut first, &mut flat, &mut doc);
            if distinct || copies > 1 {
                assert_eq!(first.lines, layer);
                assert_eq!(doc.tables[0].cells, cells);
            } else {
                assert!(first.lines.is_empty());
                assert_eq!(doc.tables[0].cells[0], ["Label", NATIVE, "Ovf_PMC1"]);
            }
            assert_eq!(
                flat,
                crate::rendering::common::render_table_markdown(&doc.tables[0].cells)
            );
        }
    }
    #[test]
    fn rendered_continuations_require_a_unique_source_run() {
        let text = "Monitoring Management Facility.";
        let upper = LineBox {
            top: 40.0,
            bottom: 50.0,
            ..at(0.0)
        };
        let lower = LineBox {
            top: 20.0,
            bottom: 30.0,
            ..at(0.0)
        };
        let source = format!("First register\n{text}\nSecond register\n{text}");
        let proof = super::RepeatedLine {
            text: text.into(),
            boxes: vec![lower, upper],
            word_boxes: vec![
                vec![
                    lower,
                    LineBox {
                        left: 12.,
                        right: 24.,
                        ..lower
                    },
                ],
                vec![
                    upper,
                    LineBox {
                        left: 12.,
                        right: 24.,
                        ..upper
                    },
                ],
            ],
            source_lines: source.lines().map(str::to_owned).collect(),
            anchors: vec![
                Some(LineBox {
                    top: 51.0,
                    bottom: 61.0,
                    ..upper
                }),
                None,
                Some(LineBox {
                    top: 31.0,
                    bottom: 41.0,
                    ..lower
                }),
                None,
            ],
        };
        let mut doc = flat_ocr_page_document(&source);
        doc.elements[0].bbox = Some(BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        });
        let rendering = crate::rendering::render_plain_with_spans(&doc);
        assert_eq!(
            proof.rendered_line_owners(&doc, &rendering.text, &rendering.spans.by_element),
            Some(vec![(upper, None), (lower, None)])
        );
        doc.elements[0].bbox.as_mut().unwrap().x1 = 10.0;
        assert!(
            proof
                .rendered_line_owners(&doc, &rendering.text, &rendering.spans.by_element)
                .is_none()
        );
        doc.elements[0].text = text.into();
        let rendering = crate::rendering::render_plain_with_spans(&doc);
        assert!(
            proof
                .rendered_line_owners(&doc, &rendering.text, &rendering.spans.by_element)
                .is_none()
        );
    }

    #[test]
    fn column_major_continuations_follow_their_positioned_anchors() {
        let text = "Repeated continuation here";
        let left = LineBox {
            top: 20.0,
            bottom: 30.0,
            ..at(0.0)
        };
        let right = LineBox {
            top: 60.0,
            bottom: 70.0,
            ..at(40.0)
        };
        let source = format!("Left anchor\n{text}\nRight anchor\n{text}");
        let proof = super::RepeatedLine {
            text: text.into(),
            boxes: vec![right, left],
            word_boxes: vec![vec![right], vec![left]],
            source_lines: source.lines().map(str::to_owned).collect(),
            anchors: vec![
                Some(LineBox {
                    top: 31.0,
                    bottom: 41.0,
                    ..left
                }),
                None,
                Some(LineBox {
                    top: 71.0,
                    bottom: 81.0,
                    ..right
                }),
                None,
            ],
        };
        let mut doc = flat_ocr_page_document(&source);
        doc.elements[0].bbox = Some(BoundingBox {
            x0: 0.,
            y0: 0.,
            x1: 100.,
            y1: 100.,
        });
        let rendering = crate::rendering::render_plain_with_spans(&doc);
        assert_eq!(
            proof.rendered_line_owners(&doc, &rendering.text, &rendering.spans.by_element),
            Some(vec![(left, None), (right, None)])
        );
    }

    #[test]
    fn repeated_continuation_keeps_an_unrelated_occurrence() {
        let text = "Monitoring Management Facility.";
        let proof = super::RepeatedLine {
            text: text.into(),
            boxes: vec![at(0.0), at(30.0)],
            word_boxes: vec![vec![at(0.0)], vec![at(30.0)]],
            source_lines: Vec::new(),
            anchors: Vec::new(),
        };
        let mut first = LayerFirst {
            lines: text.into(),
            lost_glyphs: 0,
            misread_ocr_rows: Vec::new(),
        };
        let flat = format!("{text}\n{text}");
        let unrelated = ahash::AHashMap::from([(text.into(), vec![(at(0.0), None), (at(80.0), None)])]);
        proof.trim(&mut first, &flat, &flat, &unrelated);
        assert_eq!(first.lines, text);
        let shared_box = LineBox {
            right: 100.0,
            ..at(0.0)
        };
        let shared = ahash::AHashMap::from([(text.into(), vec![(shared_box, None), (shared_box, None)])]);
        proof.trim(&mut first, &flat, &flat, &shared);
        assert_eq!(first.lines, text);
        let owned = ahash::AHashMap::from([(text.into(), vec![(at(0.0), None), (at(30.0), None)])]);
        proof.trim(&mut first, &flat, &flat, &owned);
        assert!(first.lines.is_empty());
    }

    #[test]
    fn trimming_keeps_suppression_for_an_unrelated_native_row() {
        use super::super::MisreadOcrRow;
        let row = |key: &str, native_key: &str| MisreadOcrRow {
            page: Some(1),
            line: None,
            bbox: None,
            key: key.into(),
            native_key: native_key.into(),
        };
        let mut first = LayerFirst {
            lines: "owned target\nunrelated native".into(),
            lost_glyphs: 0,
            misread_ocr_rows: vec![row("ocr target", "owned target"), row("ocr noise", "unrelated native")],
        };
        super::trim_certified_prefix(&mut first, &mut ahash::AHashMap::from([("owned", 1)]));
        assert_eq!(first.lines, "target\nunrelated native");
        assert_eq!(first.misread_ocr_rows, [row("ocr noise", "unrelated native")]);
    }

    #[test]
    fn partial_trim_of_duplicate_native_keys_releases_both_suppressions() {
        use super::super::MisreadOcrRow;
        let mut first = LayerFirst {
            lines: "owned row\nowned row".into(),
            lost_glyphs: 0,
            misread_ocr_rows: (0..2)
                .map(|line| MisreadOcrRow {
                    page: Some(1),
                    line: Some(line),
                    bbox: Some(at(line as f32 * 30.0)),
                    key: format!("OCR {line}"),
                    native_key: "owned row".into(),
                })
                .collect(),
        };
        super::trim_certified_prefix(&mut first, &mut ahash::AHashMap::from([("owned", 1)]));
        assert_eq!(first.lines, "row\nowned row");
        assert!(first.misread_ocr_rows.is_empty());
    }

    #[test]
    fn repeated_explicit_table_references_are_ambiguous() {
        use crate::types::internal::{ElementKind, InternalElement};
        let mut doc = flat_ocr_page_document("");
        doc.tables.push(Table::default());
        assert!(super::has_unique_rendered_table(&doc, 0));
        let reference = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0);
        doc.push_element(reference.clone());
        assert!(super::has_unique_rendered_table(&doc, 0));
        doc.push_element(reference);
        assert!(!super::has_unique_rendered_table(&doc, 0));
    }

    #[test]
    fn reordered_identical_identifier_rows_without_distinct_witnesses_are_ambiguous() {
        use super::{LineAt, bind_identifier_rows};
        let reads = [
            LineAt {
                key: "1ABC_REGISTER[10]".into(),
                bbox: at(0.0),
            },
            LineAt {
                key: "1ABC_REGISTER[10]".into(),
                bbox: at(30.0),
            },
        ];
        let table = Table {
            cells: vec![vec!["1ABC_REGISTER[10]".into()], vec!["1ABC_REGISTER[10]".into()]],
            ..Default::default()
        };
        let reordered = [&reads[1], &reads[0]];
        assert!(bind_identifier_rows("IABC_REGISTER.[10]", &reordered, &[table], &[]).is_none());
    }

    #[test]
    fn ocr_only_identifier_collision_declines_the_group() {
        use super::{LineAt, bind_identifier_rows};
        let reads = [
            LineAt {
                key: "1ABC_REGISTER[10]".into(),
                bbox: LineBox {
                    top: 20.0,
                    bottom: 30.0,
                    ..at(50.0)
                },
            },
            LineAt {
                key: "1ABC_REGISTER[10]".into(),
                bbox: LineBox {
                    top: 60.0,
                    bottom: 70.0,
                    ..at(50.0)
                },
            },
        ];
        let refs = [&reads[1], &reads[0]];
        let anchors = [
            LineAt {
                key: "80AH".into(),
                bbox: LineBox {
                    top: 31.0,
                    bottom: 41.0,
                    ..at(0.0)
                },
            },
            LineAt {
                key: "80BH".into(),
                bbox: LineBox {
                    top: 71.0,
                    bottom: 81.0,
                    ..at(0.0)
                },
            },
        ];
        let mut table = Table {
            cells: vec![
                vec!["80AH".into()],
                vec!["1ABC_REGISTER[10]".into()],
                vec!["80BH".into()],
                vec!["1ABC_REGISTER[10]".into()],
            ],
            ..Default::default()
        };
        let bound = bind_identifier_rows("IABC_REGISTER.[10]", &refs, std::slice::from_ref(&table), &anchors).unwrap();
        assert_eq!([bound[0].anchor.as_str(), bound[1].anchor.as_str()], ["80BH", "80AH"]);
        table
            .cells
            .insert(1, vec!["OCR-only".into(), "1ABC_REGISTER[10]".into()]);
        assert!(bind_identifier_rows("IABC_REGISTER.[10]", &refs, &[table], &anchors).is_none());
    }

    #[test]
    fn repeated_literals_require_distinct_unambiguous_locations() {
        assert!(uniquely_paired(&[at(0.0), at(30.0)], &[at(30.0), at(0.0)]));
        assert!(!uniquely_paired(&[at(0.0), at(0.0)], &[at(0.0), at(30.0)]));
        assert!(!uniquely_paired(&[at(0.0), at(7.0)], &[at(3.0), at(5.0)]));
        assert!(!uniquely_paired(&[at(0.0)], &[at(f32::NAN)]));
    }

    #[test]
    fn repeat_limit_includes_64_and_rejects_65() {
        let boxes: Vec<_> = (0..65).map(|i| at(i as f32 * 30.0)).collect();
        assert!(uniquely_paired(&boxes[..64], &boxes[..64]));
        assert!(!uniquely_paired(&boxes, &boxes));
    }

    #[test]
    fn partial_trimming_preserves_indentation_spacing_and_blank_rows() {
        let mut available = ahash::AHashMap::from([("769", 2)]);
        let input = "  \t769 native   row\n\n  left  769  right   ";
        assert_eq!(
            super::trim_rows(input, &mut available),
            "  \tnative   row\n\n  left  right   "
        );
    }

    #[test]
    fn trims_only_literals_surviving_both_views_including_table_cells() {
        let mut doc = flat_ocr_page_document("changed: Unmatched");
        doc.tables.push(Table {
            cells: vec![
                vec!["PAND".into(), "row1".into()],
                vec!["PAND".into(), "row2".into(), "OCR-only |".into()],
            ],
            bounding_box: Some(BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 100.0,
            }),
            ..Default::default()
        });
        let mut rows = vec![occurrence("PAND row1", 0), occurrence("PAND row2 OCR-only |", 0)];
        for (index, word) in rows.iter_mut().enumerate() {
            word.table = Some(TableRowPosition {
                index,
                bbox: super::line_box(doc.tables[0].bounding_box.unwrap()),
            });
        }
        let proof = MatchedLayerOccurrences(
            ahash::AHashMap::from([
                ("PAND".into(), rows),
                ("changed".into(), vec![occurrence("changed Unmatched", 0)]),
                ("Unmatched".into(), vec![occurrence("Unmatched missing", 0)]),
            ]),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let mut first = LayerFirst {
            lines: "PAND   native\nPAND\nchanged Unmatched pand".into(),
            lost_glyphs: 0,
            misread_ocr_rows: Vec::new(),
        };
        let flat = format!(
            "{}\nchanged Unmatched",
            crate::rendering::common::render_table_markdown(&doc.tables[0].cells)
        );
        proof.trim_prepend(&mut first, &flat, &doc, 1);
        assert_eq!(first.lines, "native\nchanged Unmatched pand");
        assert_eq!(doc.tables[0].cells[1], ["PAND", "row2", "OCR-only |"]);
        doc.tables[0].cells.swap(0, 1);
        let mut first = LayerFirst {
            lines: "PAND PAND".into(),
            lost_glyphs: 0,
            misread_ocr_rows: Vec::new(),
        };
        proof.trim_prepend(&mut first, &flat, &doc, 1);
        assert_eq!(first.lines, "PAND PAND");
    }

    #[test]
    fn wrapped_suffix_requires_the_next_line_in_the_last_cell() {
        use super::{LineAt, OcrRowIndex, OutputRow, wrapped_cell_continues};
        let placed = |text: &str, left: f32, top: f32| LineAt {
            key: text.into(),
            bbox: LineBox {
                left,
                top,
                right: left + 10.0,
                bottom: top + 10.0,
            },
        };
        let mut words = vec![
            placed("909", 0.0, 30.0),
            placed("description", 40.0, 30.0),
            placed("continued", 40.0, 15.0),
        ];
        let mut word = occurrence("909 description", 0);
        word.bbox = words[0].bbox;
        let table = TableRowPosition {
            index: 0,
            bbox: LineBox {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 100.0,
            },
        };
        let row = OutputRow {
            text: "909 description continued".into(),
            table: Some(table),
            last_cell_start: Some(1),
        };
        assert!(wrapped_cell_continues(&word, &row, table, &words, &OcrRowIndex::new(&words), &[]).is_some());
        let competing_row = OutputRow {
            text: "continued".into(),
            table: Some(TableRowPosition { index: 1, ..table }),
            last_cell_start: Some(0),
        };
        assert!(
            wrapped_cell_continues(&word, &row, table, &words, &OcrRowIndex::new(&words), &[competing_row],).is_none()
        );
        words[2] = placed("continued", 0.0, 15.0);
        assert!(wrapped_cell_continues(&word, &row, table, &words, &OcrRowIndex::new(&words), &[]).is_none());
        words[2] = placed("unrelated", 40.0, 15.0);
        assert!(wrapped_cell_continues(&word, &row, table, &words, &OcrRowIndex::new(&words), &[]).is_none());
    }

    #[test]
    fn preserves_literal_if_its_prepend_or_either_output_has_ambiguous_multiplicity() {
        let proof = MatchedLayerOccurrences(
            ahash::AHashMap::from([(
                "repeat".into(),
                vec![occurrence("repeat a", 0), occurrence("repeat b", 0)],
            )]),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for (prepend, flat, structured) in [
            ("repeat repeat repeat", "repeat a\nrepeat b", "repeat a\nrepeat b"),
            ("repeat repeat", "repeat a", "repeat a\nrepeat b"),
            ("repeat repeat", "repeat a\nrepeat b", "repeat a"),
            ("repeat repeat", "repeat a\nrepeat b", "repeat a\nrepeat a"),
        ] {
            let mut first = LayerFirst {
                lines: prepend.into(),
                lost_glyphs: 0,
                misread_ocr_rows: Vec::new(),
            };
            let mut doc = flat_ocr_page_document(structured);
            doc.elements[0].bbox = Some(BoundingBox {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 100.0,
            });
            proof.trim_prepend(&mut first, flat, &doc, 1);
            assert_eq!(first.lines, prepend);
        }
    }
}

#[cfg(test)]
mod matched_layer_acceptance_tests;
