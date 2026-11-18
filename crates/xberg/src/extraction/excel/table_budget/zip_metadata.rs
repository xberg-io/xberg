//! Bound XLSX shared-string allocation before the backend constructor reserves its vector.

use std::io::{BufReader, Read, Seek};

use quick_xml::events::Event;

use super::{ExcelReadCancellation, SecurityBudget, SecurityLimits};
use crate::{Result, XbergError};

pub(in crate::extraction::excel) fn validate_xlsx_string_parts<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &SecurityLimits,
    cancel: ExcelReadCancellation<'_>,
) -> Result<()> {
    for index in 0..archive.len() {
        cancel.check()?;
        let member = archive.by_index(index).map_err(xml_error)?;
        let name = member.name().map_err(xml_error)?;
        let basename = name.rsplit(['/', '\\']).next().unwrap_or(name.as_ref());
        if !basename.eq_ignore_ascii_case("sharedStrings.xml") {
            continue;
        }
        drop(name);
        let bounded = member.take((limits.max_archive_size as u64).saturating_add(1));
        validate_strings(BufReader::new(bounded), limits, cancel)?;
    }
    Ok(())
}

fn xml_error(error: impl std::fmt::Display) -> XbergError {
    XbergError::parsing(format!("Invalid spreadsheet shared strings: {error}"))
}

fn validate_strings(
    reader: impl std::io::BufRead,
    limits: &SecurityLimits,
    cancel: ExcelReadCancellation<'_>,
) -> Result<()> {
    let mut reader = quick_xml::Reader::from_reader(reader);
    reader.config_mut().expand_empty_elements = true;
    let mut budget = SecurityBudget::from_limits(limits);
    let mut count = 0usize;
    let mut capacity_bytes = 0usize;
    let mut string_depth = 0usize;
    let mut buffer = Vec::new();
    loop {
        cancel.check()?;
        budget.step()?;
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(xml_error)?;
        if reader.buffer_position() > limits.max_archive_size as u64 {
            return Err(XbergError::validation(
                "Shared strings exceed the configured archive size".to_string(),
            ));
        }
        match event {
            Event::Start(start) if start.local_name().as_ref() == "sst" => {
                for attribute in start.attributes().with_checks(false) {
                    let attribute = attribute.map_err(xml_error)?;
                    if attribute.key.as_ref() == "uniqueCount" {
                        let value = attribute
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(xml_error)?;
                        if let Ok(declared) = value.parse::<usize>() {
                            account_capacity(declared, &mut capacity_bytes, &mut budget)?;
                        }
                    }
                }
            }
            Event::Start(start) if start.local_name().as_ref() == "si" => {
                count = count.checked_add(1).ok_or_else(|| xml_error("string count overflow"))?;
                account_capacity(count, &mut capacity_bytes, &mut budget)?;
                string_depth += 1;
            }
            Event::End(end) if end.local_name().as_ref() == "si" => string_depth = string_depth.saturating_sub(1),
            Event::Text(text) if string_depth > 0 => {
                let text = text.xml10_content();
                budget.check_entity(&text)?;
                budget.account_text(text.len())?;
            }
            Event::GeneralRef(reference) if string_depth > 0 => {
                let escaped = format!("&{};", reference.as_ref());
                let text = quick_xml::escape::unescape(&escaped).map_err(xml_error)?;
                budget.account_text(text.len())?;
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn account_capacity(count: usize, capacity_bytes: &mut usize, budget: &mut SecurityBudget) -> Result<()> {
    // ~keep Calamine0.37 reserves uniqueCount String slots before reading any shared-string values.
    let bytes = count
        .checked_mul(std::mem::size_of::<String>())
        .ok_or_else(|| xml_error("string capacity overflow"))?;
    if bytes > *capacity_bytes {
        budget.account_text(bytes - *capacity_bytes)?;
        *capacity_bytes = bytes;
    }
    Ok(())
}
