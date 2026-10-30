//! Preflight dense worksheet allocation using the backend's streaming cell readers.

use std::io::{Read, Seek};

use calamine::{DataRef, Reader};

use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::{Result, XbergError};

mod legacy;
mod ods;
mod xlsb_framing;
mod zip_metadata;
pub(super) use legacy::validate_xls;
pub(super) use ods::validate_ods;
pub(super) use xlsb_framing::validate_xlsb_parts;
pub(super) use zip_metadata::validate_xlsx_string_parts;

#[derive(Default)]
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

pub(super) fn validate_xlsx<R: Read + Seek>(workbook: &mut calamine::Xlsx<R>, limits: &SecurityLimits) -> Result<()> {
    let mut budget = SecurityBudget::from_limits(limits);
    for name in workbook.sheet_names() {
        let mut bounds = SheetAllocation::default();
        let mut shared_formula_capacity = 0usize;
        let mut formula_budget = SecurityBudget::from_limits(limits);
        let mut nonempty_shared = ahash::AHashSet::new();
        let mut reader = match workbook.worksheet_cells_reader(&name) {
            Ok(reader) => reader,
            Err(calamine::XlsxError::NotAWorksheet(_) | calamine::XlsxError::WorksheetNotFound(_)) => continue,
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
                bounds.value(cell.pos, &mut budget)?;
                account_xlsx_cell_text(&cell.value, &mut budget)?;
            }
            if formula_occupies_range(cell.formula.as_ref(), &mut nonempty_shared)? {
                bounds.formula(cell.pos, &mut budget)?;
            }
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

pub(super) fn validate_xlsb<R: Read + Seek>(workbook: &mut calamine::Xlsb<R>, limits: &SecurityLimits) -> Result<()> {
    let mut budget = SecurityBudget::from_limits(limits);
    for name in workbook.sheet_names() {
        let mut bounds = SheetAllocation::default();
        let mut reader = workbook
            .worksheet_cells_reader(&name)
            .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?;
        validate_cell_reserve(reader.dimensions(), true, limits)?;
        while let Some(cell) = reader
            .next_cell()
            .map_err(|error| XbergError::parsing(format!("Failed to read worksheet '{name}': {error}")))?
        {
            if !matches!(cell.get_value(), DataRef::Empty) {
                bounds.value(cell.get_position(), &mut budget)?;
            }
        }
        drop(reader);
        // ~keep Formula metadata is optional: the later collector preserves cached values when this backend parser fails.
        let Ok(mut formulas) = workbook.worksheet_cells_reader(&name) else {
            continue;
        };
        while let Ok(Some(cell)) = formulas.next_formula() {
            if !cell.get_value().is_empty() {
                bounds.formula(cell.get_position(), &mut budget)?;
            }
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
