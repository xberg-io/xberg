//! Preflight dense worksheet allocation using the backend's streaming cell readers.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

use calamine::{DataRef, Reader};

use crate::extractors::security::{
    SecurityBudget, SecurityError, SecurityLimits, StringGrowthValidator, TableValidator,
};
use crate::{Result, XbergError};

mod legacy;
mod ods;
mod xlsb_framing;
mod zip_metadata;
pub(super) use legacy::{drop_skipped_sheets as drop_skipped_xls_sheets, validate_xls};
pub(super) use ods::{drop_skipped_sheets as drop_skipped_ods_sheets, validate_ods};
pub(super) use xlsb_framing::validate_xlsb_parts;
pub(super) use zip_metadata::{is_shared_strings_part, remove_unique_count, validate_xlsx_string_parts};

/// Worksheets whose cells did not fit in the workbook's remaining `max_table_cells` budget.
///
/// A sheet is identified by its position in the backend's sheet list. Skipping it keeps the
/// preflight's guarantee (the sheet is never materialized) while the rest of the workbook is
/// still extracted.
#[derive(Debug, Default)]
pub(super) struct SkippedSheets {
    indices: HashSet<usize>,
    max_cells: usize,
    /// Names of every ODS sheet in document order. Calamine keys ranges by name, so a skipped
    /// sheet sharing its name with another sheet is renamed in the rewritten input and reported
    /// here under its original name.
    names: Vec<String>,
    /// `(observed, max)` of the cell-limit error each skipped sheet raised.
    cell_limit_errors: HashMap<usize, (usize, usize)>,
    /// BOUNDSHEET records the XLS preflight saw; the reader must report exactly this many sheets.
    sheet_count: usize,
    /// XLSX sheets whose leading rows fit in the budget while the rest did not.
    truncated: HashMap<usize, TruncatedSheet>,
}

/// The leading rows of a worksheet that fit in the remaining `max_table_cells` budget.
#[derive(Debug, Clone, Copy)]
pub(super) struct TruncatedSheet {
    /// First row left out; every kept row lies above it.
    row_end: u32,
    /// Cells the preflight charged for the kept rows; the kept range never spans more.
    max_cells: usize,
    /// The configured `max_table_cells`, named in the warning.
    limit: usize,
}

impl SkippedSheets {
    fn new(limits: &SecurityLimits) -> Self {
        Self {
            indices: HashSet::new(),
            max_cells: limits.max_table_cells,
            names: Vec::new(),
            cell_limit_errors: HashMap::new(),
            sheet_count: 0,
            truncated: HashMap::new(),
        }
    }

    pub(super) fn truncated(&self, index: usize) -> Option<TruncatedSheet> {
        self.truncated.get(&index).copied()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    fn contains(&self, index: usize) -> bool {
        self.indices.contains(&index)
    }

    #[cfg(test)]
    fn sorted_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.indices.iter().copied().collect();
        indices.sort_unstable();
        indices
    }

    /// How many sheets carry each name.
    fn name_counts(&self) -> HashMap<&str, usize> {
        let mut counts = HashMap::new();
        for name in &self.names {
            *counts.entry(name.as_str()).or_insert(0) += 1;
        }
        counts
    }

    /// The original cell-limit error, if a skipped sheet shares its name with another sheet.
    ///
    /// Calamine keys XLS sheets by name, so the later of two equally named sheets overwrites the
    /// earlier one. `reader_names` must be the names the reader itself reports, because it decodes
    /// them (byte-order marks, code pages) in ways a preflight cannot mirror exactly.
    pub(super) fn name_collision_error(&self, reader_names: &[String]) -> Option<XbergError> {
        if self.indices.is_empty() {
            return None;
        }
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for name in reader_names {
            *counts.entry(name.as_str()).or_insert(0) += 1;
        }
        let misaligned = reader_names.len() != self.sheet_count;
        let culprit = self
            .indices
            .iter()
            .copied()
            .filter(|index| misaligned || reader_names.get(*index).is_none_or(|name| counts[name.as_str()] > 1))
            .min()?;
        let (cells, max) = self
            .cell_limit_errors
            .get(&culprit)
            .copied()
            .unwrap_or((self.max_cells.saturating_add(1), self.max_cells));
        Some(XbergError::from(SecurityError::TooManyCells { cells, max }))
    }

    /// The empty sheet standing in for sheet `index` named `name`, with its warning recorded,
    /// or `None` when that sheet was not skipped.
    pub(super) fn placeholder(
        &self,
        index: usize,
        name: &str,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> Option<crate::types::ExcelSheet> {
        if !self.contains(index) {
            return None;
        }
        let name = self
            .names
            .get(index)
            .filter(|name| !name.is_empty())
            .map_or(name, String::as_str);
        let message = format!(
            "Sheet '{name}' was skipped: its cells did not fit in the remaining table-cell budget \
             (`security_limits.max_table_cells` is {}); raise the limit to extract it",
            self.max_cells
        );
        crate::core::diagnostics::push_warning(warnings, "excel", message);
        Some(super::empty_excel_sheet(name))
    }
}

fn is_cell_limit(error: &XbergError) -> bool {
    matches!(
        error,
        XbergError::Security { source: Some(source), .. }
            if matches!(
                source.downcast_ref::<SecurityError>(),
                Some(SecurityError::TooManyCells { .. })
            )
    )
}

fn cell_limit_counts(error: &XbergError) -> Option<(usize, usize)> {
    match error {
        XbergError::Security {
            source: Some(source), ..
        } => match source.downcast_ref::<SecurityError>() {
            Some(SecurityError::TooManyCells { cells, max }) => Some((*cells, *max)),
            _ => None,
        },
        _ => None,
    }
}

/// The part of a [`SecurityBudget`] that a skipped sheet gives back: its table-cell and text
/// charges. The iteration and depth counters stay monotonic, so skipped sheets are not free
/// against `max_iterations` (sheets can alias one oversized part and be preflighted repeatedly).
struct Charges {
    table: TableValidator,
    growth: StringGrowthValidator,
}

impl Charges {
    fn of(budget: &SecurityBudget) -> Self {
        Self {
            table: budget.table.clone(),
            growth: budget.growth.clone(),
        }
    }

    fn restore(self, budget: &mut SecurityBudget) {
        budget.table = self.table;
        budget.growth = self.growth;
    }
}

/// Run one worksheet's preflight against `budget`. When the sheet alone trips the cell limit and
/// `recoverable` is set, its cell and text charges are rolled back and the sheet is recorded in
/// `skipped`; otherwise (and for any other failure) the error aborts the workbook.
fn preflight_sheet(
    index: usize,
    budget: &mut SecurityBudget,
    skipped: &mut SkippedSheets,
    recoverable: bool,
    sheet: impl FnOnce(&mut SecurityBudget) -> Result<()>,
) -> Result<()> {
    let before = Charges::of(budget);
    let result = sheet(budget);
    skip_on_cell_limit(index, before, budget, skipped, recoverable, result)
}

fn skip_on_cell_limit(
    index: usize,
    before: Charges,
    budget: &mut SecurityBudget,
    skipped: &mut SkippedSheets,
    recoverable: bool,
    result: Result<()>,
) -> Result<()> {
    match result {
        Err(error) if recoverable && is_cell_limit(&error) => {
            if let Some(counts) = cell_limit_counts(&error) {
                skipped.cell_limit_errors.insert(index, counts);
            }
            before.restore(budget);
            skipped.indices.insert(index);
            Ok(())
        }
        result => result,
    }
}

/// The row a streamed worksheet preflight is in, and the budget charged before that row.
#[derive(Default)]
struct RowProgress {
    row: Option<u32>,
    before_row: Option<Charges>,
    rows: usize,
}

impl RowProgress {
    fn enter(&mut self, row: u32, budget: &SecurityBudget) {
        if self.row != Some(row) {
            self.row = Some(row);
            self.before_row = Some(Charges::of(budget));
            self.rows += 1;
        }
    }

    /// The first row left out and the charges of the rows above it, when at least one row came before it.
    fn leading_rows(self) -> Option<(u32, Charges)> {
        if self.rows < 2 {
            return None;
        }
        self.row.zip(self.before_row)
    }
}

/// [`preflight_sheet`] for a worksheet streamed row by row: when the cell limit trips after at least one
/// complete row, the rows before the tripping one are kept and only the rest is charged back.
fn preflight_sheet_rows(
    index: usize,
    budget: &mut SecurityBudget,
    skipped: &mut SkippedSheets,
    sheet: impl FnOnce(&mut SecurityBudget, &mut RowProgress) -> Result<()>,
) -> Result<()> {
    let before = Charges::of(budget);
    let mut progress = RowProgress::default();
    let result = sheet(budget, &mut progress);
    if matches!(&result, Err(error) if is_cell_limit(error))
        && let Some((row_end, kept)) = progress.leading_rows()
    {
        let max_cells = kept.table.current_cells() - before.table.current_cells();
        kept.restore(budget);
        let limit = skipped.max_cells;
        skipped.truncated.insert(
            index,
            TruncatedSheet {
                row_end,
                max_cells,
                limit,
            },
        );
        return Ok(());
    }
    skip_on_cell_limit(index, before, budget, skipped, true, result)
}

/// Builds the kept leading rows of a truncated worksheet from its streamed cells, never spanning more
/// cells than the preflight charged for them.
pub(super) struct TruncatedRange {
    sheet: TruncatedSheet,
    cells: Vec<calamine::Cell<calamine::Data>>,
    kept: SheetBounds,
    /// Lowest kept row seen; rows can arrive out of order.
    first_row: Option<u32>,
}

impl TruncatedRange {
    pub(super) fn new(sheet: TruncatedSheet) -> Self {
        Self {
            sheet,
            cells: Vec::new(),
            kept: SheetBounds::default(),
            first_row: None,
        }
    }

    /// Adds a cell of the kept rows; returns `false` at the first cell past them, where reading stops.
    pub(super) fn push(&mut self, position: (u32, u32), value: &DataRef<'_>) -> bool {
        let row = position.0;
        if row >= self.sheet.row_end {
            return false;
        }
        self.first_row = Some(self.first_row.map_or(row, |first| first.min(row)));
        if matches!(value, DataRef::Empty) {
            return true;
        }
        let mut grown = self.kept.clone();
        if grown.grow(position).is_ok_and(|cells| cells <= self.sheet.max_cells) {
            self.kept = grown;
            self.cells
                .push(calamine::Cell::new(position, super::owned_xlsx_cell_value(value)));
        }
        true
    }

    pub(super) fn finish(
        self,
        name: &str,
        warnings: &mut Vec<crate::types::ProcessingWarning>,
    ) -> crate::types::ExcelSheet {
        let range = calamine::Range::from_sparse(self.cells);
        let sheet = super::process_sheet(name, &range, warnings);
        let first = u64::from(self.first_row.unwrap_or_default());
        let kept = range
            .end()
            .map_or(0, |(row, _)| (u64::from(row) + 1).saturating_sub(first));
        let message = format!(
            "Sheet '{name}' was truncated to its first {kept} {}: the remaining rows did not fit in the \
             remaining table-cell budget (`security_limits.max_table_cells` is {}); raise the limit to extract \
             the whole sheet",
            if kept == 1 { "row" } else { "rows" },
            self.sheet.limit
        );
        crate::core::diagnostics::push_warning(warnings, "excel", message);
        sheet
    }
}

/// Read the kept leading rows of an XLSX worksheet the preflight truncated. Reading stops at the first
/// row past them, so the rows left out are never read.
pub(super) fn read_truncated_xlsx_sheet<R: Read + Seek>(
    workbook: &mut calamine::Xlsx<R>,
    name: &str,
    sheet: TruncatedSheet,
    warnings: &mut Vec<crate::types::ProcessingWarning>,
) -> Result<crate::types::ExcelSheet> {
    let mut reader = workbook
        .worksheet_cells_reader(name)
        .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?;
    let mut kept = TruncatedRange::new(sheet);
    while let Some(cell) = reader
        .next_cell()
        .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?
    {
        if !kept.push(cell.get_position(), cell.get_value()) {
            break;
        }
    }
    Ok(kept.finish(name, warnings))
}

#[derive(Default, Clone)]
struct SheetBounds {
    bounds: Option<(u32, u32, u32, u32)>,
    #[cfg(test)]
    charged_cells: usize,
}

impl SheetBounds {
    #[cfg(test)]
    fn account(&mut self, position: (u32, u32), budget: &mut SecurityBudget) -> Result<()> {
        budget.step()?;
        let cells = self.grow(position)?;
        budget.add_cells(cells - self.charged_cells)?;
        self.charged_cells = cells;
        Ok(())
    }

    fn grow(&mut self, position: (u32, u32)) -> Result<usize> {
        let (row, col) = position;
        let (row_min, row_max, col_min, col_max) = self.bounds.get_or_insert((row, row, col, col));
        *row_min = (*row_min).min(row);
        *row_max = (*row_max).max(row);
        *col_min = (*col_min).min(col);
        *col_max = (*col_max).max(col);
        let rows = u64::from(*row_max) - u64::from(*row_min) + 1;
        let cols = u64::from(*col_max) - u64::from(*col_min) + 1;
        let cells = rows
            .checked_mul(cols)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or_else(|| {
                XbergError::validation("Spreadsheet table dimensions overflow the supported size".to_string())
            })?;
        Ok(cells)
    }
}

#[derive(Default)]
struct SheetAllocation {
    values: SheetBounds,
    formulas: SheetBounds,
    charged_cells: usize,
}

impl SheetAllocation {
    fn value(&mut self, position: (u32, u32), budget: &mut SecurityBudget) -> Result<()> {
        budget.step()?;
        let count = self.values.grow(position)?;
        self.charge(count, budget)
    }

    fn formula(&mut self, position: (u32, u32), budget: &mut SecurityBudget) -> Result<()> {
        budget.step()?;
        let count = self.formulas.grow(position)?;
        self.charge(count, budget)
    }

    fn charge(&mut self, count: usize, budget: &mut SecurityBudget) -> Result<()> {
        // ~keep Calamine constructs separate value/formula ranges; never union their distinct origins.
        let next = self.charged_cells.max(count);
        budget.add_cells(next - self.charged_cells)?;
        self.charged_cells = next;
        Ok(())
    }
}

pub(super) fn validate_xlsx<R: Read + Seek>(
    workbook: &mut calamine::Xlsx<R>,
    limits: &SecurityLimits,
) -> Result<SkippedSheets> {
    let mut budget = SecurityBudget::from_limits(limits);
    let mut skipped = SkippedSheets::new(limits);
    for (index, name) in workbook.sheet_names().into_iter().enumerate() {
        preflight_sheet_rows(index, &mut budget, &mut skipped, |budget, progress| {
            validate_xlsx_sheet(workbook, &name, limits, budget, progress)
        })?;
    }
    Ok(skipped)
}

fn validate_xlsx_sheet<R: Read + Seek>(
    workbook: &mut calamine::Xlsx<R>,
    name: &str,
    limits: &SecurityLimits,
    budget: &mut SecurityBudget,
    progress: &mut RowProgress,
) -> Result<()> {
    let mut bounds = SheetAllocation::default();
    let mut shared_formula_capacity = 0usize;
    let mut formula_budget = SecurityBudget::from_limits(limits);
    let mut nonempty_shared = ahash::AHashSet::new();
    let mut reader = match workbook.worksheet_cells_reader(name) {
        Ok(reader) => reader,
        Err(calamine::XlsxError::NotAWorksheet(_) | calamine::XlsxError::WorksheetNotFound(_)) => return Ok(()),
        Err(error) => {
            return Err(XbergError::parsing(format!(
                "Failed to read worksheet '{name}': {error}"
            )));
        }
    };
    validate_cell_reserve(reader.dimensions(), false, limits)?;
    while let Some(cell) = reader
        .next_cell_with_formula_metadata()
        .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?
    {
        progress.enter(cell.pos.0, budget);
        if let Some(calamine::XlsxFormulaMetadata::Shared {
            shared_index,
            range: Some(_),
            ..
        }) = cell.formula.as_ref()
        {
            // ~keep Calamine0.37 expands this index into Vec<Option<SharedFormula>> during its later formula pass.
            let bytes = shared_index
                .checked_add(1)
                .and_then(|count| count.checked_mul(std::mem::size_of::<Option<(String, calamine::Dimensions)>>()))
                .ok_or_else(|| XbergError::validation("Shared formula capacity overflow".to_string()))?;
            if bytes > shared_formula_capacity {
                formula_budget.account_text(bytes - shared_formula_capacity)?;
                shared_formula_capacity = bytes;
            }
        }
        if !matches!(cell.value, DataRef::Empty) {
            bounds.value(cell.pos, budget)?;
            account_xlsx_cell_text(&cell.value, budget)?;
        }
        if formula_occupies_range(cell.formula.as_ref(), &mut nonempty_shared)? {
            bounds.formula(cell.pos, budget)?;
        }
    }
    Ok(())
}

fn account_xlsx_cell_text(value: &DataRef<'_>, budget: &mut SecurityBudget) -> Result<()> {
    let length = match value {
        DataRef::String(text) | DataRef::DateTimeIso(text) => text.len(),
        DataRef::SharedString(text) => text.len(),
        DataRef::DurationIso(text) => super::DURATION_PREFIX.len().saturating_add(text.len()),
        _ => super::format_cell_to_string(&super::owned_xlsx_cell_value(value)).len(),
    };
    budget.account_text(length)?;
    Ok(())
}

fn formula_occupies_range(
    formula: Option<&calamine::XlsxFormulaMetadata>,
    nonempty_shared: &mut ahash::AHashSet<usize>,
) -> Result<bool> {
    match formula {
        None => Ok(false),
        Some(calamine::XlsxFormulaMetadata::Normal { formula }) => Ok(!formula.is_empty()),
        Some(calamine::XlsxFormulaMetadata::Shared {
            shared_index, formula, ..
        }) => {
            if formula.is_empty() {
                nonempty_shared.remove(shared_index);
            } else {
                nonempty_shared.insert(*shared_index);
            }
            Ok(!formula.is_empty())
        }
        Some(calamine::XlsxFormulaMetadata::SharedDerived { shared_index }) => {
            Ok(nonempty_shared.contains(shared_index))
        }
        Some(_) => Err(XbergError::validation(
            "Unsupported XLSX formula metadata in allocation preflight".to_string(),
        )),
    }
}

pub(super) fn validate_xlsb<R: Read + Seek>(
    workbook: &mut calamine::Xlsb<R>,
    limits: &SecurityLimits,
) -> Result<SkippedSheets> {
    let mut budget = SecurityBudget::from_limits(limits);
    let mut skipped = SkippedSheets::new(limits);
    for (index, name) in workbook.sheet_names().into_iter().enumerate() {
        preflight_sheet(index, &mut budget, &mut skipped, true, |budget| {
            validate_xlsb_sheet(workbook, &name, limits, budget)
        })?;
    }
    Ok(skipped)
}

fn validate_xlsb_sheet<R: Read + Seek>(
    workbook: &mut calamine::Xlsb<R>,
    name: &str,
    limits: &SecurityLimits,
    budget: &mut SecurityBudget,
) -> Result<()> {
    let mut bounds = SheetAllocation::default();
    let mut reader = workbook
        .worksheet_cells_reader(name)
        .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?;
    validate_cell_reserve(reader.dimensions(), true, limits)?;
    while let Some(cell) = reader
        .next_cell()
        .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?
    {
        if !matches!(cell.get_value(), DataRef::Empty) {
            bounds.value(cell.get_position(), budget)?;
        }
    }
    drop(reader);
    // ~keep Formula metadata is optional: the later collector preserves cached values when this backend parser fails.
    let Ok(mut formulas) = workbook.worksheet_cells_reader(name) else {
        return Ok(());
    };
    while let Ok(Some(cell)) = formulas.next_formula() {
        if !cell.get_value().is_empty() {
            bounds.formula(cell.get_position(), budget)?;
        }
    }
    Ok(())
}

fn validate_cell_reserve(dimensions: calamine::Dimensions, xlsb: bool, limits: &SecurityLimits) -> Result<()> {
    // ~keep Calamine0.37 preallocates sparse cells from declared dimensions before discovering actual occupied ranges.
    const SPARSE_RESERVE_THRESHOLD: u64 = 100_000;
    const XLSB_FORMULA_RESERVE_CAP: u64 = 1_000_000;
    let rows = dimensions
        .end
        .0
        .checked_sub(dimensions.start.0)
        .and_then(|count| count.checked_add(1));
    let cols = dimensions
        .end
        .1
        .checked_sub(dimensions.start.1)
        .and_then(|count| count.checked_add(1));
    let cells = rows
        .zip(cols)
        .map(|(rows, cols)| u64::from(rows) * u64::from(cols))
        .ok_or_else(|| XbergError::validation("Spreadsheet declared dimensions overflow".to_string()))?;
    let values = if cells < SPARSE_RESERVE_THRESHOLD { cells } else { 0 };
    let formulas = if xlsb {
        cells.min(XLSB_FORMULA_RESERVE_CAP)
    } else {
        values
    };
    let value_bytes = values.checked_mul(std::mem::size_of::<calamine::Cell<DataRef<'static>>>() as u64);
    let formula_bytes = formulas.checked_mul(std::mem::size_of::<calamine::Cell<String>>() as u64);
    let bytes = value_bytes
        .zip(formula_bytes)
        .map(|(values, formulas)| values.max(formulas))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| XbergError::validation("Spreadsheet sparse cell capacity overflow".to_string()))?;
    if bytes > limits.max_content_size {
        return Err(XbergError::validation(
            "Spreadsheet sparse cell allocation exceeds the configured content size".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_bound_xlsb_formula_reserves_even_when_value_reserves_are_skipped_gh2296() {
        let dimensions = calamine::Dimensions::new((0, 0), (999, 999));
        let bytes = 1_000_000 * std::mem::size_of::<calamine::Cell<String>>();
        let low = SecurityLimits {
            max_content_size: bytes - 1,
            max_table_cells: 1,
            ..Default::default()
        };
        assert!(
            validate_cell_reserve(dimensions, true, &low)
                .unwrap_err()
                .to_string()
                .contains("sparse cell allocation")
        );
        let exact = SecurityLimits {
            max_content_size: bytes,
            max_table_cells: 1,
            ..Default::default()
        };
        validate_cell_reserve(dimensions, true, &exact).unwrap();
        validate_cell_reserve(
            dimensions,
            false,
            &SecurityLimits {
                max_content_size: 0,
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn should_reject_backend_overflowing_declared_dimensions_gh2296() {
        for dimensions in [
            calamine::Dimensions::new((1, 0), (0, 0)),
            calamine::Dimensions::new((0, 0), (u32::MAX, 0)),
        ] {
            assert!(
                validate_cell_reserve(dimensions, false, &SecurityLimits::default())
                    .unwrap_err()
                    .to_string()
                    .contains("declared dimensions overflow")
            );
        }
    }

    #[test]
    fn should_share_the_table_cell_budget_across_worksheet_bounds_gh2296() {
        let limits = SecurityLimits {
            max_table_cells: 5,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let mut first = SheetBounds::default();
        first.account((0, 0), &mut budget).unwrap();
        first.account((1, 1), &mut budget).unwrap();
        let mut second = SheetBounds::default();
        second.account((0, 0), &mut budget).unwrap();
        let error = second.account((0, 1), &mut budget).unwrap_err();
        assert!(error.to_string().contains("observed 6 cells"), "{error}");
    }

    #[test]
    fn should_reject_overflowing_worksheet_bounds_before_allocation_gh2296() {
        let limits = SecurityLimits {
            max_table_cells: usize::MAX,
            ..Default::default()
        };
        let mut budget = SecurityBudget::from_limits(&limits);
        let mut bounds = SheetBounds::default();
        bounds.account((0, 0), &mut budget).unwrap();
        let error = bounds.account((u32::MAX, u32::MAX), &mut budget).unwrap_err();
        assert!(error.to_string().contains("dimensions overflow"), "{error}");
    }
}
