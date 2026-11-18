//! Validate backend-consumed BIFF12 framing before its record buffer can grow.

use std::io::{BufReader, Read, Seek};

use quick_xml::events::Event;

use super::{ExcelReadCancellation, SecurityBudget, SecurityLimits};
use crate::{Result, XbergError};

fn framing_error(error: impl std::fmt::Display) -> XbergError {
    XbergError::validation(format!("Invalid XLSB record framing: {error}"))
}

pub(in crate::extraction::excel) fn validate_xlsb_parts<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &SecurityLimits,
    cancel: ExcelReadCancellation<'_>,
) -> Result<()> {
    let mut paths = ahash::AHashSet::from_iter([
        "xl/workbook.bin".to_string(),
        "xl/styles.bin".to_string(),
        "xl/sharedstrings.bin".to_string(),
    ]);
    collect_sheet_paths(archive, limits, &mut paths, cancel)?;
    for index in 0..archive.len() {
        cancel.check()?;
        let member = archive.by_index(index).map_err(framing_error)?;
        let name = member.name().map_err(framing_error)?;
        let normalized = name.replace('\\', "/").to_ascii_lowercase();
        if !paths.contains(&normalized) {
            continue;
        }
        drop(name);
        let size = member.size();
        validate_records(BufReader::new(member), size, limits, cancel)?;
    }
    Ok(())
}

fn collect_sheet_paths<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &SecurityLimits,
    paths: &mut ahash::AHashSet<String>,
    cancel: ExcelReadCancellation<'_>,
) -> Result<()> {
    // ~keep Calamine resolves only this exact relationships member, then lowercases/backslash-normalizes target lookup.
    let Ok(member) = archive.by_name("xl/_rels/workbook.bin.rels") else {
        return Ok(());
    };
    let mut reader = quick_xml::Reader::from_reader(BufReader::new(
        member.take((limits.max_archive_size as u64).saturating_add(1)),
    ));
    reader.config_mut().expand_empty_elements = true;
    let mut buffer = Vec::new();
    let mut budget = SecurityBudget::from_limits(limits);
    loop {
        cancel.check()?;
        budget.step()?;
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(framing_error)?;
        if reader.buffer_position() > limits.max_archive_size as u64 {
            return Err(framing_error("relationships exceed archive size"));
        }
        match event {
            Event::Start(start) if start.name().as_ref() == "Relationship" => {
                for attribute in start.attributes().with_checks(false) {
                    let attribute = attribute.map_err(framing_error)?;
                    if attribute.key.as_ref() != "Target" {
                        continue;
                    }
                    let target = attribute
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(framing_error)?;
                    let path = format!("xl/{target}");
                    if matches!(
                        path.split('/').nth(1),
                        Some("worksheets" | "chartsheets" | "dialogsheets")
                    ) {
                        paths.insert(path.replace('\\', "/").to_ascii_lowercase());
                    }
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn read_byte(reader: &mut impl Read, remaining: &mut u64) -> Result<u8> {
    let mut byte = [0u8];
    reader.read_exact(&mut byte).map_err(framing_error)?;
    *remaining = remaining
        .checked_sub(1)
        .ok_or_else(|| framing_error("record exceeds member size"))?;
    Ok(byte[0])
}

fn validate_records(
    mut reader: impl Read,
    mut remaining: u64,
    limits: &SecurityLimits,
    cancel: ExcelReadCancellation<'_>,
) -> Result<()> {
    let mut budget = SecurityBudget::from_limits(limits);
    while remaining > 0 {
        cancel.check()?;
        budget.step()?;
        let kind = read_byte(&mut reader, &mut remaining)?;
        if kind & 0x80 != 0 {
            read_byte(&mut reader, &mut remaining)?;
        }
        let mut byte = read_byte(&mut reader, &mut remaining)?;
        let mut length = u64::from(byte & 0x7f);
        for shift in 1..4 {
            if byte & 0x80 == 0 {
                break;
            }
            byte = read_byte(&mut reader, &mut remaining)?;
            length += u64::from(byte & 0x7f) << (7 * shift);
        }
        // ~keep Calamine resizes a Vec to this length before read_exact discovers a truncated payload.
        if length > remaining {
            return Err(framing_error("declared payload exceeds remaining member bytes"));
        }
        if length > limits.max_content_size as u64 {
            return Err(framing_error("record buffer exceeds configured content size"));
        }
        let copied = std::io::copy(
            &mut cancel.guard_reader(reader.by_ref().take(length)),
            &mut std::io::sink(),
        );
        cancel.check()?;
        let copied = copied.map_err(framing_error)?;
        if copied != length {
            return Err(framing_error("truncated record payload"));
        }
        remaining -= length;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn should_bound_xlsb_record_buffers_at_the_content_limit_gh2296() {
        let record = [0x80, 1, 5, 1, 2, 3, 4, 5];
        let low = SecurityLimits {
            max_content_size: 4,
            ..Default::default()
        };
        assert!(
            validate_records(
                Cursor::new(record),
                record.len() as u64,
                &low,
                ExcelReadCancellation::default()
            )
            .unwrap_err()
            .to_string()
            .contains("record buffer exceeds")
        );
        let exact = SecurityLimits {
            max_content_size: 5,
            ..Default::default()
        };
        validate_records(
            Cursor::new(record),
            record.len() as u64,
            &exact,
            ExcelReadCancellation::default(),
        )
        .unwrap();
    }

    #[test]
    fn should_reject_truncated_xlsb_framing_without_buffer_growth_gh2296() {
        for record in [&[0x80][..], &[1, 0x80][..], &[1, 5, 1][..]] {
            assert!(
                validate_records(
                    Cursor::new(record),
                    record.len() as u64,
                    &SecurityLimits::default(),
                    ExcelReadCancellation::default()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn should_not_parse_unrelated_vba_compound_members_as_biff12_gh2296() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("xl/workbook.bin", options).unwrap();
        writer.write_all(&[1, 1, 7]).unwrap();
        writer.start_file("xl/vbaProject.bin", options).unwrap();
        writer.write_all(&[0xd0, 0xcf, 0x11, 0xe0]).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        validate_xlsb_parts(
            &mut archive,
            &SecurityLimits::default(),
            ExcelReadCancellation::default(),
        )
        .unwrap();
    }
}
