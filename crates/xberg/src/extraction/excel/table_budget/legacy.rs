//! Bounded BIFF preflight before calamine eagerly builds legacy XLS ranges.

use std::io::{Read, Seek, SeekFrom};

use super::{SecurityBudget, SecurityLimits, SheetBounds, SkippedSheets, preflight_sheet};
use crate::{Result, XbergError};

const BIFF_BOF: u16 = 0x0809;
const BIFF_EOF: u16 = 0x000A;
const BIFF_BOUND_SHEET: u16 = 0x0085;
const BIFF_DIMENSIONS: u16 = 0x0200;

fn next_record(reader: &mut impl Read, budget: &mut SecurityBudget) -> Result<Option<(u16, Vec<u8>)>> {
    let mut header = [0u8; 4];
    if reader.read(&mut header[..1])? == 0 {
        return Ok(None);
    }
    budget.step()?;
    reader.read_exact(&mut header[1..])?;
    let kind = u16::from_le_bytes([header[0], header[1]]);
    let length = usize::from(u16::from_le_bytes([header[2], header[3]]));
    let mut data = vec![0; length];
    reader.read_exact(&mut data)?;
    Ok(Some((kind, data)))
}

pub(in crate::extraction::excel) fn validate_xls(
    reader: &mut (impl Read + Seek),
    limits: &SecurityLimits,
) -> Result<SkippedSheets> {
    let mut budget = SecurityBudget::from_limits(limits);
    let mut skipped = SkippedSheets::new(limits);
    let mut worksheets = Vec::new();
    while let Some((kind, data)) = next_record(reader, &mut budget)? {
        if kind == BIFF_EOF {
            break;
        }
        if kind == BIFF_BOUND_SHEET && data.len() >= 4 {
            worksheets.push(u64::from(u32::from_le_bytes(data[..4].try_into().unwrap())));
        }
    }
    for (index, offset) in worksheets.into_iter().enumerate() {
        preflight_sheet(index, &mut budget, &mut skipped, |budget| {
            reader.seek(SeekFrom::Start(offset))?;
            validate_sheet(reader, budget, limits)
        })?;
    }
    Ok(skipped)
}

/// Record id no BIFF reader acts on; overwrites the records of a skipped sheet in place.
const BIFF_IGNORED: u16 = 0x0FFF;

/// Calamine builds every XLS range while constructing the reader, so a sheet that did not fit
/// in the cell budget cannot be left out afterwards. Rewrite the input with that sheet's
/// records (everything between its BOF and EOF) turned into ignored records; offsets and
/// lengths stay the same, so the reader sees an empty sheet and allocates nothing for it.
pub(in crate::extraction::excel) fn drop_skipped_sheets(
    data: &[u8],
    skipped: &SkippedSheets,
    limits: &SecurityLimits,
) -> Result<Option<Vec<u8>>> {
    if skipped.is_empty() {
        return Ok(None);
    }
    let mut compound = cfb::OpenOptions::new()
        .open_with(std::io::Cursor::new(data))
        .map_err(|error| XbergError::parsing(format!("Invalid XLS compound container: {error}")))?;
    let stream_name = if compound.is_stream("/Workbook") {
        "/Workbook"
    } else {
        "/Book"
    };
    let mut stream = Vec::new();
    compound.open_stream(stream_name)?.read_to_end(&mut stream)?;
    neutralize_sheets(&mut stream, skipped, limits)?;
    // ~keep Calamine reads only the workbook stream, so the rewritten container carries just that;
    // the document-information, VBA and picture streams of the original are dropped, and nothing
    // downstream reads them from XLS input.
    let mut rewritten = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new()))?;
    std::io::Write::write_all(&mut rewritten.create_stream(stream_name)?, &stream)?;
    Ok(Some(rewritten.into_inner().into_inner()))
}

fn neutralize_sheets(stream: &mut [u8], skipped: &SkippedSheets, limits: &SecurityLimits) -> Result<()> {
    let mut budget = SecurityBudget::from_limits(limits);
    let mut worksheets = Vec::new();
    let mut position = 0usize;
    while let Some((kind, length)) = record_header(stream, position) {
        budget.step()?;
        if kind == BIFF_EOF {
            break;
        }
        if kind == BIFF_BOUND_SHEET && length >= 4 {
            let offset = u32::from_le_bytes(stream[position + 4..position + 8].try_into().unwrap());
            worksheets.push(offset as usize);
        }
        position += 4 + length;
    }
    // Sheets may alias one substream; walk each distinct start once.
    let starts: std::collections::BTreeSet<usize> = worksheets
        .into_iter()
        .enumerate()
        .filter(|(index, _)| skipped.contains(*index))
        .map(|(_, offset)| offset)
        .collect();
    for offset in starts {
        let (mut position, mut seen_bof, mut nested) = (offset, false, 0usize);
        while let Some((kind, length)) = record_header(stream, position) {
            budget.step()?;
            if nested > 0 {
                match kind {
                    BIFF_BOF => nested += 1,
                    BIFF_EOF => nested -= 1,
                    _ => {}
                }
            } else {
                match kind {
                    BIFF_BOF if seen_bof => nested = 1,
                    BIFF_BOF => seen_bof = true,
                    BIFF_EOF => break,
                    _ => {}
                }
            }
            if !matches!(kind, BIFF_BOF | BIFF_EOF) {
                stream[position..position + 2].copy_from_slice(&BIFF_IGNORED.to_le_bytes());
            }
            position += 4 + length;
        }
    }
    Ok(())
}

/// The record at `position`, or `None` at the end of the stream or at a truncated record.
fn record_header(stream: &[u8], position: usize) -> Option<(u16, usize)> {
    let header = stream.get(position..position.checked_add(4)?)?;
    let kind = u16::from_le_bytes([header[0], header[1]]);
    let length = usize::from(u16::from_le_bytes([header[2], header[3]]));
    (position + 4 + length <= stream.len()).then_some((kind, length))
}

fn validate_sheet(reader: &mut impl Read, budget: &mut SecurityBudget, limits: &SecurityLimits) -> Result<()> {
    let mut bounds = LegacyBounds {
        max_content_size: limits.max_content_size,
        ..Default::default()
    };
    let mut seen_bof = false;
    let mut nested = 0usize;
    while let Some((kind, data)) = next_record(reader, budget)? {
        if nested > 0 {
            match kind {
                BIFF_BOF => nested += 1,
                BIFF_EOF => nested -= 1,
                _ => {}
            }
            continue;
        }
        match kind {
            BIFF_BOF if seen_bof => nested = 1,
            BIFF_BOF => seen_bof = true,
            BIFF_EOF => break,
            BIFF_DIMENSIONS => account_dimensions(&data, &mut bounds, budget)?,
            _ => account_cell_record(kind, &data, &mut bounds, budget)?,
        }
    }
    Ok(())
}

fn account_dimensions(data: &[u8], bounds: &mut LegacyBounds, budget: &mut SecurityBudget) -> Result<()> {
    let (first_row, last_row, mut first_col, last_col) = match data.len() {
        10 => (
            u32::from(word(data, 0)),
            u32::from(word(data, 2)),
            u32::from(word(data, 4)),
            u32::from(word(data, 6)),
        ),
        14 | 16 => (
            u32::from_le_bytes(data[..4].try_into().unwrap()),
            u32::from_le_bytes(data[4..8].try_into().unwrap()),
            u32::from(word(data, 8)),
            u32::from(word(data, 10)),
        ),
        _ => return Err(XbergError::parsing("Invalid XLS DIMENSIONS record length".to_string())),
    };
    // ~keep Mirror calamine0.37's BIFF column normalization and eager reserve, not an origin-padded rectangle (#2296).
    if first_col > 255 || last_col < first_col {
        first_col = 0;
    }
    if last_row == 0 || last_col == 0 {
        return bounds.reserve_values(1);
    }
    if last_row <= first_row || last_col <= first_col {
        return Err(XbergError::validation("Invalid XLS worksheet dimensions".to_string()));
    }
    let rows = u64::from(last_row - first_row);
    let cols = u64::from(last_col - first_col);
    let count = rows
        .checked_mul(cols)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| XbergError::validation("XLS worksheet dimensions overflow".to_string()))?;
    bounds.charge(count, budget)?;
    bounds.reserve_values(count)
}

#[derive(Default)]
struct LegacyBounds {
    values: SheetBounds,
    charged_cells: usize,
    formula_position: (u32, u32),
    sparse_values: usize,
    sparse_formulas: usize,
    value_capacity: usize,
    max_content_size: usize,
}

impl LegacyBounds {
    fn reserve_values(&mut self, additional: usize) -> Result<()> {
        // ~keep Vec::reserve takes an additional count beyond len, even for stale/repeated DIMENSIONS records.
        let requested = self
            .sparse_values
            .checked_add(additional)
            .ok_or_else(|| XbergError::validation("XLS sparse cell capacity overflow".to_string()))?;
        self.value_capacity = self.value_capacity.max(requested);
        self.check_sparse_bytes()
    }

    fn push_sparse(&mut self, values: usize, formulas: usize) -> Result<()> {
        self.sparse_values = self
            .sparse_values
            .checked_add(values)
            .ok_or_else(|| XbergError::validation("XLS sparse value count overflow".to_string()))?;
        self.sparse_formulas = self
            .sparse_formulas
            .checked_add(formulas)
            .ok_or_else(|| XbergError::validation("XLS sparse formula count overflow".to_string()))?;
        self.value_capacity = self.value_capacity.max(self.sparse_values);
        self.check_sparse_bytes()
    }

    fn check_sparse_bytes(&self) -> Result<()> {
        let values = self
            .value_capacity
            .checked_mul(std::mem::size_of::<calamine::Cell<calamine::Data>>());
        let formulas = self
            .sparse_formulas
            .checked_mul(std::mem::size_of::<calamine::Cell<String>>());
        let bytes = values
            .zip(formulas)
            .and_then(|(values, formulas)| values.checked_add(formulas))
            .ok_or_else(|| XbergError::validation("XLS sparse cell capacity overflow".to_string()))?;
        if bytes > self.max_content_size {
            return Err(XbergError::validation(
                "XLS sparse cell allocation exceeds the configured content size".to_string(),
            ));
        }
        Ok(())
    }

    fn account(&mut self, position: (u32, u32), budget: &mut SecurityBudget) -> Result<()> {
        let count = self.values.grow(position)?;
        self.charge(count, budget)
    }

    fn charge(&mut self, count: usize, budget: &mut SecurityBudget) -> Result<()> {
        // ~keep Stale DIMENSIONS and actual cells bound separate allocations; do not union their origins.
        let next = self.charged_cells.max(count);
        budget.add_cells(next - self.charged_cells)?;
        self.charged_cells = next;
        Ok(())
    }
}

fn word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn account_cell_record(kind: u16, data: &[u8], bounds: &mut LegacyBounds, budget: &mut SecurityBudget) -> Result<()> {
    match kind {
        // ~keep These are precisely calamine0.37's value/formula records that enter Range::from_sparse.
        0x0203 | 0x0204 | 0x00D6 | 0x0205 | 0x027E | 0x00FD if data.len() >= 4 => {
            bounds.account((u32::from(word(data, 0)), u32::from(word(data, 2))), budget)?;
            // ~keep LABELSST may not resolve; budgeting its potential push is conservative without reimplementing SST decoding.
            bounds.push_sparse(1, 0)?;
        }
        0x0006 if data.len() >= 4 => {
            bounds.formula_position = (u32::from(word(data, 0)), u32::from(word(data, 2)));
            bounds.account(bounds.formula_position, budget)?;
            if data.len() < 20 {
                return Err(XbergError::parsing("Invalid XLS FORMULA record length".to_string()));
            }
            let string_result = data[6] == 0 && data[12] == 0xff && data[13] == 0xff;
            bounds.push_sparse(usize::from(!string_result), 1)?;
        }
        // ~keep Calamine accepts STRING at the previous formula position, initially (0,0), even without FORMULA.
        0x0207 => {
            bounds.account(bounds.formula_position, budget)?;
            bounds.push_sparse(1, 0)?;
        }
        0x00BD if data.len() >= 6 => {
            let row = u32::from(word(data, 0));
            let first = u32::from(word(data, 2));
            let last = u32::from(word(data, data.len() - 2));
            if last < first {
                return Err(XbergError::parsing("Invalid XLS MULRK column range".to_string()));
            }
            let count = (last - first + 1) as usize;
            if data.len() != 6 + 6 * count {
                return Err(XbergError::parsing("Invalid XLS MULRK record length".to_string()));
            }
            bounds.account((row, first), budget)?;
            bounds.account((row, last), budget)?;
            bounds.push_sparse(count, 0)?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::excel::tests::assert_sheet_skipped_for_cell_limit;
    use std::io::{Cursor, Write};

    fn record(kind: u16, data: &[u8]) -> Vec<u8> {
        [
            kind.to_le_bytes().as_slice(),
            (data.len() as u16).to_le_bytes().as_slice(),
            data,
        ]
        .concat()
    }

    fn bof(kind: u16) -> Vec<u8> {
        let mut data = vec![0; 16];
        data[..2].copy_from_slice(&0x0600u16.to_le_bytes());
        data[2..4].copy_from_slice(&kind.to_le_bytes());
        record(BIFF_BOF, &data)
    }

    fn dimensions(first: (u32, u16), last: (u32, u16)) -> Vec<u8> {
        let mut data = vec![0; 14];
        data[..4].copy_from_slice(&first.0.to_le_bytes());
        data[4..8].copy_from_slice(&last.0.to_le_bytes());
        data[8..10].copy_from_slice(&first.1.to_le_bytes());
        data[10..12].copy_from_slice(&last.1.to_le_bytes());
        record(BIFF_DIMENSIONS, &data)
    }

    fn number(row: u16, col: u16, value: f64) -> Vec<u8> {
        let mut data = vec![0; 14];
        data[..2].copy_from_slice(&row.to_le_bytes());
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[6..].copy_from_slice(&value.to_le_bytes());
        record(0x0203, &data)
    }

    fn workbook(sheet: &[u8], count: usize) -> Vec<u8> {
        workbook_of(&vec![sheet.to_vec(); count])
    }

    fn workbook_of(sheets: &[Vec<u8>]) -> Vec<u8> {
        workbook_with_globals(sheets, &[])
    }

    /// `extra_globals` are whole records placed in the globals substream after the BOUNDSHEETs.
    fn workbook_with_globals(sheets: &[Vec<u8>], extra_globals: &[u8]) -> Vec<u8> {
        let mut starts = Vec::new();
        let mut next = bof(0x0005).len() + sheets.len() * 14 + extra_globals.len() + 4;
        for sheet in sheets {
            starts.push(next);
            next += sheet.len();
        }
        workbook_at(sheets, &starts, extra_globals)
    }

    /// One BOUNDSHEET per entry of `starts` (absolute offsets in the stream), then the substreams.
    fn workbook_at(substreams: &[Vec<u8>], starts: &[usize], extra_globals: &[u8]) -> Vec<u8> {
        let mut globals = bof(0x0005);
        for (index, start) in starts.iter().enumerate() {
            let mut data = vec![0; 10];
            data[..4].copy_from_slice(&(*start as u32).to_le_bytes());
            data[6] = 2;
            data[8] = b'0' + ((index / 10) % 10) as u8;
            data[9] = b'0' + (index % 10) as u8;
            globals.extend(record(BIFF_BOUND_SHEET, &data));
        }
        globals.extend_from_slice(extra_globals);
        globals.extend(record(BIFF_EOF, &[]));
        for sheet in substreams {
            globals.extend_from_slice(sheet);
        }
        globals
    }

    fn compound(workbook: &[u8]) -> Vec<u8> {
        let mut file = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
        file.create_stream("/Workbook").unwrap().write_all(workbook).unwrap();
        file.into_inner().into_inner()
    }

    #[test]
    fn should_reject_xls_dimensions_before_the_backend_reserves_cells_gh2296() {
        let sheet = [bof(0x0010), dimensions((0, 0), (1999, 1000)), record(BIFF_EOF, &[])].concat();
        let limits = SecurityLimits {
            max_table_cells: 100_000,
            ..Default::default()
        };
        for count in [1, 10, 50] {
            let bytes = compound(&workbook(&sheet, count));
            let skipped = validate_xls(&mut Cursor::new(workbook(&sheet, count)), &limits).unwrap();
            assert_eq!(skipped.indices, (0..count).collect::<Vec<_>>());
            // ~keep Calamine's own CFB reader cannot open these generated containers at 50 sheets
            // ("Cannot find Workbook stream") even unmodified, also on origin/main, so only the smaller
            // ones are read end to end.
            if count <= 10 {
                let (result, warnings) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits).unwrap();
                assert_eq!(result.sheets.len(), count);
                assert!(result.sheets.iter().all(|sheet| sheet.table_cells.is_none()));
                assert_eq!(warnings.len(), count);
            }
        }
    }

    #[test]
    fn should_bound_actual_xls_cells_when_dimensions_are_missing_gh2296() {
        let sheet = [bof(0x0010), number(0, 0, 1.0), number(2, 2, 2.0), record(BIFF_EOF, &[])].concat();
        let limits = SecurityLimits {
            max_table_cells: 8,
            ..Default::default()
        };
        let skipped = validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &limits).unwrap();
        assert_eq!(skipped.indices, [0]);
    }

    #[test]
    fn should_charge_non_origin_xls_bounds_without_origin_padding_gh2296() {
        let sheet = [
            bof(0x0010),
            dimensions((10, 10), (12, 12)),
            number(10, 10, 1.0),
            number(11, 11, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let bytes = compound(&workbook(&sheet, 1));
        let limits = SecurityLimits {
            max_table_cells: 4,
            ..Default::default()
        };
        let (result, _) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits).unwrap();
        assert_eq!(result.sheets.len(), 1);
        assert_eq!((result.sheets[0].row_count, result.sheets[0].col_count), (2, 2));
    }

    #[test]
    fn should_not_union_stale_xls_dimensions_with_actual_cell_origins_gh2296() {
        let sheet = [
            bof(0x0010),
            dimensions((1000, 100), (1001, 101)),
            number(0, 0, 1.0),
            number(0, 1, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let limits = SecurityLimits {
            max_table_cells: 2,
            ..Default::default()
        };
        validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &limits).unwrap();
    }

    #[test]
    fn should_ignore_embedded_xls_chart_cell_records_gh2296() {
        let sheet = [
            bof(0x0010),
            number(0, 0, 1.0),
            bof(0x0020),
            dimensions((0, 0), (65535, 256)),
            number(65534, 255, 9.0),
            record(BIFF_EOF, &[]),
            number(0, 1, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let limits = SecurityLimits {
            max_table_cells: 2,
            ..Default::default()
        };
        validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &limits).unwrap();
    }

    #[test]
    fn should_charge_xls_sheet_grids_against_one_document_budget_gh2296() {
        let sheet = [bof(0x0010), dimensions((0, 0), (2, 2)), record(BIFF_EOF, &[])].concat();
        let limits = SecurityLimits {
            max_table_cells: 7,
            ..Default::default()
        };
        let skipped = validate_xls(&mut Cursor::new(workbook(&sheet, 2)), &limits).unwrap();
        assert_eq!(skipped.indices, [1]);
    }

    #[test]
    fn should_charge_the_implicit_origin_of_an_orphan_xls_string_gh2296() {
        let sheet = [
            bof(0x0010),
            record(0x0207, &[1, 0, 0, b'x']),
            number(5, 5, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let bytes = compound(&workbook(&sheet, 1));
        let limits = SecurityLimits {
            max_table_cells: 8,
            ..Default::default()
        };
        assert_sheet_skipped_for_cell_limit(
            crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits),
            "00",
            8,
        );
        let ample = SecurityLimits {
            max_table_cells: 36,
            ..Default::default()
        };
        let (result, _) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &ample).unwrap();
        let cells = result.sheets[0].table_cells.as_ref().unwrap();
        assert_eq!((cells.len(), cells[0].len()), (6, 6));
        assert_eq!(cells[0][0], "x");
        assert_eq!(cells[5][5], "2");
    }

    #[test]
    fn should_preserve_a_small_backend_accepted_orphan_xls_string_gh2296() {
        let sheet = [
            bof(0x0010),
            record(0x0207, &[1, 0, 0, b'x']),
            number(0, 1, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let bytes = compound(&workbook(&sheet, 1));
        let limits = SecurityLimits {
            max_table_cells: 2,
            ..Default::default()
        };
        let (result, _) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits).unwrap();
        assert_eq!(result.sheets.len(), 1);
        assert_eq!(
            result.sheets[0].table_cells.as_ref().unwrap(),
            &[vec!["x".to_string(), "2".to_string()]]
        );
    }

    #[test]
    fn should_keep_legacy_cell_guards_when_an_xls_file_is_renamed_gh2296() {
        let sheet = [
            bof(0x0010),
            dimensions((0, 0), (2, 2)),
            number(0, 0, 1.0),
            number(1, 1, 2.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let bytes = compound(&workbook(&sheet, 1));
        let directory = tempfile::tempdir().unwrap();
        for filename in ["legacy.dat", "no_extension"] {
            let path = directory.path().join(filename);
            std::fs::write(&path, &bytes).unwrap();
            let low = SecurityLimits {
                max_table_cells: 3,
                ..Default::default()
            };
            assert_sheet_skipped_for_cell_limit(
                crate::extraction::excel::read_excel_file(path.to_str().unwrap(), &low),
                "00",
                3,
            );
            let exact = SecurityLimits {
                max_table_cells: 4,
                ..Default::default()
            };
            let (result, _) = crate::extraction::excel::read_excel_file(path.to_str().unwrap(), &exact).unwrap();
            assert_eq!((result.sheets[0].row_count, result.sheets[0].col_count), (2, 2));
        }
    }

    #[test]
    fn should_bound_duplicate_xls_sparse_values_without_inflating_the_dense_grid_gh2296() {
        let sheet = [bof(0x0010), number(0, 0, 1.0), number(0, 0, 2.0), record(BIFF_EOF, &[])].concat();
        let bytes = compound(&workbook(&sheet, 1));
        let required = 2 * std::mem::size_of::<calamine::Cell<calamine::Data>>();
        let low = SecurityLimits {
            max_content_size: required - 1,
            max_table_cells: 1,
            ..Default::default()
        };
        let error = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &low).unwrap_err();
        assert!(error.to_string().contains("XLS sparse cell allocation"), "{error}");
        let exact = SecurityLimits {
            max_content_size: required,
            max_table_cells: 1,
            ..Default::default()
        };
        let (result, _) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &exact).unwrap();
        assert_eq!(result.sheets[0].table_cells.as_ref().unwrap(), &[vec!["2".to_string()]]);
    }

    #[test]
    fn should_include_existing_xls_values_in_later_dimension_reservations_gh2296() {
        let sheet = [
            bof(0x0010),
            number(0, 0, 1.0),
            dimensions((0, 0), (1, 2)),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let required = 3 * std::mem::size_of::<calamine::Cell<calamine::Data>>();
        let low = SecurityLimits {
            max_content_size: required - 1,
            max_table_cells: 2,
            ..Default::default()
        };
        let error = validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &low).unwrap_err();
        assert!(error.to_string().contains("XLS sparse cell allocation"), "{error}");
        let exact = SecurityLimits {
            max_content_size: required,
            max_table_cells: 2,
            ..Default::default()
        };
        validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &exact).unwrap();
    }

    #[test]
    fn should_count_mulrk_values_and_formula_string_slots_in_xls_sparse_bytes_gh2296() {
        let mut multiple = vec![0; 18];
        multiple[16..].copy_from_slice(&1u16.to_le_bytes());
        let sheet = [
            bof(0x0010),
            record(0x00bd, &multiple),
            dimensions((0, 0), (1, 1)),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let required = 3 * std::mem::size_of::<calamine::Cell<calamine::Data>>();
        let low = SecurityLimits {
            max_content_size: required - 1,
            max_table_cells: 2,
            ..Default::default()
        };
        assert!(
            validate_xls(&mut Cursor::new(workbook(&sheet, 1)), &low)
                .unwrap_err()
                .to_string()
                .contains("XLS sparse cell allocation")
        );
        let mut formula = vec![0; 20];
        formula[12] = 0xff;
        formula[13] = 0xff;
        let formula_sheet = [
            bof(0x0010),
            record(0x0006, &formula),
            record(0x0207, &[1, 0, 0, b'x']),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let required =
            std::mem::size_of::<calamine::Cell<calamine::Data>>() + std::mem::size_of::<calamine::Cell<String>>();
        let low = SecurityLimits {
            max_content_size: required - 1,
            max_table_cells: 1,
            ..Default::default()
        };
        assert!(
            validate_xls(&mut Cursor::new(workbook(&formula_sheet, 1)), &low)
                .unwrap_err()
                .to_string()
                .contains("XLS sparse cell allocation")
        );
        let exact = SecurityLimits {
            max_content_size: required,
            max_table_cells: 1,
            ..Default::default()
        };
        validate_xls(&mut Cursor::new(workbook(&formula_sheet, 1)), &exact).unwrap();
    }

    #[test]
    fn should_skip_only_the_xls_sheets_over_the_table_budget_gh2820() {
        let small = [
            bof(0x0010),
            number(0, 0, 1.0),
            number(0, 1, 2.0),
            number(1, 0, 3.0),
            number(1, 1, 4.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let big = [
            bof(0x0010),
            dimensions((0, 0), (5, 5)),
            number(0, 0, 9.0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let tail = [bof(0x0010), number(0, 0, 7.0), number(0, 1, 8.0), record(BIFF_EOF, &[])].concat();
        let bytes = compound(&workbook_of(&[small, big, tail]));
        let limits = SecurityLimits {
            max_table_cells: 10,
            ..Default::default()
        };
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("budget.xla");
        std::fs::write(&path, &bytes).unwrap();
        for (label, result) in [
            (
                "xls bytes",
                crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits),
            ),
            (
                "xla bytes",
                crate::extraction::excel::read_excel_bytes(&bytes, ".xla", &limits),
            ),
            (
                "xla file",
                crate::extraction::excel::read_excel_file(path.to_str().unwrap(), &limits),
            ),
        ] {
            let (workbook, warnings) = result.unwrap_or_else(|error| panic!("{label}: {error}"));
            let names: Vec<&str> = workbook.sheets.iter().map(|sheet| sheet.name.as_str()).collect();
            assert_eq!(names, ["00", "01", "02"], "{label}");
            assert_eq!(
                workbook.sheets[0].table_cells.as_ref().unwrap(),
                &[
                    vec!["1".to_string(), "2".to_string()],
                    vec!["3".to_string(), "4".to_string()]
                ],
                "{label}"
            );
            assert!(workbook.sheets[1].table_cells.is_none(), "{label}");
            assert_eq!(
                workbook.sheets[2].table_cells.as_ref().unwrap(),
                &[vec!["7".to_string(), "8".to_string()]],
                "{label}"
            );
            let skipped: Vec<&str> = warnings
                .iter()
                .filter(|warning| warning.message.contains("max_table_cells"))
                .map(|warning| warning.message.as_ref())
                .collect();
            assert_eq!(skipped.len(), 1, "{label}: {warnings:?}");
            assert!(skipped[0].contains("Sheet '01' was skipped"), "{label}: {}", skipped[0]);
        }
    }

    fn sst(strings: &[&str]) -> Vec<u8> {
        let mut data = (strings.len() as u32).to_le_bytes().to_vec();
        data.extend((strings.len() as u32).to_le_bytes());
        for text in strings {
            data.extend((text.len() as u16).to_le_bytes());
            data.push(0);
            data.extend(text.bytes());
        }
        record(0x00FC, &data)
    }

    fn label_sst(row: u16, col: u16, index: u32) -> Vec<u8> {
        let mut data = vec![0; 10];
        data[..2].copy_from_slice(&row.to_le_bytes());
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[6..].copy_from_slice(&index.to_le_bytes());
        record(0x00FD, &data)
    }

    #[test]
    fn should_keep_shared_strings_for_a_sheet_after_a_skipped_xls_sheet_gh2820() {
        let big = [
            bof(0x0010),
            dimensions((0, 0), (5, 5)),
            label_sst(0, 0, 0),
            record(0x003C, &[0; 8]),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let strings = [
            bof(0x0010),
            label_sst(0, 0, 1),
            label_sst(0, 1, 0),
            record(BIFF_EOF, &[]),
        ]
        .concat();
        let bytes = compound(&workbook_with_globals(&[big, strings], &sst(&["hello", "world"])));
        let limits = SecurityLimits {
            max_table_cells: 10,
            ..Default::default()
        };
        let (result, warnings) = crate::extraction::excel::read_excel_bytes(&bytes, ".xls", &limits).unwrap();
        assert_eq!(result.sheets.len(), 2);
        assert!(result.sheets[0].table_cells.is_none());
        assert_eq!(
            result.sheets[1].table_cells.as_ref().unwrap(),
            &[vec!["world".to_string(), "hello".to_string()]]
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    #[test]
    fn should_skip_every_xls_sheet_that_shares_one_over_budget_substream_gh2820() {
        let sheet = [bof(0x0010), dimensions((0, 0), (5, 5)), record(BIFF_EOF, &[])].concat();
        let stream = workbook_at(std::slice::from_ref(&sheet), &[bof(0x0005).len() + 3 * 14 + 4; 3], &[]);
        let limits = SecurityLimits {
            max_table_cells: 10,
            ..Default::default()
        };
        let skipped = validate_xls(&mut Cursor::new(stream.clone()), &limits).unwrap();
        assert_eq!(skipped.indices, [0, 1, 2]);
        let (result, warnings) =
            crate::extraction::excel::read_excel_bytes(&compound(&stream), ".xls", &limits).unwrap();
        assert_eq!(result.sheets.len(), 3);
        assert!(result.sheets.iter().all(|sheet| sheet.table_cells.is_none()));
        assert_eq!(warnings.len(), 3);
    }

    /// `count` BOUNDSHEETs that all point at one substream of `records` junk records, the first
    /// of which is a DIMENSIONS record that trips the cell limit.
    fn aliased_workbook(count: usize, records: usize) -> Vec<u8> {
        let mut sheet = [bof(0x0010), dimensions((0, 0), (1999, 1000))].concat();
        for _ in 0..records {
            sheet.extend(record(0x0FFE, &[0; 4]));
        }
        sheet.extend(record(BIFF_EOF, &[]));
        workbook_at(&[sheet], &vec![bof(0x0005).len() + count * 14 + 4; count], &[])
    }

    #[test]
    fn should_walk_a_shared_xls_substream_once_when_blanking_skipped_sheets_gh2820() {
        let stream = aliased_workbook(200, 2000);
        let limits = SecurityLimits {
            max_table_cells: 10,
            max_iterations: 3000,
            ..Default::default()
        };
        let skipped = validate_xls(&mut Cursor::new(stream.clone()), &limits).unwrap();
        assert_eq!(skipped.indices.len(), 200);
        let rewritten = drop_skipped_sheets(&compound(&stream), &skipped, &limits).unwrap();
        assert!(rewritten.is_some());
    }

    #[test]
    fn should_keep_the_iteration_budget_monotonic_across_skipped_xls_sheets_gh2820() {
        let stream = aliased_workbook(100, 0);
        let limits = SecurityLimits {
            max_table_cells: 10,
            max_iterations: 150,
            ..Default::default()
        };
        let error = validate_xls(&mut Cursor::new(stream), &limits).unwrap_err();
        assert!(error.to_string().contains("iterations"), "{error}");
    }

    #[test]
    fn should_stop_blanking_a_skipped_xls_sheet_at_a_truncated_record_gh2820() {
        let sheet = [bof(0x0010), dimensions((0, 0), (5, 5)), vec![0x03, 0x02, 0xFF, 0xFF]].concat();
        let stream = workbook_of(&[sheet]);
        let limits = SecurityLimits {
            max_table_cells: 10,
            ..Default::default()
        };
        assert_eq!(
            validate_xls(&mut Cursor::new(stream.clone()), &limits).unwrap().indices,
            [0]
        );
        let mut forced = SkippedSheets::new(&limits);
        forced.indices.push(0);
        assert!(
            drop_skipped_sheets(&compound(&stream), &forced, &limits)
                .unwrap()
                .is_some()
        );
    }
}
