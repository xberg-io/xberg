//! Project explicit tagged decimal header roles onto an independently detected ruled grid.

use super::{PdfDocument, RuledSourceTables, Table};
use crate::cache::MutexExt;
use crate::document::ReadingOrder;
use crate::structure::table_extractor::{TableCell, extract_table_from_spans};
use crate::structure::types::McidScope;
use crate::structure::{StructChild, StructElem, StructTreeRoot, StructType};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

const MAX_TAGGED_TABLES: usize = 16;
const MAX_TAGGED_SPANS: usize = 4096;
const MAX_TAGGED_NODES: usize = 512;
const MAX_PAGE_LOOKUP_NODES: usize = 4096;
const TAGGED_PARSE_BUDGET: Duration = Duration::from_millis(250);

impl PdfDocument {
    pub(super) fn retain_tagged_decimal_roles(&self, page: usize, results: &mut [RuledSourceTables]) {
        if !results
            .iter()
            .flat_map(|(tables, _)| tables)
            .any(has_decimal_leaf_candidate)
        {
            return;
        }
        let Some(tree) = self.decimal_role_structure_tree() else {
            return;
        };
        let Some(elements) = borrowed_page_tables(&tree.root_elements, page as u32) else {
            return;
        };
        if elements.is_empty() {
            return;
        }
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
            .into_iter()
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

    fn decimal_role_structure_tree(&self) -> Option<Arc<StructTreeRoot>> {
        if self.mark_info().is_ok_and(|mark| mark.suspects) {
            return None;
        }
        if let Some(cached) = self.structure_tree_cache.lock_or_recover().as_ref() {
            return cached.clone();
        }
        let parsed = Arc::new(self.structure_tree_with_budget(Some(TAGGED_PARSE_BUDGET)).ok()??);
        // A budget miss is local to this optional proof; other readers may still need the tree. ~keep
        self.structure_tree_cache
            .lock_or_recover()
            .get_or_insert_with(|| Some(parsed))
            .clone()
    }
}

// Charge children before adding them to the stack. A partial traversal cannot prove
// that a seemingly matching table is unique. Borrow elements from the cached tree. ~keep
fn borrowed_page_tables(roots: &[StructElem], page: u32) -> Option<Vec<&StructElem>> {
    let mut remaining = MAX_PAGE_LOOKUP_NODES.checked_sub(roots.len())?;
    let mut pending: Vec<_> = roots.iter().collect();
    let mut matches = Vec::new();
    while let Some(elem) = pending.pop() {
        if elem.struct_type == StructType::Table {
            if table_is_on_page(elem, page, &mut remaining)? {
                if matches.len() == MAX_TAGGED_TABLES {
                    return None;
                }
                matches.push(elem);
            }
            continue;
        }
        remaining = remaining.checked_sub(elem.children.len())?;
        pending.extend(child_elements(&elem.children));
    }
    Some(matches)
}

fn table_is_on_page(table: &StructElem, page: u32, remaining: &mut usize) -> Option<bool> {
    let mut on_page = false;
    let mut size = 1;
    let mut pending = vec![table];
    while let Some(elem) = pending.pop() {
        *remaining = remaining.checked_sub(elem.children.len())?;
        size += elem.children.len();
        on_page |= elem.page == Some(page)
            || elem
                .children
                .iter()
                .any(|child| matches!(child, StructChild::MarkedContentRef { page: actual, .. } if *actual == page));
        if on_page && size > MAX_TAGGED_NODES {
            return None;
        }
        pending.extend(child_elements(&elem.children));
    }
    Some(on_page)
}

fn child_elements(children: &[StructChild]) -> impl Iterator<Item = &StructElem> {
    children.iter().filter_map(|child| match child {
        StructChild::StructElem(elem) => Some(elem.as_ref()),
        _ => None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tagged_table(page: u32, children: usize) -> StructElem {
        let mut table = StructElem::new(StructType::Table);
        table.children = (0..children)
            .map(|mcid| StructChild::MarkedContentRef {
                mcid: mcid as u32,
                page,
                scope: McidScope::Page(page),
            })
            .collect();
        table
    }

    #[test]
    fn tagged_lookup_charges_off_page_work_without_claiming_off_page_tables() {
        let roots = [tagged_table(1, MAX_TAGGED_NODES), tagged_table(0, 1)];
        let page = borrowed_page_tables(&roots, 0).unwrap();
        assert!(matches!(page.as_slice(), [table] if std::ptr::eq(*table, &roots[1])));
        assert!(borrowed_page_tables(&roots, 1).is_none());
    }

    #[test]
    fn tagged_lookup_discards_matches_on_global_exhaustion() {
        let roots = [tagged_table(1, MAX_PAGE_LOOKUP_NODES), tagged_table(0, 1)];
        assert!(borrowed_page_tables(&roots, 0).is_none());
    }
}
