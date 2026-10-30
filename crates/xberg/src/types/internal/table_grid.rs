#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
use crate::types::BoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, NATIVE_TABLE_GRID_ATTRIBUTE};
use crate::types::{Table, TableGrid};

type TableIdentity<'a> = (u32, Option<[u64; 4]>, &'a [Vec<String>]);
type GridCandidates<'a> = ahash::AHashMap<TableIdentity<'a>, Option<&'a TableGridSidecar>>;

#[derive(Debug, Clone)]
pub(crate) struct TableGridSidecar {
    pub(crate) table: Table,
    pub(crate) grid: TableGrid,
}

struct HeaderUpdate {
    index: usize,
    columns: Option<Vec<String>>,
}

fn identity(table: &Table) -> TableIdentity<'_> {
    let bbox = table.bounding_box.map(|bbox| {
        [
            bbox.x0.to_bits(),
            bbox.y0.to_bits(),
            bbox.x1.to_bits(),
            bbox.y1.to_bits(),
        ]
    });
    (table.page_number, bbox, &table.cells)
}

pub(crate) fn retained_grid(table: &Table, attributes: Option<&ahash::AHashMap<String, String>>) -> Option<TableGrid> {
    let encoded = attributes?.get(NATIVE_TABLE_GRID_ATTRIBUTE)?;
    match serde_json::from_str::<TableGrid>(encoded) {
        Ok(grid) => crate::extraction::derive::native_grid_matches_table(table, &grid).then_some(grid),
        Err(error) => {
            tracing::warn!(%error, "discarding invalid retained table geometry");
            None
        }
    }
}

fn matching_sources(sidecars: &[TableGridSidecar]) -> GridCandidates<'_> {
    let mut candidates = GridCandidates::default();
    for sidecar in sidecars {
        let candidate =
            crate::extraction::derive::native_grid_matches_table(&sidecar.table, &sidecar.grid).then_some(sidecar);
        // Multiple producers for one identity cannot prove which geometry survived deduplication. ~keep
        candidates
            .entry(identity(&sidecar.table))
            .and_modify(|value| *value = None)
            .or_insert(candidate);
    }
    candidates
}

#[cfg(feature = "pdf")]
pub(crate) fn attach_table_grids(
    document: &mut InternalDocument,
    sidecars: &[TableGridSidecar],
) -> serde_json::Result<()> {
    attach_table_grids_inner(document, sidecars, false)
}

#[cfg(all(feature = "layout-detection", any(feature = "ocr", feature = "ocr-wasm")))]
pub(crate) fn attach_recognized_table_grids(
    document: &mut InternalDocument,
    sidecars: &[TableGridSidecar],
) -> serde_json::Result<()> {
    attach_table_grids_inner(document, sidecars, true)
}

fn attach_table_grids_inner(
    document: &mut InternalDocument,
    sidecars: &[TableGridSidecar],
    update_headers: bool,
) -> serde_json::Result<()> {
    let candidates = matching_sources(sidecars);
    let mut final_counts = ahash::AHashMap::<TableIdentity<'_>, usize>::new();
    for table in &document.tables {
        *final_counts.entry(identity(table)).or_default() += 1;
    }
    let mut headers = Vec::new();
    for element in &mut document.elements {
        let ElementKind::Table { table_index } = element.kind else {
            continue;
        };
        let Some(table) = document.tables.get(table_index as usize) else {
            continue;
        };
        if final_counts.get(&identity(table)) != Some(&1)
            || element.page.is_some_and(|page| page != table.page_number)
            || retained_grid(table, element.attributes.as_ref()).is_some()
        {
            continue;
        }
        let Some(Some(source)) = candidates.get(&identity(table)) else {
            continue;
        };
        element.attributes.get_or_insert_with(Default::default).insert(
            NATIVE_TABLE_GRID_ATTRIBUTE.to_owned(),
            serde_json::to_string(&source.grid)?,
        );
        if update_headers {
            headers.push(HeaderUpdate {
                index: table_index as usize,
                columns: source.table.columns.clone(),
            });
        }
    }
    drop(final_counts);
    synchronize_headers(document, headers);
    Ok(())
}

fn synchronize_headers(document: &mut InternalDocument, updates: Vec<HeaderUpdate>) {
    if updates.is_empty() {
        return;
    }
    for update in &updates {
        document.tables[update.index].columns = update.columns.clone();
    }
    let updated = updates
        .iter()
        .map(|update| &document.tables[update.index])
        .collect::<Vec<_>>();
    if let Some(pages) = &mut document.prebuilt_pages {
        for table in pages.iter_mut().flat_map(|page| &mut page.tables) {
            if let Some(source) = updated.iter().find(|source| identity(source) == identity(table)) {
                std::sync::Arc::make_mut(table).columns = source.columns.clone();
            }
        }
    }
}

#[cfg(all(feature = "pdf", any(feature = "ocr", feature = "ocr-pipeline")))]
pub(crate) fn transform_retained_grid_bboxes(
    document: &mut InternalDocument,
    external_tables: &[Table],
    mut transform: impl FnMut(&mut BoundingBox),
) {
    for element in &mut document.elements {
        let ElementKind::Table { table_index } = element.kind else {
            continue;
        };
        let Some(table) = document
            .tables
            .get(table_index as usize)
            .or_else(|| external_tables.get(table_index as usize))
        else {
            continue;
        };
        let Some(mut grid) = retained_grid(table, element.attributes.as_ref()) else {
            continue;
        };
        for bbox in grid.cells.iter_mut().filter_map(|cell| cell.bbox.as_mut()) {
            transform(bbox);
        }
        match serde_json::to_string(&grid) {
            Ok(encoded) => {
                element
                    .attributes
                    .get_or_insert_with(Default::default)
                    .insert(NATIVE_TABLE_GRID_ATTRIBUTE.to_owned(), encoded);
            }
            Err(error) => {
                tracing::warn!(%error, "discarding retained table geometry after coordinate conversion failed");
                if let Some(attributes) = element.attributes.as_mut() {
                    attributes.remove(NATIVE_TABLE_GRID_ATTRIBUTE);
                }
            }
        }
    }
}
