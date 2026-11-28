use calamine::ExcelDateTime;

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
    // ~keep Serial 0 maps to 1904-01-01 only in the 1904 system, where it is a date rather than midnight.
    let is_1904_epoch = serial == 0.0 && (year, month, day) == (1904, 1, 1);
    if (0.0..1.0).contains(&serial) && !is_1904_epoch {
        format!("{hour:02}:{min:02}:{sec:02}")
    } else if serial.fract() == 0.0 {
        format!("{year:04}-{month:02}-{day:02}")
    } else {
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
    }
}
