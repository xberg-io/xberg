//! Tracked deletions (`sprmCFRMarkDel`) in legacy `.doc` text, #3435.

use super::tests::{build_doc_ole, build_fib, test_fc_clx_offset, test_rg_fc_lcb_offset, write_u16, write_u32};
use super::{DocExtractionResult, DocSubdocumentKind, extract_doc_text};
use std::ops::Range;

const TEXT_OFFSET: usize = 900;
const PAGE_BYTES: usize = 512;
const DELETE_SPRM: [u8; 3] = [0x00, 0x08, 1];
const IN_TABLE: &[u8] = &[0x16, 0x24, 0x01];
const ROW_END: &[u8] = &[0x16, 0x24, 0x01, 0x17, 0x24, 0x01];

type Papx<'a> = &'a [(Range<usize>, &'a [u8])];

/// A single-story-per-piece UTF-16 document with optional paragraph properties, CHPX deletions
/// and piece properties. Character indices address `text`, which must be BMP-only.
struct Spec<'a> {
    text: &'a str,
    footnote_chars: usize,
    papx: Papx<'a>,
    deleted: &'a [(usize, usize)],
    /// `(first character, prm)` of each piece, in order.
    pieces: &'a [(usize, u16)],
    /// Property groups (`PrcData`) the complex piece prms index.
    prc: &'a [&'a [u8]],
}

impl<'a> Spec<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            footnote_chars: 0,
            papx: &[],
            deleted: &[],
            pieces: &[(0, 0)],
            prc: &[],
        }
    }
}

fn chars_of(text: &str, needle: &str) -> (usize, usize) {
    let byte = text.find(needle).expect("needle in fixture text");
    let start = text[..byte].chars().count();
    (start, start + needle.chars().count())
}

fn write_chpx_page(page: &mut [u8], deleted: &[(usize, usize)], len: usize) {
    let fc = |char_index: usize| (TEXT_OFFSET + char_index * 2) as u32;
    let mut runs: Vec<(Range<usize>, bool)> = Vec::new();
    let mut at = 0;
    for &(start, end) in deleted {
        if at < start {
            runs.push((at..start, false));
        }
        runs.push((start..end, true));
        at = end;
    }
    if at < len {
        runs.push((at..len, false));
    }
    for (i, (range, deleted)) in runs.iter().enumerate() {
        page[i * 4..i * 4 + 4].copy_from_slice(&fc(range.start).to_le_bytes());
        page[(i + 1) * 4..(i + 2) * 4].copy_from_slice(&fc(range.end).to_le_bytes());
        if *deleted {
            let at = 256 + i * 4;
            page[(runs.len() + 1) * 4 + i] = (at / 2) as u8;
            page[at..at + 4].copy_from_slice(&[3, DELETE_SPRM[0], DELETE_SPRM[1], DELETE_SPRM[2]]);
        }
    }
    page[PAGE_BYTES - 1] = runs.len() as u8;
}

fn build(spec: &Spec) -> Vec<u8> {
    let units: Vec<u16> = spec.text.encode_utf16().collect();
    assert_eq!(units.len(), spec.text.chars().count(), "BMP-only fixture text");
    let fc = |char_index: usize| (TEXT_OFFSET + char_index * 2) as u32;
    let papx_page = (TEXT_OFFSET + units.len() * 2).div_ceil(PAGE_BYTES) * PAGE_BYTES;
    let chpx_page = papx_page + PAGE_BYTES;
    let mut word = build_fib(
        chpx_page + PAGE_BYTES,
        (units.len() - spec.footnote_chars) as u32,
        spec.footnote_chars as u32,
        0,
        0,
    );
    let pairs = test_rg_fc_lcb_offset();
    write_u16(&mut word, pairs - 2, 93);
    for (i, unit) in units.iter().enumerate() {
        write_u16(&mut word, TEXT_OFFSET + i * 2, *unit);
    }

    let page = &mut word[papx_page..papx_page + PAGE_BYTES];
    let mut papx_at = 256usize;
    for (i, (range, grpprl)) in spec.papx.iter().enumerate() {
        page[i * 4..i * 4 + 4].copy_from_slice(&fc(range.start).to_le_bytes());
        page[(i + 1) * 4..(i + 2) * 4].copy_from_slice(&fc(range.end).to_le_bytes());
        page[(spec.papx.len() + 1) * 4 + i * 13] = (papx_at / 2) as u8;
        let cb = (2 + grpprl.len() + 1).div_ceil(2);
        page[papx_at] = cb as u8;
        page[papx_at + 3..papx_at + 3 + grpprl.len()].copy_from_slice(grpprl);
        papx_at += (1 + cb * 2 - 1).div_ceil(2) * 2;
    }
    page[PAGE_BYTES - 1] = spec.papx.len() as u8;

    write_chpx_page(&mut word[chpx_page..chpx_page + PAGE_BYTES], spec.deleted, units.len());

    let mut clx = Vec::new();
    for group in spec.prc {
        clx.push(1);
        clx.extend_from_slice(&(group.len() as i16).to_le_bytes());
        clx.extend_from_slice(group);
    }
    clx.push(2);
    clx.extend_from_slice(&0u32.to_le_bytes());
    for (start, _) in spec.pieces {
        clx.extend_from_slice(&(*start as u32).to_le_bytes());
    }
    clx.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for (start, prm) in spec.pieces {
        clx.extend_from_slice(&[0, 0]);
        clx.extend_from_slice(&fc(*start).to_le_bytes());
        clx.extend_from_slice(&prm.to_le_bytes());
    }
    let mut table = vec![0u8; 8];
    write_u32(&mut word, test_fc_clx_offset(), 8);
    write_u32(&mut word, test_fc_clx_offset() + 4, clx.len() as u32);
    table.extend_from_slice(&clx);

    for (pair, page) in [(12, chpx_page), (13, papx_page)] {
        if pair == 13 && spec.papx.is_empty() {
            continue;
        }
        let at = table.len();
        table.extend_from_slice(&fc(0).to_le_bytes());
        table.extend_from_slice(&fc(units.len()).to_le_bytes());
        table.extend_from_slice(&((page / PAGE_BYTES) as u32).to_le_bytes());
        write_u32(&mut word, pairs + pair * 8, at as u32);
        write_u32(&mut word, pairs + pair * 8 + 4, 12);
    }
    build_doc_ole(&word, &table)
}

fn extract(spec: &Spec) -> DocExtractionResult {
    extract_doc_text(&build(spec)).expect("DOC extraction")
}

fn contents(result: &DocExtractionResult) -> Vec<&str> {
    result.paragraphs.iter().map(|p| p.content.as_str()).collect()
}

fn table_marks(result: &DocExtractionResult) -> Vec<(&str, Option<(bool, bool)>)> {
    result
        .paragraphs
        .iter()
        .map(|p| (p.content.as_str(), p.table.map(|t| (t.cell_end, t.row_end))))
        .collect()
}

#[test]
fn should_drop_a_run_deleted_by_a_complex_piece_property_group() {
    let text = "keep gone\rnext\r";
    let spec = Spec {
        pieces: &[(0, 0), (5, 1), (9, 0)],
        prc: &[&DELETE_SPRM],
        ..Spec::new(text)
    };
    let result = extract(&spec);
    assert_eq!(contents(&result), ["keep", "next"]);
    assert_eq!(result.content, "keep \nnext");
}

#[test]
fn should_drop_a_run_deleted_by_a_compact_piece_property() {
    let text = "keep gone\rnext\r";
    let deleted = (0x41 << 1) | (1 << 8);
    let spec = Spec {
        pieces: &[(0, 0), (5, deleted), (9, 0)],
        ..Spec::new(text)
    };
    let result = extract(&spec);
    assert_eq!(contents(&result), ["keep", "next"]);
    assert_eq!(result.content, "keep \nnext");
}

#[test]
fn should_let_a_compact_piece_property_clear_a_chpx_deletion() {
    let text = "keep gone\r";
    let cleared = 0x41 << 1;
    let spec = Spec {
        deleted: &[(5, 9)],
        pieces: &[(0, 0), (5, cleared)],
        ..Spec::new(text)
    };
    assert_eq!(extract(&spec).content, "keep gone");
}

#[test]
fn should_merge_paragraphs_across_a_deleted_paragraph_mark() {
    let spec = Spec {
        deleted: &[(3, 4)],
        ..Spec::new("One\rTwo\rThree\r")
    };
    let result = extract(&spec);
    assert_eq!(contents(&result), ["OneTwo", "Three"]);
    assert_eq!(result.content, "OneTwo\nThree");
}

const ROWS: &str = "Intro\rA1\x07B1\x07\x07A2\x07B2\x07\x07After\r";
const ROW_RUNS: Papx<'static> = &[
    (0..6, &[]),
    (6..12, IN_TABLE),
    (12..13, ROW_END),
    (13..19, IN_TABLE),
    (19..20, ROW_END),
    (20..26, &[]),
];

#[test]
fn should_remove_a_table_row_whose_content_and_marks_are_all_deleted() {
    let spec = Spec {
        papx: ROW_RUNS,
        deleted: &[(13, 20)],
        ..Spec::new(ROWS)
    };
    let cell = Some((true, false));
    assert_eq!(
        table_marks(&extract(&spec)),
        [
            ("Intro", None),
            ("A1", cell),
            ("B1", cell),
            ("", Some((false, true))),
            ("After", None)
        ]
    );
}

#[test]
fn should_remove_a_fully_deleted_first_row_without_leaking_the_preceding_paragraph_into_it() {
    let spec = Spec {
        papx: ROW_RUNS,
        deleted: &[(6, 13)],
        ..Spec::new(ROWS)
    };
    let cell = Some((true, false));
    assert_eq!(
        table_marks(&extract(&spec)),
        [
            ("Intro", None),
            ("A2", cell),
            ("B2", cell),
            ("", Some((false, true))),
            ("After", None)
        ]
    );
}

#[test]
fn should_keep_rows_apart_when_only_a_row_end_mark_with_surviving_cells_is_deleted() {
    let spec = Spec {
        papx: ROW_RUNS,
        deleted: &[(12, 13)],
        ..Spec::new(ROWS)
    };
    let cell = Some((true, false));
    let row = Some((false, true));
    assert_eq!(
        table_marks(&extract(&spec)),
        [
            ("Intro", None),
            ("A1", cell),
            ("B1", cell),
            ("", row),
            ("A2", cell),
            ("B2", cell),
            ("", row),
            ("After", None)
        ]
    );
}

const FIELD: &str = "A\x13 HYPERLINK x\x14link\x15B\r";

#[test]
fn should_drop_a_field_deleted_as_a_whole() {
    let spec = Spec {
        deleted: &[chars_of(FIELD, "\x13 HYPERLINK x\x14link\x15")],
        ..Spec::new(FIELD)
    };
    assert_eq!(extract(&spec).content, "AB");
}

#[test]
fn should_not_leak_the_instruction_of_a_field_whose_end_marker_is_deleted() {
    let spec = Spec {
        deleted: &[chars_of(FIELD, "\x15")],
        ..Spec::new(FIELD)
    };
    assert_eq!(extract(&spec).content, "AlinkB");
}

#[test]
fn should_drop_the_deleted_result_of_a_field_and_keep_the_rest() {
    let spec = Spec {
        deleted: &[chars_of(FIELD, "link")],
        ..Spec::new(FIELD)
    };
    assert_eq!(extract(&spec).content, "AB");
}

#[test]
fn should_drop_deleted_text_from_a_footnote_story() {
    let text = "Hi\rKeep GONE\r\r";
    let spec = Spec {
        footnote_chars: 12,
        deleted: &[chars_of(text, " GONE")],
        ..Spec::new(text)
    };
    let result = extract(&spec);
    let footnote = result
        .subdocuments
        .iter()
        .find(|story| story.kind == DocSubdocumentKind::Footnote)
        .expect("footnote story");
    assert_eq!(footnote.text, "Keep");
    assert_eq!(contents(&result), ["Hi"]);
}

#[test]
fn should_keep_all_text_when_a_deletion_run_is_unreadable() {
    let mut bytes = build(&Spec {
        deleted: &[(0, 3)],
        ..Spec::new("old new\r")
    });
    let mut compound = cfb::CompoundFile::open(std::io::Cursor::new(&mut bytes[..])).expect("fixture CFB");
    let mut word = Vec::new();
    std::io::Read::read_to_end(&mut compound.open_stream("/WordDocument").unwrap(), &mut word).unwrap();
    let last = word.len() - 1;
    word[last] = 255;
    std::io::Write::write_all(&mut compound.open_stream("/WordDocument").unwrap(), &word).unwrap();
    drop(compound);
    assert_eq!(extract_doc_text(&bytes).expect("DOC").content, "old new");
}

#[tokio::test]
async fn should_drop_a_deleted_picture_character() {
    use super::test_images::{change_stream, image_config, picture_doc};
    use crate::plugins::InternalDocumentExtractor;
    let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
    let bytes = change_stream(bytes, "/WordDocument", |word| {
        // The picture's property run: 9 bytes of sprms at 1536 + 20; append sprmCFRMarkDel.
        assert_eq!(word[1536 + 20], 9);
        word[1536 + 20] = 12;
        word[1536 + 30..1536 + 33].copy_from_slice(&DELETE_SPRM);
        // The fixture's FIB declares no CHPX pair, which the text path would ignore.
        let pairs = test_rg_fc_lcb_offset();
        word[pairs - 2..pairs].copy_from_slice(&93u16.to_le_bytes());
    });
    let doc = crate::extractors::doc::DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("DOC extraction");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(
        doc.elements.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(),
        ["BEFORE", "AFTER"]
    );
}
