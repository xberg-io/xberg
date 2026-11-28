//! Formula cells whose saved result is unavailable (xberg-io/xberg#3442).
//!
//! A spreadsheet cell stores a formula plus the result the producing application last
//! calculated. When that result is missing or empty, calamine yields an empty value, and the
//! extractor used to drop the cell, trimming leading blanks off the table so the cell looked
//! deliberately empty. Such a cell is shown as its formula text with the leading `=` Excel
//! displays, and a warning counts the cells per sheet. Formulas are never evaluated; a cell with a
//! saved result, including a saved empty string, keeps that result.

use std::fmt::Display;
use std::io::{Read, Seek};

use calamine::{Cell, Data, Range, Reader};

use super::MAX_METADATA_ENTRIES_PER_SHEET;
use crate::core::diagnostics::push_warning;
use crate::types::ProcessingWarning;

/// How a sheet's formula cells without a saved result are handled while it is read.
#[derive(Clone, Copy, Default)]
pub(super) struct SheetFormulas<'a> {
    /// Whether such cells are output as formula text (`false` when the preflight suppressed them).
    pub(super) show: bool,
    /// The sheet's formulas, read once for the dense path.
    pub(super) range: Option<&'a Range<String>>,
}

impl<'a> SheetFormulas<'a> {
    pub(super) fn new(show: bool, range: Option<&'a Range<String>>) -> Self {
        Self { show, range }
    }
}

/// The text a cell with formula `formula` and no saved result is shown as: `=<formula>`.
///
/// ODS stores formulas behind a namespace prefix (`of:=SUM([.A1])`); the prefix is dropped.
pub(super) fn unavailable_formula_value(formula: &str) -> Data {
    let expression = match formula.split_once(":=") {
        Some((namespace, rest)) if !namespace.is_empty() && namespace.bytes().all(|b| b.is_ascii_alphanumeric()) => {
            rest
        }
        _ => formula.strip_prefix('=').unwrap_or(formula),
    };
    Data::String(format!("={expression}"))
}

/// Warn that `count` formula cells of `sheet_name` have no saved result and are shown as formula text.
pub(super) fn warn_unavailable_formulas(warnings: &mut Vec<ProcessingWarning>, sheet_name: &str, count: usize) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "cell" } else { "cells" };
    push_warning(
        warnings,
        "excel",
        format!(
            "Sheet '{sheet_name}' has {count} formula {noun} with no saved result; shown as the formula text \
             prefixed with '=' (formulas are not evaluated)"
        ),
    );
}

/// Warn that `count` formula cells of `sheet_name` have no saved result and were left empty because
/// showing their text would exceed the content-size limit.
pub(super) fn warn_suppressed_formulas(warnings: &mut Vec<ProcessingWarning>, sheet_name: &str, count: usize) {
    let noun = if count == 1 { "cell" } else { "cells" };
    push_warning(
        warnings,
        "excel",
        format!(
            "Sheet '{sheet_name}' has {count} formula {noun} with no saved result that were left empty: their \
             formula text would exceed the content size limit (`security_limits.max_content_size`); the values \
             of the other cells are kept"
        ),
    );
}

/// Read a sheet's formulas once, for both the formula text shown in place of a missing result and
/// the `formulas_<sheet>` metadata. An error is the backend's message, for [`warn_unreadable_formulas`].
pub(super) fn read_sheet_formulas<RS, R>(workbook: &mut R, sheet_name: &str) -> Result<Range<String>, String>
where
    RS: Read + Seek,
    R: Reader<RS>,
    R::Error: Display,
{
    workbook
        .worksheet_formula(sheet_name)
        .map_err(|error| error.to_string())
}

/// Warn that a sheet's formulas could not be read, so cells with no saved result may appear empty.
pub(super) fn warn_unreadable_formulas(warnings: &mut Vec<ProcessingWarning>, sheet_name: &str, error: &str) {
    push_warning(
        warnings,
        "excel",
        format!(
            "Sheet '{sheet_name}' formulas could not be read ({error}); formula cells with no saved result may \
             appear empty"
        ),
    );
}

/// Show every formula cell of `range` that has no saved result as its formula text.
///
/// The value range is trimmed to its non-empty cells, so adding the formula cells can widen it.
/// Every format's preflight charges that widened box against the table-cell budget.
pub(super) fn show_unavailable_formulas(
    mut range: Range<Data>,
    formulas: &Range<String>,
    sheet_name: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Range<Data> {
    let Some(formula_start) = formulas.start() else {
        return range;
    };

    let mut missing = Vec::new();
    for (row, col, text) in formulas.cells() {
        if text.is_empty() {
            continue;
        }
        let position = (formula_start.0 + row as u32, formula_start.1 + col as u32);
        if matches!(range.get_value(position), None | Some(Data::Empty)) {
            missing.push(Cell::new(position, unavailable_formula_value(text)));
        }
    }
    if missing.is_empty() {
        return range;
    }
    warn_unavailable_formulas(warnings, sheet_name, missing.len());

    let (mut lo, mut hi) = missing.iter().fold(
        (missing[0].get_position(), missing[0].get_position()),
        |(lo, hi), cell| {
            let (row, col) = cell.get_position();
            ((lo.0.min(row), lo.1.min(col)), (hi.0.max(row), hi.1.max(col)))
        },
    );
    let old = range.start().zip(range.end());
    if let Some((start, end)) = old {
        lo = (lo.0.min(start.0), lo.1.min(start.1));
        hi = (hi.0.max(end.0), hi.1.max(end.1));
    }

    // Move the existing cells into a range that also covers the formula cells.
    let mut widened = Range::new(lo, hi);
    if let Some((start, _)) = old {
        let (rows, cols) = range.get_size();
        for row in 0..rows {
            for col in 0..cols {
                let value = std::mem::take(&mut range[(row, col)]);
                if value != Data::Empty {
                    widened[((start.0 - lo.0) as usize + row, (start.1 - lo.1) as usize + col)] = value;
                }
            }
        }
    }
    for cell in missing {
        let (row, col) = cell.get_position();
        widened[((row - lo.0) as usize, (col - lo.1) as usize)] = cell.get_value().clone();
    }
    widened
}

/// Formula entries as `"<cell_ref>=<formula>"` joined by `"; "`, at most
/// [`MAX_METADATA_ENTRIES_PER_SHEET`] of them, with each reference relative to `origin` (the top-left
/// formula cell, matching the convention of the other cell references this extractor reports).
/// `None` when there are no formulas.
pub(super) fn format_formula_entries<'a>(
    entries: impl Iterator<Item = ((u32, u32), &'a str)>,
    origin: (u32, u32),
) -> Option<String> {
    let joined = entries
        .filter(|(_, formula)| !formula.is_empty())
        .take(MAX_METADATA_ENTRIES_PER_SHEET)
        .map(|((row, col), formula)| format!("{}={formula}", super::cell_reference(row - origin.0, col - origin.1)))
        .collect::<Vec<_>>()
        .join("; ");
    (!joined.is_empty()).then_some(joined)
}

/// The sheet's non-empty formulas, as [`format_formula_entries`] lists them.
pub(super) fn format_sheet_formulas(formulas: &Range<String>) -> Option<String> {
    let origin = formulas.start()?;
    format_formula_entries(
        formulas
            .cells()
            .map(|(row, col, formula)| ((origin.0 + row as u32, origin.1 + col as u32), formula.as_str())),
        origin,
    )
}
