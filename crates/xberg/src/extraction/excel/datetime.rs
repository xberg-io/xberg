use std::collections::HashSet;
use std::io::{Read, Seek};

use calamine::{Cell, Data, ExcelDateTime, Range};

// ~keep: Calamine retains only the duration flag, so other kinds must be inferred from the serial.
pub(super) fn format_excel_datetime(dt: &ExcelDateTime) -> String {
    if dt.is_duration() {
        let total_seconds = (dt.as_f64() * 86_400.0).round() as i64;
        let sign = if total_seconds < 0 { "-" } else { "" };
        let seconds = total_seconds.unsigned_abs();
        return format!("{sign}{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60);
    }
    let serial = dt.as_f64();
    let (year, month, day, hour, min, sec, _milli) = dt.to_ymd_hms_milli();
    if (0.0..1.0).contains(&serial) {
        format!("{hour:02}:{min:02}:{sec:02}")
    } else if serial.fract() == 0.0 {
        format!("{year:04}-{month:02}-{day:02}")
    } else {
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
    }
}

/// Positions of 1904-system zero serials that render as a time rather than the epoch date.
///
/// Serial 0 is 1904-01-01 there, but calamine turns every serial in `0..1` into the same `ExcelDateTime`, so a
/// date-formatted 0 and a time-formatted 0 differ only in the cell's number format. XLSX supplies it per cell, and
/// only for the zero-serial cells, so a sheet full of styled cells costs no memory. XLS and XLSB expose no per-cell
/// format, so all their zero serials are read as the date; a time-only zero in a 1904 XLS/XLSB workbook shows as
/// 1904-01-01. Cells on the default style (index 0) carry no style entry and are read as the date as well.
pub(super) type TimeOnlyZeros = HashSet<(u32, u32)>;

/// One number-format section: its `[>0]`-style condition, if any, and its text without literals and brackets.
struct FormatSection {
    condition: Option<String>,
    text: String,
}

fn format_sections(code: &str) -> Vec<FormatSection> {
    let mut sections = vec![FormatSection {
        condition: None,
        text: String::new(),
    }];
    let (mut quoted, mut escaped, mut bracket) = (false, false, None::<String>);
    for symbol in code.chars() {
        let section = sections.last_mut().expect("one section");
        if escaped {
            escaped = false;
        } else if quoted {
            quoted = symbol != '"';
        } else if let Some(content) = bracket.as_mut() {
            if symbol == ']' {
                if content.starts_with(['<', '>', '=']) {
                    section.condition = bracket.take();
                } else {
                    if content.chars().all(|c| matches!(c, 'h' | 'H' | 'm' | 'M' | 's' | 'S')) {
                        // Elapsed-time tokens such as [h] or [mm] are always time components, unlike colors and
                        // locales, and [mm] must not be read as a month.
                        section
                            .text
                            .push(if content.starts_with(['h', 'H']) { 'H' } else { 'S' });
                    }
                    bracket = None;
                }
            } else {
                content.push(symbol);
            }
        } else {
            match symbol {
                '\\' | '_' | '*' => escaped = true,
                '"' => quoted = true,
                '[' => bracket = Some(String::new()),
                ';' => sections.push(FormatSection {
                    condition: None,
                    text: String::new(),
                }),
                _ => section.text.push(symbol.to_ascii_uppercase()),
            }
        }
    }
    sections
}

fn condition_admits_zero(condition: &str) -> bool {
    let operator_len = condition
        .find(|c: char| !matches!(c, '<' | '>' | '='))
        .unwrap_or(condition.len());
    let (operator, operand) = condition.split_at(operator_len);
    let Ok(operand) = operand.trim().parse::<f64>() else {
        return false;
    };
    match operator {
        "<" => 0.0 < operand,
        "<=" => 0.0 <= operand,
        ">" => 0.0 > operand,
        ">=" => 0.0 >= operand,
        "=" => operand == 0.0,
        "<>" => operand != 0.0,
        _ => false,
    }
}

/// The section Excel applies to zero: the first whose condition holds, else the third of an unconditional
/// three-section format, else the first.
fn section_for_zero(sections: &[FormatSection]) -> &FormatSection {
    if sections.iter().any(|section| section.condition.is_some()) {
        let admitted = sections
            .iter()
            .find(|section| section.condition.as_deref().is_none_or(condition_admits_zero));
        return admitted.unwrap_or(&sections[sections.len() - 1]);
    }
    &sections[if sections.len() >= 3 { 2 } else { 0 }]
}

pub(super) fn is_time_only_format(code: &str, format_id: Option<u32>) -> bool {
    if code == "General" {
        // Calamine reports "General" for built-in ids it has no code for; 18-21, 32, 33 and 45-47 are clock times.
        return matches!(format_id, Some(18..=21 | 32 | 33 | 45..=47));
    }
    let sections = format_sections(code);
    let text = &section_for_zero(&sections).text;
    let meridiem = text.contains("AM/PM") || text.contains("A/P");
    let letters = text.replace("AM/PM", "").replace("A/P", "");
    let date = letters.contains(['D', 'Y']) || has_month_token(&letters);
    let time = letters.contains(['H', 'S']) || meridiem;
    time && !date
}

/// An `M` or `MM` run is minutes beside an hour or second token; longer runs and other positions are months.
fn has_month_token(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(char::is_ascii_alphabetic).collect();
    let mut index = 0;
    while index < letters.len() {
        if letters[index] != 'M' {
            index += 1;
            continue;
        }
        let start = index;
        while index < letters.len() && letters[index] == 'M' {
            index += 1;
        }
        let minute_width = index - start <= 2;
        let after_hour = start > 0 && letters[start - 1] == 'H';
        let before_second = letters.get(index) == Some(&'S');
        if !(minute_width && (after_hour || before_second)) {
            return true;
        }
    }
    false
}

fn is_zero_serial(value: &Data) -> bool {
    matches!(value, Data::DateTime(serial) if !serial.is_duration() && serial.as_f64() == 0.0)
}

pub(super) fn zero_serial_positions_in_range(range: &Range<Data>) -> Vec<(u32, u32)> {
    let Some((row_start, col_start)) = range.start() else {
        return Vec::new();
    };
    range
        .used_cells()
        .filter(|(_, _, value)| is_zero_serial(value))
        .map(|(row, col, _)| (row_start + row as u32, col_start + col as u32))
        .collect()
}

/// Streams the sheet's style ids once and keeps only the zero-serial cells (`positions`) whose format is a time,
/// stopping as soon as every one of them has been seen.
fn time_only_zero_cells<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    name: &str,
    positions: &[(u32, u32)],
) -> TimeOnlyZeros {
    let wanted: HashSet<(u32, u32)> = positions.iter().copied().collect();
    let mut time_only = TimeOnlyZeros::new();
    let Ok(mut reader) = workbook.worksheet_cells_reader(name) else {
        return time_only;
    };
    let mut remaining = wanted.len();
    while remaining > 0 {
        let Ok(Some((row, col, style_id))) = reader.next_style_id() else {
            break;
        };
        if !wanted.contains(&(row, col)) {
            continue;
        }
        remaining -= 1;
        let is_time = reader
            .styles()
            .get(style_id)
            .and_then(|style| style.number_format.as_ref())
            .is_some_and(|format| is_time_only_format(&format.format_code, format.format_id));
        if is_time {
            time_only.insert((row, col));
        }
    }
    time_only
}

fn epoch_zero_text(position: (u32, u32), time_only: &TimeOnlyZeros) -> Data {
    let text = if time_only.contains(&position) {
        "00:00:00"
    } else {
        "1904-01-01"
    };
    Data::DateTimeIso(text.to_owned())
}

pub(super) fn apply_epoch_zero_in_range(range: &mut Range<Data>, positions: &[(u32, u32)], time_only: &TimeOnlyZeros) {
    for &position in positions {
        range.set_value(position, epoch_zero_text(position, time_only));
    }
}

pub(super) fn resolve_xlsx_epoch_zero_in_range<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    name: &str,
    range: &mut Range<Data>,
) {
    let positions = zero_serial_positions_in_range(range);
    if positions.is_empty() {
        return;
    }
    let time_only = time_only_zero_cells(workbook, name, &positions);
    apply_epoch_zero_in_range(range, &positions, &time_only);
}

pub(super) fn resolve_xlsx_epoch_zero_in_cells<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    name: &str,
    cells: &mut [((u32, u32), Data)],
) {
    let positions: Vec<_> = cells
        .iter()
        .filter(|(_, value)| is_zero_serial(value))
        .map(|(position, _)| *position)
        .collect();
    if positions.is_empty() {
        return;
    }
    let time_only = time_only_zero_cells(workbook, name, &positions);
    for (position, value) in cells.iter_mut().filter(|(_, value)| is_zero_serial(value)) {
        *value = epoch_zero_text(*position, &time_only);
    }
}

pub(super) fn resolve_xlsx_epoch_zero_in_sparse<RS: Read + Seek>(
    workbook: &mut calamine::Xlsx<RS>,
    name: &str,
    cells: &mut [Cell<Data>],
) {
    let positions: Vec<_> = cells
        .iter()
        .filter(|cell| is_zero_serial(cell.get_value()))
        .map(Cell::get_position)
        .collect();
    if positions.is_empty() {
        return;
    }
    let time_only = time_only_zero_cells(workbook, name, &positions);
    for cell in cells.iter_mut().filter(|cell| is_zero_serial(cell.get_value())) {
        let position = cell.get_position();
        *cell = Cell::new(position, epoch_zero_text(position, &time_only));
    }
}

/// XLS and XLSB: no per-cell format is available, so every zero serial reads as the date.
pub(super) fn resolve_unstyled_epoch_zero_in_range(range: &mut Range<Data>) {
    let positions = zero_serial_positions_in_range(range);
    apply_epoch_zero_in_range(range, &positions, &TimeOnlyZeros::new());
}
