//! OfficeArt (MS-ODRAW) records, shared by the legacy binary Office formats.
//!
//! DOC and PPT store their drawings and pictures as OfficeArt record trees. A PowerPoint
//! record (MS-PPT 2.3.1) has the same 8-byte header, so the walkers here also serve the
//! `PowerPoint Document` stream. ~keep

use crate::extractors::security::{SecurityBudget, SecurityError};
use std::collections::HashMap;

/// `OfficeArtBlip` record types for the raster formats (MS-ODRAW 2.2.23).
/// `RT_BLIP_JPEG_ALT` (0xF02A) is an alternate `recType` documented for JPEG blips
/// written by older Office versions; it uses the same `OfficeArtBlipJPEG` layout as
/// 0xF01D.
pub(crate) const RT_BLIP_JPEG: u16 = 0xF01D;
const RT_BLIP_JPEG_ALT: u16 = 0xF02A;
pub(crate) const RT_BLIP_PNG: u16 = 0xF01E;
pub(crate) const RT_BLIP_DIB: u16 = 0xF01F;
/// `msofbtBlipFirst` and `msofbtBlipLast`: every `OfficeArtBlip` record type, raster or
/// metafile, lies in this range (MS-ODRAW 2.2.23). ~keep
const RT_BLIP_FIRST: u16 = 0xF018;
const RT_BLIP_LAST: u16 = 0xF117;

/// `OfficeArtDggContainer`: the drawing group, holding the `BStoreContainer`.
pub(crate) const MSOFBT_DGG_CONTAINER: u16 = 0xF000;
/// `OfficeArtBStoreContainer`: the document's blips, one file block each.
pub(crate) const MSOFBT_BSTORE_CONTAINER: u16 = 0xF001;
/// `OfficeArtDgContainer`: one drawing, holding its shapes.
pub(crate) const MSOFBT_DG_CONTAINER: u16 = 0xF002;
/// `OfficeArtSpgrContainer`: a group of shapes. PowerPoint 97-2003 draws a table as one.
pub(crate) const MSOFBT_SPGR_CONTAINER: u16 = 0xF003;
/// `OfficeArtSpContainer`: one shape.
pub(crate) const MSOFBT_SP_CONTAINER: u16 = 0xF004;
/// `OfficeArtFSP`; its `recInstance` is the shape type.
pub(crate) const MSOFBT_FSP: u16 = 0xF00A;
/// `fGroup`, the low bit of the `OfficeArtFSP` flags that follow its 4-byte `spid`.
const FSP_FLAG_GROUP: u32 = 0x0001;

/// `msofbtBSE` -- one per blip in the document's `BStoreContainer`. A shape's `pib`
/// property is a 1-based index into these, in container order (MS-ODRAW 2.2.32). ~keep
pub(crate) const MSOFBT_BSE: u16 = 0xF007;
/// Byte offset of `foDelay` inside an `msofbtBSE`'s content, past `btWin32` (1), `btMacOS`
/// (1), `rgbUid` (16), `tag` (2), `size` (4) and `cRef` (4). `foDelay` is the blip record's
/// own start offset in the delay stream: `Pictures` in a PPT, `WordDocument` in a DOC. ~keep
const BSE_FO_DELAY_OFFSET: usize = 28;
/// Byte offset of `cbName` inside an `msofbtBSE`'s content: the length of the name that
/// follows the fixed part.
const BSE_NAME_LENGTH_OFFSET: usize = 33;
/// Length of the fixed part of an `msofbtBSE`'s content; the name, then the blip when the
/// file block holds it, follow.
const BSE_HEADER_LEN: usize = 36;

/// `msofbtOPT` and its secondary/tertiary siblings, each a table of `OfficeArtFOPTE`
/// property entries (MS-ODRAW 2.3.1). `pib` is normally in the primary table, but a
/// shape that overflows its property set spills into the other two. ~keep
pub(crate) const MSOFBT_OPT: [u16; 3] = [0xF00B, 0xF121, 0xF122];
/// An `OfficeArtFOPTE` is a 2-byte `opid` followed by a 4-byte value.
const FOPT_ENTRY_LEN: usize = 6;
/// The low 14 bits of `opid` are the property number; bit 14 is `fBid` and bit 15 is
/// `fComplex`, neither of which changes where the entry sits. ~keep
const FOPT_PROPERTY_ID_MASK: u16 = 0x3FFF;
/// `pib` -- the 1-based `BStoreContainer` index of the blip a picture shape displays.
pub(crate) const MSO_PROPERTY_PIB: u16 = 0x0104;

/// Depth cap for the OfficeArt container walk. Real nesting is under a dozen levels; this
/// only stops a hostile self-describing container tree from exhausting the stack. ~keep
const MAX_OFFICE_ART_DEPTH: usize = 16;

/// One record header from the PowerPoint/OfficeArt record tree, flattened out of its
/// containers so callers can filter by type without re-walking.
#[derive(Clone, Copy)]
pub(crate) struct ArtRecord {
    rec_ver: u16,
    pub(crate) rec_instance: u16,
    pub(crate) rec_type: u16,
    pub(crate) content_start: usize,
    pub(crate) content_end: usize,
}

impl ArtRecord {
    /// A record is a container when `recVer` (the low nibble of the first field) is 0xF;
    /// its children are the records inside its own declared byte range.
    fn is_container(&self) -> bool {
        self.rec_ver == 0x0F
    }

    /// The record's content within the `data` it was read from.
    fn content<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        data.get(self.content_start..self.content_end).unwrap_or_default()
    }

    /// The whole record, header included, within the `data` it was read from.
    fn bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        data.get(self.content_start.saturating_sub(8)..self.content_end)
            .unwrap_or_default()
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// The record whose header starts at `data[pos]`, or `None` when its header or content
/// would run past `end`.
pub(crate) fn record_at(data: &[u8], pos: usize, end: usize) -> Option<ArtRecord> {
    let content_start = pos.checked_add(8).filter(|&start| start <= end)?;
    let header = data.get(pos..content_start)?;
    let rec_ver_instance = u16::from_le_bytes([header[0], header[1]]);
    let rec_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let content_end = content_start
        .checked_add(rec_len)
        .filter(|&candidate| candidate <= end)?;
    Some(ArtRecord {
        rec_ver: rec_ver_instance & 0x000F,
        rec_instance: rec_ver_instance >> 4,
        rec_type: u16::from_le_bytes([header[2], header[3]]),
        content_start,
        content_end,
    })
}

/// Collect every record header in `data[start..end]`, descending into containers.
///
/// A `recLen` that would run past `end` stops the walk at that level rather than
/// over-reading -- the stream is untrusted, and a truncated tail is the common shape of a
/// salvageable corrupt file.
pub(crate) fn collect_art_records(data: &[u8], start: usize, end: usize, depth: usize, out: &mut Vec<ArtRecord>) {
    collect_art_records_while(data, start, end, depth, out, &mut || true);
}

pub(crate) fn collect_art_records_while(
    data: &[u8],
    start: usize,
    end: usize,
    depth: usize,
    out: &mut Vec<ArtRecord>,
    keep_going: &mut impl FnMut() -> bool,
) -> bool {
    if depth > MAX_OFFICE_ART_DEPTH {
        return true;
    }
    let mut pos = start;
    while let Some(record) = record_at(data, pos, end) {
        if !keep_going() {
            return false;
        }
        out.push(record);
        if record.is_container()
            && !collect_art_records_while(
                data,
                record.content_start,
                record.content_end,
                depth + 1,
                out,
                keep_going,
            )
        {
            return false;
        }
        pos = record.content_end;
    }
    true
}

/// The records directly inside `data[start..end]`, without descending into containers.
pub(crate) fn child_records(data: &[u8], start: usize, end: usize) -> Vec<ArtRecord> {
    let mut out = Vec::new();
    let mut pos = start;
    while let Some(record) = record_at(data, pos, end) {
        out.push(record);
        pos = record.content_end;
    }
    out
}

/// The `(opid, value)` entries of a property table, whose `recInstance` is the entry count.
/// Empty when `record` is not a property table; entries that do not fit in the record are
/// not returned.
fn fopt_entries<'a>(data: &'a [u8], record: &ArtRecord) -> impl Iterator<Item = (u16, u32)> + 'a {
    let count = if MSOFBT_OPT.contains(&record.rec_type) {
        usize::from(record.rec_instance)
    } else {
        0
    };
    record
        .content(data)
        .as_chunks::<FOPT_ENTRY_LEN>()
        .0
        .iter()
        .take(count)
        .map(|entry| {
            (
                u16::from_le_bytes([entry[0], entry[1]]),
                u32::from_le_bytes([entry[2], entry[3], entry[4], entry[5]]),
            )
        })
}

/// The value of `property` in a shape's own property tables, if they set it.
pub(crate) fn shape_property(data: &[u8], shape: &ArtRecord, property: u16) -> Option<u32> {
    child_records(data, shape.content_start, shape.content_end)
        .iter()
        .find_map(|record| {
            fopt_entries(data, record)
                .find(|(opid, _)| opid & FOPT_PROPERTY_ID_MASK == property)
                .map(|(_, value)| value)
        })
}

/// The non-zero `pib` values a property table sets.
pub(crate) fn pib_values<'a>(data: &'a [u8], record: &ArtRecord) -> impl Iterator<Item = u32> + 'a {
    fopt_entries(data, record)
        .filter(|(opid, _)| opid & FOPT_PROPERTY_ID_MASK == MSO_PROPERTY_PIB)
        .map(|(_, pib)| pib)
        .filter(|&pib| pib > 0)
}

/// `foDelay` of an `msofbtBSE`, or `None` when `record` is not one or is too short to hold
/// it.
fn bse_fo_delay(data: &[u8], record: &ArtRecord) -> Option<u32> {
    if record.rec_type != MSOFBT_BSE {
        return None;
    }
    u32_at(record.content(data), BSE_FO_DELAY_OFFSET)
}

/// The blips of a document, in order, so that a shape's 1-based `pib` indexes straight
/// into them.
pub(crate) struct BlipStore<'a> {
    data: &'a [u8],
    blocks: Vec<ArtRecord>,
}

impl<'a> BlipStore<'a> {
    /// Every `msofbtBSE` among `records` that is long enough to hold `foDelay`, in stream
    /// order. This is how the PPT picture map has always numbered blips: a BSE too short to
    /// hold `foDelay` is skipped and does not take a `pib`.
    pub(crate) fn from_bse_records(data: &'a [u8], records: &[ArtRecord]) -> Self {
        let blocks = records
            .iter()
            .filter(|record| bse_fo_delay(data, record).is_some())
            .copied()
            .collect();
        Self { data, blocks }
    }

    /// The `BStoreContainer` of an `OfficeArtDggContainer` read from `data`; empty when the
    /// drawing group has none.
    pub(crate) fn from_drawing_group(data: &'a [u8], group: &ArtRecord) -> Self {
        let blocks = child_records(data, group.content_start, group.content_end)
            .into_iter()
            .find(|record| record.rec_type == MSOFBT_BSTORE_CONTAINER)
            .map(|bstore| child_records(data, bstore.content_start, bstore.content_end))
            .unwrap_or_default();
        Self { data, blocks }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    fn block(&self, pib: u32) -> Option<&ArtRecord> {
        self.blocks.get(usize::try_from(pib).ok()?.checked_sub(1)?)
    }

    /// `foDelay` of the blip `pib` names.
    pub(crate) fn fo_delay(&self, pib: u32) -> Option<u32> {
        bse_fo_delay(self.data, self.block(pib)?)
    }

    /// The blip `pib` names: the one its file block holds or, for a BSE that holds none,
    /// the blip record at its `foDelay` in `delay_stream`. A `pib` past the store, or a
    /// file block that is not a BSE and holds no blip, is [`Blip::Unsupported`].
    ///
    /// The picture bytes are borrowed: a caller that copies them counts the copy
    /// against the content budget.
    pub(crate) fn blip(
        &self,
        pib: u32,
        delay_stream: &'a [u8],
        formats: &[BlipFormat],
        budget: &mut SecurityBudget,
    ) -> Result<Blip<'a>, SecurityError> {
        let Some(block) = self.block(pib) else {
            return Ok(Blip::Unsupported);
        };
        let held = find_blip(block.bytes(self.data), formats, budget)?;
        if held != Blip::Empty {
            return Ok(held);
        }
        let Some(delay) = bse_fo_delay(self.data, block) else {
            return Ok(Blip::Unsupported);
        };
        // 0xFFFFFFFF says the blip is not in the delay stream (MS-ODRAW 2.2.32). ~keep
        if delay == u32::MAX {
            return Ok(Blip::Empty);
        }
        // `foDelay` is where the blip record itself starts, so only that record is read. The
        // BSE's `size` is not used: a wrong one would reach into the next blip. ~keep
        let Some(delayed) = usize::try_from(delay)
            .ok()
            .and_then(|start| record_at(delay_stream, start, delay_stream.len()))
            .filter(|record| (RT_BLIP_FIRST..=RT_BLIP_LAST).contains(&record.rec_type))
        else {
            return Ok(Blip::Unsupported);
        };
        find_blip(delayed.bytes(delay_stream), formats, budget)
    }
}

/// A raster picture format a blip can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlipFormat {
    Jpeg,
    Png,
    Dib,
}

impl BlipFormat {
    /// The format name an `ExtractedImage` carries.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Dib => "dib",
        }
    }
}

/// The raster format of a blip record type, or `None` for a metafile blip or any other
/// record.
pub(crate) fn blip_format(rec_type: u16) -> Option<BlipFormat> {
    match rec_type {
        RT_BLIP_JPEG | RT_BLIP_JPEG_ALT => Some(BlipFormat::Jpeg),
        RT_BLIP_PNG => Some(BlipFormat::Png),
        RT_BLIP_DIB => Some(BlipFormat::Dib),
        _ => None,
    }
}

/// Length of a raster blip's header before its picture bytes.
///
/// One `rgbUid` (16 bytes) + `tag` (1 byte) = 17-byte header, or two `rgbUid`s + `tag` =
/// 33 bytes; per MS-ODRAW 2.2.27-2.2.29 the low bit of `recInstance` is what distinguishes
/// the two UID counts for every raster blip type (e.g. JPEG 0x46A vs 0x46B, PNG 0x6E0 vs
/// 0x6E1, DIB 0x7A8 vs 0x7A9).
pub(crate) fn blip_header_len(rec_instance: u16) -> usize {
    if rec_instance & 0x1 == 1 { 33 } else { 17 }
}

/// What a search for a blip found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Blip<'a> {
    /// A blip in one of the formats asked for, and its picture bytes.
    Image(BlipFormat, &'a [u8]),
    /// Malformed records, or blips only in other formats.
    Unsupported,
    /// Well-formed records without any blip.
    Empty,
}

/// The first blip in `data` whose format is one of `formats`, with its picture bytes.
///
/// `data` is a run of OfficeArt records; containers and the file blocks of a
/// `BStoreContainer` are searched in turn. A blip in another format is skipped, and a
/// malformed record ends the search of the records around it; when no blip is found,
/// either makes the result [`Blip::Unsupported`] rather than [`Blip::Empty`].
pub(crate) fn find_blip<'a>(
    data: &'a [u8],
    formats: &[BlipFormat],
    budget: &mut SecurityBudget,
) -> Result<Blip<'a>, SecurityError> {
    let mut found = Blip::Empty;
    let mut position = 0;
    while position < data.len() {
        budget.step()?;
        let Some(record) = record_at(data, position, data.len()) else {
            return Ok(Blip::Unsupported);
        };
        let content = record.content(data);
        if record.is_container() || record.rec_type == MSOFBT_BSE {
            let children = if record.rec_type == MSOFBT_BSE {
                let Some(name_length) = content.get(BSE_NAME_LENGTH_OFFSET) else {
                    return Ok(Blip::Unsupported);
                };
                content.get(BSE_HEADER_LEN + usize::from(*name_length)..)
            } else {
                Some(content)
            };
            budget.enter()?;
            let result = match children {
                Some(children) => find_blip(children, formats, budget)?,
                None => Blip::Unsupported,
            };
            budget.leave();
            match result {
                Blip::Image(..) => return Ok(result),
                Blip::Unsupported => found = Blip::Unsupported,
                Blip::Empty => {}
            }
        } else if let Some(format) = blip_format(record.rec_type).filter(|format| formats.contains(format)) {
            let Some(bytes) = content
                .get(blip_header_len(record.rec_instance)..)
                .filter(|bytes| !bytes.is_empty())
            else {
                return Ok(Blip::Unsupported);
            };
            return Ok(Blip::Image(format, bytes));
        } else if (RT_BLIP_FIRST..=RT_BLIP_LAST).contains(&record.rec_type) {
            found = Blip::Unsupported;
        }
        position = record.content_end;
    }
    Ok(found)
}

/// What a shape in a drawing displays, as far as its pictures go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A shape displaying the blip its 1-based `pib` names.
    Picture(u32),
    /// The own shape of a group that holds a picture; its members are shapes of their own.
    Group,
    /// A shape without a picture, or a group that holds none.
    Other,
}

/// Every shape in `drawing` by its `spid`, the members of its groups included. A `spid`
/// stated twice keeps its first shape.
pub(crate) fn drawing_shapes(
    data: &[u8],
    drawing: &ArtRecord,
    budget: &mut SecurityBudget,
) -> Result<HashMap<u32, Shape>, SecurityError> {
    let mut records = Vec::new();
    collect_art_records(data, drawing.content_start, drawing.content_end, 0, &mut records);
    // The records are in stream order, so these offsets are sorted. ~keep
    let pictures: Vec<usize> = records
        .iter()
        .filter(|record| pib_values(data, record).next().is_some())
        .map(|record| record.content_start)
        .collect();
    let holds_picture = |group: &ArtRecord| {
        let first = pictures.partition_point(|&at| at < group.content_start);
        pictures.get(first).is_some_and(|&at| at < group.content_end)
    };
    let mut shapes = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        budget.step()?;
        if record.rec_type != MSOFBT_SP_CONTAINER {
            continue;
        }
        let Some(fsp) = child_records(data, record.content_start, record.content_end)
            .into_iter()
            .find(|child| child.rec_type == MSOFBT_FSP)
        else {
            continue;
        };
        let fsp = fsp.content(data);
        let (Some(spid), Some(flags)) = (u32_at(fsp, 0), u32_at(fsp, 4)) else {
            continue;
        };
        let shape = if flags & FSP_FLAG_GROUP != 0 {
            // A group's own shape opens its `SpgrContainer`, the record just before it. Any
            // other `SpgrContainer` just before a shape has no records walked inside it, so
            // it holds no picture. ~keep
            let group = index
                .checked_sub(1)
                .and_then(|parent| records.get(parent))
                .filter(|parent| parent.rec_type == MSOFBT_SPGR_CONTAINER);
            if group.is_some_and(holds_picture) {
                Shape::Group
            } else {
                Shape::Other
            }
        } else {
            shape_property(data, record, MSO_PROPERTY_PIB)
                .filter(|&pib| pib != 0)
                .map_or(Shape::Other, Shape::Picture)
        };
        shapes.entry(spid).or_insert(shape);
    }
    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::security::SecurityLimits;

    fn record(rec_ver_instance: u16, rec_type: u16, content: &[u8]) -> Vec<u8> {
        let mut bytes = rec_ver_instance.to_le_bytes().to_vec();
        bytes.extend_from_slice(&rec_type.to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(content);
        bytes
    }

    #[test]
    fn should_descend_into_containers_and_stop_at_a_record_that_overruns() {
        let mut data = record(0x000F, MSOFBT_SP_CONTAINER, &record(0x0002, MSOFBT_FSP, &[0; 8]));
        data.extend(record(0x0000, MSOFBT_BSE, &[0; 4]));
        let truncated = data.len();
        data.extend(record(0x0000, MSOFBT_BSE, &[0; 4]));
        data.truncate(data.len() - 1);

        let mut records = Vec::new();
        collect_art_records(&data, 0, data.len(), 0, &mut records);
        let types: Vec<u16> = records.iter().map(|record| record.rec_type).collect();
        assert_eq!(types, [MSOFBT_SP_CONTAINER, MSOFBT_FSP, MSOFBT_BSE]);
        assert!(records[0].is_container());
        assert_eq!(records[1].content(&data), [0; 8]);
        assert_eq!(child_records(&data, 0, data.len()).len(), 2);
        assert!(record_at(&data, truncated, data.len()).is_none());
        assert!(record_at(&data, usize::MAX, data.len()).is_none());
    }

    #[test]
    fn should_stop_descending_at_the_depth_cap() {
        let mut data = record(0x0002, MSOFBT_FSP, &[0; 8]);
        for _ in 0..MAX_OFFICE_ART_DEPTH + 3 {
            data = record(0x000F, MSOFBT_SPGR_CONTAINER, &data);
        }

        let mut records = Vec::new();
        collect_art_records(&data, 0, data.len(), 0, &mut records);
        assert_eq!(records.len(), MAX_OFFICE_ART_DEPTH + 1);
        assert!(records.iter().all(|record| record.rec_type == MSOFBT_SPGR_CONTAINER));
    }

    #[test]
    fn should_read_shape_property_from_every_property_table_within_its_entry_count() {
        let entry = |opid: u16, value: u32| [opid.to_le_bytes().as_slice(), &value.to_le_bytes()].concat();
        // Two entries declared, three present: the third is complex data, not an entry. ~keep
        let primary = [entry(0x0080, 7), entry(0x0081, 8), entry(MSO_PROPERTY_PIB, 9)].concat();
        let tertiary = entry(MSO_PROPERTY_PIB | 0x4000, 3);
        let mut shape = record(0x0023, MSOFBT_OPT[0], &primary);
        shape.extend(record(0x0013, MSOFBT_OPT[2], &tertiary));
        let data = record(0x000F, MSOFBT_SP_CONTAINER, &shape);
        let container = record_at(&data, 0, data.len()).expect("shape container");

        assert_eq!(shape_property(&data, &container, 0x0081), Some(8));
        assert_eq!(shape_property(&data, &container, MSO_PROPERTY_PIB), Some(3));
        assert_eq!(shape_property(&data, &container, 0x0182), None);
    }

    fn blip(rec_instance: u16, rec_type: u16, picture: &[u8]) -> Vec<u8> {
        let header = vec![0; blip_header_len(rec_instance)];
        record(rec_instance << 4, rec_type, &[header.as_slice(), picture].concat())
    }

    /// Byte offset of `size` inside an `msofbtBSE`'s content, which the reader ignores.
    const BSE_SIZE_OFFSET: usize = 20;

    fn bse(size: u32, fo_delay: u32, name: &[u8], embedded: &[u8]) -> Vec<u8> {
        let mut content = vec![0; BSE_HEADER_LEN];
        content[BSE_SIZE_OFFSET..BSE_SIZE_OFFSET + 4].copy_from_slice(&size.to_le_bytes());
        content[BSE_FO_DELAY_OFFSET..BSE_FO_DELAY_OFFSET + 4].copy_from_slice(&fo_delay.to_le_bytes());
        content[BSE_NAME_LENGTH_OFFSET] = name.len() as u8;
        content.extend_from_slice(name);
        content.extend_from_slice(embedded);
        record(0x0002, MSOFBT_BSE, &content)
    }

    fn search<'a>(data: &'a [u8], formats: &[BlipFormat]) -> Blip<'a> {
        let limits = SecurityLimits::default();
        find_blip(data, formats, &mut SecurityBudget::from_limits(&limits)).expect("budget")
    }

    #[test]
    fn should_number_only_the_bse_records_that_hold_fo_delay() {
        let mut data = bse(0, 10, b"", b"");
        data.extend(record(0x0002, MSOFBT_BSE, &[0; BSE_FO_DELAY_OFFSET + 3]));
        data.extend(record(0x0000, MSOFBT_FSP, &[0; 8]));
        data.extend(bse(0, 20, b"", b""));
        let mut records = Vec::new();
        collect_art_records(&data, 0, data.len(), 0, &mut records);
        let store = BlipStore::from_bse_records(&data, &records);

        assert_eq!(store.fo_delay(1), Some(10));
        assert_eq!(store.fo_delay(2), Some(20));
        assert_eq!(store.fo_delay(3), None);
        assert_eq!(store.fo_delay(0), None);
    }

    #[test]
    fn should_find_blip_in_file_blocks_and_skip_formats_not_asked_for() {
        let mut data = blip(0x7A8, RT_BLIP_DIB, b"dib");
        data.extend(record(
            0x000F,
            MSOFBT_BSTORE_CONTAINER,
            &bse(0, 0, b"nm", &blip(0x6E1, RT_BLIP_PNG, b"png")),
        ));

        assert_eq!(search(&data, &[BlipFormat::Png]), Blip::Image(BlipFormat::Png, b"png"));
        assert_eq!(search(&data, &[BlipFormat::Dib]), Blip::Image(BlipFormat::Dib, b"dib"));
        assert_eq!(search(&data, &[BlipFormat::Jpeg]), Blip::Unsupported);
        assert_eq!(search(&data[..data.len() - 1], &[BlipFormat::Png]), Blip::Unsupported);
        assert_eq!(
            search(
                &record(0x000F, MSOFBT_BSTORE_CONTAINER, &bse(0, 0, b"", b"")),
                &[BlipFormat::Png]
            ),
            Blip::Empty
        );
    }

    #[test]
    fn should_read_a_held_blip_or_the_one_record_at_fo_delay() {
        let mut delay_stream = b"text".to_vec();
        let jpeg = blip(0x46A, RT_BLIP_JPEG, b"jpeg");
        let jpeg_at = delay_stream.len() as u32;
        delay_stream.extend(&jpeg);
        // A metafile blip followed by a PNG that belongs to another BSE. ~keep
        let emf = record(0x3D40, 0xF01A, &[0; 50]);
        let emf_at = delay_stream.len() as u32;
        delay_stream.extend(&emf);
        delay_stream.extend(blip(0x6E0, RT_BLIP_PNG, b"next"));
        // A container that holds a blip is not itself a blip record. ~keep
        let wrapped_at = delay_stream.len() as u32;
        delay_stream.extend(record(
            0x000F,
            MSOFBT_BSTORE_CONTAINER,
            &blip(0x6E0, RT_BLIP_PNG, b"wrapped"),
        ));
        let jpeg_len = jpeg.len() as u32;
        let blocks = [
            bse(0, 0, b"", &blip(0x6E0, RT_BLIP_PNG, b"png")),
            bse(jpeg_len, jpeg_at, b"", b""),
            bse(0, jpeg_at, b"", b""),
            bse(jpeg_len, u32::MAX, b"", b""),
            bse(jpeg_len + 1, jpeg_at, b"", b""),
            bse(delay_stream.len() as u32, emf_at, b"", b""),
            bse(jpeg_len, 0, b"", b""),
            bse(jpeg_len, delay_stream.len() as u32, b"", b""),
            bse(0, wrapped_at, b"", b""),
            record(0x0000, MSOFBT_FSP, &[0; 8]),
        ]
        .concat();
        let data = record(
            0x000F,
            MSOFBT_DGG_CONTAINER,
            &record(0x005F, MSOFBT_BSTORE_CONTAINER, &blocks),
        );
        let group = record_at(&data, 0, data.len()).expect("drawing group");
        let store = BlipStore::from_drawing_group(&data, &group);
        let limits = SecurityLimits::default();
        let mut budget = SecurityBudget::from_limits(&limits);
        let mut lookup = |pib| {
            store
                .blip(pib, &delay_stream, &[BlipFormat::Jpeg, BlipFormat::Png], &mut budget)
                .expect("budget")
        };

        assert_eq!(lookup(1), Blip::Image(BlipFormat::Png, b"png"));
        assert_eq!(lookup(2), Blip::Image(BlipFormat::Jpeg, b"jpeg"));
        assert_eq!(lookup(3), Blip::Image(BlipFormat::Jpeg, b"jpeg"), "size is not read");
        assert_eq!(lookup(4), Blip::Empty);
        assert_eq!(lookup(5), Blip::Image(BlipFormat::Jpeg, b"jpeg"), "size is not read");
        assert_eq!(lookup(6), Blip::Unsupported, "only the record at foDelay");
        assert_eq!(lookup(7), Blip::Unsupported, "foDelay names no blip record");
        assert_eq!(lookup(8), Blip::Unsupported, "foDelay past the stream");
        assert_eq!(lookup(9), Blip::Unsupported, "foDelay names a container");
        assert_eq!(lookup(10), Blip::Unsupported, "a file block that is not a BSE");
        assert_eq!(lookup(0), Blip::Unsupported);
        assert_eq!(lookup(11), Blip::Unsupported);
        assert_eq!(store.fo_delay(2), Some(jpeg_at));
    }

    #[test]
    fn should_map_each_spid_to_what_its_shape_displays() {
        let shape = |spid: u32, flags: u32, properties: &[u8]| {
            let fsp = [spid.to_le_bytes(), flags.to_le_bytes()].concat();
            let mut body = record(0x0002, MSOFBT_FSP, &fsp);
            body.extend_from_slice(properties);
            record(0x000F, MSOFBT_SP_CONTAINER, &body)
        };
        let pib = |table: u16, pib: u32| {
            let entry = [MSO_PROPERTY_PIB.to_le_bytes().as_slice(), &pib.to_le_bytes()].concat();
            record(0x0013, table, &entry)
        };
        let group = [
            shape(1, FSP_FLAG_GROUP, &[]),
            shape(2, 0, &pib(MSOFBT_OPT[1], 4)),
            shape(3, 0, &pib(MSOFBT_OPT[0], 0)),
        ]
        .concat();
        let without_pictures = [shape(5, FSP_FLAG_GROUP, &[]), shape(6, 0, &pib(MSOFBT_OPT[0], 0))].concat();
        let mut shapes = record(0x000F, MSOFBT_SPGR_CONTAINER, &group);
        shapes.extend(shape(2, 0, &[]));
        shapes.extend(record(0x000F, MSOFBT_SPGR_CONTAINER, &without_pictures));
        shapes.extend(shape(7, FSP_FLAG_GROUP, &pib(MSOFBT_OPT[0], 1)));
        let data = record(0x000F, MSOFBT_DG_CONTAINER, &shapes);
        let drawing = record_at(&data, 0, data.len()).expect("drawing");
        let limits = SecurityLimits::default();

        let shapes = drawing_shapes(&data, &drawing, &mut SecurityBudget::from_limits(&limits)).expect("budget");
        assert_eq!(shapes.len(), 6);
        assert_eq!(shapes[&1], Shape::Group);
        assert_eq!(shapes[&2], Shape::Picture(4));
        assert_eq!(shapes[&3], Shape::Other);
        assert_eq!(shapes[&5], Shape::Other, "a group of shapes without a picture");
        assert_eq!(shapes[&6], Shape::Other);
        assert_eq!(shapes[&7], Shape::Other, "fGroup outside an SpgrContainer");
    }

    #[test]
    fn should_size_blip_header_for_one_or_two_uids() {
        assert_eq!(blip_header_len(0x6E0), 17);
        assert_eq!(blip_header_len(0x6E1), 33);
    }
}
