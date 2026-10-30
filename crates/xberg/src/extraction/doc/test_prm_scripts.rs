#[path = "test_complex_prm_scripts.rs"]
mod complex;

use super::test_scripts::{script_doc, story_script_doc};
use super::tests::{
    TestPiece, build_doc_ole, build_plc_pcd, compressed_fc, test_rg_fc_lcb_offset, test_rg_lw_offset, write_u32,
};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::doc::DocExtractor;
use crate::plugins::InternalDocumentExtractor;
use crate::types::{TextAnnotation, builder};
use std::io::{Cursor, Read};

const TEXT_OFFSET: u32 = 1024;
// ~keep MS-DOC Prm0: bit 0 is clear, isprm 0x68 occupies bits 1..7, operand bits 8..15.
const PRM_NORMAL: u16 = 0x00d0;
const PRM_SUPERSCRIPT: u16 = 0x01d0;
const PRM_SUBSCRIPT: u16 = 0x02d0;
const PRM_BOLD: u16 = 0x01aa;

type Piece = (u32, u32, u32, u16);

fn with_piece_prms(bytes: Vec<u8>, compressed: bool, pieces: &[Piece], main_cp: u32) -> Vec<u8> {
    let mut compound = cfb::CompoundFile::open(Cursor::new(bytes)).expect("valid source CFB");
    let mut word = Vec::new();
    compound
        .open_stream("/WordDocument")
        .unwrap()
        .read_to_end(&mut word)
        .unwrap();
    let mut table = Vec::new();
    compound
        .open_stream("/1Table")
        .unwrap()
        .read_to_end(&mut table)
        .unwrap();
    let width = if compressed { 1 } else { 2 };
    let entries: Vec<_> = pieces
        .iter()
        .map(|&(start, end, source_cp, _)| {
            let fc = TEXT_OFFSET + source_cp * width;
            TestPiece {
                cp_start: start,
                cp_end: end,
                fc_raw: if compressed { compressed_fc(fc) } else { fc },
            }
        })
        .collect();
    let mut plc = build_plc_pcd(&entries);
    for (index, &(_, _, _, prm)) in pieces.iter().enumerate() {
        let offset = (pieces.len() + 1) * 4 + index * 8 + 6;
        plc[offset..offset + 2].copy_from_slice(&prm.to_le_bytes());
        assert_eq!(u16::from_le_bytes(plc[offset..offset + 2].try_into().unwrap()), prm);
    }
    let clx_offset = table.len();
    table.push(2);
    table.extend_from_slice(&(plc.len() as u32).to_le_bytes());
    table.extend_from_slice(&plc);
    let pair = test_rg_fc_lcb_offset() + super::FIB_FC_LCB_IDX_CLX * 8;
    write_u32(&mut word, pair, clx_offset as u32);
    write_u32(&mut word, pair + 4, (plc.len() + 5) as u32);
    write_u32(&mut word, test_rg_lw_offset() + super::FIB_LW_IDX_CCP_TEXT * 4, main_cp);
    build_doc_ole(&word, &table)
}

async fn assert_body_scripts(bytes: Vec<u8>, text: &str, annotations: Vec<TextAnnotation>, markdown: &str) {
    let parsed = super::extract_doc_text(&bytes).expect("valid PRM DOC");
    assert!(
        parsed.processing_warnings.is_empty(),
        "{:?}",
        parsed.processing_warnings
    );
    assert_eq!(parsed.paragraphs.len(), 1);
    assert_eq!(parsed.content, text);
    assert_eq!(parsed.paragraphs[0].content, text);
    assert_eq!(parsed.paragraphs[0].annotations, annotations);
    for (output_format, expected) in [(OutputFormat::Plain, text), (OutputFormat::Markdown, markdown)] {
        let output = crate::extract(
            crate::ExtractInput::from_bytes(bytes.clone(), "application/msword", None),
            &ExtractionConfig {
                output_format,
                use_cache: false,
                escape_markdown: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(output.errors.is_empty(), "{:?}", output.errors);
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content.trim_end(), expected);
    }
}

#[tokio::test]
async fn should_apply_compact_prm_scripts_by_logical_piece_gh2275() {
    for (compressed, prefix) in [(true, "é"), (false, "é"), (false, "😀é")] {
        let base = format!("{prefix}10");
        let runs = [(base.as_str(), 0), ("9/l; H2O\r", 0)];
        let offset = base.encode_utf16().count() as u32;
        let pieces = [
            (0, offset, 0, 0),
            (offset, offset + 1, offset, PRM_SUPERSCRIPT),
            (offset + 1, offset + 6, offset + 1, 0),
            (offset + 6, offset + 7, offset + 6, PRM_SUBSCRIPT),
            (offset + 7, offset + 9, offset + 7, 0),
        ];
        let bytes = with_piece_prms(script_doc(&runs, compressed), compressed, &pieces, offset + 9);
        let start = prefix.len() as u32;
        assert_body_scripts(
            bytes,
            &format!("{prefix}109/l; H2O"),
            vec![
                builder::superscript(start + 2, start + 3),
                builder::subscript(start + 8, start + 9),
            ],
            &format!("{prefix}10^9^/l; H~2~O"),
        )
        .await;
    }
}

#[tokio::test]
async fn should_reset_chpx_with_explicit_normal_compact_prm_gh2275() {
    for compressed in [false, true] {
        let runs = [("10", 0), ("9", 1), ("/l\r", 0)];
        let pieces = [(0, 2, 0, 0), (2, 3, 2, PRM_NORMAL), (3, 6, 3, 0)];
        let bytes = with_piece_prms(script_doc(&runs, compressed), compressed, &pieces, 6);
        assert_body_scripts(bytes, "109/l", Vec::new(), "109/l").await;
    }
}

#[tokio::test]
async fn should_preserve_chpx_for_unrelated_compact_prm_gh2275() {
    for compressed in [false, true] {
        for prm in [0, PRM_BOLD] {
            let runs = [("10", 0), ("9", 1), ("/l\r", 0)];
            let pieces = [(0, 2, 0, 0), (2, 3, 2, prm), (3, 6, 3, 0)];
            let bytes = with_piece_prms(script_doc(&runs, compressed), compressed, &pieces, 6);
            assert_body_scripts(bytes, "109/l", vec![builder::superscript(2, 3)], "10^9^/l").await;
        }
    }
}

#[tokio::test]
async fn should_keep_distinct_compact_prms_for_reused_physical_bytes_gh2275() {
    for compressed in [false, true] {
        let pieces = [
            (0, 1, 0, PRM_NORMAL),
            (1, 2, 0, PRM_SUPERSCRIPT),
            (2, 3, 0, PRM_SUBSCRIPT),
            (3, 4, 1, 0),
        ];
        let bytes = with_piece_prms(script_doc(&[("9\r", 0)], compressed), compressed, &pieces, 4);
        assert_body_scripts(
            bytes,
            "999",
            vec![builder::superscript(1, 2), builder::subscript(2, 3)],
            "9^9^~9~",
        )
        .await;
    }
}

#[tokio::test]
async fn should_project_compact_prms_into_adjacent_footnotes_gh2275() {
    for (compressed, prefix) in [(true, "é"), (false, "é"), (false, "😀é")] {
        let first = format!("{prefix}10");
        let runs = [("body\r", 0), (first.as_str(), 0), ("9/l\rH2O\r\r", 0)];
        let script_cp = 5 + first.encode_utf16().count() as u32;
        let split = script_cp - 5 + 4;
        let end = split + 4;
        let bytes = story_script_doc(&runs, compressed, Some((5, &[0, split, end, end + 1])));
        let pieces = [
            (0, script_cp, 0, 0),
            (script_cp, script_cp + 1, script_cp, PRM_SUPERSCRIPT),
            (script_cp + 1, script_cp + 5, script_cp + 1, 0),
            (script_cp + 5, script_cp + 6, script_cp + 5, PRM_SUBSCRIPT),
            (script_cp + 6, script_cp + 9, script_cp + 6, 0),
        ];
        let bytes = with_piece_prms(bytes, compressed, &pieces, 5);
        let doc = DocExtractor::new()
            .extract_content(
                &bytes,
                "application/msword",
                &ExtractionConfig {
                    use_cache: false,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let definitions: Vec<_> = doc
            .elements
            .iter()
            .filter(|element| element.kind == crate::types::internal::ElementKind::FootnoteDefinition)
            .collect();
        assert_eq!(definitions.len(), 2);
        assert_eq!(definitions[0].text, format!("{prefix}109/l"));
        assert_eq!(definitions[1].text, "H2O");
        let start = prefix.len() as u32 + 2;
        assert_eq!(definitions[0].annotations, [builder::superscript(start, start + 1)]);
        assert_eq!(definitions[1].annotations, [builder::subscript(1, 2)]);
    }
}
