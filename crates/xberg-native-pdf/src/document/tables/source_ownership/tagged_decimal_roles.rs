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

    fn tagged_cutoff_fixture(two_witnesses: bool, root_array: bool) -> Vec<u8> {
        use crate::document::tests::pdf_fixtures::build_catalog_test_pdf;
        use std::fmt::Write as _;

        let mut content = String::from("0 G 1 w\n");
        for y in [40, 60, 80, 100] {
            writeln!(content, "20 {y} m 170 {y} l S").unwrap();
        }
        for x in [20, 70, 120, 170] {
            writeln!(content, "{x} 40 m {x} 100 l S").unwrap();
        }
        let mut table = String::from("<< /S /Table /Pg 3 0 R /K [");
        for (row, cells) in [
            ["Group", "East", "West"],
            ["Version", "1.0", "2.0"],
            ["Body", "0.95", "0.85"],
        ]
        .iter()
        .enumerate()
        {
            table.push_str("<< /S /TR /K [");
            for (column, text) in cells.iter().enumerate() {
                let id = row * 3 + column;
                let role = if row < 2 { "TH" } else { "TD" };
                let x = 24 + 50 * column;
                let y = 85 - 20 * row;
                write!(table, "<< /S /{role} /K {id} >>").unwrap();
                writeln!(
                    content,
                    "/Span <</MCID {id}>> BDC BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({text}) Tj ET EMC"
                )
                .unwrap();
            }
            table.push_str("]>>");
        }
        table.push_str("]>>");
        let later = if two_witnesses { table.as_str() } else { "" };
        let root = format!("<< /S /Document /K [{table}{later}] >>");
        let entry = if root_array {
            format!("[<< /S /P >> {root}]")
        } else {
            root
        };
        let tree = format!("<< /Type /StructTreeRoot /K {entry} >>");
        let stream = format!("<< /Length {} >>\nstream\n{content}endstream", content.len());
        build_catalog_test_pdf(
            b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &[
                (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 130] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
                (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
                (5, stream.as_bytes()),
                (6, tree.as_bytes()),
            ],
        )
    }

    #[test]
    fn tagged_cutoff_discards_partial_final_root_and_allows_full_cache_retry() {
        use crate::structure::TableDetectionConfig;
        use crate::structure::with_structure_parse_cutoff;

        let config = TableDetectionConfig::strict();
        for root_array in [false, true] {
            let eligible = PdfDocument::from_bytes(tagged_cutoff_fixture(false, root_array)).unwrap();
            let [tables] = eligible.extract_tables_with_configs(0, [&config]).unwrap();
            assert_eq!(tables.len(), 1);
            assert!(
                tables[0].rows[1].cells[1].is_header,
                "the first witness must be eligible"
            );

            let bytes = tagged_cutoff_fixture(true, root_array);
            let complete = PdfDocument::from_bytes(bytes.clone()).unwrap();
            let [tables] = complete.extract_tables_with_configs(0, [&config]).unwrap();
            assert_eq!(tables.len(), 1);
            assert_eq!(tables[0].rows[1].cells[1].text, "1.0");
            assert!(
                !tables[0].rows[1].cells[1].is_header,
                "the second witness must conflict"
            );
            let mut ruled = [(tables, None)];
            let cold = PdfDocument::from_bytes(bytes).unwrap();
            assert!(cold.structure_tree_cache.lock_or_recover().is_none());

            // Document + Table + three TRs + nine cells, plus the optional leading root. ~keep
            with_structure_parse_cutoff(14 + usize::from(root_array), || {
                cold.retain_tagged_decimal_roles(0, &mut ruled);
                let bounded = cold.structure_tree_with_budget(Some(TAGGED_PARSE_BUDGET)).unwrap();
                assert!(bounded.is_none(), "bounded parser returned an incomplete tree");
            });
            assert!(
                !ruled[0].0[0].rows[1].cells[1].is_header,
                "partial proof promoted a header"
            );
            assert!(
                cold.structure_tree_cache.lock_or_recover().is_none(),
                "partial tree entered the cache"
            );

            let full = cold
                .struct_tree_trustworthy()
                .expect("full parse succeeds after cutoff");
            assert_eq!(borrowed_page_tables(&full.root_elements, 0).unwrap().len(), 2);
            cold.retain_tagged_decimal_roles(0, &mut ruled);
            assert!(!ruled[0].0[0].rows[1].cells[1].is_header);
        }
    }
}
