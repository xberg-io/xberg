use super::super::test_scripts::{script_doc, story_script_doc};
use super::super::tests::{build_doc_ole, test_rg_fc_lcb_offset, write_u32};
use super::{assert_body_scripts, with_piece_prms};
use crate::core::config::ExtractionConfig;
use crate::extractors::doc::DocExtractor;
use crate::plugins::InternalDocumentExtractor;
use crate::types::builder;
use std::io::{Cursor, Read};

const SUPERSCRIPT: &[u8] = &[0x48, 0x2a, 1];
const SUBSCRIPT: &[u8] = &[0x48, 0x2a, 2];
const NORMAL: &[u8] = &[0x48, 0x2a, 0];
const BOLD: &[u8] = &[0x35, 0x08, 1];

fn replace_clx(bytes: Vec<u8>, transform: impl FnOnce(&[u8]) -> Vec<u8>) -> Vec<u8> {
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
    let pair = test_rg_fc_lcb_offset() + super::super::FIB_FC_LCB_IDX_CLX * 8;
    let offset = u32::from_le_bytes(word[pair..pair + 4].try_into().unwrap()) as usize;
    let length = u32::from_le_bytes(word[pair + 4..pair + 8].try_into().unwrap()) as usize;
    let clx = transform(&table[offset..offset + length]);
    let new_offset = table.len();
    table.extend_from_slice(&clx);
    write_u32(&mut word, pair, new_offset as u32);
    write_u32(&mut word, pair + 4, clx.len() as u32);
    build_doc_ole(&word, &table)
}

fn with_groups(bytes: Vec<u8>, groups: &[&[u8]]) -> Vec<u8> {
    replace_clx(bytes, |pcdt| {
        assert_eq!(pcdt[0], 2);
        let mut clx = Vec::new();
        for group in groups {
            clx.push(1);
            clx.extend_from_slice(&i16::try_from(group.len()).unwrap().to_le_bytes());
            clx.extend_from_slice(group);
        }
        clx.extend_from_slice(pcdt);
        clx
    })
}

fn scripted_nine(compressed: bool, prm: u16) -> Vec<u8> {
    let source = script_doc(&[("10", 0), ("9", 1), ("/l\r", 0)], compressed);
    with_piece_prms(source, compressed, &[(0, 2, 0, 0), (2, 3, 2, prm), (3, 6, 3, 0)], 6)
}

#[tokio::test]
async fn should_apply_complex_prm_group_indexes_by_piece_gh2275() {
    for (compressed, prefix) in [(true, "é"), (false, "é"), (false, "😀é")] {
        let base = format!("{prefix}10");
        let offset = base.encode_utf16().count() as u32;
        let source = script_doc(&[(base.as_str(), 0), ("9/l; H2O\r", 0)], compressed);
        let pieces = [
            (0, offset, 0, 0),
            (offset, offset + 1, offset, 1),
            (offset + 1, offset + 6, offset + 1, 0),
            (offset + 6, offset + 7, offset + 6, 3),
            (offset + 7, offset + 9, offset + 7, 0),
        ];
        let bytes = with_groups(
            with_piece_prms(source, compressed, &pieces, offset + 9),
            &[SUPERSCRIPT, SUBSCRIPT],
        );
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
async fn should_apply_complex_prm_last_script_and_explicit_normal_after_chpx_gh2275() {
    for compressed in [false, true] {
        for (group, expected, markdown) in [
            ([NORMAL, SUBSCRIPT].concat(), vec![builder::subscript(2, 3)], "10~9~/l"),
            ([SUBSCRIPT, NORMAL].concat(), Vec::new(), "109/l"),
        ] {
            let bytes = with_groups(scripted_nine(compressed, 1), &[&group]);
            assert_body_scripts(bytes, "109/l", expected, markdown).await;
        }
    }
}

#[tokio::test]
async fn should_keep_complex_prms_distinct_for_reused_physical_bytes_gh2275() {
    for compressed in [false, true] {
        let source = script_doc(&[("9\r", 0)], compressed);
        let pieces = [(0, 1, 0, 1), (1, 2, 0, 3), (2, 3, 0, 5), (3, 4, 1, 0)];
        let bytes = with_groups(
            with_piece_prms(source, compressed, &pieces, 4),
            &[SUPERSCRIPT, SUBSCRIPT, NORMAL],
        );
        assert_body_scripts(
            bytes,
            "999",
            vec![builder::superscript(0, 1), builder::subscript(1, 2)],
            "^9^~9~9",
        )
        .await;
    }
}

#[tokio::test]
async fn should_preserve_chpx_for_unrelated_complex_prm_properties_gh2275() {
    for compressed in [false, true] {
        let bytes = with_groups(scripted_nine(compressed, 1), &[BOLD]);
        assert_body_scripts(bytes, "109/l", vec![builder::superscript(2, 3)], "10^9^/l").await;
    }
}

#[tokio::test]
async fn should_accept_maximum_bounded_complex_prm_group_gh2275() {
    const MAX_GROUP_BYTES: usize = 0x3fa2;
    let mut group = BOLD.repeat(MAX_GROUP_BYTES / BOLD.len());
    assert_eq!(group.len(), MAX_GROUP_BYTES);
    group[MAX_GROUP_BYTES - SUBSCRIPT.len()..].copy_from_slice(SUBSCRIPT);
    let bytes = with_groups(scripted_nine(false, 1), &[&group]);
    assert_body_scripts(bytes, "109/l", vec![builder::subscript(2, 3)], "10~9~/l").await;
}

#[test]
fn should_warn_and_preserve_chpx_for_invalid_complex_prm_indexes_gh2275() {
    for prm in [3, u16::MAX] {
        let bytes = with_groups(scripted_nine(false, prm), &[SUBSCRIPT]);
        let parsed = super::super::extract_doc_text(&bytes).expect("invalid property index preserves valid text");
        assert_eq!(parsed.content, "109/l");
        assert_eq!(parsed.paragraphs.len(), 1);
        assert_eq!(parsed.paragraphs[0].annotations, [builder::superscript(2, 3)]);
        assert_eq!(
            parsed.processing_warnings.len(),
            1,
            "invalid property reference must not disappear silently"
        );
    }
}

#[test]
fn should_reject_partial_complex_prm_properties_without_partial_formatting_gh2275() {
    for group in [vec![0x48], vec![0x48, 0x2a], [SUBSCRIPT, &[0x35]].concat()] {
        let bytes = with_groups(scripted_nine(false, 1), &[&group]);
        let parsed = super::super::extract_doc_text(&bytes).expect("invalid property payload preserves valid text");
        assert_eq!(parsed.content, "109/l");
        assert_eq!(parsed.paragraphs.len(), 1);
        assert_eq!(parsed.paragraphs[0].annotations, [builder::superscript(2, 3)]);
        assert_eq!(
            parsed.processing_warnings.len(),
            1,
            "malformed property group must be reported"
        );
    }
}

#[test]
fn should_reject_unbounded_complex_prm_record_framing_gh2275() {
    let source = scripted_nine(false, 1);
    for declared in [-1i16, 0x3fa3] {
        let bytes = replace_clx(source.clone(), |pcdt| {
            let mut clx = vec![1];
            clx.extend_from_slice(&declared.to_le_bytes());
            clx.resize(3 + usize::from(declared as u16), 0);
            clx.extend_from_slice(pcdt);
            clx
        });
        assert!(
            super::super::extract_doc_text(&bytes).is_err(),
            "invalid signed/over-limit cbGrpprl {declared}"
        );
    }
    let truncated = replace_clx(source, |_| vec![1, 3, 0, 0x48, 0x2a]);
    assert!(
        super::super::extract_doc_text(&truncated).is_err(),
        "truncated property framing must not guess text"
    );
}

#[tokio::test]
async fn should_project_complex_prms_into_adjacent_footnotes_gh2275() {
    for (compressed, prefix) in [(true, "é"), (false, "é"), (false, "😀é")] {
        let first = format!("{prefix}10");
        let runs = [("body\r", 0), (first.as_str(), 0), ("9/l\rH2O\r\r", 0)];
        let script_cp = 5 + first.encode_utf16().count() as u32;
        let split = script_cp - 5 + 4;
        let end = split + 4;
        let bytes = story_script_doc(&runs, compressed, Some((5, &[0, split, end, end + 1])));
        let pieces = [
            (0, script_cp, 0, 0),
            (script_cp, script_cp + 1, script_cp, 1),
            (script_cp + 1, script_cp + 5, script_cp + 1, 0),
            (script_cp + 5, script_cp + 6, script_cp + 5, 3),
            (script_cp + 6, script_cp + 9, script_cp + 6, 0),
        ];
        let bytes = with_groups(
            with_piece_prms(bytes, compressed, &pieces, 5),
            &[SUPERSCRIPT, SUBSCRIPT],
        );
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
