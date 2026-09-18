//! xberg-io/xberg#1660: a legacy binary `.ppt` can carry a table as an embedded OLE
//! object, a Word document or an Excel sheet inserted as an object, the way PowerPoint
//! 97-2003 decks routinely do. The object's own bytes are in the file, inside an
//! `ExOleObjStg` record the deck's `ExOleObjAtom` names by persist id, but the legacy
//! extractor walked every external-object record as opaque bytes and the slide came out
//! with its title and no content.
//!
//! The `.pptx` path has had this since #78/#307: `extraction::ooxml_embedded` reads
//! `ppt/embeddings/` and attaches each member to `ExtractedDocument.children`. These
//! tests pin the legacy path to the same contract.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]
// ~keep: test/bench binaries print by design; org logging policy exempts tests
// Gated on `excel` as well as `office` for the reason `issue_78_excel_embedded_objects`
// records: `office` alone leaves the inner document unextractable, which would turn a
// real assertion into a spurious red.
#![cfg(all(feature = "office", feature = "excel"))]

use std::io::Write as _;
use xberg::ExtractionConfig;

mod helpers;
use helpers::extract_bytes_document;

const PPT_MIME: &str = "application/vnd.ms-powerpoint";

// --- PowerPoint record types this test assembles (MS-PPT) ---------------------------
const RT_DOCUMENT: u16 = 0x03E8;
const RT_SLIDE: u16 = 0x03EE;
const RT_SLIDE_PERSIST_ATOM: u16 = 0x03F3;
const RT_SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0;
const RT_TEXT_CHARS_ATOM: u16 = 0x0FA0;
const RT_EXTERNAL_OBJECT_LIST: u16 = 0x0409;
const RT_EXTERNAL_OLE_EMBED: u16 = 0x0FCC;
const RT_EXTERNAL_OLE_OBJECT_ATOM: u16 = 0x0FC3;
const RT_EXTERNAL_OLE_OBJECT_STG: u16 = 0x1011;
const RT_PERSIST_DIRECTORY_ATOM: u16 = 0x1772;
const RT_USER_EDIT_ATOM: u16 = 0x0FF5;
const RT_CURRENT_USER_ATOM: u16 = 0x0FF6;

const DOCUMENT_PERSIST_ID: u32 = 1;
const SLIDE_PERSIST_ID: u32 = 2;
const STORAGE_PERSIST_ID: u32 = 3;

fn record_header(rec_ver_instance: u16, rec_type: u16, rec_len: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.extend_from_slice(&rec_ver_instance.to_le_bytes());
    buf.extend_from_slice(&rec_type.to_le_bytes());
    buf.extend_from_slice(&rec_len.to_le_bytes());
    buf
}

/// A container record: `recVer` (the low nibble) is 0xF, and its children are its payload.
fn container(rec_type: u16, children: &[u8]) -> Vec<u8> {
    let mut buf = record_header(0x000F, rec_type, children.len() as u32);
    buf.extend_from_slice(children);
    buf
}

fn text_chars_atom(text: &str) -> Vec<u8> {
    let utf16: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut buf = record_header(0x0000, RT_TEXT_CHARS_ATOM, utf16.len() as u32);
    buf.extend_from_slice(&utf16);
    buf
}

/// `ExOleObjAtom` (MS-PPT 2.10.12): `drawAspect`, `type`, `exObjId`, `subType`,
/// `persistIdRef`, `options`. `persistIdRef` is what names the object's storage.
fn ex_ole_obj_atom(persist_id: u32) -> Vec<u8> {
    let mut buf = record_header(0x0000, RT_EXTERNAL_OLE_OBJECT_ATOM, 24);
    buf.extend_from_slice(&1u32.to_le_bytes()); // drawAspect
    buf.extend_from_slice(&[0u8; 4]); // type: embedded, not linked
    buf.extend_from_slice(&1u32.to_le_bytes()); // exObjId
    buf.extend_from_slice(&[0u8; 4]); // subType
    buf.extend_from_slice(&persist_id.to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]); // options
    buf
}

/// `ExOleObjStgCompressedAtom` (MS-PPT 2.10.35): `recInstance` 1, a 4-byte decompressed
/// size, then a zlib stream carrying the object's own compound file.
fn ex_ole_obj_stg(object: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(object).expect("deflate the embedded object");
    let deflated = encoder.finish().expect("finish the zlib stream");

    let mut content = Vec::with_capacity(4 + deflated.len());
    content.extend_from_slice(&(object.len() as u32).to_le_bytes());
    content.extend_from_slice(&deflated);

    let mut buf = record_header(1 << 4, RT_EXTERNAL_OLE_OBJECT_STG, content.len() as u32);
    buf.extend_from_slice(&content);
    buf
}

fn persist_directory(entries: &[(u32, u32)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (id, offset) in entries {
        // One id per entry: `cPersist` (the top 12 bits) is 1.
        body.extend_from_slice(&((1u32 << 20) | (id & 0x000F_FFFF)).to_le_bytes());
        body.extend_from_slice(&offset.to_le_bytes());
    }
    let mut buf = record_header(0x0000, RT_PERSIST_DIRECTORY_ATOM, body.len() as u32);
    buf.extend_from_slice(&body);
    buf
}

fn user_edit_atom(offset_persist_directory: u32) -> Vec<u8> {
    let mut buf = record_header(0x0000, RT_USER_EDIT_ATOM, 28);
    buf.extend_from_slice(&1u32.to_le_bytes()); // lastSlideIdRef
    buf.extend_from_slice(&[0u8; 4]); // version fields
    buf.extend_from_slice(&[0u8; 4]); // offsetLastEdit: 0, this is the first save
    buf.extend_from_slice(&offset_persist_directory.to_le_bytes());
    buf.extend_from_slice(&DOCUMENT_PERSIST_ID.to_le_bytes());
    buf.extend_from_slice(&[0u8; 8]);
    buf
}

fn current_user_stream(offset_to_current_edit: u32) -> Vec<u8> {
    let mut buf = record_header(0x0000, RT_CURRENT_USER_ATOM, 0x14);
    buf.extend_from_slice(&0x14u32.to_le_bytes()); // size
    buf.extend_from_slice(&0xE391_C05Fu32.to_le_bytes()); // headerToken
    buf.extend_from_slice(&offset_to_current_edit.to_le_bytes());
    buf.extend_from_slice(&[0u8; 8]);
    buf
}

/// An OLE compound file carrying `payload` in a `Package` stream. This is the shape an Office
/// application writes when it embeds a whole modern document as an object, and the one
/// `extraction::ooxml_embedded::extract_ole_embedded_object` identifies by sniffing the
/// stream's own bytes.
fn ole_object_with_package(payload: &[u8]) -> Vec<u8> {
    let mut comp = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).expect("create the embedded object");
    comp.create_stream("/Package")
        .expect("create Package stream")
        .write_all(payload)
        .expect("write Package stream");
    comp.into_inner().into_inner()
}

/// Assemble a one-slide `.ppt` whose external-object list declares a single embedded
/// object, with a persist chain naming the document, the slide and the object's storage.
fn ppt_with_embedded_object(slide_text: &str, object: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();

    let document_offset = stream.len() as u32;
    let mut document_body = container(
        RT_EXTERNAL_OBJECT_LIST,
        &container(RT_EXTERNAL_OLE_EMBED, &ex_ole_obj_atom(STORAGE_PERSIST_ID)),
    );
    let mut slide_persist = record_header(0x0000, RT_SLIDE_PERSIST_ATOM, 20);
    slide_persist.extend_from_slice(&SLIDE_PERSIST_ID.to_le_bytes());
    slide_persist.extend_from_slice(&[0u8; 16]);
    document_body.extend_from_slice(&container(RT_SLIDE_LIST_WITH_TEXT, &slide_persist));
    stream.extend_from_slice(&container(RT_DOCUMENT, &document_body));

    let slide_offset = stream.len() as u32;
    stream.extend_from_slice(&container(RT_SLIDE, &text_chars_atom(slide_text)));

    let storage_offset = stream.len() as u32;
    stream.extend_from_slice(&ex_ole_obj_stg(object));

    let directory_offset = stream.len() as u32;
    stream.extend_from_slice(&persist_directory(&[
        (DOCUMENT_PERSIST_ID, document_offset),
        (SLIDE_PERSIST_ID, slide_offset),
        (STORAGE_PERSIST_ID, storage_offset),
    ]));
    let edit_offset = stream.len() as u32;
    stream.extend_from_slice(&user_edit_atom(directory_offset));

    let mut comp = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).expect("create the deck");
    comp.create_stream("/PowerPoint Document")
        .expect("create PowerPoint Document stream")
        .write_all(&stream)
        .expect("write PowerPoint Document stream");
    comp.create_stream("/Current User")
        .expect("create Current User stream")
        .write_all(&current_user_stream(edit_offset))
        .expect("write Current User stream");
    comp.into_inner().into_inner()
}

const TABLE: &str = "COMPOUND\tSTATE\nhexogen\tExplosive solid\n";

#[tokio::test]
async fn should_attach_an_embedded_ole_object_as_a_child_document() {
    let deck = ppt_with_embedded_object("Embedded Table Slide", &ole_object_with_package(TABLE.as_bytes()));

    let result = extract_bytes_document(&deck, PPT_MIME, &ExtractionConfig::default())
        .await
        .expect("the deck must extract");

    let children = result
        .children
        .as_ref()
        .expect("an embedded OLE object must be attached as a child document");
    assert_eq!(children.len(), 1, "exactly one embedded object expected");
    assert_eq!(children[0].path, "embedded-object-1");
    assert!(
        children[0].result.content.contains("hexogen"),
        "the child must carry the embedded object's own content: {:?}",
        children[0].result.content
    );
}

/// The slide's own text is unaffected: this adds content rather than rerouting any.
#[tokio::test]
async fn should_keep_the_deck_text_when_an_object_is_attached() {
    let deck = ppt_with_embedded_object("Embedded Table Slide", &ole_object_with_package(TABLE.as_bytes()));

    let result = extract_bytes_document(&deck, PPT_MIME, &ExtractionConfig::default())
        .await
        .expect("the deck must extract");

    assert!(
        result.content.contains("Embedded Table Slide"),
        "the slide's text must survive: {:?}",
        result.content
    );
}

/// `max_archive_depth` is the budget that stops an embedding chain recursing without
/// bound, and it governs this path exactly as it governs `ppt/embeddings/` for `.pptx`.
#[tokio::test]
async fn should_not_descend_into_objects_when_the_archive_depth_budget_is_exhausted() {
    let deck = ppt_with_embedded_object("Embedded Table Slide", &ole_object_with_package(TABLE.as_bytes()));
    let config = ExtractionConfig {
        max_archive_depth: 0,
        ..Default::default()
    };

    let result = extract_bytes_document(&deck, PPT_MIME, &config)
        .await
        .expect("the deck must still extract");

    assert!(result.children.is_none(), "no object may be descended into at depth 0");
    assert!(
        result.content.contains("Embedded Table Slide"),
        "the deck's own text is not what the budget withholds"
    );
}

/// An object whose container holds none of the streams the identifier knows is reported
/// and skipped. One unreadable object must not cost the caller the deck.
#[tokio::test]
async fn should_warn_and_continue_when_an_object_cannot_be_identified() {
    let mut comp = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).expect("create the embedded object");
    comp.create_stream("/Unknown")
        .expect("create stream")
        .write_all(b"nothing the identifier recognises")
        .expect("write stream");
    let unidentifiable = comp.into_inner().into_inner();

    let deck = ppt_with_embedded_object("Embedded Table Slide", &unidentifiable);

    let result = extract_bytes_document(&deck, PPT_MIME, &ExtractionConfig::default())
        .await
        .expect("the deck must still extract");

    assert!(result.children.is_none(), "an unidentifiable object produces no child");
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|w| w.source == "ppt_embedded_objects"),
        "the skipped object must be reported: {:?}",
        result.processing_warnings
    );
    assert!(result.content.contains("Embedded Table Slide"));
}
