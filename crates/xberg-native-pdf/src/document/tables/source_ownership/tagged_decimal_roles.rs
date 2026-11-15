//! Project explicit tagged decimal header roles onto an independently detected ruled grid.

use super::{PdfDocument, RuledSourceTables, Table};
use crate::cache::MutexExt;
use crate::document::ReadingOrder;
use crate::structure::table_extractor::{TableCell, extract_table_from_spans, find_table_elements_all_pages};
use crate::structure::types::McidScope;
use crate::structure::{StructChild, StructElem};
use std::collections::{HashMap, HashSet};

const MAX_TAGGED_TABLES: usize = 16;
const MAX_TAGGED_SPANS: usize = 4096;
const MAX_TAGGED_NODES: usize = 512;

impl PdfDocument {
    pub(super) fn retain_tagged_decimal_roles(&self, page: usize, results: &mut [RuledSourceTables]) {
        if !results
            .iter()
            .flat_map(|(tables, _)| tables)
            .any(has_decimal_leaf_candidate)
        {
            return;
        }
        let Some(tree) = self.struct_tree_trustworthy() else {
            return;
        };
        let elements = {
            let mut cache = self.table_elements_cache.lock_or_recover();
            let pages = cache.get_or_insert_with(|| find_table_elements_all_pages(&tree));
            let Some(elements) = pages.get(&(page as u32)) else {
                return;
            };
            if elements.len() > MAX_TAGGED_TABLES || !elements.iter().all(bounded_tag_tree) {
                return;
            }
            elements.clone()
        };
        let Ok(source) = self.extract_page_text_with_script_geometry(page, ReadingOrder::TopToBottom) else {
            return;
        };
        if source.spans.len() > MAX_TAGGED_SPANS {
            return;
        }
        let mut occurrences = HashMap::new();
        for span in &source.spans {
            if let Some(id) = span.mcid {
                let count = occurrences.entry(id).or_insert(0);
                // Reuse of a page MCID in a Form makes the word path's unscoped ID ambiguous. ~keep
                *count += if span.mcid_scope == Some(McidScope::Page(page as u32)) {
                    1
                } else {
                    2
                };
            }
        }
        let tagged: Vec<_> = elements
            .iter()
            .filter_map(|elem| extract_table_from_spans(elem, &source.spans).ok())
            .filter(has_decimal_leaf_candidate)
            .collect();
        for table in results
            .iter_mut()
            .flat_map(|(tables, _)| tables)
            .filter(|table| has_decimal_leaf_candidate(table))
        {
            let mut matching = tagged.iter().filter(|tagged| is_same_grid(table, tagged, &occurrences));
            if let Some(witness) = matching.next()
                && matching.next().is_none()
            {
                table.has_header = witness.has_header;
                for (row, tagged_row) in table.rows.iter_mut().zip(&witness.rows) {
                    row.is_header = tagged_row.is_header;
                    for (cell, tagged_cell) in row.cells.iter_mut().zip(&tagged_row.cells) {
                        cell.is_header = tagged_cell.is_header;
                    }
                }
            }
        }
    }
}

fn bounded_tag_tree(root: &StructElem) -> bool {
    let mut pending = vec![root];
    let mut remaining = MAX_TAGGED_NODES;
    while let Some(elem) = pending.pop() {
        let Some(next) = remaining.checked_sub(elem.children.len()) else {
            return false;
        };
        remaining = next;
        pending.extend(elem.children.iter().filter_map(|child| match child {
            StructChild::StructElem(elem) => Some(elem.as_ref()),
            _ => None,
        }));
    }
    true
}

fn has_decimal_leaf_candidate(table: &Table) -> bool {
    table.rows.len() >= 3
        && table.rows[1]
            .cells
            .iter()
            .filter(|cell| {
                cell.text.trim().split_once('.').is_some_and(|(whole, fraction)| {
                    !whole.is_empty()
                        && !fraction.is_empty()
                        && whole
                            .bytes()
                            .chain(fraction.bytes())
                            .all(|digit| digit.is_ascii_digit())
                })
            })
            .count()
            > table.col_count / 2
}

fn is_same_grid(ruled: &Table, tagged: &Table, occurrences: &HashMap<u32, usize>) -> bool {
    let mut claimed = HashSet::new();
    ruled.col_count == tagged.col_count
        && ruled.rows.len() == tagged.rows.len()
        && tagged.rows[1].cells.iter().all(|cell| cell.is_header)
        && tagged.rows[2..]
            .iter()
            .all(|row| !row.is_header && row.cells.iter().all(|cell| !cell.is_header))
        && ruled.rows.iter().zip(&tagged.rows).all(|(row, tagged_row)| {
            row.cells.len() == tagged_row.cells.len()
                && row.cells.iter().zip(&tagged_row.cells).all(|(cell, tag)| {
                    let [id] = tag.mcids.as_slice() else { return false };
                    occurrences.get(id) == Some(&1) && claimed.insert(*id) && same_marked_cell(cell, tag)
                })
        })
}

fn same_marked_cell(cell: &TableCell, tag: &TableCell) -> bool {
    if cell.colspan != 1
        || cell.rowspan != 1
        || tag.colspan != 1
        || tag.rowspan != 1
        || cell.text.trim() != tag.text.trim()
        || cell.mcids != tag.mcids
    {
        return false;
    }
    let ([word], [marked]) = (cell.spans.as_slice(), tag.spans.as_slice()) else {
        return false;
    };
    const GLYPH_BOX_ROUNDING: f32 = 0.01;
    word.text.trim() == marked.text.trim()
        && word.mcid == marked.mcid
        && word.mcid == tag.mcids.first().copied()
        && [
            word.bbox.left() - marked.bbox.left(),
            word.bbox.right() - marked.bbox.right(),
            word.bbox.y - marked.bbox.y,
            word.bbox.height - marked.bbox.height,
        ]
        .into_iter()
        .all(|delta| delta.abs() <= GLYPH_BOX_ROUNDING)
}
