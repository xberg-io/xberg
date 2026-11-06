use super::{
    BitReader, Error, Result, RowStatus, append_transition_row, decode_row_g4, packed_output_buffer, transition_buffer,
};

/// Decode the complete byte-unaligned T.6 stream used by a geometry probe.
#[doc(hidden)]
pub fn decode_complete_group4(data: &[u8], width: u16, height: u32, byte_limit: usize) -> Result<Vec<u8>> {
    let invalid = || Error::Decode("CCITT Group 4 probe requires complete rows and an exact EOFB".to_string());
    let columns = usize::from(width);
    let rows = usize::try_from(height).map_err(|_| invalid())?;
    let row_bytes = columns.div_ceil(8);
    let packed_bytes = row_bytes.checked_mul(rows).ok_or_else(invalid)?;
    let transition_bytes = columns.checked_mul(2 * size_of::<u16>()).ok_or_else(invalid)?;
    let working_bytes = data
        .len()
        .checked_add(packed_bytes)
        .and_then(|bytes| bytes.checked_add(transition_bytes))
        .and_then(|bytes| bytes.checked_add(row_bytes));
    let minimum_bits = rows.checked_add(24).ok_or_else(invalid)?;
    if data.len().checked_mul(8).is_none_or(|bits| bits < minimum_bits)
        || width == 0
        || rows == 0
        || working_bytes.is_none_or(|bytes| bytes > byte_limit)
    {
        return Err(Error::Decode(
            "CCITT Group 4 probe exceeds its workspace or dimensions".to_string(),
        ));
    }
    let mut reader = BitReader::new(data);
    let mut reference = transition_buffer(width)?;
    let mut current = transition_buffer(width)?;
    let mut packed = packed_output_buffer(row_bytes, Some(height))?;
    for row in 0..rows {
        current.clear();
        if !matches!(
            decode_row_g4::<true>(&mut reader, &reference, width, &mut current),
            RowStatus::Ok
        ) {
            return Err(Error::Decode(format!("Invalid CCITT Group 4 probe row {}", row + 1)));
        }
        append_transition_row(&mut packed, &current, columns)?;
        std::mem::swap(&mut reference, &mut current);
    }
    if packed.len() != packed_bytes || !complete_group4_end(&mut reader) {
        return Err(invalid());
    }
    Ok(packed)
}

fn complete_group4_end(reader: &mut BitReader<'_>) -> bool {
    for _ in 0..2 {
        if reader.peek(12) != Some(1) {
            return false;
        }
        reader.consume(12);
    }
    let padding = reader.bits_to_byte_boundary();
    if reader.peek(padding) != Some(0) {
        return false;
    }
    reader.consume(padding);
    reader.eod()
}
