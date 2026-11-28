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

fn xlsx_with_zero_serials(date1904: bool) -> Vec<u8> {
    use std::io::Write;
    let original = make_xlsx_with_worksheet(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1">
        <c r="A1" s="1"><v>0</v></c><c r="B1" s="2"><v>0</v></c><c r="C1" s="2"><v>0.7027777777777777</v></c>
        </row></sheetData></worksheet>"#,
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().unwrap().into_owned();
        let mut xml = String::new();
        entry.read_to_string(&mut xml).unwrap();
        if name == "xl/workbook.xml" && date1904 {
            xml = xml.replace("<sheets>", r#"<workbookPr date1904="1"/><sheets>"#);
        } else if name == "xl/_rels/workbook.xml.rels" {
            xml = xml.replace("</Relationships>", r#"<Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#);
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    writer
        .start_file("xl/styles.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(
            br#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <numFmts count="2"><numFmt numFmtId="164" formatCode="dd/mm/yyyy"/><numFmt numFmtId="165" formatCode="h:mm"/></numFmts>
            <cellXfs count="3"><xf numFmtId="0"/><xf numFmtId="164" applyNumberFormat="1"/><xf numFmtId="165" applyNumberFormat="1"/></cellXfs>
            </styleSheet>"#,
        )
        .unwrap();
    writer.finish().unwrap().into_inner()
}

fn first_row(bytes: &[u8]) -> Vec<String> {
    let (workbook, _) = read_excel_bytes(bytes, ".xlsx", &SecurityLimits::default()).unwrap();
    workbook.sheets[0].table_cells.as_ref().unwrap()[0].clone()
}

#[test]
fn should_render_a_date_formatted_zero_in_the_1904_system_as_its_epoch_date_gh3441() {
    assert_eq!(first_row(&xlsx_with_zero_serials(true))[0], "1904-01-01");
}

#[test]
fn should_keep_a_time_formatted_zero_in_the_1904_system_as_midnight_gh3441() {
    let row = first_row(&xlsx_with_zero_serials(true));
    assert_eq!(&row[1..], ["00:00:00", "16:52:00"]);
}

#[test]
fn should_keep_serial_zero_in_the_1900_system_as_midnight_gh3441() {
    assert_eq!(
        first_row(&xlsx_with_zero_serials(false)),
        ["00:00:00", "00:00:00", "16:52:00"]
    );
}

#[test]
fn should_read_every_zero_serial_without_cell_styles_as_the_date_for_xls_and_xlsb_gh3441() {
    let zero = Data::DateTime(ExcelDateTime::new(0.0, ExcelDateTimeType::DateTime, true));
    let mut range = calamine::Range::new((0, 0), (0, 1));
    range.set_value((0, 0), zero.clone());
    range.set_value(
        (0, 1),
        Data::DateTime(ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, true)),
    );
    crate::extraction::excel::datetime::resolve_epoch_zero_in_range(&mut range, Some(&calamine::StyleRange::default()));
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::DateTimeIso("1904-01-01".to_owned()))
    );
    assert_eq!(
        range.get_value((0, 1)).map(format_cell_to_string).as_deref(),
        Some("12:00:00")
    );
    let mut untouched = calamine::Range::new((0, 0), (0, 0));
    untouched.set_value((0, 0), zero.clone());
    crate::extraction::excel::datetime::resolve_epoch_zero_in_range(&mut untouched, None);
    assert_eq!(untouched.get_value((0, 0)), Some(&zero));
}
