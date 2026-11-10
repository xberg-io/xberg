use super::*;

pub(super) fn audit_truetype(document: &PdfDocument, dict: &HashMap<String, Object>) -> ProofResult<()> {
    let unsupported = ["Encoding", "FontMatrix", "DescendantFonts", "CIDToGIDMap"];
    require(
        !unsupported.iter().any(|key| dict.contains_key(*key)),
        "unsupported simple font encoding",
    )?;
    let descriptor = dict
        .get("FontDescriptor")
        .ok_or(Failure::Declined("missing font descriptor"))?;
    let descriptor = document.resolve_object(descriptor)?;
    let descriptor = descriptor
        .as_dict()
        .ok_or(Failure::Declined("font descriptor dictionary"))?;
    require(
        descriptor.contains_key("FontFile2")
            && !descriptor.contains_key("FontFile")
            && !descriptor.contains_key("FontFile3"),
        "unsupported embedded font program",
    )?;
    let program = descriptor
        .get("FontFile2")
        .ok_or(Failure::Declined("missing font program"))?;
    let decoded = bounded_stream(document, program, MAX_FONT_BYTES)?;
    require(byte_indexed_truetype(&decoded), "unsupported embedded font cmap")?;
    audit_unicode(document, dict)
}
pub(super) fn bounded_stream(document: &PdfDocument, object: &Object, limit: usize) -> ProofResult<Vec<u8>> {
    let Object::Stream { dict, data } = document.resolve_object(object)? else {
        return Err(Failure::Declined("font audit stream"));
    };
    let mut decoded = Vec::new();
    require(data.len() <= limit, "font byte budget")?;
    match dict.get("Filter") {
        None => decoded.extend_from_slice(&data),
        Some(filter) if flate_filter(filter) => {
            flate2::read::ZlibDecoder::new(data.as_ref())
                .take(limit as u64 + 1)
                .read_to_end(&mut decoded)?;
        }
        _ => return Err(Failure::Declined("font audit stream filter")),
    }
    require(decoded.len() <= limit, "font byte budget")?;
    Ok(decoded)
}
pub(super) fn audit_unicode(document: &PdfDocument, dict: &HashMap<String, Object>) -> ProofResult<()> {
    let unicode = dict
        .get("ToUnicode")
        .ok_or(Failure::Declined("missing explicit Unicode claim map"))?;
    let decoded = bounded_stream(document, unicode, MAX_UNICODE_BYTES)?;
    let text = std::str::from_utf8(&decoded).map_err(|_| Failure::Declined("non-ASCII ToUnicode program"))?;
    require(
        !["beginbfrange", "beginnotdefrange", "usecmap"]
            .iter()
            .any(|token| text.contains(token))
            && text.matches('<').count() <= MAX_UNICODE_HEX_TOKENS,
        "unsupported ToUnicode program",
    )?;
    scalar_hex_tokens(text)?;
    let cmap = xberg_native_pdf::fonts::parse_tounicode_cmap(&decoded)?;
    require(cmap.code_width == 1 && cmap.wmode == 0, "nonbyte or vertical ToUnicode")?;
    require(
        (0..=255).all(|code| cmap.get(&code).is_none_or(|claim| claim.chars().count() == 1)),
        "non-scalar ToUnicode claim",
    )
}
pub(super) fn scalar_hex_tokens(text: &str) -> ProofResult<()> {
    for token in text.split('<').skip(1) {
        // Dictionary << is not a hexadecimal string. All admitted claims are BMP scalars. ~keep
        if token.is_empty() || token.starts_with(" /") {
            continue;
        }
        let hex = token
            .split('>')
            .next()
            .ok_or(Failure::Declined("ToUnicode hex token"))?;
        require(
            matches!(hex.len(), 2 | 4) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "non-scalar ToUnicode claim",
        )?;
        let scalar = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
        require(scalar.is_some(), "non-scalar ToUnicode claim")?;
    }
    Ok(())
}
pub(super) fn byte_indexed_truetype(bytes: &[u8]) -> bool {
    font_tables(bytes)
        .and_then(|tables| {
            let glyphs = bounded_outlines(&tables)?;
            let cmap = *tables.get(b"cmap".as_slice())?;
            let count = usize::from(read_u16(cmap, 2)?);
            if count == 0 || count > MAX_CMAP_RECORDS || read_u16(cmap, 0)? != 0 {
                return None;
            }
            for record in cmap.get(4..4 + count * 8)?.as_chunks::<8>().0 {
                if read_u16(record, 0)? != 1 || read_u16(record, 2)? != 0 {
                    return None;
                }
                let offset = read_u32(record, 4)? as usize;
                bounded_byte_map(cmap.get(offset..)?, glyphs)?;
            }
            Some(())
        })
        .is_some()
}
pub(super) fn font_tables(bytes: &[u8]) -> Option<HashMap<&[u8], &[u8]>> {
    if bytes.get(..4)? != [0, 1, 0, 0] || bytes.len() > MAX_FONT_BYTES {
        return None;
    }
    let count = usize::from(read_u16(bytes, 4)?);
    if count == 0 || count > MAX_FONT_TABLES {
        return None;
    }
    let directory_end = 12 + count * 16;
    let mut tables = HashMap::new();
    let mut ranges = Vec::new();
    for record in bytes.get(12..directory_end)?.as_chunks::<16>().0 {
        let tag = record.get(..4)?;
        let allowed = [
            b"OS/2", b"cmap", b"cvt ", b"fpgm", b"glyf", b"head", b"hhea", b"hmtx", b"loca", b"maxp", b"name", b"post",
            b"prep",
        ];
        if !allowed.iter().any(|allowed| tag == allowed.as_slice()) || tables.contains_key(tag) {
            return None;
        }
        let start = read_u32(record, 8)? as usize;
        let end = start.checked_add(read_u32(record, 12)? as usize)?;
        if start < directory_end || ranges.iter().any(|&(a, b)| start < b && a < end) {
            return None;
        }
        ranges.push((start, end));
        tables.insert(tag, bytes.get(start..end)?);
    }
    Some(tables)
}
pub(super) fn bounded_byte_map(bytes: &[u8], glyphs: usize) -> Option<()> {
    let length = usize::from(read_u16(bytes, 2)?);
    let bytes = bytes.get(..length)?;
    match read_u16(bytes, 0)? {
        0 if length == 262 => bytes
            .get(6..)?
            .iter()
            .all(|gid| usize::from(*gid) < glyphs)
            .then_some(()),
        6 => {
            let first = usize::from(read_u16(bytes, 6)?);
            let count = usize::from(read_u16(bytes, 8)?);
            if count > MAX_GLYPHS || first + count > MAX_GLYPHS || length != 10 + count * 2 {
                return None;
            }
            bytes
                .get(10..)?
                .as_chunks::<2>()
                .0
                .iter()
                .all(|gid| usize::from(u16::from_be_bytes([gid[0], gid[1]])) < glyphs)
                .then_some(())
        }
        _ => None,
    }
}
pub(super) fn bounded_outlines(tables: &HashMap<&[u8], &[u8]>) -> Option<usize> {
    let maxp = *tables.get(b"maxp".as_slice())?;
    if read_u32(maxp, 0)? != 0x0001_0000 {
        return None;
    }
    // maxp includes hinting VM storage/stack and composite scratch maxima even for simple outlines. ~keep
    for index in 0..14 {
        let cap = match index {
            0 => MAX_GLYPHS,
            1 | 3 | 6 | 8 | 9 => MAX_POINTS,
            2 | 4 | 12 => MAX_CONTOURS,
            5 => 2,
            13 => MAX_DEPTH,
            _ => MAX_INSTRUCTIONS,
        };
        if usize::from(read_u16(maxp, 4 + index * 2)?) > cap {
            return None;
        }
    }
    for tag in [b"fpgm", b"prep", b"cvt "] {
        if tables
            .get(tag.as_slice())
            .is_some_and(|table| table.len() > MAX_INSTRUCTIONS)
        {
            return None;
        }
    }
    let glyphs = usize::from(read_u16(maxp, 4)?);
    let head = *tables.get(b"head".as_slice())?;
    let format = read_u16(head, 50)?;
    if glyphs == 0 || format > 1 || read_u16(head, 52)? != 0 {
        return None;
    }
    let hhea = *tables.get(b"hhea".as_slice())?;
    let metrics = usize::from(read_u16(hhea, 34)?);
    if metrics == 0 || metrics > glyphs || tables.get(b"hmtx".as_slice())?.len() < metrics * 4 + (glyphs - metrics) * 2
    {
        return None;
    }
    validate_loca(tables, glyphs, format)?;
    Some(glyphs)
}
pub(super) fn validate_loca(tables: &HashMap<&[u8], &[u8]>, glyphs: usize, format: u16) -> Option<()> {
    let loca = *tables.get(b"loca".as_slice())?;
    let glyf = *tables.get(b"glyf".as_slice())?;
    let offset = |index| -> Option<usize> {
        if format == 0 {
            Some(usize::from(read_u16(loca, index * 2)?) * 2)
        } else {
            Some(read_u32(loca, index * 4)? as usize)
        }
    };
    let mut total_points = 0;
    for index in 0..glyphs {
        let start = offset(index)?;
        let end = offset(index + 1)?;
        let data = glyf.get(start..end)?;
        if !data.is_empty() {
            total_points += simple_outline_points(data)?;
            if total_points > MAX_FONT_POINTS {
                return None;
            }
        }
    }
    Some(())
}
pub(super) fn simple_outline_points(data: &[u8]) -> Option<usize> {
    let contours = usize::from(read_u16(data, 0)?);
    // Negative contour counts are composites and are deliberately unsupported. ~keep
    if contours > MAX_CONTOURS {
        return None;
    }
    let mut points = 0;
    for index in 0..contours {
        let next = usize::from(read_u16(data, 10 + index * 2)?) + 1;
        if next <= points || next > MAX_POINTS {
            return None;
        }
        points = next;
    }
    let instructions = usize::from(read_u16(data, 10 + contours * 2)?);
    if instructions > MAX_INSTRUCTIONS {
        return None;
    }
    let flags = 12 + contours * 2 + instructions;
    validate_coordinates(data, flags, points)?;
    Some(points)
}
pub(super) fn validate_coordinates(data: &[u8], mut cursor: usize, points: usize) -> Option<()> {
    let mut seen = 0;
    let mut coordinates = 0;
    data.get(..cursor)?;
    while seen < points {
        let flag = *data.get(cursor)?;
        cursor += 1;
        let repeat = if flag & 8 != 0 {
            cursor += 1;
            usize::from(*data.get(cursor - 1)?) + 1
        } else {
            1
        };
        seen += repeat;
        if seen > points {
            return None;
        }
        coordinates += repeat * (coordinate_bytes(flag, 2, 16) + coordinate_bytes(flag, 4, 32));
    }
    data.get(cursor..cursor.checked_add(coordinates)?)?;
    Some(())
}
pub(super) fn coordinate_bytes(flag: u8, short: u8, same: u8) -> usize {
    match (flag & short, flag & same) {
        (0, 0) => 2,
        (0, _) => 0,
        _ => 1,
    }
}
pub(super) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}
pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
