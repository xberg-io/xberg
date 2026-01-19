//! Core RTF parsing logic.

use crate::extractors::rtf::encoding::{decode_windows_1252, parse_hex_byte, parse_rtf_control_word};
use crate::extractors::rtf::formatting::normalize_whitespace;
use crate::extractors::rtf::images::extract_image_metadata;
use crate::extractors::rtf::tables::TableState;
use crate::types::Table;

/// Extract text and image metadata from RTF document.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tokenizing control sequences and text
/// 2. Converting encoded characters to Unicode
/// 3. Extracting text while skipping formatting groups
/// 4. Detecting and extracting image metadata (\pict sections)
/// 5. Normalizing whitespace
pub fn extract_text_from_rtf(content: &str) -> (String, Vec<Table>) {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut tables: Vec<Table> = Vec::new();
    let mut table_state: Option<TableState> = None;

    let ensure_table = |table_state: &mut Option<TableState>| {
        if table_state.is_none() {
            *table_state = Some(TableState::new());
        }
    };

    let finalize_table = |state_opt: &mut Option<TableState>, tables: &mut Vec<Table>| {
        if let Some(state) = state_opt.take() {
            if let Some(table) = state.finalize() {
                tables.push(table);
            }
        }
    };

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '\\' | '{' | '}' => {
                            chars.next();
                            result.push(next_ch);
                        }
                        '\'' => {
                            chars.next();
                            let hex1 = chars.next();
                            let hex2 = chars.next();
                            if let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                let decoded = decode_windows_1252(byte);
                                result.push(decoded);
                                if let Some(state) = table_state.as_mut()
                                    && state.in_row
                                {
                                    state.current_cell.push(decoded);
                                }
                            }
                        }
                        'u' => {
                            chars.next();
                            let mut num_str = String::new();
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_digit() || c == '-' {
                                    num_str.push(c);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if let Ok(code_num) = num_str.parse::<i32>() {
                                let code_u = if code_num < 0 {
                                    (code_num + 65536) as u32
                                } else {
                                    code_num as u32
                                };
                                if let Some(c) = char::from_u32(code_u) {
                                    result.push(c);
                                    if let Some(state) = table_state.as_mut()
                                        && state.in_row
                                    {
                                        state.current_cell.push(c);
                                    }
                                }
                            }
                        }
                        _ => {
                            let (control_word, _) = parse_rtf_control_word(&mut chars);
                            handle_control_word(
                                &control_word,
                                &mut chars,
                                &mut result,
                                &mut table_state,
                                &mut tables,
                                &ensure_table,
                                &finalize_table,
                            );
                        }
                    }
                }
            }
            '{' | '}' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
            }
            ' ' | '\t' | '\n' | '\r' => {
                if !result.is_empty() && !result.ends_with(' ') {
                    result.push(' ');
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                    && !state.current_cell.ends_with(' ')
                {
                    state.current_cell.push(' ');
                }
            }
            _ => {
                if let Some(state) = table_state.as_ref()
                    && !state.in_row
                    && !state.rows.is_empty()
                {
                    finalize_table(&mut table_state, &mut tables);
                }
                result.push(ch);
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push(ch);
                }
            }
        }
    }

    if table_state.is_some() {
        finalize_table(&mut table_state, &mut tables);
    }

    (normalize_whitespace(&result), tables)
}

/// Handle an RTF control word during parsing.
#[allow(clippy::too_many_arguments)]
fn handle_control_word(
    control_word: &str,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
    table_state: &mut Option<TableState>,
    tables: &mut Vec<Table>,
    ensure_table: &dyn Fn(&mut Option<TableState>),
    finalize_table: &dyn Fn(&mut Option<TableState>, &mut Vec<Table>),
) {
    match control_word {
        "pict" => {
            let image_metadata = extract_image_metadata(chars);
            if !image_metadata.is_empty() {
                result.push('!');
                result.push('[');
                result.push_str("image");
                result.push(']');
                result.push('(');
                result.push_str(&image_metadata);
                result.push(')');
                result.push(' ');
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push('!');
                    state.current_cell.push('[');
                    state.current_cell.push_str("image");
                    state.current_cell.push(']');
                    state.current_cell.push('(');
                    state.current_cell.push_str(&image_metadata);
                    state.current_cell.push(')');
                    state.current_cell.push(' ');
                }
            }
        }
        "par" => {
            if table_state.is_some() {
                finalize_table(table_state, tables);
            }
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
                result.push('\n');
            }
        }
        "tab" => {
            result.push('\t');
            if let Some(state) = table_state.as_mut()
                && state.in_row
            {
                state.current_cell.push('\t');
            }
        }
        "bullet" => {
            result.push('•');
        }
        "lquote" => {
            result.push('\u{2018}');
        }
        "rquote" => {
            result.push('\u{2019}');
        }
        "ldblquote" => {
            result.push('\u{201C}');
        }
        "rdblquote" => {
            result.push('\u{201D}');
        }
        "endash" => {
            result.push('\u{2013}');
        }
        "emdash" => {
            result.push('\u{2014}');
        }
        "trowd" => {
            ensure_table(table_state);
            if let Some(state) = table_state.as_mut() {
                state.start_row();
            }
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            if !result.ends_with('|') {
                result.push('|');
                result.push(' ');
            }
        }
        "cell" => {
            if !result.ends_with('|') {
                if !result.ends_with(' ') && !result.is_empty() {
                    result.push(' ');
                }
                result.push('|');
            }
            if !result.ends_with(' ') {
                result.push(' ');
            }
        }
        "row" => {
            ensure_table(table_state);
            if let Some(state) = table_state.as_mut()
                && (state.in_row || !state.current_cell.is_empty())
            {
                state.push_row();
            }
            if !result.ends_with('|') {
                result.push('|');
            }
            if !result.ends_with('\n') {
                result.push('\n');
            }
        }
        _ => {}
    }
}
