use crate::error::{Result, XbergError};

const COMPONENTS: usize = 3;
const MAX_HEADER_SEGMENTS: usize = 128;

#[derive(Clone, Copy)]
pub(super) struct Header {
    pub width: u32,
    pub height: u32,
    pub horizontal_sampling: u8,
    pub vertical_sampling: u8,
    pub metadata_bytes: u64,
    pub metadata_segments: usize,
    component_ids: [u8; COMPONENTS],
}

struct Segment<'a> {
    marker: u8,
    payload: &'a [u8],
    end: usize,
}

pub(super) fn invalid(message: &str) -> XbergError {
    XbergError::Validation {
        message: format!("Reduced OCR JPEG: {message}"),
        source: None,
    }
}

fn segment(bytes: &[u8], mut position: usize) -> Result<Segment<'_>> {
    if bytes.get(position) != Some(&0xff) {
        return Err(invalid("expected a header marker"));
    }
    while bytes.get(position) == Some(&0xff) {
        position += 1;
    }
    let marker = *bytes.get(position).ok_or_else(|| invalid("truncated marker"))?;
    position += 1;
    let length_bytes = bytes
        .get(position..position + 2)
        .ok_or_else(|| invalid("truncated segment length"))?;
    let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
    if length < 2 {
        return Err(invalid("segment length must include its two length bytes"));
    }
    let end = position
        .checked_add(length)
        .ok_or_else(|| invalid("segment length overflow"))?;
    let payload = bytes
        .get(position + 2..end)
        .ok_or_else(|| invalid("segment exceeds the encoded image"))?;
    Ok(Segment { marker, payload, end })
}

fn is_frame(marker: u8) -> bool {
    matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
}

pub(super) fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut position = 2;
    while position < bytes.len() {
        let value = segment(bytes, position).ok()?;
        if is_frame(value.marker) {
            let header = value.payload.get(..6)?;
            return Some((
                u32::from(u16::from_be_bytes([header[3], header[4]])),
                u32::from(u16::from_be_bytes([header[1], header[2]])),
            ));
        }
        if value.marker == 0xda {
            return None;
        }
        position = value.end;
    }
    None
}

fn frame(payload: &[u8]) -> Result<Header> {
    if payload.len() != 6 + COMPONENTS * 3 || payload[0] != 8 || usize::from(payload[5]) != COMPONENTS {
        return Err(invalid(
            "reduced decoding supports only 8-bit three-component baseline JPEG",
        ));
    }
    let width = u32::from(u16::from_be_bytes([payload[3], payload[4]]));
    let height = u32::from(u16::from_be_bytes([payload[1], payload[2]]));
    if width == 0 || height == 0 {
        return Err(invalid("baseline JPEG dimensions must be nonzero"));
    }
    let mut component_ids = [0; COMPONENTS];
    let (mut horizontal_sampling, mut vertical_sampling, mut blocks) = (0, 0, 0u32);
    for (index, fields) in payload[6..].chunks_exact(3).enumerate() {
        let (horizontal, vertical) = (fields[1] >> 4, fields[1] & 15);
        if !(1..=4).contains(&horizontal) || !(1..=4).contains(&vertical) || fields[2] > 3 {
            return Err(invalid(
                "unsupported baseline JPEG component sampling or quantization selector",
            ));
        }
        if component_ids[..index].contains(&fields[0]) {
            return Err(invalid("duplicate baseline JPEG component identifier"));
        }
        component_ids[index] = fields[0];
        horizontal_sampling = horizontal_sampling.max(horizontal);
        vertical_sampling = vertical_sampling.max(vertical);
        blocks += u32::from(horizontal) * u32::from(vertical);
    }
    if blocks > 10 {
        return Err(invalid("baseline JPEG exceeds ten blocks per MCU"));
    }
    Ok(Header {
        width,
        height,
        horizontal_sampling,
        vertical_sampling,
        metadata_bytes: 0,
        metadata_segments: 0,
        component_ids,
    })
}

fn validate_huffman(mut payload: &[u8]) -> Result<()> {
    while !payload.is_empty() {
        let table = payload.get(..17).ok_or_else(|| invalid("truncated Huffman table"))?;
        if table[0] >> 4 > 1 || table[0] & 15 > 3 {
            return Err(invalid("unsupported Huffman table selector"));
        }
        let values = table[1..].iter().map(|value| usize::from(*value)).sum::<usize>();
        if values > 256 {
            return Err(invalid("Huffman table exceeds 256 symbols"));
        }
        payload = payload
            .get(17 + values..)
            .ok_or_else(|| invalid("truncated Huffman symbols"))?;
    }
    Ok(())
}

fn validate_quantization(mut payload: &[u8]) -> Result<()> {
    const TABLE_BYTES: usize = 65;
    while !payload.is_empty() {
        let table = payload
            .get(..TABLE_BYTES)
            .ok_or_else(|| invalid("truncated quantization table"))?;
        if table[0] >> 4 != 0 || table[0] & 15 > 3 {
            return Err(invalid("baseline JPEG requires 8-bit quantization tables"));
        }
        payload = &payload[TABLE_BYTES..];
    }
    Ok(())
}

fn validate_scan(payload: &[u8], header: &Header) -> Result<()> {
    if payload.len() != 1 + COMPONENTS * 2 + 3 || usize::from(payload[0]) != COMPONENTS {
        return Err(invalid("reduced decoding requires one complete interleaved scan"));
    }
    let mut ids = [0; COMPONENTS];
    for (index, fields) in payload[1..1 + COMPONENTS * 2].chunks_exact(2).enumerate() {
        if !header.component_ids.contains(&fields[0])
            || ids[..index].contains(&fields[0])
            || fields[1] >> 4 > 3
            || fields[1] & 15 > 3
        {
            return Err(invalid("invalid baseline JPEG scan component or table selector"));
        }
        ids[index] = fields[0];
    }
    if payload[payload.len() - 3..] != [0, 63, 0] {
        return Err(invalid(
            "reduced decoding requires a complete baseline JPEG coefficient scan",
        ));
    }
    Ok(())
}

fn validate_entropy_end(bytes: &[u8], mut position: usize) -> Result<()> {
    while position < bytes.len() {
        if bytes[position] != 0xff {
            position += 1;
            continue;
        }
        while bytes.get(position) == Some(&0xff) {
            position += 1;
        }
        match bytes.get(position) {
            Some(0x00) => position += 1,
            Some(0xd0..=0xd7) => return Err(invalid("JPEG restart intervals are unsupported in reduced decoding")),
            Some(0xd9) => return Ok(()),
            Some(_) => return Err(invalid("reduced decoding requires a single baseline JPEG scan")),
            None => break,
        }
    }
    Err(invalid("baseline JPEG is missing its end marker"))
}

fn apply_header_segment(value: &Segment<'_>, header: &mut Option<Header>) -> Result<()> {
    match value.marker {
        0xc0 if header.is_none() => *header = Some(frame(value.payload)?),
        marker if is_frame(marker) => return Err(invalid("reduced decoding supports only one baseline JPEG frame")),
        0xc4 => validate_huffman(value.payload)?,
        0xdb => validate_quantization(value.payload)?,
        // A unified downstream Rayon feature only allocates bands for nonzero DRI.
        // Refuse that path before entering the pinned decoder, regardless of its features. ~keep
        0xdd if value.payload == [0, 0] => {}
        0xdd => return Err(invalid("JPEG restart intervals are unsupported in reduced decoding")),
        // The pinned decoder fixes color interpretation at SOF, before reading these markers. ~keep
        0xe0 | 0xee if header.is_some() => {
            return Err(invalid("color marker after frame is unsupported in reduced decoding"));
        }
        0xe0..=0xef | 0xfe => {}
        _ => return Err(invalid("unsupported marker in reduced baseline JPEG")),
    }
    Ok(())
}

pub(super) fn inspect(bytes: &[u8]) -> Result<Header> {
    let mut position = 2;
    let (mut header, mut metadata_bytes, mut metadata_segments) = (None, 0u64, 0usize);
    for _ in 0..MAX_HEADER_SEGMENTS {
        let value = segment(bytes, position)?;
        if value.marker == 0xda {
            let mut header = header.ok_or_else(|| invalid("scan precedes its frame"))?;
            validate_scan(value.payload, &header)?;
            validate_entropy_end(bytes, value.end)?;
            header.metadata_bytes = metadata_bytes;
            header.metadata_segments = metadata_segments;
            return Ok(header);
        }
        apply_header_segment(&value, &mut header)?;
        if matches!(value.marker, 0xe0..=0xef | 0xfe) {
            metadata_segments += 1;
            metadata_bytes = metadata_bytes
                .checked_add(value.payload.len() as u64)
                .ok_or_else(|| invalid("metadata byte count overflow"))?;
        }
        position = value.end;
    }
    Err(invalid("JPEG header segment limit exceeded"))
}
