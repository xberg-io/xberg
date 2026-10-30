//! Bounded CLX records and character properties parsed once per group. ~keep

use super::super::chpx;
use crate::error::{Result, XbergError};
use crate::extractors::security::SecurityBudget;

pub(super) struct PieceTable<'a> {
    pub(super) plc_pcd: &'a [u8],
    pub(super) scripts: Vec<chpx::ScriptProperty>,
}

pub(super) fn read_piece_table<'a>(clx: &'a [u8], budget: &mut SecurityBudget) -> Result<Option<PieceTable<'a>>> {
    let mut pos = 0;
    let mut scripts = Vec::new();
    while let Some(&kind) = clx.get(pos) {
        budget
            .step()
            .map_err(|error| XbergError::validation(error.to_string()))?;
        match kind {
            2 => {
                let remaining = &clx[pos..];
                let plc_pcd = remaining
                    .get(5..)
                    .ok_or_else(|| XbergError::parsing("Pcdt truncated at lcb"))?;
                return Ok(Some(PieceTable { plc_pcd, scripts }));
            }
            1 => {
                let (end, script) = read_piece_property(&clx[pos..], budget)?;
                scripts.push(script);
                pos += end;
            }
            _ => break,
        }
    }
    if !scripts.is_empty() {
        return Err(XbergError::parsing("DOC complex properties have no valid piece table"));
    }
    Ok(None)
}

fn read_piece_property(remaining: &[u8], budget: &mut SecurityBudget) -> Result<(usize, chpx::ScriptProperty)> {
    const MAX_GROUP_BYTES: i16 = 0x3fa2;
    let header = remaining
        .get(1..3)
        .ok_or_else(|| XbergError::parsing("DOC property group header is truncated"))?;
    let length = i16::from_le_bytes([header[0], header[1]]);
    if !(0..=MAX_GROUP_BYTES).contains(&length) {
        return Err(XbergError::parsing(
            "DOC property group length is outside its valid range",
        ));
    }
    let end = 3 + length as usize;
    let properties = remaining
        .get(3..end)
        .ok_or_else(|| XbergError::parsing("DOC property group is truncated"))?;
    Ok((end, chpx::piece_script_property(properties, budget)?))
}
