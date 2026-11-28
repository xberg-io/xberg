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

use crate::core::diagnostics::push_warning;
use crate::types::ProcessingWarning;

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

/// Warn that `count` formula cells of `sheet_name` have no saved result.
///
/// `shown` is whether their formula text is in the sheet output; when it is not, the formulas are
/// still listed in the sheet's `formulas_<sheet>` metadata.
pub(super) fn warn_unavailable_formulas(
    warnings: &mut Vec<ProcessingWarning>,
    sheet_name: &str,
    count: usize,
    shown: bool,
) {
    if count == 0 {
        return;
    }
    let noun = if count == 1 { "cell" } else { "cells" };
    let message = if shown {
        format!(
            "Sheet '{sheet_name}' has {count} formula {noun} with no saved result; shown as the formula text \
             prefixed with '=' (formulas are not evaluated)"
        )
    } else {
        format!(
            "Sheet '{sheet_name}' has {count} formula {noun} with no saved result that could not be shown in the \
             table without exceeding the table-cell budget; see the 'formulas_{sheet_name}' metadata"
        )
    };
    push_warning(warnings, "excel", message);
}

fn area(start: (u32, u32), end: (u32, u32)) -> u64 {
    (u64::from(end.0 - start.0) + 1) * (u64::from(end.1 - start.1) + 1)
}

/// Show every formula cell of `range` that has no saved result as its formula text.
///
/// The value range is trimmed to its non-empty cells, so adding the formula cells can widen it.
/// XLSX preflight charges that widened box against the table-cell budget (`widening_charged`).
/// For the other formats it charged only the larger of the value and formula areas, so there the
/// range is widened only to a box within that; otherwise `range` is returned as it was and the
/// warning says so. If the formulas cannot be read the range is also unchanged, with a warning.
pub(super) fn show_unavailable_formulas<RS, R>(
    workbook: &mut R,
    sheet_name: &str,
    range: Range<Data>,
    widening_charged: bool,
    warnings: &mut Vec<ProcessingWarning>,
) -> Range<Data>
where
    RS: Read + Seek,
    R: Reader<RS>,
    R::Error: Display,
{
    let formulas = match workbook.worksheet_formula(sheet_name) {
        Ok(formulas) => formulas,
        Err(error) => {
            push_warning(
                warnings,
                "excel",
                format!(
                    "Sheet '{sheet_name}' formulas could not be read ({error}); formula cells with no saved result \
                     may appear empty"
                ),
            );
            return range;
        }
    };
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

    let (mut lo, mut hi) = missing.iter().fold(
        (missing[0].get_position(), missing[0].get_position()),
        |(lo, hi), cell| {
            let (row, col) = cell.get_position();
            ((lo.0.min(row), lo.1.min(col)), (hi.0.max(row), hi.1.max(col)))
        },
    );
    let mut charged = formulas.end().map_or(0, |end| area(formula_start, end));
    if let (Some(start), Some(end)) = (range.start(), range.end()) {
        charged = charged.max(area(start, end));
        lo = (lo.0.min(start.0), lo.1.min(start.1));
        hi = (hi.0.max(end.0), hi.1.max(end.1));
    }
    if !widening_charged && area(lo, hi) > charged {
        warn_unavailable_formulas(warnings, sheet_name, missing.len(), false);
        return range;
    }

    warn_unavailable_formulas(warnings, sheet_name, missing.len(), true);
    let mut cells: Vec<Cell<Data>> = match range.start() {
        Some(start) => range
            .used_cells()
            .map(|(row, col, value)| Cell::new((start.0 + row as u32, start.1 + col as u32), value.clone()))
            .collect(),
        None => Vec::new(),
    };
    cells.extend(missing);
    Range::from_sparse(cells)
}
