//! ODS repeat preflight before calamine's eager dense-range construction.

use std::collections::HashSet;
use std::io::{BufReader, Read, Seek};

use quick_xml::events::{BytesStart, Event};

use super::{Charges, SecurityBudget, SecurityLimits, SheetAllocation, SkippedSheets, is_cell_limit};
use crate::{Result, XbergError};

// ~keep Match calamine0.37's ODS row/column caps so formatted empty tails are not charged as dense data.
const MAX_ROWS: u32 = 1_048_576;
const MAX_COLS: u32 = 16_384;

#[derive(Default)]
struct Sheet {
    bounds: SheetAllocation,
    row: u32,
    row_repeats: u32,
    in_row: bool,
    col: u32,
}

#[derive(Default)]
struct Cell {
    repeats: u32,
    value_populated: bool,
    formula_populated: bool,
    string_content: bool,
    text_bytes: usize,
    first_paragraph: bool,
    annotation_depth: usize,
}

fn xml_error(error: impl std::fmt::Display) -> XbergError {
    XbergError::parsing(format!("Invalid ODS table XML: {error}"))
}

fn repeat(event: &BytesStart<'_>, key: &str) -> Result<u64> {
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.map_err(xml_error)?;
        if attribute.key.as_ref() == key {
            let value = attribute
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map_err(xml_error)?;
            let count = value.parse::<u64>().map_err(xml_error)?;
            if count == 0 {
                return Err(xml_error("a row or column repeat count must be positive"));
            }
            return Ok(count);
        }
    }
    Ok(1)
}

fn cell_state(event: &BytesStart<'_>, remaining: u32, budget: &SecurityBudget) -> Result<Cell> {
    let mut cell = Cell {
        repeats: repeat(event, "table:number-columns-repeated")?.min(u64::from(remaining)) as u32,
        first_paragraph: true,
        ..Default::default()
    };
    let mut value_set = false;
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.map_err(xml_error)?;
        let value = attribute
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(xml_error)?;
        match attribute.key.as_ref() {
            "office:value-type" if !value_set => cell.string_content = value == "string",
            "office:value"
            | "office:string-value"
            | "office:date-value"
            | "office:time-value"
            | "office:boolean-value"
                if !value_set =>
            {
                value_set = true;
                cell.value_populated = true;
                add_text(&mut cell, value.len())?;
                budget.check_entity(&value)?;
            }
            "table:formula" if !value.is_empty() => {
                cell.formula_populated = true;
                budget.check_entity(&value)?;
                add_text(&mut cell, value.len())?;
            }
            _ => {}
        }
    }
    cell.string_content &= !value_set;
    cell.value_populated |= cell.string_content;
    Ok(cell)
}

fn add_text(cell: &mut Cell, length: usize) -> Result<()> {
    cell.text_bytes = cell
        .text_bytes
        .checked_add(length)
        .ok_or_else(|| xml_error("cell text size overflow"))?;
    Ok(())
}

fn account_cell(sheet: &mut Sheet, cell: Cell, budget: &mut SecurityBudget) -> Result<()> {
    // ~keep Calamine constructs one cell string even if its repeated row/column is outside its fixed caps.
    budget.account_text(cell.text_bytes)?;
    if (cell.value_populated || cell.formula_populated) && cell.repeats > 0 && sheet.row_repeats > 0 {
        let start = (sheet.row, sheet.col);
        let end = (sheet.row + sheet.row_repeats - 1, sheet.col + cell.repeats - 1);
        if cell.value_populated {
            sheet.bounds.value(start, budget)?;
            sheet.bounds.value(end, budget)?;
        }
        if cell.formula_populated {
            sheet.bounds.formula(start, budget)?;
            sheet.bounds.formula(end, budget)?;
        }
        let copies = u64::from(cell.repeats) * u64::from(sheet.row_repeats);
        let length = u64::try_from(cell.text_bytes)
            .ok()
            .and_then(|value| value.checked_mul(copies.saturating_sub(1)))
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| xml_error("repeated cell text size overflow"))?;
        budget.account_text(length)?;
    }
    sheet.col += cell.repeats;
    Ok(())
}

pub(in crate::extraction::excel) fn validate_ods<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &SecurityLimits,
) -> Result<SkippedSheets> {
    let member = archive.by_name("content.xml").map_err(xml_error)?;
    let mut reader = quick_xml::Reader::from_reader(BufReader::new(
        member.take((limits.max_archive_size as u64).saturating_add(1)),
    ));
    reader.config_mut().expand_empty_elements = true;
    let mut budget = SecurityBudget::from_limits(limits);
    let mut skipped = SkippedSheets::new(limits);
    let mut sheet: Option<Sheet> = None;
    let mut cell: Option<Cell> = None;
    let mut buffer = Vec::new();
    // Charges before the current sheet, given back if the sheet alone trips the cell limit.
    let mut before_sheet: Option<Charges> = None;
    let mut sheet_index = 0usize;
    // Open `table:table` elements left to close while the over-budget sheet is being passed over.
    let mut skipping_depth = 0usize;
    loop {
        budget.step()?;
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(xml_error)?;
        if reader.buffer_position() > limits.max_archive_size as u64 {
            return Err(XbergError::validation(
                "ODS content exceeds the configured archive size".to_string(),
            ));
        }
        if skipping_depth > 0 {
            match &event {
                Event::Start(start) if start.name().as_ref() == "table:table" => skipping_depth += 1,
                Event::End(end) if end.name().as_ref() == "table:table" => skipping_depth -= 1,
                Event::Eof => return Err(xml_error("unterminated table or cell")),
                _ => {}
            }
            continue;
        }
        if matches!(&event, Event::Start(start) if start.name().as_ref() == "table:table")
            && sheet.is_none()
            && cell.is_none()
        {
            before_sheet = Some(Charges::of(&budget));
            if let Event::Start(start) = &event {
                skipped.names.push(table_name(start));
            }
        }
        match handle_event(event, &mut sheet, &mut cell, &mut budget) {
            Ok(true) => return Ok(skipped),
            Ok(false) => {}
            Err(error) if sheet.is_some() && is_cell_limit(&error) => {
                if let Some(before) = before_sheet.take() {
                    before.restore(&mut budget);
                }
                skipped.indices.insert(sheet_index);
                sheet_index += 1;
                sheet = None;
                cell = None;
                skipping_depth = 1;
                continue;
            }
            Err(error) => return Err(error),
        }
        if sheet.is_none() && before_sheet.is_some() {
            before_sheet = None;
            sheet_index += 1;
        }
    }
}

/// Calamine builds every ODS range while constructing the reader, so a sheet that did not fit
/// in the cell budget cannot be left out afterwards. Empty the body of each skipped
/// `table:table` in `content.xml`, keeping its start tag so the sheet keeps its name and position.
pub(in crate::extraction::excel) fn drop_skipped_sheets(
    xml: &[u8],
    skipped: &SkippedSheets,
    limits: &SecurityLimits,
) -> Result<(Vec<u8>, bool)> {
    let mut budget = SecurityBudget::from_limits(limits);
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len()));
    let mut buffer = Vec::new();
    let (mut sheet_index, mut table_depth, mut changed) = (0usize, 0usize, false);
    let mut current_skipped = false;
    let counts = skipped.name_counts();
    let mut taken: HashSet<String> = skipped.names.iter().cloned().collect();
    loop {
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(xml_error)?;
        match &event {
            Event::Start(start) if start.name().as_ref() == "table:table" => {
                if table_depth == 0 {
                    current_skipped = skipped.contains(sheet_index);
                    sheet_index += 1;
                }
                table_depth += 1;
                if current_skipped && table_depth > 1 {
                    changed = true;
                    continue;
                }
                if current_skipped && counts.get(table_name(start).as_str()).is_some_and(|count| *count > 1) {
                    let name = unused_name(&mut taken, sheet_index - 1, &mut budget)?;
                    writer
                        .write_event(Event::Start(renamed_table(start, &name)))
                        .map_err(xml_error)?;
                    changed = true;
                    continue;
                }
            }
            Event::Empty(start) if start.name().as_ref() == "table:table" && table_depth == 0 => {
                sheet_index += 1;
            }
            Event::End(end) if end.name().as_ref() == "table:table" => {
                table_depth = table_depth.saturating_sub(1);
                if current_skipped && table_depth > 0 {
                    continue;
                }
            }
            Event::Eof => break,
            _ if current_skipped && table_depth > 0 => {
                changed = true;
                continue;
            }
            _ => {}
        }
        writer.write_event(event.into_owned()).map_err(xml_error)?;
    }
    Ok((writer.into_inner(), changed))
}

/// A replacement sheet name that no sheet in the workbook (or earlier replacement) carries.
/// Candidates are per-index (`xberg-skipped-sheet-{index}[-{attempt}]`), so a name already in the
/// workbook can only push the attempts of one index; the total over all calls is bounded by the
/// number of sheet names, and every attempt is charged to `budget`.
fn unused_name(taken: &mut HashSet<String>, index: usize, budget: &mut SecurityBudget) -> Result<String> {
    let mut candidate = format!("xberg-skipped-sheet-{index}");
    let mut attempt = 0usize;
    while taken.contains(&candidate) {
        budget.step()?;
        attempt += 1;
        candidate = format!("xberg-skipped-sheet-{index}-{attempt}");
    }
    taken.insert(candidate.clone());
    Ok(candidate)
}

fn renamed_table(start: &BytesStart<'_>, name: &str) -> BytesStart<'static> {
    let mut renamed = BytesStart::new("table:table");
    for attribute in start.attributes().flatten() {
        if attribute.key.as_ref() == "table:name" {
            renamed.push_attribute(("table:name", name));
        } else {
            renamed.push_attribute(attribute);
        }
    }
    renamed
}

fn table_name(start: &BytesStart<'_>) -> String {
    start
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == "table:name")
        .and_then(|attribute| {
            quick_xml::escape::unescape(&attribute.value)
                .ok()
                .map(|name| name.into_owned())
        })
        .unwrap_or_default()
}

fn active_string_cell(cell: &mut Option<Cell>) -> Option<&mut Cell> {
    cell.as_mut()
        .filter(|cell| cell.string_content && cell.annotation_depth == 0)
}

fn handle_event(
    event: Event<'_>,
    sheet: &mut Option<Sheet>,
    cell: &mut Option<Cell>,
    budget: &mut SecurityBudget,
) -> Result<bool> {
    match event {
        Event::Start(start) => handle_start(&start, sheet, cell, budget)?,
        Event::Text(text) => {
            if let Some(cell) = active_string_cell(cell) {
                let text = text.xml10_content();
                budget.check_entity(&text)?;
                add_text(cell, text.len())?;
            }
        }
        Event::GeneralRef(reference) => {
            if let Some(cell) = active_string_cell(cell) {
                let escaped = format!("&{};", reference.as_ref());
                let decoded = quick_xml::escape::unescape(&escaped).map_err(xml_error)?;
                add_text(cell, decoded.len())?;
            }
        }
        Event::End(end) => handle_end(end.name().as_ref(), sheet, cell, budget)?,
        Event::Eof => {
            if sheet.is_some() || cell.is_some() {
                return Err(xml_error("unterminated table or cell"));
            }
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_end(
    name: &str,
    sheet: &mut Option<Sheet>,
    cell: &mut Option<Cell>,
    budget: &mut SecurityBudget,
) -> Result<()> {
    match name {
        "office:annotation" => {
            if let Some(cell) = cell.as_mut() {
                cell.annotation_depth = cell.annotation_depth.saturating_sub(1);
            }
        }
        "table:table-cell" | "table:covered-table-cell" => {
            if let (Some(sheet), Some(cell)) = (sheet.as_mut(), cell.take()) {
                account_cell(sheet, cell, budget)?;
            }
        }
        "table:table-row" => {
            if let Some(sheet) = sheet.as_mut() {
                if !sheet.in_row || cell.is_some() {
                    return Err(xml_error("invalid worksheet row end"));
                }
                sheet.row = sheet
                    .row
                    .checked_add(sheet.row_repeats)
                    .ok_or_else(|| xml_error("worksheet row overflow"))?;
                sheet.in_row = false;
            }
        }
        "table:table" if cell.is_none() => *sheet = None,
        _ => {}
    }
    Ok(())
}

fn handle_start(
    start: &BytesStart<'_>,
    sheet: &mut Option<Sheet>,
    cell: &mut Option<Cell>,
    budget: &SecurityBudget,
) -> Result<()> {
    match start.name().as_ref() {
        "table:table" if cell.is_none() => {
            if sheet.is_some() {
                return Err(xml_error("nested worksheet table"));
            }
            *sheet = Some(Sheet::default());
        }
        "table:table-row" => {
            if let Some(sheet) = sheet.as_mut() {
                if sheet.in_row || cell.is_some() {
                    return Err(xml_error("nested worksheet row"));
                }
                sheet.row_repeats = repeat(start, "table:number-rows-repeated")?
                    .min(u64::from(MAX_ROWS.saturating_sub(sheet.row))) as u32;
                sheet.col = 0;
                sheet.in_row = true;
            }
        }
        "table:table-cell" | "table:covered-table-cell" => {
            if let Some(sheet) = sheet.as_ref() {
                if cell.is_some() {
                    return Err(xml_error("nested spreadsheet cell"));
                }
                if !sheet.in_row {
                    return Err(xml_error("spreadsheet cell outside a row"));
                }
                *cell = Some(cell_state(start, MAX_COLS.saturating_sub(sheet.col), budget)?);
            }
        }
        _ => handle_cell_start(start, cell)?,
    }
    Ok(())
}

fn handle_cell_start(start: &BytesStart<'_>, cell: &mut Option<Cell>) -> Result<()> {
    match start.name().as_ref() {
        "office:annotation" => {
            if let Some(cell) = cell.as_mut() {
                if cell.annotation_depth > 0 {
                    return Err(xml_error("nested annotations are not supported"));
                }
                cell.annotation_depth = 1;
            }
        }
        "text:p" => {
            if let Some(cell) = active_string_cell(cell) {
                if !cell.first_paragraph {
                    add_text(cell, 1)?;
                }
                cell.first_paragraph = false;
            }
        }
        "text:s" => {
            if let Some(cell) = active_string_cell(cell) {
                let spaces = usize::try_from(repeat(start, "text:c")?).map_err(xml_error)?;
                add_text(cell, spaces)?;
            }
        }
        _ => {}
    }
    Ok(())
}
