use super::tests::{
    TestPiece, build_doc_ole, build_fib, build_plc_pcd, build_table_stream, compressed_fc, test_rg_fc_lcb_offset,
    write_u32,
};
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::doc::DocExtractor;
use crate::plugins::InternalDocumentExtractor;
use crate::types::builder;
use std::io::{Cursor, Read};

const TEXT_OFFSET: usize = 1024;
const FKP_OFFSET: usize = 1536;
const FKP_SIZE: usize = 512;
const PROPERTY_OFFSET: usize = 256;
const SCRIPT_RUNS: &[(&str, u8)] = &[("7,2 x 10", 0), ("9", 1), ("/l; H", 0), ("2", 2), ("O\r", 0)];

pub(super) fn script_doc(runs: &[(&str, u8)], compressed: bool) -> Vec<u8> {
    story_script_doc(runs, compressed, None)
}

pub(super) fn story_script_doc(runs: &[(&str, u8)], compressed: bool, footnotes: Option<(u32, &[u32])>) -> Vec<u8> {
    assert!(!runs.is_empty() && runs.len() <= 32);
    let text: String = runs.iter().map(|(text, _)| *text).collect();
    let bytes: Vec<u8> = if compressed {
        text.chars()
            .map(|character| match character {
                '€' => 0x80,
                character if u32::from(character) <= 0xff => character as u8,
                _ => panic!("fixture character is not in its CP1252 subset"),
            })
            .collect()
    } else {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    };
    assert!(bytes.len() < FKP_OFFSET - TEXT_OFFSET);
    let mut word = build_fib(FKP_OFFSET + FKP_SIZE, text.encode_utf16().count() as u32, 0, 0, 0);
    word[TEXT_OFFSET..TEXT_OFFSET + bytes.len()].copy_from_slice(&bytes);
    write_u32(&mut word, 0x18, TEXT_OFFSET as u32);
    write_u32(&mut word, 0x1c, (TEXT_OFFSET + bytes.len()) as u32);
    let pairs = test_rg_fc_lcb_offset();
    word[pairs - 2..pairs].copy_from_slice(&93u16.to_le_bytes());
    let piece = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: text.encode_utf16().count() as u32,
        fc_raw: if compressed {
            compressed_fc(TEXT_OFFSET as u32)
        } else {
            TEXT_OFFSET as u32
        },
    }]);
    let mut table = build_table_stream(&mut word, &piece);
    write_u32(&mut table, 9, piece.len() as u32);
    add_script_fkp(&mut word, &mut table, runs, compressed);
    if let Some((main_cp, bounds)) = footnotes {
        add_footnote_story_bounds(
            &mut word,
            &mut table,
            main_cp,
            text.encode_utf16().count() as u32,
            bounds,
        );
    }
    build_doc_ole(&word, &table)
}

fn add_footnote_story_bounds(word: &mut [u8], table: &mut Vec<u8>, main_cp: u32, total_cp: u32, bounds: &[u32]) {
    let lw = super::tests::test_rg_lw_offset();
    write_u32(word, lw + super::FIB_LW_IDX_CCP_TEXT * 4, main_cp);
    write_u32(word, lw + super::FIB_LW_IDX_CCP_FTN * 4, total_cp - main_cp);
    let offset = table.len();
    for bound in bounds {
        table.extend_from_slice(&bound.to_le_bytes());
    }
    let pairs = test_rg_fc_lcb_offset();
    write_u32(word, pairs + 3 * 8, offset as u32);
    write_u32(word, pairs + 3 * 8 + 4, (bounds.len() * 4) as u32);
}

fn add_script_fkp(word: &mut [u8], table: &mut Vec<u8>, runs: &[(&str, u8)], compressed: bool) {
    let mut fc = TEXT_OFFSET as u32;
    let width = if compressed { 1 } else { 2 };
    let end_fc = fc
        + runs
            .iter()
            .map(|(text, _)| text.encode_utf16().count() as u32 * width)
            .sum::<u32>();
    let bte_offset = table.len();
    table.extend_from_slice(&fc.to_le_bytes());
    table.extend_from_slice(&end_fc.to_le_bytes());
    table.extend_from_slice(&3u32.to_le_bytes());
    let pair = test_rg_fc_lcb_offset() + 12 * 8;
    write_u32(word, pair, bte_offset as u32);
    write_u32(word, pair + 4, 12);
    let fkp = &mut word[FKP_OFFSET..FKP_OFFSET + FKP_SIZE];
    write_u32(fkp, 0, fc);
    for (index, (text, script)) in runs.iter().enumerate() {
        fc += text.encode_utf16().count() as u32 * width;
        write_u32(fkp, (index + 1) * 4, fc);
        let property = PROPERTY_OFFSET + index * 4;
        fkp[4 * (runs.len() + 1) + index] = (property / 2) as u8;
        fkp[property..property + 4].copy_from_slice(&[3, 0x48, 0x2a, *script]);
    }
    fkp[FKP_SIZE - 1] = runs.len() as u8;
}

#[test]
fn should_decode_both_doc_script_fixture_encodings_without_losing_text() {
    for compressed in [false, true] {
        let bytes = script_doc(SCRIPT_RUNS, compressed);
        assert_fixture_script_properties(&bytes, compressed);
        let result = super::extract_doc_text(&bytes).expect("valid DOC text");
        assert_eq!(result.content, "7,2 x 109/l; H2O");
        assert_eq!(result.paragraphs.len(), 1);
        assert_eq!(result.paragraphs[0].content, "7,2 x 109/l; H2O");
        assert_eq!(result.processing_warnings.len(), 0);
        eprintln!("DOC_CASE validity_ascii compressed={compressed}");
        let non_ascii = script_doc(&[("café €", 0), ("9", 1), ("\r", 0)], compressed);
        let decoded = super::extract_doc_text(&non_ascii).expect("non-ASCII DOC text");
        assert_eq!(decoded.content, "café €9");
        assert_eq!(decoded.paragraphs.len(), 1);
        assert_eq!(decoded.paragraphs[0].content, "café €9");
        assert!(decoded.processing_warnings.is_empty());
        eprintln!("DOC_CASE validity_non_ascii compressed={compressed}");
    }
}

fn assert_fixture_script_properties(bytes: &[u8], compressed: bool) {
    let mut compound = cfb::CompoundFile::open(Cursor::new(bytes)).expect("fixture CFB");
    let mut word = Vec::new();
    compound
        .open_stream("/WordDocument")
        .expect("fixture WordDocument")
        .read_to_end(&mut word)
        .expect("read fixture WordDocument");
    let mut table = Vec::new();
    compound
        .open_stream("/1Table")
        .expect("fixture 1Table")
        .read_to_end(&mut table)
        .expect("read fixture 1Table");
    let pair = test_rg_fc_lcb_offset() + 12 * 8;
    let bte_offset = u32::from_le_bytes(word[pair..pair + 4].try_into().expect("BTE offset")) as usize;
    assert_eq!(
        u32::from_le_bytes(word[pair + 4..pair + 8].try_into().expect("BTE length")),
        12
    );
    let bte = &table[bte_offset..bte_offset + 12];
    let end_fc = if compressed { 1041u32 } else { 1058u32 };
    assert_eq!(u32::from_le_bytes(bte[0..4].try_into().expect("BTE start")), 1024);
    assert_eq!(u32::from_le_bytes(bte[4..8].try_into().expect("BTE end")), end_fc);
    assert_eq!(u32::from_le_bytes(bte[8..12].try_into().expect("BTE page")), 3);
    let fkp = &word[1536..2048];
    assert_eq!(fkp[511], 5);
    assert_eq!(u32::from_le_bytes(fkp[0..4].try_into().expect("first FC")), 1024);
    let first_end = if compressed { 1032 } else { 1040 };
    assert_eq!(u32::from_le_bytes(fkp[4..8].try_into().expect("run end FC")), first_end);
    assert_eq!(fkp[24..29], [128, 130, 132, 134, 136]);
    assert_eq!(
        fkp[256..276],
        [
            3, 0x48, 0x2a, 0, 3, 0x48, 0x2a, 1, 3, 0x48, 0x2a, 0, 3, 0x48, 0x2a, 2, 3, 0x48, 0x2a, 0
        ]
    );
}

#[tokio::test]
async fn should_preserve_doc_script_markers_through_public_markdown_extraction() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        escape_markdown: false,
        use_cache: false,
        ..Default::default()
    };
    let mut actual = Vec::new();
    for compressed in [false, true] {
        let output = crate::extract(
            crate::ExtractInput::from_bytes(script_doc(SCRIPT_RUNS, compressed), "application/msword", None),
            &config,
        )
        .await
        .expect("public DOC extraction");
        assert_eq!(output.errors.len(), 0, "{:?}", output.errors);
        assert_eq!(output.results.len(), 1);
        eprintln!(
            "DOC_CASE markdown compressed={compressed} actual={:?}",
            output.results[0].content
        );
        actual.push(output.results[0].content.clone());
    }
    assert_eq!(actual, ["7,2 x 10^9^/l; H~2~O\n", "7,2 x 10^9^/l; H~2~O\n"]);
}

#[tokio::test]
async fn should_keep_doc_script_characters_plain_in_plain_output() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        use_cache: false,
        ..Default::default()
    };
    let output = crate::extract(
        crate::ExtractInput::from_bytes(script_doc(SCRIPT_RUNS, false), "application/msword", None),
        &config,
    )
    .await
    .expect("public plain DOC extraction");
    assert_eq!(output.errors.len(), 0, "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    eprintln!("DOC_CASE plain");
    assert_eq!(output.results[0].content, "7,2 x 109/l; H2O");
}

#[tokio::test]
async fn should_align_doc_script_annotations_after_field_and_whitespace_normalization() {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let mut actual = Vec::new();
    for prefix in ["  é", "  😀é"] {
        let runs = [
            (prefix, 0),
            ("\x13HYPERLINK x\x14", 0),
            ("10", 0),
            ("9", 1),
            ("\x15/l \x0bH", 0),
            ("2", 2),
            ("O  \r", 0),
        ];
        let doc = DocExtractor::new()
            .extract_content(&script_doc(&runs, false), "application/msword", &config)
            .await
            .expect("DOC with field markup");
        assert_eq!(doc.elements.len(), 1);
        eprintln!(
            "DOC_CASE annotations supplementary={} text={:?} annotation_count={}",
            prefix.contains('😀'),
            doc.elements[0].text,
            doc.elements[0].annotations.len()
        );
        actual.push((doc.elements[0].text.clone(), doc.elements[0].annotations.clone()));
    }
    assert_eq!(
        actual,
        vec![
            (
                "é109/l \nH2O".to_owned(),
                vec![builder::superscript(4, 5), builder::subscript(10, 11)]
            ),
            (
                "😀é109/l \nH2O".to_owned(),
                vec![builder::superscript(8, 9), builder::subscript(14, 15)]
            ),
        ]
    );
}

#[tokio::test]
async fn should_preserve_scripts_in_adjacent_doc_footnotes_gh2275() {
    for (compressed, prefix) in [(true, "é"), (false, "é"), (false, "😀é")] {
        let first = format!("  {prefix}10");
        let runs = [
            ("body\r", 0),
            (first.as_str(), 0),
            ("9", 1),
            ("/l \r H", 0),
            ("2", 2),
            ("O \r\r", 0),
        ];
        let first_cp = first.encode_utf16().count() as u32 + 5;
        let total_cp = first_cp + 6;
        let bytes = story_script_doc(&runs, compressed, Some((5, &[0, first_cp, total_cp, total_cp + 1])));
        let config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };
        let doc = DocExtractor::new()
            .extract_content(&bytes, "application/msword", &config)
            .await
            .unwrap();
        let definitions: Vec<_> = doc
            .elements
            .iter()
            .filter(|element| element.kind == crate::types::internal::ElementKind::FootnoteDefinition)
            .collect();
        assert_eq!(definitions.len(), 2);
        assert_eq!(definitions[0].text, format!("{prefix}109/l"));
        assert_eq!(
            definitions[0].annotations,
            [builder::superscript(prefix.len() as u32 + 2, prefix.len() as u32 + 3)]
        );
        assert_eq!(definitions[1].text, "H2O");
        assert_eq!(definitions[1].annotations, [builder::subscript(1, 2)]);
        assert_subdocument_script_output(&bytes, prefix, &config).await;
    }
}

async fn assert_subdocument_script_output(bytes: &[u8], prefix: &str, config: &ExtractionConfig) {
    for (format, superscript, subscript) in [
        (OutputFormat::Markdown, "^9^", "H~2~O"),
        (OutputFormat::Djot, "^9^", "H~2~O"),
        (OutputFormat::Html, "<sup>9</sup>", "H<sub>2</sub>O"),
    ] {
        let output = crate::extract(
            crate::ExtractInput::from_bytes(bytes.to_vec(), "application/msword", None),
            &ExtractionConfig {
                output_format: format.clone(),
                escape_markdown: false,
                ..config.clone()
            },
        )
        .await
        .unwrap();
        assert_eq!(output.results.len(), 1);
        assert!(output.errors.is_empty());
        let content = &output.results[0].content;
        assert!(
            content.contains(&format!("{prefix}10{superscript}/l")),
            "{format:?}: {content}"
        );
        assert!(content.contains(subscript), "{format:?}: {content}");
    }
    let plain = crate::extract(
        crate::ExtractInput::from_bytes(bytes.to_vec(), "application/msword", None),
        &ExtractionConfig {
            output_format: OutputFormat::Plain,
            ..config.clone()
        },
    )
    .await
    .unwrap();
    assert_eq!(plain.results.len(), 1);
    assert!(plain.errors.is_empty());
    assert!(!plain.results[0].content.contains(['^', '~']));
    assert!(plain.results[0].content.contains(&format!("{prefix}109/l")));
}

#[path = "test_style_scripts.rs"]
mod style_scripts;
