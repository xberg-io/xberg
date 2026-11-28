use calamine::{Cell, Data, ExcelDateTime, Range, StyleRange};

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

/// Workbooks of the 1904 date system, each with the cell styles that tell a date from a time.
///
/// Serial 0 is 1904-01-01 there, but calamine turns every serial in `0..1` into the same `ExcelDateTime`, so a
/// date-formatted 0 and a time-formatted 0 differ only in the cell's number format. XLSX supplies it per cell. XLS
/// and XLSB expose no per-cell format, so their zero serials are all read as the date; a time-only zero in a 1904
/// XLS/XLSB workbook therefore shows as 1904-01-01.
pub(super) type EpochStyles<'a> = Option<&'a StyleRange>;

/// The format sections' literal-free text: quoted text, escaped characters and `[...]` conditions removed.
fn format_sections(code: &str) -> Vec<String> {
    let mut sections = vec![String::new()];
    let (mut quoted, mut escaped, mut bracketed) = (false, false, false);
    for symbol in code.chars() {
        if escaped {
            escaped = false;
        } else if quoted {
            quoted = symbol != '"';
        } else {
            match symbol {
                '\\' | '_' | '*' => escaped = true,
                '"' => quoted = true,
                '[' => bracketed = true,
                ']' => bracketed = false,
                ';' if !bracketed => sections.push(String::new()),
                _ if !bracketed => sections
                    .last_mut()
                    .expect("one section")
                    .push(symbol.to_ascii_uppercase()),
                _ => {}
            }
        }
    }
    sections
}

pub(super) fn is_time_only_format(code: &str, format_id: Option<u32>) -> bool {
    if code == "General" {
        // Calamine reports "General" for built-in ids it has no code for; 18-21 and 45-47 are the clock-time ones.
        return matches!(format_id, Some(18..=21 | 45..=47));
    }
    let sections = format_sections(code);
    // Zero uses the third section of a three-section format and the first otherwise.
    let section = if sections.len() >= 3 {
        &sections[2]
    } else {
        &sections[0]
    };
    let date = section.contains(['D', 'Y']);
    let time = section.contains(['H', 'S']) || section.contains("AM/PM") || section.contains("A/P");
    time && !date
}

fn resolve_epoch_zero(value: &Data, position: (u32, u32), styles: &StyleRange) -> Option<Data> {
    let Data::DateTime(serial) = value else { return None };
    if serial.is_duration() || serial.as_f64() != 0.0 {
        return None;
    }
    let time_only = styles
        .get(position)
        .and_then(|style| style.number_format.as_ref())
        .is_some_and(|format| is_time_only_format(&format.format_code, format.format_id));
    let text = if time_only { "00:00:00" } else { "1904-01-01" };
    Some(Data::DateTimeIso(text.to_owned()))
}

pub(super) fn resolve_epoch_zero_in_range(range: &mut Range<Data>, styles: EpochStyles<'_>) {
    let (Some(styles), Some((row_start, col_start))) = (styles, range.start()) else {
        return;
    };
    let resolved: Vec<_> = range
        .used_cells()
        .filter_map(|(row, col, value)| {
            let position = (row_start + row as u32, col_start + col as u32);
            resolve_epoch_zero(value, position, styles).map(|data| (position, data))
        })
        .collect();
    for (position, data) in resolved {
        range.set_value(position, data);
    }
}

pub(super) fn resolve_epoch_zero_in_cells(cells: &mut [((u32, u32), Data)], styles: EpochStyles<'_>) {
    let Some(styles) = styles else { return };
    for (position, value) in cells {
        if let Some(data) = resolve_epoch_zero(value, *position, styles) {
            *value = data;
        }
    }
}

pub(super) fn resolve_epoch_zero_in_sparse(cells: &mut [Cell<Data>], styles: EpochStyles<'_>) {
    let Some(styles) = styles else { return };
    for cell in cells {
        let position = cell.get_position();
        if let Some(data) = resolve_epoch_zero(cell.get_value(), position, styles) {
            *cell = Cell::new(position, data);
        }
    }
}
