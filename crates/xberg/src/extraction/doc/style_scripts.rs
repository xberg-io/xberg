//! Bounded resolution of the character-script properties in Word's style hierarchy. ~keep

use super::{DOC_WARNING_SOURCE, Script, ScriptProperty, piece_script_property, step};
use crate::error::Result;
use crate::extractors::security::SecurityBudget;
use crate::types::ProcessingWarning;

const STYLE_SHEET_PAIR: usize = 1;
const NO_BASE: u16 = 0x0fff;
const MAX_STYLE_DEPTH: usize = 64;

#[derive(Default)]
pub(super) struct StyleScripts(Vec<Option<Script>>);

struct Definition {
    kind: u16,
    base: u16,
    script: ScriptProperty,
}

impl StyleScripts {
    pub(super) fn read(
        word: &[u8],
        table: &[u8],
        pairs: usize,
        budget: &mut SecurityBudget,
        warnings: &mut Vec<ProcessingWarning>,
    ) -> Result<Self> {
        let Some((start, length)) = super::papx::read_fc_lcb(word, pairs, STYLE_SHEET_PAIR) else {
            return Ok(Self::default());
        };
        if length == 0 {
            return Ok(Self::default());
        }
        let sheet = start.checked_add(length).and_then(|end| table.get(start..end));
        let definitions = match sheet {
            Some(sheet) => read_definitions(sheet, budget)?,
            None => None,
        };
        let mut malformed = definitions.is_none();
        let mut scripts = Vec::new();
        if let Some(definitions) = definitions {
            for index in 0..definitions.len() {
                let resolved = resolve(&definitions, index, budget)?;
                malformed |= resolved.is_none();
                scripts.push(resolved.flatten());
            }
        }
        if malformed {
            crate::core::diagnostics::push_warning(
                warnings,
                DOC_WARNING_SOURCE,
                "Malformed DOC style character properties were ignored; text and direct formatting were preserved",
            );
        }
        Ok(Self(scripts))
    }

    pub(super) fn at(&self, style: u16) -> Option<Script> {
        self.0.get(usize::from(style)).copied().flatten()
    }
}

fn read_definitions(sheet: &[u8], budget: &mut SecurityBudget) -> Result<Option<Vec<Option<Definition>>>> {
    let Some((header_size, count, base_size)) = word(sheet, 0)
        .zip(word(sheet, 2))
        .zip(word(sheet, 4))
        .map(|((header, count), base)| (usize::from(header), count, usize::from(base)))
    else {
        return Ok(None);
    };
    if !matches!(base_size, 10 | 18) || count >= 0x0ffe {
        return Ok(None);
    }
    let mut position = 2 + header_size;
    let mut definitions = Vec::new();
    for _ in 0..count {
        step(budget)?;
        let Some(length) = word(sheet, position).map(usize::from) else {
            return Ok(None);
        };
        position += 2;
        let Some(record) = sheet.get(position..position + length) else {
            return Ok(None);
        };
        position += length;
        definitions.push(if length == 0 {
            None
        } else {
            read_definition(record, base_size, budget)?
        });
    }
    Ok(Some(definitions))
}

fn read_definition(record: &[u8], base_size: usize, budget: &mut SecurityBudget) -> Result<Option<Definition>> {
    let Some(flags) = word(record, 2) else { return Ok(None) };
    let kind = flags & 0x000f;
    if !matches!(kind, 1 | 2) {
        return Ok(None);
    }
    let Some(name_length) = word(record, base_size).map(usize::from) else {
        return Ok(None);
    };
    let position = base_size + 2 + (name_length + 1) * 2;
    let Some(mut properties) = record.get(position..) else {
        return Ok(None);
    };
    if kind == 1 {
        let Some((_, remaining)) = upx(properties) else {
            return Ok(None);
        };
        properties = remaining;
    }
    let Some((properties, _)) = upx(properties) else {
        return Ok(None);
    };
    Ok(Some(Definition {
        kind,
        base: flags >> 4,
        script: piece_script_property(properties, budget)?,
    }))
}

fn resolve(
    definitions: &[Option<Definition>],
    index: usize,
    budget: &mut SecurityBudget,
) -> Result<Option<Option<Script>>> {
    let Some(Some(first)) = definitions.get(index) else {
        return Ok(Some(None));
    };
    let mut current = index;
    let mut visited = Vec::new();
    let mut script = None;
    for _ in 0..MAX_STYLE_DEPTH {
        step(budget)?;
        if visited.contains(&current) {
            return Ok(None);
        }
        visited.push(current);
        let Some(Some(definition)) = definitions.get(current) else {
            return Ok(None);
        };
        if definition.kind != first.kind {
            return Ok(None);
        }
        match definition.script {
            ScriptProperty::Malformed => return Ok(None),
            ScriptProperty::Override(value) if script.is_none() => script = Some(value),
            _ => {}
        }
        if definition.base == NO_BASE {
            return Ok(Some(script));
        }
        current = usize::from(definition.base);
    }
    Ok(None)
}

fn word(bytes: &[u8], position: usize) -> Option<u16> {
    let bytes = bytes.get(position..position + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn upx(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let length = usize::from(word(bytes, 0)?);
    Some((bytes.get(2..2 + length)?, bytes.get(2 + length + length % 2..)?))
}
