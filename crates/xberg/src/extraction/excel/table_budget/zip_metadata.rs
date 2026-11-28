//! Bound XLSX shared-string allocation before the backend constructor reserves its vector.

use std::io::{BufReader, Read, Seek};

use quick_xml::events::Event;

use super::{SecurityBudget, SecurityLimits};
use crate::{Result, XbergError};

/// Validate every shared-strings part against the budget, sizing it by the strings it actually holds.
///
/// Calamine reserves a vector of `uniqueCount` strings before reading any of them, so a declared count is only
/// trusted while that reservation fits the budget on top of the strings present. Returns `true` when some part
/// declares more than fits; the caller must then drop the attribute before handing the workbook to calamine.
pub(in crate::extraction::excel) fn validate_xlsx_string_parts<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    limits: &SecurityLimits,
) -> Result<bool> {
    let mut needs_rewrite = false;
    for index in 0..archive.len() {
        let member = archive.by_index(index).map_err(xml_error)?;
        let name = member.name().map_err(xml_error)?;
        if !is_shared_strings_part(&name) {
            continue;
        }
        drop(name);
        let bounded = member.take((limits.max_archive_size as u64).saturating_add(1));
        needs_rewrite |= validate_strings(BufReader::new(bounded), limits)?;
    }
    Ok(needs_rewrite)
}

fn xml_error(error: impl std::fmt::Display) -> XbergError {
    XbergError::parsing(format!("Invalid spreadsheet shared strings: {error}"))
}

pub(in crate::extraction::excel) fn is_shared_strings_part(name: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .is_some_and(|basename| basename.eq_ignore_ascii_case("sharedStrings.xml"))
}

/// Returns whether the declared `uniqueCount` reservation would not fit the budget left after the actual strings.
fn validate_strings(reader: impl std::io::BufRead, limits: &SecurityLimits) -> Result<bool> {
    let mut declared = None;
    let mut reader = quick_xml::Reader::from_reader(reader);
    reader.config_mut().expand_empty_elements = true;
    let mut budget = SecurityBudget::from_limits(limits);
    let mut count = 0usize;
    let mut capacity_bytes = 0usize;
    let mut string_depth = 0usize;
    let mut buffer = Vec::new();
    loop {
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
                        if let Ok(value) = value.parse::<usize>() {
                            declared = Some(value);
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
            Event::CData(text) if string_depth > 0 => {
                budget.account_text(text.xml10_content().len())?;
            }
            Event::Eof => {
                return Ok(declared.is_some_and(|declared| {
                    declared > count && account_capacity(declared, &mut capacity_bytes, &mut budget).is_err()
                }));
            }
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

/// Drop the `uniqueCount` attribute from a shared-strings part so calamine sizes its vector from the strings it
/// reads rather than from the declared count. Returns the rewritten part and whether anything was removed.
pub(in crate::extraction::excel) fn remove_unique_count(xml: &[u8]) -> Result<(Vec<u8>, bool)> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut writer = quick_xml::Writer::new(Vec::with_capacity(xml.len()));
    let mut buffer = Vec::new();
    let mut changed = false;
    loop {
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).map_err(xml_error)?;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) if start.local_name().as_ref() == "sst" => {
                let mut rewritten = quick_xml::events::BytesStart::new(start.name().as_ref().to_owned());
                for attribute in start.attributes().with_checks(false) {
                    let attribute = attribute.map_err(xml_error)?;
                    if attribute.key.as_ref() == "uniqueCount" {
                        changed = true;
                    } else {
                        rewritten.push_attribute(attribute);
                    }
                }
                let event = if matches!(event, Event::Empty(_)) {
                    Event::Empty(rewritten)
                } else {
                    Event::Start(rewritten)
                };
                writer.write_event(event).map_err(xml_error)?;
            }
            Event::Eof => break,
            event => writer.write_event(event).map_err(xml_error)?,
        }
    }
    Ok((writer.into_inner(), changed))
}
