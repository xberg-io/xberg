use super::*;
use calamine::{ExcelDateTime, ExcelDateTimeType};

fn cell(value: f64, kind: ExcelDateTimeType) -> String {
    format_cell_to_string(&Data::DateTime(ExcelDateTime::new(value, kind, false)))
}

#[test]
fn test_format_cell_value_date_time_and_duration_kinds() {
    assert_eq!(cell(45565.0, ExcelDateTimeType::DateTime), "2024-09-30");
    assert_eq!(cell(45565.5, ExcelDateTimeType::DateTime), "2024-09-30 12:00:00");
    assert_eq!(cell(0.5, ExcelDateTimeType::DateTime), "12:00:00");
    assert_eq!(cell(0.0, ExcelDateTimeType::DateTime), "00:00:00");
    assert_eq!(cell(1.5, ExcelDateTimeType::TimeDelta), "36:00:00");
    assert_eq!(cell(0.0625, ExcelDateTimeType::TimeDelta), "1:30:00");
    assert_eq!(cell(-0.25, ExcelDateTimeType::TimeDelta), "-6:00:00");
    assert_eq!(
        cell(1.0 / 24.0 + 1.0 / 1440.0 + 1.0 / 86400.0, ExcelDateTimeType::TimeDelta),
        "1:01:01"
    );
}

#[test]
fn test_format_cell_value_date_in_the_1904_system() {
    let date = Data::DateTime(ExcelDateTime::new(44103.0, ExcelDateTimeType::DateTime, true));
    assert_eq!(format_cell_to_string(&date), "2024-09-30");
}

#[test]
fn should_render_serial_zero_in_the_1904_system_as_its_epoch_date_gh3441() {
    let epoch = Data::DateTime(ExcelDateTime::new(0.0, ExcelDateTimeType::DateTime, true));
    assert_eq!(format_cell_to_string(&epoch), "1904-01-01");
    let time = Data::DateTime(ExcelDateTime::new(
        0.7027777777777777,
        ExcelDateTimeType::DateTime,
        true,
    ));
    assert_eq!(format_cell_to_string(&time), "16:52:00");
    let next_day = Data::DateTime(ExcelDateTime::new(1.0, ExcelDateTimeType::DateTime, true));
    assert_eq!(format_cell_to_string(&next_day), "1904-01-02");
}
