/// Keep the accepted transcript's bytes outside complete, typed table blocks. ~keep
pub(super) fn render_transcript_tables_as_plain(
    accepted: &str,
    document: &crate::types::internal::InternalDocument,
    page: u32,
) -> Option<String> {
    let mut groups = ahash::AHashMap::<_, TranscriptTableGroup>::new();
    let blocks: Vec<_> = top_level_table_ranges(accepted)
        .into_iter()
        .map(|range| {
            let key = fallback_table_key(&accepted[range.clone()]);
            groups.entry(key.clone()).or_default().source_count += 1;
            (range, key)
        })
        .collect();
    record_typed_table_occurrences(document, page, &mut groups);
    splice_unambiguous_tables(accepted, blocks, &groups)
}

type TableCells = Vec<Vec<String>>;

#[derive(Clone, Eq, Hash, PartialEq)]
enum TableKey {
    Cells(TableCells),
    Exact(String),
}

#[derive(Default)]
struct TranscriptTableGroup {
    source_count: usize,
    typed_count: usize,
    rows: Option<String>,
    ambiguous: bool,
}

fn record_typed_table_occurrences(
    document: &crate::types::internal::InternalDocument,
    page: u32,
    groups: &mut ahash::AHashMap<TableKey, TranscriptTableGroup>,
) {
    use crate::rendering::common::{render_headerless_table_markdown, render_table_markdown, render_table_plain};
    use crate::types::internal::ElementKind;

    let mut seen = ahash::AHashSet::new();
    for element in &document.elements {
        let ElementKind::Table { table_index } = element.kind else {
            continue;
        };
        let Some(table) = document.tables.get(table_index as usize) else {
            continue;
        };
        if element.page != Some(page) || table.page_number != page || !seen.insert(table_index) {
            continue;
        }
        let headed = render_table_markdown(&table.cells);
        let headerless = render_headerless_table_markdown(&table.cells);
        let headed = fallback_table_key(&headed);
        let headerless = fallback_table_key(&headerless);
        let has_headed = groups.contains_key(&headed);
        let has_headerless = groups.contains_key(&headerless);
        if !has_headed && !has_headerless {
            continue;
        }
        let rows = render_table_plain(&table.cells);
        if rows.trim().is_empty() {
            continue;
        }
        if let Some(group) = groups.get_mut(&headed) {
            group.record(&rows, has_headerless && headed != headerless);
        }
        if headed != headerless
            && let Some(group) = groups.get_mut(&headerless)
        {
            group.record(&rows, has_headed);
        }
    }
}

impl TranscriptTableGroup {
    fn record(&mut self, rows: &str, competing_source: bool) {
        self.typed_count += 1;
        self.ambiguous |= competing_source || self.rows.as_deref().is_some_and(|prior| prior != rows);
        if self.rows.is_none() {
            self.rows = Some(rows.to_owned());
        }
    }
}

fn fallback_table_key(source: &str) -> TableKey {
    let mut cells = transcript_table_key(source);
    let Some(columns) = cells.first().map(Vec::len) else {
        return TableKey::Exact(source.trim().to_owned());
    };
    // ~keep The shared writer can emit even-parity escaped pipes. Exact source
    // identity is required when the raw row delimiters disagree with the GFM header.
    let separator_columns = source
        .lines()
        .nth(1)
        .map(|row| row.trim().trim_matches('|').split('|').count());
    if separator_columns != Some(columns) || cells.iter().any(|row| row.len() > columns) {
        return TableKey::Exact(source.trim().to_owned());
    }
    for row in &mut cells {
        row.resize(columns, String::new());
    }
    TableKey::Cells(cells)
}

fn transcript_table_key(source: &str) -> TableCells {
    source
        .lines()
        .enumerate()
        .filter(|(index, _)| *index != 1)
        .map(|(_, row)| {
            let row = row.trim();
            let row = row.strip_prefix('|').unwrap_or(row);
            let mut cells = Vec::new();
            let mut start = 0;
            for (index, _) in row
                .match_indices('|')
                .filter(|&(index, _)| is_cell_delimiter(row, index))
            {
                cells.push(row[start..index].trim().to_owned());
                start = index + 1;
            }
            if start < row.len() || cells.is_empty() {
                cells.push(row[start..].trim().to_owned());
            }
            cells
        })
        .collect()
}

fn top_level_table_ranges(accepted: &str) -> Vec<std::ops::Range<usize>> {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    let mut ranges = Vec::new();
    let mut depth = 0;
    for (event, range) in Parser::new_ext(accepted, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0 && matches!(tag, Tag::Table(_)) {
                    ranges.push(range);
                }
                depth += 1;
            }
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    ranges
}

fn splice_unambiguous_tables(
    accepted: &str,
    blocks: Vec<(std::ops::Range<usize>, TableKey)>,
    groups: &ahash::AHashMap<TableKey, TranscriptTableGroup>,
) -> Option<String> {
    let mut plain = String::new();
    let mut offset = 0;
    for (range, key) in blocks {
        let group = &groups[&key];
        let Some(rows) = group.rows.as_deref() else {
            continue;
        };
        if group.ambiguous || group.source_count != group.typed_count {
            continue;
        }
        let source = &accepted[range.clone()];
        let body = source.trim_end_matches(['\r', '\n']);
        plain.push_str(&accepted[offset..range.start]);
        plain.push_str(rows.strip_suffix('\n').unwrap_or(rows));
        plain.push_str(&source[body.len()..]);
        offset = range.end;
    }
    if offset == 0 {
        return None;
    }
    plain.push_str(&accepted[offset..]);
    Some(plain)
}

pub(super) fn source_table_has_cells(source: &str, cells: &[Vec<String>]) -> bool {
    // ~keep Verify every source cell before relying on the typed table. GFM can omit excess
    // cells from its events, and a source <br> can represent either a line break or literal text.
    // ~keep The shared writer copies existing backslashes when escaping a cell pipe.
    // Its full serialization proves those cells even when the pipe has even parity.
    if source.trim() == crate::rendering::common::render_table_markdown(cells).trim() {
        return true;
    }
    let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
    let key = transcript_table_key(source);
    key.len() == cells.len()
        && key.iter().zip(cells).all(|(source_row, typed_row)| {
            source_row.len() <= columns
                && (0..columns).all(|column| {
                    let mut encoded = String::new();
                    if let Some(cell) = typed_row.get(column) {
                        crate::rendering::common::push_escaped_cell(&mut encoded, cell);
                    }
                    source_row.get(column).map_or("", String::as_str) == encoded.trim()
                })
        })
}

fn is_cell_delimiter(row: &str, index: usize) -> bool {
    row.as_bytes()[..index]
        .iter()
        .rev()
        .take_while(|&&byte| byte == b'\\')
        .count()
        .is_multiple_of(2)
}
