//! Word97+ piece-table (`PlcPcd`) decoding: FIB field reads, per-piece text
//! decoding, and assembly into the document's subdocument text ranges.

use super::*;

#[path = "clx.rs"]
mod clx;

/// Read a little-endian `u16` at `offset`, or an error naming `what` when `data` is too
/// short to contain it.
fn read_u16_checked(data: &[u8], offset: usize, what: &str) -> Result<u16> {
    if data.len() < offset + 2 {
        return Err(XbergError::parsing(format!("FIB too short for {what}")));
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Read a little-endian `u32` at `offset`, or an error naming `what` when `data` is too
/// short to contain it.
fn read_u32_checked(data: &[u8], offset: usize, what: &str) -> Result<u32> {
    if data.len() < offset + 4 {
        return Err(XbergError::parsing(format!("FIB too short for {what}")));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

struct Word97Layout {
    ranges: SubdocRanges,
    total_cp: usize,
    pairs: usize,
    clx_start: usize,
    clx_length: usize,
}

impl Word97Layout {
    fn read(word_doc: &[u8]) -> Result<Self> {
        let fib_base_size = 32;
        let csw_offset = fib_base_size;
        let csw = read_u16_checked(word_doc, csw_offset, "csw")? as usize;
        let rg_w_offset = csw_offset + 2;
        let cslw_offset = rg_w_offset + csw * 2;

        let cslw = read_u16_checked(word_doc, cslw_offset, "cslw")? as usize;
        let rg_lw_offset = cslw_offset + 2;

        let ccp_text_offset = rg_lw_offset + FIB_LW_IDX_CCP_TEXT * 4;
        let ccp_text = read_u32_checked(word_doc, ccp_text_offset, "ccpText")? as usize;

        let subdoc_ranges = SubdocRanges::from_fib(word_doc, rg_lw_offset, ccp_text);
        let mut total_cp = subdoc_ranges.total_cp();
        if total_cp > 0 {
            total_cp += 1;
        }

        let cbrgfclcb_offset = rg_lw_offset + cslw * 4;
        read_u16_checked(word_doc, cbrgfclcb_offset, "cbRgFcLcb")?;
        let rg_fc_lcb_offset = cbrgfclcb_offset + 2;

        let fc_clx_offset = rg_fc_lcb_offset + FIB_FC_LCB_IDX_CLX * 8;
        let lcb_clx_offset = fc_clx_offset + 4;
        let fc_clx = read_u32_checked(word_doc, fc_clx_offset, "fcClx/lcbClx")? as usize;
        let lcb_clx = read_u32_checked(word_doc, lcb_clx_offset, "fcClx/lcbClx")? as usize;

        Ok(Self {
            ranges: subdoc_ranges,
            total_cp,
            pairs: rg_fc_lcb_offset,
            clx_start: fc_clx,
            clx_length: lcb_clx,
        })
    }
}

/// Extract text from Word 97/2000/XP/2003 files using the piece table.
pub(super) fn extract_text_word97(
    word_doc: &[u8],
    table_stream: &[u8],
    warnings: &mut Vec<ProcessingWarning>,
    picture_limits: Option<&crate::extractors::security::SecurityLimits>,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> Result<MainText> {
    let fib = Word97Layout::read(word_doc)?;

    if fib.clx_start == 0 || fib.clx_length == 0 {
        images::warn_unsupported_fallback(picture_limits.is_some(), warnings, "contiguous-text fallback");
        return extract_text_contiguous(word_doc, fib.ranges.main.len()).map(MainText::text_only);
    }

    if table_stream.len() < fib.clx_start + fib.clx_length {
        return Err(XbergError::parsing("CLX extends beyond table stream"));
    }

    let clx = &table_stream[fib.clx_start..fib.clx_start + fib.clx_length];

    if let Some(piece_table) = clx::read_piece_table(clx, budget)? {
        let scripts = chpx::ScriptIndex::read(word_doc, table_stream, fib.pairs, budget, warnings)?;
        let tables = DocTables {
            lists: papx::ListTables::build(word_doc, table_stream, fib.pairs),
            stories: StoryTables::read(word_doc, table_stream, fib.pairs, &fib.ranges),
            picture_limits,
            scripts,
            piece_scripts: piece_table.scripts,
            ranges: &fib.ranges,
            total_cp: fib.total_cp,
        };
        return extract_text_from_piece_table(word_doc, piece_table.plc_pcd, warnings, &tables, budget);
    }

    images::warn_unsupported_fallback(picture_limits.is_some(), warnings, "unavailable-piece-table fallback");
    extract_text_fallback(word_doc, fib.ranges.main.len()).map(MainText::text_only)
}

/// Record that a piece's declared byte range runs past the end of the
/// WordDocument stream (#92). The piece table on a malformed or truncated
/// `.doc` can declare an FC/length pair that overruns the stream; previously
/// this was silently clamped (compressed pieces) or dropped entirely
/// (uncompressed pieces), producing a document that looked complete but was
/// truncated.
fn push_piece_overrun_warning(
    warnings: &mut Vec<ProcessingWarning>,
    piece_index: usize,
    declared_end: usize,
    stream_len: usize,
) {
    let message = format!(
        "Piece {piece_index} in the .doc piece table declares a byte range ending at \
         byte {declared_end}, past the end of the WordDocument stream ({stream_len} bytes); \
         the piece's text beyond the stream end was dropped"
    );
    crate::core::diagnostics::push_warning(warnings, DOC_WARNING_SOURCE, message);
}

/// A decoded piece: its characters, plus for each character the byte offset
/// (`FC`) in the WordDocument stream it was read from and its offset in the
/// piece's CP range.
///
/// The vectors are always the same length. Paragraph properties are
/// FC-addressed while text is CP-addressed, so #1550 needs this pairing to
/// bind a paragraph to its `PAPX`; the uncompressed path can drop a code unit
/// that is not a scalar value, which is why the offsets are recorded during
/// decoding rather than recomputed from an index afterwards.
///
/// `fc_ends` holds the offset one *past* each character, because that is what
/// an FKP's `rgfc` entry stores for a paragraph. Recording the end during
/// decoding avoids re-deriving it as `fc + 1`, which is only correct for
/// compressed pieces -- an uncompressed character is two bytes wide. ~keep
///
/// `cp_offsets` is recorded for the same reason: floating shapes are
/// CP-addressed (`PlcSpa`), and a dropped code unit shifts every later
/// character's index away from its CP. ~keep
#[derive(Default)]
struct DecodedPiece {
    chars: Vec<char>,
    fc_ends: Vec<u32>,
    cp_offsets: Vec<u32>,
    scripts: Vec<chpx::Script>,
}

impl DecodedPiece {
    fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

/// Decode one piece's characters, recording each character's `FC`.
fn decode_piece_chars(
    word_doc: &[u8],
    fc_raw: u32,
    char_count: usize,
    piece_index: usize,
    warnings: &mut Vec<ProcessingWarning>,
) -> DecodedPiece {
    let is_compressed = (fc_raw & 0x4000_0000) != 0;
    let fc = (fc_raw & 0x3FFF_FFFF) as usize;
    // Compressed (CP1252) pieces address `word_doc` at half the raw FC value;
    // uncompressed (UTF-16LE) pieces address it directly. ~keep
    let byte_offset = if is_compressed { fc / 2 } else { fc };

    let decode_cp1252 = |start: usize, end: usize| -> DecodedPiece {
        if start >= end {
            return DecodedPiece::default();
        }
        DecodedPiece {
            chars: word_doc[start..end].iter().map(|&b| cp1252_to_char(b)).collect(),
            fc_ends: (start..end).map(|fc| (fc + 1) as u32).collect(),
            cp_offsets: (0..end - start).map(|offset| offset as u32).collect(),
            scripts: Vec::new(),
        }
    };

    if is_compressed {
        let end = byte_offset + char_count;
        let available_end = if end > word_doc.len() {
            push_piece_overrun_warning(warnings, piece_index, end, word_doc.len());
            word_doc.len()
        } else {
            end
        };
        decode_cp1252(byte_offset, available_end)
    } else {
        let end = byte_offset + char_count * 2;
        let available_end = if end > word_doc.len() {
            push_piece_overrun_warning(warnings, piece_index, end, word_doc.len());
            byte_offset + ((word_doc.len().saturating_sub(byte_offset)) / 2) * 2
        } else {
            end
        };
        if byte_offset >= available_end {
            DecodedPiece::default()
        } else {
            let mut chars = Vec::new();
            let mut fc_ends = Vec::new();
            let mut cp_offsets = Vec::new();
            let units = word_doc[byte_offset..available_end].as_chunks::<2>().0;
            let mut cp = 0;
            for decoded in char::decode_utf16(units.iter().map(|bytes| u16::from_le_bytes(*bytes))) {
                match decoded {
                    Ok(character) => {
                        chars.push(character);
                        cp_offsets.push(cp as u32);
                        cp += character.len_utf16();
                        fc_ends.push((byte_offset + cp * 2) as u32);
                    }
                    Err(_) => cp += 1,
                }
            }
            DecodedPiece {
                chars,
                fc_ends,
                cp_offsets,
                scripts: Vec::new(),
            }
        }
    }
}

#[derive(Default)]
struct StoryPositions<'a> {
    fc_ends: Option<&'a mut Vec<u32>>,
    cps: Option<&'a mut Vec<u32>>,
    scripts: Option<&'a mut Vec<chpx::Script>>,
}

/// Select by source UTF16/CP1252 positions, not decoded scalar count; a surrogate
/// pair or dropped malformed unit must not shift a later story boundary. ~keep
fn append_range_overlap(
    piece: &DecodedPiece,
    cp_start: usize,
    range: SubdocRange,
    out: &mut String,
    positions: StoryPositions<'_>,
) {
    if range.len() == 0 {
        return;
    }
    let from = piece
        .cp_offsets
        .partition_point(|&cp| cp_start + (cp as usize) < range.start);
    let to = piece
        .cp_offsets
        .partition_point(|&cp| cp_start + (cp as usize) < range.end);
    out.extend(&piece.chars[from..to]);
    if let Some(scripts) = positions.scripts {
        scripts.extend_from_slice(&piece.scripts[from..to]);
    }
    if let Some(out_fcs) = positions.fc_ends {
        out_fcs.extend_from_slice(&piece.fc_ends[from..to]);
    }
    if let Some(out_cps) = positions.cps {
        out_cps.extend(
            piece.cp_offsets[from..to]
                .iter()
                .map(|&offset| (cp_start + offset as usize) as u32),
        );
    }
}

/// Fields of [`extract_text_from_piece_table`] that [`process_piece`] needs but never
/// mutates, bundled purely to keep that function's parameter list manageable.
struct PieceTableContext<'a> {
    /// Total number of declared pieces (`PlcPcd` entries).
    n: usize,
    plc_size: usize,
    plc_pcd: &'a [u8],
    word_doc: &'a [u8],
    total_cp: usize,
    ranges: &'a SubdocRanges,
    scripts: &'a chpx::ScriptIndex,
    piece_scripts: &'a [chpx::ScriptProperty],
}

/// Process one entry of the piece table (`PlcPcd`), appending its decoded characters to
/// the matching subdocument range(s) in `text`. Returns `false` when the outer loop over
/// pieces in [`extract_text_from_piece_table`] must stop (a truncated table, or a piece
/// starting past `total_cp`); `true` otherwise, including when this particular piece
/// contributed nothing.
fn process_piece(
    i: usize,
    ctx: &PieceTableContext,
    warnings: &mut Vec<ProcessingWarning>,
    text: &mut SubdocumentText,
) -> bool {
    let cp_start_off = i * 4;
    let cp_end_off = (i + 1) * 4;
    let pcd_off = (ctx.n + 1) * 4 + i * 8;

    if cp_end_off + 4 > ctx.plc_size || pcd_off + 8 > ctx.plc_size {
        let n = ctx.n;
        crate::core::diagnostics::push_warning(
            warnings,
            DOC_WARNING_SOURCE,
            format!(
                "Piece table truncated after {i} of {n} declared pieces; remaining document text was not extracted"
            ),
        );
        return false;
    }

    let plc_pcd = ctx.plc_pcd;
    let cp_start = read_piece_value(plc_pcd, cp_start_off) as usize;
    let cp_end = read_piece_value(plc_pcd, cp_end_off) as usize;

    if cp_start >= ctx.total_cp {
        return false;
    }

    let fc_raw = read_piece_value(plc_pcd, pcd_off + 2);

    let mut char_count = cp_end.saturating_sub(cp_start);
    if cp_start + char_count > ctx.total_cp {
        char_count = ctx.total_cp.saturating_sub(cp_start);
    }
    if char_count == 0 {
        return true;
    }

    let mut piece = decode_piece_chars(ctx.word_doc, fc_raw, char_count, i, warnings);
    if piece.is_empty() {
        return true;
    }

    let prm = u16::from_le_bytes([plc_pcd[pcd_off + 6], plc_pcd[pcd_off + 7]]);
    let override_script = piece_script_override(prm, ctx.piece_scripts, warnings);
    piece.scripts = piece
        .fc_ends
        .iter()
        .map(|&fc| override_script.unwrap_or_else(|| ctx.scripts.at(fc)))
        .collect();
    append_piece_to_stories(&piece, cp_start, ctx.ranges, text);
    true
}

fn piece_script_override(
    prm: u16,
    groups: &[chpx::ScriptProperty],
    warnings: &mut Vec<ProcessingWarning>,
) -> Option<chpx::Script> {
    if prm & 1 == 0 {
        return chpx::compact_prm_script(prm);
    }
    match groups.get(usize::from(prm >> 1)).copied() {
        Some(chpx::ScriptProperty::Override(script)) => Some(script),
        Some(chpx::ScriptProperty::Unchanged) => None,
        Some(chpx::ScriptProperty::Malformed) | None => {
            crate::core::diagnostics::push_warning(
                warnings,
                DOC_WARNING_SOURCE,
                "Missing or malformed DOC piece character properties were ignored; text and CHPX formatting were preserved",
            );
            None
        }
    }
}

fn read_piece_value(plc: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([plc[offset], plc[offset + 1], plc[offset + 2], plc[offset + 3]])
}

fn append_piece_to_stories(piece: &DecodedPiece, cp_start: usize, ranges: &SubdocRanges, text: &mut SubdocumentText) {
    append_range_overlap(
        piece,
        cp_start,
        ranges.main,
        &mut text.main,
        StoryPositions {
            fc_ends: Some(&mut text.main_fc_ends),
            cps: Some(&mut text.main_cps),
            scripts: Some(&mut text.main_scripts),
        },
    );
    append_range_overlap(
        piece,
        cp_start,
        ranges.footnote,
        &mut text.footnote,
        StoryPositions {
            cps: Some(&mut text.footnote_cps),
            scripts: Some(&mut text.footnote_scripts),
            ..Default::default()
        },
    );
    append_range_overlap(
        piece,
        cp_start,
        ranges.header,
        &mut text.header,
        StoryPositions {
            cps: Some(&mut text.header_cps),
            scripts: Some(&mut text.header_scripts),
            ..Default::default()
        },
    );
    append_range_overlap(
        piece,
        cp_start,
        ranges.annotation,
        &mut text.annotation,
        StoryPositions {
            cps: Some(&mut text.annotation_cps),
            scripts: Some(&mut text.annotation_scripts),
            ..Default::default()
        },
    );
    append_range_overlap(
        piece,
        cp_start,
        ranges.textbox,
        &mut text.textbox,
        StoryPositions {
            scripts: Some(&mut text.textbox_scripts),
            ..Default::default()
        },
    );
}

/// Extract text from the piece table (PlcPcd), bucketing each piece's
/// characters into the subdocument range they fall in (#77: previously any
/// piece whose CP range started at or after `ccpText` -- i.e. every
/// footnote, header/footer, comment and text-box piece -- was silently
/// skipped).
fn extract_text_from_piece_table(
    word_doc: &[u8],
    plc_pcd: &[u8],
    warnings: &mut Vec<ProcessingWarning>,
    tables: &DocTables<'_>,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> Result<MainText> {
    let ranges = tables.ranges;
    let plc_size = plc_pcd.len();
    if plc_size < 16 {
        return Err(XbergError::parsing("PlcPcd too small"));
    }

    let n = (plc_size - 4) / 12;
    if n == 0 {
        return Ok(MainText::text_only(String::new()));
    }

    let mut text = SubdocumentText::default();
    let piece_ctx = PieceTableContext {
        n,
        plc_size,
        plc_pcd,
        word_doc,
        total_cp: tables.total_cp,
        ranges,
        scripts: &tables.scripts,
        piece_scripts: &tables.piece_scripts,
    };

    for i in 0..n {
        if !process_piece(i, &piece_ctx, warnings, &mut text) {
            break;
        }
    }

    if ranges.has_unextracted_subdocument() {
        crate::core::diagnostics::push_warning(
            warnings,
            DOC_WARNING_SOURCE,
            "Document contains endnote and/or header-text-box content that is not extracted",
        );
    }

    assemble_main_text(&text, tables, warnings, budget)
}

fn assemble_main_text(
    text: &SubdocumentText,
    tables: &DocTables<'_>,
    warnings: &mut Vec<ProcessingWarning>,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> Result<MainText> {
    let content = joined_subdocument_text(text);
    let paragraph_context = ParagraphContext {
        lists: &tables.lists,
        picture_limits: tables.picture_limits,
        scripts: &text.main_scripts,
    };
    let (paragraphs, picture_positions) = split_paragraphs_and_pictures(
        &text.main,
        &text.main_fc_ends,
        &text.main_cps,
        &paragraph_context,
        budget,
    )?;
    let unsupported_pictures = [&text.footnote, &text.header, &text.annotation, &text.textbox]
        .iter()
        .any(|story| story.contains(['\x01', '\x08']));
    Ok(MainText {
        paragraphs,
        picture_positions,
        unsupported_pictures,
        subdocuments: collect_subdocuments(text, tables, warnings, budget)?,
        content,
    })
}

fn joined_subdocument_text(text: &SubdocumentText) -> String {
    let mut content = normalize_doc_text(&text.main);
    for (label, section) in [
        ("Footnotes", &text.footnote),
        ("Headers and Footers", &text.header),
        ("Comments", &text.annotation),
        ("Text Boxes", &text.textbox),
    ] {
        let normalized_section = normalize_doc_text(section);
        if !normalized_section.is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(label);
            content.push_str("\n\n");
            content.push_str(&normalized_section);
        }
    }

    content
}

/// Table-stream structures the piece-table walk resolves text against.
struct DocTables<'a> {
    picture_limits: Option<&'a crate::extractors::security::SecurityLimits>,
    lists: papx::ListTables,
    stories: StoryTables,
    scripts: chpx::ScriptIndex,
    piece_scripts: Vec<chpx::ScriptProperty>,
    ranges: &'a SubdocRanges,
    total_cp: usize,
}

/// Story boundaries of the subdocuments that hold more than one story, as CPs
/// relative to the subdocument's start. `None` when the table is absent or
/// unusable.
struct StoryTables {
    footnote: Option<Vec<usize>>,
    header: Option<Vec<usize>>,
    annotation: Option<Vec<usize>>,
}

impl StoryTables {
    fn read(word_doc: &[u8], table_stream: &[u8], rg_fc_lcb_offset: usize, ranges: &SubdocRanges) -> Self {
        let read =
            |index, range: SubdocRange| read_story_bounds(word_doc, table_stream, rg_fc_lcb_offset, index, range.len());
        Self {
            footnote: read(FIB_FC_LCB_IDX_PLCFFND_TXT, ranges.footnote),
            header: read(FIB_FC_LCB_IDX_PLCF_HDD, ranges.header)
                .filter(|bounds| header_table_has_whole_sections(bounds)),
            annotation: read(FIB_FC_LCB_IDX_PLCFAND_TXT, ranges.annotation),
        }
    }
}

/// Read a story table (`PlcffndTxt`, `PlcfHdd`, `PlcfandTxt`) into the CP each
/// story starts at, followed by the end of the last story.
///
/// [MS-DOC] gives these tables one CP per story, then the end of the last
/// story, which must equal `ccp - 1`, then a final CP readers must ignore. A
/// table that does not fit the table stream, runs backwards or does not cover
/// the subdocument from CP 0 to `ccp - 1` is treated as absent; splitting by it
/// would silently drop the uncovered text. ~keep
fn read_story_bounds(
    word_doc: &[u8],
    table_stream: &[u8],
    rg_fc_lcb_offset: usize,
    index: usize,
    ccp: usize,
) -> Option<Vec<usize>> {
    let (fc, lcb) = papx::read_fc_lcb(word_doc, rg_fc_lcb_offset, index)?;
    // A single story already takes three 4-byte CPs.
    if lcb < 12 || lcb % 4 != 0 {
        return None;
    }
    let plc = table_stream.get(fc..fc.checked_add(lcb)?)?;
    let mut bounds: Vec<usize> = plc
        .as_chunks::<4>()
        .0
        .iter()
        .map(|cp| u32::from_le_bytes([cp[0], cp[1], cp[2], cp[3]]) as usize)
        .collect();
    bounds.pop();
    let ordered = bounds.windows(2).all(|pair| pair[0] <= pair[1]);
    let covers_subdocument = bounds.first() == Some(&0) && bounds.last() == Some(&ccp.checked_sub(1)?);
    (ordered && covers_subdocument).then_some(bounds)
}

/// The header subdocument opens with six footnote and endnote separator
/// stories, then holds six stories per section: even header, odd header, even
/// footer, odd footer, first-page header, first-page footer ([MS-DOC]
/// `Plcfhdd`). ~keep
const HEADER_SEPARATOR_STORIES: usize = 6;
const HEADER_STORIES_PER_SECTION: usize = 6;

/// Whether a `PlcfHdd` has the shape above: the separator stories and whole
/// sections, at least one. Any other count would file stories under the wrong
/// kind or drop them as separators, so such a table counts as malformed.
fn header_table_has_whole_sections(bounds: &[usize]) -> bool {
    let stories = bounds.len().saturating_sub(1);
    stories > HEADER_SEPARATOR_STORIES
        && (stories - HEADER_SEPARATOR_STORIES).is_multiple_of(HEADER_STORIES_PER_SECTION)
}

/// Whether header story `index` is a header or a footer; `None` for a separator.
fn header_story_kind(index: usize) -> Option<DocSubdocumentKind> {
    match index.checked_sub(HEADER_SEPARATOR_STORIES)? % HEADER_STORIES_PER_SECTION {
        0 | 1 | 4 => Some(DocSubdocumentKind::Header),
        _ => Some(DocSubdocumentKind::Footer),
    }
}

/// Story bounds use source CPs: decoded scalars can occupy two units, and
/// malformed units can disappear. Keep their original positions. ~keep
fn split_stories(cps: &[u32], start_cp: usize, bounds: &[usize]) -> Vec<std::ops::Range<usize>> {
    bounds
        .windows(2)
        .map(|story| {
            let start = cps.partition_point(|&cp| (cp as usize) < start_cp.saturating_add(story[0]));
            let end = cps.partition_point(|&cp| (cp as usize) < start_cp.saturating_add(story[1]));
            start..end
        })
        .collect()
}

fn normalize_story(
    raw: &str,
    scripts: &[chpx::Script],
    budget: &mut crate::extractors::security::SecurityBudget,
    kind: DocSubdocumentKind,
) -> Result<DocSubdocument> {
    if raw.chars().count() != scripts.len() {
        return Err(XbergError::parsing(
            "DOC subdocument character-property alignment is invalid",
        ));
    }
    let mut visible = String::new();
    let mut properties = Vec::new();
    for ((character, script), markup) in raw.chars().zip(scripts).zip(field_markup_mask(raw)) {
        if !markup {
            visible.push(character);
            properties.push(*script);
        }
    }
    let (text, annotations) = chpx::normalize(&visible, &properties, budget)?;
    Ok(DocSubdocument {
        kind,
        text,
        annotations,
    })
}

/// Split the footnote, header and comment subdocuments into their stories, in
/// the order `content` lists them, and add the text-box subdocument whole.
/// Without a usable story table a subdocument stays one story, with a warning.
fn collect_subdocuments(
    text: &SubdocumentText,
    tables: &DocTables<'_>,
    warnings: &mut Vec<ProcessingWarning>,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> Result<Vec<DocSubdocument>> {
    let mut subdocuments = collect_story_subdocuments(text, tables, warnings, budget)?;
    let textbox = normalize_story(
        &text.textbox,
        &text.textbox_scripts,
        budget,
        DocSubdocumentKind::TextBox,
    )?;
    if !textbox.text.is_empty() {
        subdocuments.push(textbox);
    }
    Ok(subdocuments)
}

fn collect_story_subdocuments(
    text: &SubdocumentText,
    tables: &DocTables<'_>,
    warnings: &mut Vec<ProcessingWarning>,
    budget: &mut crate::extractors::security::SecurityBudget,
) -> Result<Vec<DocSubdocument>> {
    let ranges = tables.ranges;
    let mut subdocuments = Vec::new();
    for (raw, scripts, cps, start_cp, bounds, kind, fallback_kind, fallback_warning) in [
        (
            &text.footnote,
            &text.footnote_scripts,
            &text.footnote_cps,
            ranges.footnote.start,
            &tables.stories.footnote,
            DocSubdocumentKind::Footnote,
            DocSubdocumentKind::Footnote,
            "Footnote table (PlcffndTxt) is missing or malformed; all footnote text is reported as one footnote",
        ),
        (
            &text.header,
            &text.header_scripts,
            &text.header_cps,
            ranges.header.start,
            &tables.stories.header,
            DocSubdocumentKind::Header,
            DocSubdocumentKind::HeaderFooter,
            "Header table (PlcfHdd) is missing or malformed; all header and footer text is reported as combined header/footer text",
        ),
        (
            &text.annotation,
            &text.annotation_scripts,
            &text.annotation_cps,
            ranges.annotation.start,
            &tables.stories.annotation,
            DocSubdocumentKind::Comment,
            DocSubdocumentKind::Comment,
            "Comment table (PlcfandTxt) is missing or malformed; all comment text is reported as one comment",
        ),
    ] {
        // A malformed piece table can resume at an earlier CP. Binary search
        // would then assign text to the wrong story; keep that stream whole. ~keep
        let positions_valid = cps.len() == raw.chars().count() && cps.windows(2).all(|pair| pair[0] < pair[1]);
        let fallback_warning = if bounds.is_some() && !positions_valid {
            "DOC character positions are malformed; subdocument text is reported as one story"
        } else {
            fallback_warning
        };
        let Some(bounds) = bounds.as_ref().filter(|_| positions_valid) else {
            let whole = normalize_story(raw, scripts, budget, fallback_kind)?;
            if !whole.text.is_empty() {
                crate::core::diagnostics::push_warning(warnings, DOC_WARNING_SOURCE, fallback_warning);
                subdocuments.push(whole);
            }
            continue;
        };
        let chars: Vec<char> = raw.chars().collect();
        for (index, range) in split_stories(cps, start_cp, bounds).into_iter().enumerate() {
            let raw_story: String = chars[range.clone()].iter().collect();
            let story_kind = if kind == DocSubdocumentKind::Header {
                header_story_kind(index)
            } else {
                Some(kind)
            };
            let Some(kind) = story_kind else {
                continue;
            };
            let story = normalize_story(&raw_story, &scripts[range], budget, kind)?;
            if !story.text.is_empty() {
                subdocuments.push(story);
            }
        }
    }

    Ok(subdocuments)
}

#[cfg(test)]
mod test_script_offsets {
    use super::*;

    #[test]
    fn should_decode_surrogate_pair_with_source_fc_and_cp_offsets() {
        let bytes: Vec<u8> = "😀A".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut warnings = Vec::new();
        let decoded = decode_piece_chars(&bytes, 0, 3, 0, &mut warnings);
        assert_eq!(decoded.chars, ['😀', 'A']);
        assert_eq!(decoded.fc_ends, [4, 6]);
        assert_eq!(decoded.cp_offsets, [0, 2]);
        assert!(warnings.is_empty());
    }

    #[test]
    fn should_select_subdocument_by_source_cp_after_surrogate_pair() {
        let piece = DecodedPiece {
            chars: vec!['😀', 'A'],
            fc_ends: vec![4, 6],
            cp_offsets: vec![0, 2],
            scripts: Vec::new(),
        };
        let mut text = String::new();
        let mut fcs = Vec::new();
        let mut cps = Vec::new();
        append_range_overlap(
            &piece,
            10,
            SubdocRange { start: 12, end: 13 },
            &mut text,
            StoryPositions {
                fc_ends: Some(&mut fcs),
                cps: Some(&mut cps),
                ..Default::default()
            },
        );
        assert_eq!(text, "A");
        assert_eq!(fcs, [6]);
        assert_eq!(cps, [12]);
    }

    #[test]
    fn should_keep_source_offsets_after_dropping_unpaired_surrogate() {
        let bytes: Vec<u8> = [0xd800u16, 0x0041, 0xdc00, 0x0042]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect();
        let mut warnings = Vec::new();
        let decoded = decode_piece_chars(&bytes, 0, 4, 0, &mut warnings);
        assert_eq!(decoded.chars, ['A', 'B']);
        assert_eq!(decoded.fc_ends, [4, 8]);
        assert_eq!(decoded.cp_offsets, [1, 3]);
        assert!(warnings.is_empty());
    }

    fn two_footnote_story_doc(units: &[u16]) -> Vec<u8> {
        use super::super::tests::{
            TestPiece, build_doc_ole, build_fib, build_plc_pcd, build_table_stream, test_rg_fc_lcb_offset, write_u32,
        };
        assert_eq!(units.len(), 6);
        let mut word = build_fib(2048, 1, 6, 0, 0);
        let encoded: Vec<u8> = std::iter::once(13u16)
            .chain(units.iter().copied())
            .flat_map(u16::to_le_bytes)
            .collect();
        word[1024..1024 + encoded.len()].copy_from_slice(&encoded);
        let pairs = test_rg_fc_lcb_offset();
        word[pairs - 2..pairs].copy_from_slice(&93u16.to_le_bytes());
        let piece = build_plc_pcd(&[TestPiece {
            cp_start: 0,
            cp_end: 7,
            fc_raw: 1024,
        }]);
        let mut table = build_table_stream(&mut word, &piece);
        write_u32(&mut table, 9, piece.len() as u32);
        let offset = table.len();
        for bound in [0u32, 3, 5, 6] {
            table.extend_from_slice(&bound.to_le_bytes());
        }
        write_u32(&mut word, pairs + 3 * 8, offset as u32);
        write_u32(&mut word, pairs + 3 * 8 + 4, 16);
        build_doc_ole(&word, &table)
    }

    fn assert_two_footnotes(bytes: &[u8], first: &str) {
        let parsed = extract_doc_text(bytes).expect("two valid footnote story bounds");
        assert_eq!(parsed.content, format!("Footnotes\n\n{first}\nB"));
        assert!(parsed.paragraphs.is_empty());
        assert!(
            parsed.processing_warnings.is_empty(),
            "{:?}",
            parsed.processing_warnings
        );
        let actual: Vec<_> = parsed
            .subdocuments
            .iter()
            .map(|story| (story.kind, story.text.as_str()))
            .collect();
        assert_eq!(
            actual,
            [
                (DocSubdocumentKind::Footnote, first),
                (DocSubdocumentKind::Footnote, "B")
            ]
        );
        assert_eq!(actual.len(), 2);
    }

    #[test]
    fn should_keep_two_footnote_stories_after_utf16_surrogate_pair() {
        let units: Vec<_> = "😀\rB\r\r".encode_utf16().collect();
        assert_two_footnotes(&two_footnote_story_doc(&units), "😀");
    }

    #[test]
    fn should_keep_two_footnote_stories_after_dropped_malformed_utf16_unit() {
        assert_two_footnotes(&two_footnote_story_doc(&[0xd800, 65, 13, 66, 13, 13]), "A");
    }

    #[test]
    fn should_fall_back_to_whole_story_when_piece_cps_resume_in_reverse_order() {
        use super::super::tests::{
            TestPiece, build_doc_ole, build_fib, build_plc_pcd, build_table_stream, test_rg_fc_lcb_offset, write_u32,
        };
        let mut word = build_fib(2048, 0, 7, 0, 0);
        let encoded: Vec<u8> = "ABCDwxyz!".encode_utf16().flat_map(u16::to_le_bytes).collect();
        word[1024..1024 + encoded.len()].copy_from_slice(&encoded);
        let pairs = test_rg_fc_lcb_offset();
        word[pairs - 2..pairs].copy_from_slice(&93u16.to_le_bytes());
        let piece = build_plc_pcd(&[
            TestPiece {
                cp_start: 0,
                cp_end: 4,
                fc_raw: 1024,
            },
            TestPiece {
                cp_start: 4,
                cp_end: 2,
                fc_raw: 1032,
            },
            TestPiece {
                cp_start: 2,
                cp_end: 7,
                fc_raw: 1032,
            },
        ]);
        assert_eq!(piece[..16], [0, 0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0]);
        let mut table = build_table_stream(&mut word, &piece);
        write_u32(&mut table, 9, piece.len() as u32);
        let offset = table.len();
        for bound in [0u32, 3, 6, 7] {
            table.extend_from_slice(&bound.to_le_bytes());
        }
        write_u32(&mut word, pairs + 3 * 8, offset as u32);
        write_u32(&mut word, pairs + 3 * 8 + 4, 16);
        let parsed = extract_doc_text(&build_doc_ole(&word, &table)).expect("malformed CP order retains text");
        assert_eq!(parsed.content, "Footnotes\n\nABCDwxyz!");
        assert!(parsed.paragraphs.is_empty());
        let actual: Vec<_> = parsed
            .subdocuments
            .iter()
            .map(|story| (story.kind, story.text.as_str()))
            .collect();
        assert_eq!(actual, [(DocSubdocumentKind::Footnote, "ABCDwxyz!")]);
        assert_eq!(parsed.processing_warnings.len(), 1);
        assert!(
            parsed.processing_warnings[0]
                .message
                .contains("character positions are malformed")
        );
        assert!(parsed.processing_warnings[0].message.contains("one story"));
    }
}
