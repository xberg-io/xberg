//! Embedded OLE objects in a legacy `.ppt` (GH#1660).
//!
//! A PowerPoint 97-2003 deck carries a Word or Excel table inserted as an object as an
//! `ExOleObjStg` record: the object's own compound file, zlib-deflated when the record's
//! `recInstance` is 1 (MS-PPT 2.10.35). The `.pptx` path already recurses into
//! `ppt/embeddings/`; this is the legacy counterpart, returning each object's storage bytes
//! and the slide whose shape displays it so the caller can hand them to the matching
//! extractor.

use super::{
    LiveSlides, PPT_WARNING_SOURCE, live_slide_offsets_with_cancellation, read_stream_with_cancellation, read_u32_le,
};
use crate::error::{Result, XbergError};
use crate::extraction::office_art::{ArtRecord, collect_art_records_while};
use crate::types::ProcessingWarning;
use std::borrow::Cow;
use std::io::{Cursor, Read};

/// `ExObjListContainer` (MS-PPT 2.10.28): the document's list of external objects.
const RT_EXTERNAL_OBJECT_LIST: u16 = 0x0409;
/// `ExEmbedContainer` (MS-PPT 2.10.27): one embedded object.
const RT_EXTERNAL_OLE_EMBED: u16 = 0x0FCC;
/// `ExOleObjAtom` (MS-PPT 2.10.30): the object's id, kind and storage reference.
const RT_EXTERNAL_OLE_OBJECT_ATOM: u16 = 0x0FC3;
/// `ExOleObjStg` (MS-PPT 2.10.34/35): the object's storage, compressed when `recInstance` is 1.
const RT_EXTERNAL_OLE_OBJECT_STG: u16 = 0x1011;
/// `ExObjRefAtom` (MS-PPT 2.8.29): in a shape's `ClientData`, names the `ExEmbed` it shows.
/// Not 0x0BC3, which is `OutlineTextRefAtom`. ~keep
const RT_EXTERNAL_OBJECT_REF_ATOM: u16 = 0x0BC1;

/// `ExOleObjAtom.type` for an embedded object (a linked object keeps only its picture).
const EX_OLE_OBJECT_TYPE_EMBEDDED: u32 = 0;
/// Field offsets inside `ExOleObjAtom`: `drawAspect`, `type`, `exObjId`, `subType`,
/// `persistIdRef`, `unused` -- six dwords.
const EX_OLE_OBJECT_ATOM_TYPE: usize = 4;
const EX_OLE_OBJECT_ATOM_EX_OBJ_ID: usize = 8;
const EX_OLE_OBJECT_ATOM_PERSIST_ID_REF: usize = 16;
const EX_OLE_OBJECT_ATOM_LEN: usize = 24;
/// `ExOleObjStgCompressedAtom`: a dword `decompressedSize` precedes the zlib stream.
const EX_OLE_OBJECT_STG_COMPRESSED_INSTANCE: u16 = 1;
const EX_OLE_OBJECT_STG_SIZE_PREFIX: usize = 4;

/// One embedded object's storage, ready for the matching extractor.
pub(crate) struct PptEmbeddedObject {
    /// `ExOleObjAtom.exObjId` -- the deck's own identifier for the object.
    pub ex_obj_id: u32,
    /// 1-based live slide whose shape displays the object, when a shape references it.
    pub slide_number: Option<u32>,
    /// The object's compound file, decompressed.
    pub data: Vec<u8>,
}

/// Every embedded object storage in `content`, in `ExObjList` order.
///
/// `max_object_bytes` bounds one object's decompressed size (the declared
/// `decompressedSize` is untrusted); an object over it is skipped with a warning rather than
/// allocated. Objects are read through the persist chain, so a superseded save's storage is
/// never returned; when the chain cannot be resolved the stream's `ExOleObjStg` records are
/// returned in stream order with no slide attribution, which is better than losing the
/// tables outright. ~keep
#[cfg(test)]
fn extract_ppt_embedded_objects(
    content: &[u8],
    max_object_bytes: usize,
) -> Result<(Vec<PptEmbeddedObject>, Vec<ProcessingWarning>)> {
    extract_ppt_embedded_objects_with_cancellation(content, max_object_bytes, super::PptReadCancellation::default())
}

pub(crate) fn extract_ppt_embedded_objects_with_cancellation(
    content: &[u8],
    max_object_bytes: usize,
    cancel: super::PptReadCancellation<'_>,
) -> Result<(Vec<PptEmbeddedObject>, Vec<ProcessingWarning>)> {
    cancel.check()?;
    let mut comp = cfb::CompoundFile::open(Cursor::new(content))
        .map_err(|e| XbergError::parsing(format!("Failed to open PPT as OLE container: {e}")))?;
    let ppt_stream = read_stream_with_cancellation(&mut comp, "/PowerPoint Document", cancel)?;
    let current_user_stream = read_stream_with_cancellation(&mut comp, "/Current User", cancel).unwrap_or_default();

    extract_embedded_from_streams(&ppt_stream, &current_user_stream, max_object_bytes, cancel)
}

pub(super) fn extract_embedded_from_streams(
    ppt_stream: &[u8],
    current_user_stream: &[u8],
    max_object_bytes: usize,
    cancel: super::PptReadCancellation<'_>,
) -> Result<(Vec<PptEmbeddedObject>, Vec<ProcessingWarning>)> {
    let mut warnings = Vec::new();
    let mut objects = Vec::new();

    cancel.check()?;
    tracing::debug!(target: "xberg::ppt::embedding_walk", "discovering embedded PPT storage");
    let live = live_slide_offsets_with_cancellation(ppt_stream, current_user_stream, cancel);
    cancel.check()?;
    let Some(live) = live else {
        return read_unresolved_objects(ppt_stream, max_object_bytes, cancel);
    };

    let slide_by_ex_obj_id = slide_numbers_by_ex_obj_id(ppt_stream, &live, cancel)?;
    for (ex_obj_id, persist_id_ref) in embedded_object_refs(ppt_stream, &live, cancel)? {
        cancel.check()?;
        let Some(&offset) = live.persist.get(&persist_id_ref) else {
            warnings.push(warning(format!(
                "Skipped embedded object {ex_obj_id}: persist id {persist_id_ref} is not in the persist directory"
            )));
            continue;
        };
        let Some((rec_instance, rec_type)) = record_instance_and_type(ppt_stream, offset) else {
            warnings.push(warning(format!(
                "Skipped embedded object {ex_obj_id}: storage offset {offset} is out of range"
            )));
            continue;
        };
        if rec_type != RT_EXTERNAL_OLE_OBJECT_STG {
            warnings.push(warning(format!(
                "Skipped embedded object {ex_obj_id}: persist id {persist_id_ref} resolves to record type {rec_type:#06x}, not ExOleObjStg"
            )));
            continue;
        }
        let data = read_storage(
            ppt_stream,
            offset,
            rec_instance,
            max_object_bytes,
            &mut warnings,
            cancel,
        );
        cancel.check()?;
        let Some(data) = data else {
            continue;
        };
        objects.push(PptEmbeddedObject {
            ex_obj_id,
            slide_number: slide_by_ex_obj_id.get(&ex_obj_id).copied(),
            data,
        });
    }
    Ok((objects, warnings))
}

fn read_unresolved_objects(
    ppt_stream: &[u8],
    max_object_bytes: usize,
    cancel: super::PptReadCancellation<'_>,
) -> Result<(Vec<PptEmbeddedObject>, Vec<ProcessingWarning>)> {
    let mut warnings = Vec::new();
    let mut objects = Vec::new();
    for (offset, rec_instance) in all_object_storages(ppt_stream, cancel)? {
        cancel.check()?;
        let data = read_storage(
            ppt_stream,
            offset,
            rec_instance,
            max_object_bytes,
            &mut warnings,
            cancel,
        );
        cancel.check()?;
        let Some(data) = data else {
            continue;
        };
        objects.push(PptEmbeddedObject {
            ex_obj_id: objects.len() as u32 + 1,
            slide_number: None,
            data,
        });
    }
    Ok((objects, warnings))
}

fn warning(message: String) -> ProcessingWarning {
    ProcessingWarning {
        source: Cow::Borrowed(PPT_WARNING_SOURCE),
        message: Cow::Owned(message),
    }
}

fn record_instance_and_type(data: &[u8], offset: usize) -> Option<(u16, u16)> {
    let header = data.get(offset..offset.checked_add(8)?)?;
    let ver_instance = u16::from_le_bytes([header[0], header[1]]);
    let rec_type = u16::from_le_bytes([header[2], header[3]]);
    Some((ver_instance >> 4, rec_type))
}

/// `(exObjId, persistIdRef)` for every embedded `ExEmbed` in the live document's
/// `ExObjList`, in list order. Linked objects and controls have no storage and are skipped.
fn embedded_object_refs(
    ppt_stream: &[u8],
    live: &LiveSlides,
    cancel: super::PptReadCancellation<'_>,
) -> Result<Vec<(u32, u32)>> {
    let (start, end) = live.document_payload;
    let mut records = Vec::new();
    collect_art_records_while(ppt_stream, start, end, 0, &mut records, &mut || cancel.check().is_ok());
    cancel.check()?;
    let Some(list) = records.iter().find(|r| r.rec_type == RT_EXTERNAL_OBJECT_LIST) else {
        return Ok(Vec::new());
    };
    let (list_start, list_end) = (list.content_start, list.content_end);
    let embeds: Vec<&ArtRecord> = records
        .iter()
        .filter(|r| r.rec_type == RT_EXTERNAL_OLE_EMBED && r.content_start >= list_start && r.content_end <= list_end)
        .collect();
    let mut output = Vec::new();
    for embed in embeds {
        cancel.check()?;
        let parsed = (|| {
            let atom = records.iter().find(|r| {
                r.rec_type == RT_EXTERNAL_OLE_OBJECT_ATOM
                    && r.content_start >= embed.content_start
                    && r.content_end <= embed.content_end
            })?;
            if atom.content_end - atom.content_start < EX_OLE_OBJECT_ATOM_LEN {
                return None;
            }
            let kind = read_u32_le(ppt_stream, atom.content_start + EX_OLE_OBJECT_ATOM_TYPE)?;
            if kind != EX_OLE_OBJECT_TYPE_EMBEDDED {
                return None;
            }
            let ex_obj_id = read_u32_le(ppt_stream, atom.content_start + EX_OLE_OBJECT_ATOM_EX_OBJ_ID)?;
            let persist_id_ref = read_u32_le(ppt_stream, atom.content_start + EX_OLE_OBJECT_ATOM_PERSIST_ID_REF)?;
            Some((ex_obj_id, persist_id_ref))
        })();
        output.extend(parsed);
    }
    Ok(output)
}

/// `exObjId -> 1-based live slide number`, from the `ExObjRefAtom` in each slide's shapes.
/// The first slide that references an object wins.
fn slide_numbers_by_ex_obj_id(
    ppt_stream: &[u8],
    live: &LiveSlides,
    cancel: super::PptReadCancellation<'_>,
) -> Result<ahash::AHashMap<u32, u32>> {
    let mut map = ahash::AHashMap::new();
    for (index, &slide_offset) in live.offsets.iter().enumerate() {
        cancel.check()?;
        let Some(len) = read_u32_le(ppt_stream, slide_offset + 4) else {
            continue;
        };
        let start = slide_offset + 8;
        let Some(end) = start.checked_add(len as usize).filter(|&e| e <= ppt_stream.len()) else {
            continue;
        };
        let mut records = Vec::new();
        collect_art_records_while(ppt_stream, start, end, 0, &mut records, &mut || cancel.check().is_ok());
        cancel.check()?;
        for record in records.iter().filter(|r| r.rec_type == RT_EXTERNAL_OBJECT_REF_ATOM) {
            if let Some(ex_obj_id) = read_u32_le(ppt_stream, record.content_start) {
                map.entry(ex_obj_id).or_insert(index as u32 + 1);
            }
        }
    }
    Ok(map)
}

/// `(offset, recInstance)` of every top-level `ExOleObjStg` in stream order -- the fallback
/// when the persist chain cannot be read.
fn all_object_storages(ppt_stream: &[u8], cancel: super::PptReadCancellation<'_>) -> Result<Vec<(usize, u16)>> {
    let mut records = Vec::new();
    collect_art_records_while(ppt_stream, 0, ppt_stream.len(), 0, &mut records, &mut || {
        cancel.check().is_ok()
    });
    cancel.check()?;
    Ok(records
        .iter()
        .filter(|r| r.rec_type == RT_EXTERNAL_OLE_OBJECT_STG)
        .map(|r| (r.content_start - 8, r.rec_instance))
        .collect())
}

/// The storage's compound-file bytes, decompressed when the record says so. `None` (with a
/// warning pushed) when the payload is malformed or would exceed `max_object_bytes`.
fn read_storage(
    ppt_stream: &[u8],
    offset: usize,
    rec_instance: u16,
    max_object_bytes: usize,
    warnings: &mut Vec<ProcessingWarning>,
    cancel: super::PptReadCancellation<'_>,
) -> Option<Vec<u8>> {
    let len = read_u32_le(ppt_stream, offset + 4)? as usize;
    let start = offset + 8;
    let Some(payload) = start.checked_add(len).and_then(|end| ppt_stream.get(start..end)) else {
        warnings.push(warning(format!(
            "Skipped embedded object storage at {offset}: declared length {len} runs past the stream"
        )));
        return None;
    };
    if rec_instance != EX_OLE_OBJECT_STG_COMPRESSED_INSTANCE {
        if payload.len() > max_object_bytes {
            warnings.push(warning(format!(
                "Skipped embedded object storage at {offset}: {} bytes exceeds cap {max_object_bytes} bytes",
                payload.len()
            )));
            return None;
        }
        let mut data = Vec::new();
        cancel.read_into(payload, &mut data).ok()?;
        return Some(data);
    }
    let declared = read_u32_le(payload, 0)? as usize;
    if declared > max_object_bytes {
        warnings.push(warning(format!(
            "Skipped embedded object storage at {offset}: declared {declared} bytes exceeds cap {max_object_bytes} bytes"
        )));
        return None;
    }
    // The declared size is untrusted: read at most one byte past the cap so an
    // understated declaration cannot slip an oversized object through, and never
    // allocate more than the cap up front. ~keep
    let mut data = Vec::with_capacity(declared);
    let read_cap = (max_object_bytes as u64).saturating_add(1);
    let decoder = flate2::read::ZlibDecoder::new(&payload[EX_OLE_OBJECT_STG_SIZE_PREFIX..]).take(read_cap);
    if let Err(e) = cancel.read_into(decoder, &mut data) {
        warnings.push(warning(format!(
            "Skipped embedded object storage at {offset}: zlib stream is invalid: {e}"
        )));
        return None;
    }
    if data.len() > max_object_bytes {
        warnings.push(warning(format!(
            "Skipped embedded object storage at {offset}: decompressed size exceeds cap {max_object_bytes} bytes"
        )));
        return None;
    }
    Some(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture() -> Vec<u8> {
        std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ppt/embedded_word_object.ppt"),
        )
        .expect("fixture")
    }

    #[test]
    fn extracts_the_embedded_word_storage_and_attributes_it_to_slide_two() {
        let (objects, warnings) = extract_ppt_embedded_objects(&fixture(), 50 * 1024 * 1024).expect("parse");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(objects.len(), 1);
        let object = &objects[0];
        assert_eq!(object.ex_obj_id, 1);
        assert_eq!(object.slide_number, Some(2));
        assert_eq!(&object.data[..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        let mut inner = cfb::CompoundFile::open(Cursor::new(object.data.as_slice())).expect("inner CFB");
        assert!(inner.exists("WordDocument"));
        let mut word = Vec::new();
        inner
            .open_stream("WordDocument")
            .unwrap()
            .read_to_end(&mut word)
            .unwrap();
        assert!(!word.is_empty());
    }

    #[test]
    fn skips_an_object_whose_declared_size_exceeds_the_cap() {
        let (objects, warnings) = extract_ppt_embedded_objects(&fixture(), 1024).expect("parse");
        assert!(objects.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].message.contains("exceeds cap 1024 bytes"),
            "{}",
            warnings[0].message
        );
    }

    /// A zlib stream that inflates past what its prefix declares must still be bounded by
    /// the cap, not by the declaration.
    #[test]
    fn bounds_the_inflated_size_by_the_cap_not_the_declaration() {
        let big = vec![0u8; 8192];
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&big).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut payload = 16u32.to_le_bytes().to_vec();
        payload.extend_from_slice(&compressed);
        let mut stream = Vec::new();
        stream.extend_from_slice(&(EX_OLE_OBJECT_STG_COMPRESSED_INSTANCE << 4).to_le_bytes());
        stream.extend_from_slice(&RT_EXTERNAL_OLE_OBJECT_STG.to_le_bytes());
        stream.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        stream.extend_from_slice(&payload);

        let mut warnings = Vec::new();
        let result = read_storage(
            &stream,
            0,
            EX_OLE_OBJECT_STG_COMPRESSED_INSTANCE,
            4096,
            &mut warnings,
            super::super::PptReadCancellation::default(),
        );

        assert!(result.is_none());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("decompressed size exceeds cap"));
    }
}
