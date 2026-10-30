use super::super::tests::{build_doc_ole, test_rg_fc_lcb_offset, write_u32};
use super::story_script_doc;
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::types::builder;
use std::io::{Cursor, Read};

const TEXT_OFFSET: u32 = 1024;
const CHPX_PAGE: usize = 1536;
const PAGE_SIZE: usize = 512;
const NO_BASE: u16 = 0x0fff;
const SUPERSCRIPT: &[u8] = &[0x48, 0x2a, 1];
const SUBSCRIPT: &[u8] = &[0x48, 0x2a, 2];
const NORMAL: &[u8] = &[0x48, 0x2a, 0];
const CHAR_STYLE_ONE: &[u8] = &[0x30, 0x4a, 1, 0];
const CHAR_STYLE_TWO: &[u8] = &[0x30, 0x4a, 2, 0];

struct Style<'a> {
    kind: u16,
    base: u16,
    properties: &'a [u8],
}

fn paragraph_style(base: u16, properties: &[u8]) -> Style<'_> {
    Style {
        kind: 1,
        base,
        properties,
    }
}

fn character_style(base: u16, properties: &[u8]) -> Style<'_> {
    Style {
        kind: 2,
        base,
        properties,
    }
}

fn word(value: u16, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn upx(properties: &[u8], output: &mut Vec<u8>) {
    word(properties.len() as u16, output);
    output.extend_from_slice(properties);
    if !properties.len().is_multiple_of(2) {
        output.push(0);
    }
}

fn style_record(index: u16, style: &Style<'_>) -> Vec<u8> {
    let mut record = Vec::new();
    word(if index == 0 { 0 } else { 0x0ffe }, &mut record);
    word((style.base << 4) | style.kind, &mut record);
    word((index << 4) | if style.kind == 1 { 2 } else { 1 }, &mut record);
    word(0, &mut record);
    word(0, &mut record);
    let name: Vec<u16> = format!("Style {index}").encode_utf16().collect();
    word(name.len() as u16, &mut record);
    for unit in name {
        word(unit, &mut record);
    }
    word(0, &mut record);
    if style.kind == 1 {
        upx(&index.to_le_bytes(), &mut record);
    }
    upx(style.properties, &mut record);
    let length = record.len() as u16;
    record[6..8].copy_from_slice(&length.to_le_bytes());
    record
}

fn add_styles(word_doc: &mut [u8], table: &mut Vec<u8>, styles: &[Style<'_>]) {
    let offset = table.len();
    let mut sheet = vec![0; 20];
    sheet[0..2].copy_from_slice(&18u16.to_le_bytes());
    sheet[2..4].copy_from_slice(&(styles.len() as u16).to_le_bytes());
    sheet[4..6].copy_from_slice(&10u16.to_le_bytes());
    for (index, style) in styles.iter().enumerate() {
        let record = style_record(index as u16, style);
        word(record.len() as u16, &mut sheet);
        sheet.extend_from_slice(&record);
    }
    let pair = test_rg_fc_lcb_offset() + 8;
    write_u32(word_doc, pair, offset as u32);
    write_u32(word_doc, pair + 4, sheet.len() as u32);
    table.extend_from_slice(&sheet);
}

fn add_paragraph_styles(word_doc: &mut Vec<u8>, table: &mut Vec<u8>, paragraphs: &[(u32, u32, u16)]) {
    assert!(!paragraphs.is_empty() && paragraphs.len() <= 8);
    let page = word_doc.len().div_ceil(PAGE_SIZE) * PAGE_SIZE;
    word_doc.resize(page + PAGE_SIZE, 0);
    let fkp = &mut word_doc[page..page + PAGE_SIZE];
    for (index, &(start, end, style)) in paragraphs.iter().enumerate() {
        write_u32(fkp, index * 4, start);
        write_u32(fkp, (index + 1) * 4, end);
        let offset = 256 + index * 4;
        fkp[4 * (paragraphs.len() + 1) + index * 13] = (offset / 2) as u8;
        fkp[offset] = 0;
        fkp[offset + 1] = 1;
        fkp[offset + 2..offset + 4].copy_from_slice(&style.to_le_bytes());
    }
    fkp[PAGE_SIZE - 1] = paragraphs.len() as u8;
    let offset = table.len();
    word_doc[test_rg_fc_lcb_offset() - 2..test_rg_fc_lcb_offset()].copy_from_slice(&93u16.to_le_bytes());
    table.extend_from_slice(&paragraphs[0].0.to_le_bytes());
    table.extend_from_slice(&paragraphs.last().unwrap().1.to_le_bytes());
    table.extend_from_slice(&((page / PAGE_SIZE) as u32).to_le_bytes());
    let pair = test_rg_fc_lcb_offset() + 13 * 8;
    write_u32(word_doc, pair, offset as u32);
    write_u32(word_doc, pair + 4, 12);
}

fn replace_character_properties(word_doc: &mut [u8], runs: &[(&str, &[u8])], compressed: bool) {
    let fkp = &mut word_doc[CHPX_PAGE..CHPX_PAGE + PAGE_SIZE];
    fkp.fill(0);
    let mut fc = TEXT_OFFSET;
    write_u32(fkp, 0, fc);
    for (index, &(text, properties)) in runs.iter().enumerate() {
        fc += text.encode_utf16().count() as u32 * if compressed { 1 } else { 2 };
        write_u32(fkp, (index + 1) * 4, fc);
        if !properties.is_empty() {
            let offset = 256 + index * 16;
            assert!(properties.len() < 16 && offset + properties.len() + 1 < PAGE_SIZE - 1);
            fkp[4 * (runs.len() + 1) + index] = (offset / 2) as u8;
            fkp[offset] = properties.len() as u8;
            fkp[offset + 1..offset + 1 + properties.len()].copy_from_slice(properties);
        }
    }
    fkp[PAGE_SIZE - 1] = runs.len() as u8;
}

fn style_doc(
    runs: &[(&str, &[u8])],
    compressed: bool,
    styles: &[Style<'_>],
    paragraph_styles: &[(u32, u32, u16)],
    footnotes: Option<(u32, &[u32])>,
) -> Vec<u8> {
    let original: Vec<_> = runs.iter().map(|(text, _)| (*text, 0)).collect();
    let bytes = story_script_doc(&original, compressed, footnotes);
    let mut compound = cfb::CompoundFile::open(Cursor::new(bytes)).expect("valid source DOC");
    let mut word_doc = Vec::new();
    compound
        .open_stream("/WordDocument")
        .unwrap()
        .read_to_end(&mut word_doc)
        .unwrap();
    let mut table = Vec::new();
    compound
        .open_stream("/1Table")
        .unwrap()
        .read_to_end(&mut table)
        .unwrap();
    replace_character_properties(&mut word_doc, runs, compressed);
    let width = if compressed { 1 } else { 2 };
    let paragraphs: Vec<_> = paragraph_styles
        .iter()
        .map(|&(start, end, style)| (TEXT_OFFSET + start * width, TEXT_OFFSET + end * width, style))
        .collect();
    add_paragraph_styles(&mut word_doc, &mut table, &paragraphs);
    add_styles(&mut word_doc, &mut table, styles);
    build_doc_ole(&word_doc, &table)
}

async fn assert_style_output(bytes: Vec<u8>, plain: &str, marked: &str) {
    let parsed = super::super::extract_doc_text(&bytes).expect("valid styled DOC");
    assert_eq!(parsed.content, plain);
    assert_eq!(parsed.paragraphs.len(), 1);
    assert!(
        parsed.processing_warnings.is_empty(),
        "{:?}",
        parsed.processing_warnings
    );
    for (output_format, expected) in [(OutputFormat::Plain, plain), (OutputFormat::Markdown, marked)] {
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
async fn should_inherit_paragraph_style_scripts_without_direct_chpx_gh2275() {
    for (compressed, text) in [(true, "é9\r"), (false, "é9\r"), (false, "😀é9\r")] {
        let length = text.encode_utf16().count() as u32;
        let styles = [paragraph_style(NO_BASE, SUPERSCRIPT), paragraph_style(0, &[])];
        let bytes = style_doc(&[(text, &[])], compressed, &styles, &[(0, length, 1)], None);
        let plain = text.trim_end_matches('\r');
        assert_style_output(bytes, plain, &format!("^{plain}^")).await;
    }
}

#[tokio::test]
async fn should_inherit_selected_character_style_scripts_gh2275() {
    let styles = [
        paragraph_style(NO_BASE, &[]),
        character_style(NO_BASE, SUPERSCRIPT),
        character_style(1, &[]),
    ];
    let bytes = style_doc(
        &[("10", &[]), ("9", CHAR_STYLE_TWO), ("\r", &[])],
        true,
        &styles,
        &[(0, 4, 0)],
        None,
    );
    assert_style_output(bytes, "109", "10^9^").await;
}

#[tokio::test]
async fn should_distinguish_absent_chpx_from_explicit_normal_over_a_style_gh2275() {
    let styles = [paragraph_style(NO_BASE, SUPERSCRIPT)];
    let bytes = style_doc(
        &[("9", &[]), ("2", NORMAL), ("\r", &[])],
        true,
        &styles,
        &[(0, 3, 0)],
        None,
    );
    assert_style_output(bytes, "92", "^9^2").await;
}

#[tokio::test]
async fn should_reset_direct_scripts_when_a_character_style_is_selected_gh2275() {
    let styles = [paragraph_style(NO_BASE, &[]), character_style(NO_BASE, &[])];
    let properties = [SUPERSCRIPT, CHAR_STYLE_ONE].concat();
    let bytes = style_doc(&[("9", &properties), ("\r", &[])], true, &styles, &[(0, 2, 0)], None);
    assert_style_output(bytes, "9", "9").await;
}

#[tokio::test]
async fn should_reset_direct_scripts_to_the_paragraph_style_with_cplain_gh2275() {
    let styles = [paragraph_style(NO_BASE, SUBSCRIPT)];
    let properties = [SUPERSCRIPT, &[0x33, 0x2a, 0]].concat();
    let bytes = style_doc(&[("2", &properties), ("\r", &[])], true, &styles, &[(0, 2, 0)], None);
    assert_style_output(bytes, "2", "~2~").await;
}

#[tokio::test]
async fn should_apply_direct_chpx_after_inherited_styles_gh2275() {
    let styles = [paragraph_style(NO_BASE, SUPERSCRIPT)];
    let bytes = style_doc(&[("2\r", SUBSCRIPT)], false, &styles, &[(0, 2, 0)], None);
    assert_style_output(bytes, "2", "~2~").await;
}

#[tokio::test]
async fn should_keep_explicit_normal_in_a_derived_character_style_gh2275() {
    let styles = [
        paragraph_style(NO_BASE, &[]),
        character_style(NO_BASE, SUPERSCRIPT),
        character_style(1, NORMAL),
    ];
    let bytes = style_doc(&[("9", CHAR_STYLE_TWO), ("\r", &[])], true, &styles, &[(0, 2, 0)], None);
    assert_style_output(bytes, "9", "9").await;
}

#[tokio::test]
async fn should_apply_piece_scripts_after_inherited_style_properties_gh2275() {
    let styles = [paragraph_style(NO_BASE, SUPERSCRIPT)];
    let bytes = style_doc(&[("2\r", &[])], true, &styles, &[(0, 2, 0)], None);
    let mut compound = cfb::CompoundFile::open(Cursor::new(bytes)).unwrap();
    let mut word_doc = Vec::new();
    compound
        .open_stream("/WordDocument")
        .unwrap()
        .read_to_end(&mut word_doc)
        .unwrap();
    let mut table = Vec::new();
    compound
        .open_stream("/1Table")
        .unwrap()
        .read_to_end(&mut table)
        .unwrap();
    let pair = test_rg_fc_lcb_offset() + super::super::FIB_FC_LCB_IDX_CLX * 8;
    let offset = u32::from_le_bytes(word_doc[pair..pair + 4].try_into().unwrap()) as usize;
    assert_eq!(table[offset], 2);
    table[offset + 19..offset + 21].copy_from_slice(&0x02d0u16.to_le_bytes());
    assert_style_output(build_doc_ole(&word_doc, &table), "2", "~2~").await;
}

#[tokio::test]
async fn should_preserve_inherited_style_scripts_in_footnote_stories_gh2275() {
    let styles = [paragraph_style(NO_BASE, &[]), paragraph_style(NO_BASE, SUPERSCRIPT)];
    let bytes = style_doc(
        &[("Main\x02\r", &[]), ("109\r", &[])],
        false,
        &styles,
        &[(0, 6, 0), (6, 10, 1)],
        Some((6, &[0, 4])),
    );
    let parsed = super::super::extract_doc_text(&bytes).expect("valid styled footnote DOC");
    assert_eq!(parsed.content, "Main\n\nFootnotes\n\n109");
    assert_eq!(parsed.paragraphs.len(), 1);
    assert_eq!(parsed.paragraphs[0].content, "Main");
    assert_eq!(parsed.subdocuments.len(), 1);
    assert_eq!(parsed.subdocuments[0].text, "109");
    assert_eq!(parsed.subdocuments[0].annotations, vec![builder::superscript(0, 3)]);
    let output = crate::extract(
        crate::ExtractInput::from_bytes(bytes, "application/msword", None),
        &ExtractionConfig {
            output_format: OutputFormat::Markdown,
            use_cache: false,
            escape_markdown: false,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.trim_end(), "Main\n\n[^1]:\n    ^109^");
}

#[test]
fn should_preserve_direct_scripts_when_the_style_chain_is_invalid_gh2275() {
    for styles in [
        vec![paragraph_style(1, &[]), paragraph_style(0, &[])],
        vec![paragraph_style(42, &[])],
        vec![paragraph_style(NO_BASE, &[0x48, 0x2a])],
    ] {
        let bytes = style_doc(&[("2\r", SUBSCRIPT)], true, &styles, &[(0, 2, 0)], None);
        let parsed = super::super::extract_doc_text(&bytes).expect("preserved DOC text after invalid style");
        assert_eq!(parsed.content, "2");
        assert_eq!(parsed.paragraphs.len(), 1);
        assert_eq!(parsed.paragraphs[0].annotations, vec![builder::subscript(0, 1)]);
        assert_eq!(parsed.processing_warnings.len(), 1);
        assert_eq!(parsed.processing_warnings[0].source, "doc");
    }
}
