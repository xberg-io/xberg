#[cfg(test)]
use super::flat_ocr_page_document;
use super::{
    LayerFirst, LineAt, LineBox, MergedOcrAssets, OcrRowIndex, processed_ocr_layout_dimensions,
    reindex_structured_ocr_page, resolved_ocr_correction_degrees,
};

/// Exact literals whose complete set of native occurrences has a one-to-one OCR match.
pub(in crate::extractors::pdf::ocr) struct MatchedLayerOccurrences(ahash::AHashMap<String, Vec<OutputOccurrence>>);

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
        (!matched.is_empty()).then_some(Self(matched))
    }

    pub(in crate::extractors::pdf::ocr) fn apply_prepend(
        &self,
        first: &mut LayerFirst,
        flat: &mut String,
        structured: &mut crate::types::internal::InternalDocument,
        page: u32,
    ) {
        self.trim_prepend(first, flat, structured, page);
        use crate::types::{
            OcrElementLevel,
            internal::{ElementKind, InternalElement},
        };

        if !first.lines.trim().is_empty() {
            let element = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                first.lines.as_str(),
                0,
            );
            super::insert_layer_elements(structured, None, vec![element]);
        }
        *flat = super::text_with_layer_first(flat, first);
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
        let structured_text = crate::rendering::render_plain(&projection);
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
        first.lines = trim_rows(&first.lines, &mut available);
    }
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
        let proof = MatchedLayerOccurrences(ahash::AHashMap::from([
            ("PAND".into(), rows),
            ("changed".into(), vec![occurrence("changed Unmatched", 0)]),
            ("Unmatched".into(), vec![occurrence("Unmatched missing", 0)]),
        ]));
        let mut first = LayerFirst {
            lines: "PAND   native\nPAND\nchanged Unmatched pand".into(),
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
        let proof = MatchedLayerOccurrences(ahash::AHashMap::from([(
            "repeat".into(),
            vec![occurrence("repeat a", 0), occurrence("repeat b", 0)],
        )]));
        for (prepend, flat, structured) in [
            ("repeat repeat repeat", "repeat a\nrepeat b", "repeat a\nrepeat b"),
            ("repeat repeat", "repeat a", "repeat a\nrepeat b"),
            ("repeat repeat", "repeat a\nrepeat b", "repeat a"),
            ("repeat repeat", "repeat a\nrepeat b", "repeat a\nrepeat a"),
        ] {
            let mut first = LayerFirst { lines: prepend.into() };
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
#[path = "matched_layer_acceptance_tests.rs"]
mod matched_layer_acceptance_tests;
