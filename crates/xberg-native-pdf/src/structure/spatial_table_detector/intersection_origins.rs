use std::collections::{BTreeMap, HashSet};
use std::ops::Range;

use super::{
    IntersectionCell, SNAP_TOL, Table, TableCell, TableDetectionConfig, TableRow, TextSpan, extract_cell_text,
};
use crate::geometry::Rect;

pub(super) struct AssignedGrid {
    pub rows: Vec<TableRow>,
    pub indices: Vec<Vec<Vec<usize>>>,
    pub origins: Vec<AssignedRow>,
}

pub(super) struct AssignedRow {
    pub band: usize,
    pub columns: Vec<Range<usize>>,
}

#[derive(Clone)]
struct CellOrigin {
    columns: Range<usize>,
    indices: Vec<usize>,
}

#[derive(Clone)]
pub(super) struct RowOrigin {
    group: usize,
    band: usize,
    emitted: usize,
    cells: Vec<Option<CellOrigin>>,
}

pub(super) struct IntersectionTable {
    pub table: Table,
    pub origins: Option<Vec<RowOrigin>>,
}

impl IntersectionTable {
    pub(super) fn validate_shape(&mut self) {
        if self.origins.as_ref().is_some_and(|rows| {
            rows.len() != self.table.rows.len()
                || rows
                    .iter()
                    .zip(&self.table.rows)
                    .any(|(origin, row)| origin.cells.len() != row.cells.len())
        }) {
            self.origins = None;
        }
    }
    #[cfg(test)]
    pub fn untracked(table: Table) -> Self {
        Self { table, origins: None }
    }
}

struct Footprint {
    bands: Range<usize>,
    columns: Range<usize>,
    bbox: Rect,
    indices: Vec<usize>,
    aligned: bool,
}

pub(super) struct GroupOrigin {
    footprints: Vec<Footprint>,
    owners: Vec<Vec<Option<usize>>>,
    emissions: Vec<usize>,
    valid: bool,
}

fn boundary(value: f32, boundaries: &[f32]) -> Option<usize> {
    boundaries
        .iter()
        .position(|&candidate| (candidate - value).abs() <= SNAP_TOL)
}

fn finite_rect(rect: Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height, rect.right(), rect.bottom()]
        .into_iter()
        .all(f32::is_finite)
        && rect.width > 0.0
        && rect.height > 0.0
}

impl GroupOrigin {
    pub(super) fn from_grid(
        cells: &[IntersectionCell],
        xs: &[f32],
        ys: &[f32],
        grid: &AssignedGrid,
        physical: bool,
    ) -> Option<Self> {
        if !physical
            || xs.len() < 2
            || ys.len() < 2
            || grid.rows.len() != grid.origins.len()
            || grid.rows.len() != grid.indices.len()
        {
            return None;
        }
        let mut plan = Self {
            footprints: Vec::new(),
            owners: vec![vec![None; xs.len() - 1]; ys.len() - 1],
            emissions: vec![0; ys.len() - 1],
            valid: true,
        };
        for cell in cells {
            let columns = boundary(cell.x1, xs)?..boundary(cell.x2, xs)?;
            let bands = boundary(cell.y1, ys)?..boundary(cell.y2, ys)?;
            let bbox = Rect::new(cell.x1, cell.y1, cell.x2 - cell.x1, cell.y2 - cell.y1);
            if columns.is_empty() || bands.is_empty() || !finite_rect(bbox) {
                return None;
            }
            let id = plan.footprints.len();
            for row in &mut plan.owners[bands.clone()] {
                for owner in &mut row[columns.clone()] {
                    if owner.replace(id).is_some() {
                        return None;
                    }
                }
            }
            plan.footprints.push(Footprint {
                bands,
                columns,
                bbox,
                indices: Vec::new(),
                aligned: true,
            });
        }
        plan.bind_assignment(grid)?;
        Some(plan)
    }

    fn owner(&self, band: usize, columns: &Range<usize>) -> Option<usize> {
        let row = self.owners.get(band)?;
        let cells = row.get(columns.clone())?;
        let owner = (*cells.first()?)?;
        (cells.iter().all(|&id| id == Some(owner)) && self.footprints[owner].columns == *columns).then_some(owner)
    }

    fn bind_assignment(&mut self, grid: &AssignedGrid) -> Option<()> {
        for (origin, indices) in grid.origins.iter().zip(&grid.indices) {
            if origin.columns.len() != indices.len() {
                return None;
            }
            for (columns, indices) in origin.columns.iter().zip(indices) {
                if let Some(id) = self.owner(origin.band, columns) {
                    self.footprints[id].indices.extend(indices);
                } else {
                    for id in self.owners.get(origin.band)?.get(columns.clone())?.iter().flatten() {
                        self.footprints[*id].aligned = false;
                    }
                }
            }
        }
        Some(())
    }

    pub(super) fn record_emissions(&mut self, rows: &[RowOrigin]) {
        for row in rows {
            let Some(count) = self.emissions.get_mut(row.band) else {
                self.valid = false;
                return;
            };
            *count = (*count).max(row.emitted + 1);
        }
    }
}

pub(super) fn record_emission(
    out: &mut Vec<RowOrigin>,
    assignment: Option<(usize, &[AssignedRow])>,
    row: usize,
    emitted: usize,
    indices: &[Vec<usize>],
) {
    let Some((group, assigned)) = assignment else {
        return;
    };
    let Some(source) = assigned.get(row) else {
        return;
    };
    if source.columns.len() != indices.len() {
        return;
    }
    out.push(RowOrigin {
        group,
        band: source.band,
        emitted,
        cells: source
            .columns
            .iter()
            .zip(indices)
            .map(|(columns, indices)| {
                Some(CellOrigin {
                    columns: columns.clone(),
                    indices: indices.clone(),
                })
            })
            .collect(),
    });
}

pub(super) fn pad_origins(origins: &mut Option<Vec<RowOrigin>>, count: usize) {
    if let Some(rows) = origins {
        for row in rows {
            row.cells.extend((0..count).map(|_| None));
        }
    }
}

pub(super) fn append_origins(previous: &mut Option<Vec<RowOrigin>>, incoming: Option<Vec<RowOrigin>>) {
    match (previous.as_mut(), incoming) {
        (Some(previous), Some(incoming)) => previous.extend(incoming),
        _ => *previous = None,
    }
}

type Location = (usize, usize);
struct Change {
    locations: Vec<Location>,
    text: String,
    mcids: Vec<u32>,
    bbox: Rect,
    colspan: u32,
    rowspan: u32,
    is_header: bool,
}

struct CandidateView<'a> {
    table: &'a Table,
    origins: &'a [RowOrigin],
    groups: &'a [Option<GroupOrigin>],
    spans: &'a [TextSpan],
    emissions: BTreeMap<(usize, usize), Vec<(usize, usize)>>,
    cells: BTreeMap<(usize, usize), Vec<Location>>,
}

impl<'a> CandidateView<'a> {
    fn new(
        table: &'a Table,
        origins: &'a [RowOrigin],
        groups: &'a [Option<GroupOrigin>],
        spans: &'a [TextSpan],
    ) -> Option<Self> {
        if table.rows.len() != origins.len()
            || table
                .rows
                .iter()
                .flat_map(|row| &row.cells)
                .flat_map(|cell| &cell.spans)
                .any(|span| span.rotation_degrees != 0.0)
        {
            return None;
        }
        let mut view = Self {
            table,
            origins,
            groups,
            spans,
            emissions: BTreeMap::new(),
            cells: BTreeMap::new(),
        };
        let mut seen = HashSet::new();
        for (row_index, (row, origin)) in table.rows.iter().zip(origins).enumerate() {
            let group = groups.get(origin.group)?.as_ref().filter(|group| group.valid)?;
            if row.cells.len() != origin.cells.len() {
                return None;
            }
            view.emissions
                .entry((origin.group, origin.band))
                .or_default()
                .push((row_index, origin.emitted));
            for (cell_index, cell) in origin.cells.iter().enumerate() {
                let Some(cell) = cell else {
                    continue;
                };
                if cell
                    .indices
                    .iter()
                    .any(|index| *index >= spans.len() || !seen.insert(*index))
                {
                    return None;
                }
                if let Some(owner) = group.owner(origin.band, &cell.columns) {
                    view.cells
                        .entry((origin.group, owner))
                        .or_default()
                        .push((row_index, cell_index));
                }
            }
        }
        Some(view)
    }

    fn footprint_rows(&self, group: usize, footprint: &Footprint) -> Option<Vec<usize>> {
        let plan = self.groups.get(group)?.as_ref()?;
        let mut rows = Vec::new();
        for band in footprint.bands.clone().rev() {
            let emitted = self.emissions.get(&(group, band))?;
            if emitted.len() != plan.emissions[band] || emitted.is_empty() {
                return None;
            }
            for (ordinal, &(row, original)) in emitted.iter().enumerate() {
                if ordinal != original {
                    return None;
                }
                rows.push(row);
            }
        }
        rows.windows(2).all(|pair| pair[0] + 1 == pair[1]).then_some(rows)
    }

    fn change(&self, group: usize, id: usize, locations: &[Location]) -> Option<Change> {
        let footprint = &self.groups.get(group)?.as_ref()?.footprints[id];
        if !footprint.aligned || footprint.bands.len() < 2 || footprint.indices.is_empty() {
            return None;
        }
        let rows = self.footprint_rows(group, footprint)?;
        if rows.len() != locations.len() || !rows.iter().zip(locations).all(|(row, location)| *row == location.0) {
            return None;
        }
        let first = self.cell(*locations.first()?);
        let row_header = self.table.rows[locations[0].0].is_header;
        let mut indices = Vec::new();
        for &location in locations {
            let cell = self.cell(location);
            let origin = self.origins[location.0].cells[location.1].as_ref()?;
            if cell.rowspan != 1
                || cell.colspan as usize != footprint.columns.len()
                || cell.spans.len() != origin.indices.len()
                || cell.is_header != first.is_header
                || self.table.rows[location.0].is_header != row_header
                || !cell.bbox.is_some_and(finite_rect)
                || cell.text != extract_cell_text(&origin.indices, self.spans)
                || cell.mcids != mcids(&origin.indices, self.spans)
            {
                return None;
            }
            indices.extend(&origin.indices);
        }
        let mut expected = footprint.indices.clone();
        let mut actual = indices.clone();
        expected.sort_unstable();
        actual.sort_unstable();
        if actual != expected {
            return None;
        }
        Some(Change {
            locations: locations.to_vec(),
            text: extract_cell_text(&indices, self.spans),
            mcids: mcids(&indices, self.spans),
            bbox: footprint.bbox,
            colspan: u32::try_from(footprint.columns.len()).ok()?,
            rowspan: u32::try_from(rows.len()).ok()?,
            is_header: first.is_header,
        })
    }

    fn cell(&self, (row, cell): Location) -> &TableCell {
        &self.table.rows[row].cells[cell]
    }
}

fn mcids(indices: &[usize], spans: &[TextSpan]) -> Vec<u32> {
    indices.iter().filter_map(|&index| spans[index].mcid).collect()
}

fn admitted_changes(view: &CandidateView<'_>, config: &TableDetectionConfig) -> Option<Vec<Change>> {
    let changes: Vec<_> = view
        .cells
        .iter()
        .filter_map(|(&(group, id), locations)| view.change(group, id, locations))
        .collect();
    let mut filled = view
        .table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| !cell.text.is_empty())
        .count();
    for change in &changes {
        let previous = change
            .locations
            .iter()
            .filter(|&&location| !view.cell(location).text.is_empty())
            .count();
        filled = filled
            .checked_sub(previous)?
            .checked_add(usize::from(!change.text.is_empty()))?;
    }
    (filled >= config.min_table_cells).then_some(changes)
}

pub(super) fn normalize(
    mut candidate: IntersectionTable,
    groups: &[Option<GroupOrigin>],
    spans: &[TextSpan],
    config: &TableDetectionConfig,
) -> Table {
    let changes = candidate
        .origins
        .as_ref()
        .and_then(|origins| CandidateView::new(&candidate.table, origins, groups, spans))
        .and_then(|view| admitted_changes(&view, config));
    let Some(changes) = changes else {
        return candidate.table;
    };
    let mut covered = HashSet::new();
    for change in changes {
        let mut retained = Vec::new();
        for &(row, cell) in &change.locations {
            retained.append(&mut candidate.table.rows[row].cells[cell].spans);
        }
        let (row, cell) = change.locations[0];
        candidate.table.rows[row].cells[cell] = TableCell {
            text: change.text,
            spans: retained,
            colspan: change.colspan,
            rowspan: change.rowspan,
            mcids: change.mcids,
            bbox: Some(change.bbox),
            is_header: change.is_header,
        };
        covered.extend(change.locations.into_iter().skip(1));
    }
    for (row_index, row) in candidate.table.rows.iter_mut().enumerate() {
        row.cells = std::mem::take(&mut row.cells)
            .into_iter()
            .enumerate()
            .filter_map(|(cell, value)| (!covered.contains(&(row_index, cell))).then_some(value))
            .collect();
    }
    candidate.table
}

#[cfg(test)]
#[path = "intersection_origin_tests.rs"]
mod tests;
