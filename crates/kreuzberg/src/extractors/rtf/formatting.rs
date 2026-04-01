//! Text formatting utilities for RTF content.

/// Normalize whitespace in a string, also producing a byte-offset mapping from
/// input positions to output positions. The mapping is a sorted list of
/// `(old_offset, new_offset)` pairs that covers every byte boundary in the
/// input. Callers can use [`map_offset`] to translate an arbitrary input byte
/// offset to the corresponding output byte offset.
pub fn normalize_whitespace_with_mapping(s: &str) -> (String, Vec<(usize, usize)>) {
    // Phase 1: split into lines, trim each, collapse blank runs — same as
    // normalize_whitespace but we also record a mapping from the position in `s`
    // to the position in the rebuilt string.
    //
    // We rebuild the string character-by-character and track (old_byte, new_byte)
    // at each step so that formatting spans can be remapped precisely.

    let mut mapping: Vec<(usize, usize)> = Vec::new();
    // We build an intermediate "joined" representation that mirrors lines.join("\n")
    // but with offset tracking from the original string `s`.

    // Step 1: identify which portions of `s` survive the line-trim + blank-collapse pass.
    struct LineInfo<'a> {
        trimmed: &'a str,
        /// Byte offset of `trimmed` within the original string `s`.
        start_in_s: usize,
    }
    let mut kept_lines: Vec<LineInfo> = Vec::new();
    let mut last_blank = false;
    let mut line_start = 0usize;
    for line in s.split('\n') {
        let trimmed = line.trim();
        let trim_offset = line_start + (line.len() - line.trim_start().len());
        if trimmed.is_empty() {
            if !last_blank && !kept_lines.is_empty() {
                // Emit a blank line (empty trimmed)
                kept_lines.push(LineInfo {
                    trimmed: "",
                    start_in_s: line_start,
                });
                last_blank = true;
            }
        } else {
            last_blank = false;
            kept_lines.push(LineInfo {
                trimmed,
                start_in_s: trim_offset,
            });
        }
        line_start += line.len() + 1; // +1 for the '\n' consumed by split
    }
    // Trim trailing blank lines
    while kept_lines.last().is_some_and(|l| l.trimmed.is_empty()) {
        kept_lines.pop();
    }

    // Step 2: build the joined string with newlines between lines, tracking
    // (old_byte, new_byte) for each character.
    let mut joined = String::with_capacity(s.len());
    let mut new_pos = 0usize;
    for (li, line_info) in kept_lines.iter().enumerate() {
        if li > 0 {
            // The newline between lines. Map it to the newline in the original that separated them.
            // We don't have a perfect original byte for synthetic newlines in blank-line collapse,
            // but for non-blank lines, the newline came from the original.
            joined.push('\n');
            new_pos += 1;
        }
        let mut old_pos = line_info.start_in_s;
        for ch in line_info.trimmed.chars() {
            mapping.push((old_pos, new_pos));
            joined.push(ch);
            old_pos += ch.len_utf8();
            new_pos += ch.len_utf8();
        }
    }
    // Sentinel at end
    mapping.push((s.len(), new_pos));

    // Step 3: collapse runs of spaces within lines (same as normalize_whitespace)
    let mut result = String::with_capacity(joined.len());
    let mut mapping2: Vec<(usize, usize)> = Vec::new();
    let mut last_was_space = false;
    let mut joined_byte = 0usize;
    let mut result_byte = 0usize;
    for ch in joined.chars() {
        if ch == '\n' {
            mapping2.push((joined_byte, result_byte));
            result.push('\n');
            joined_byte += 1;
            result_byte += 1;
            last_was_space = false;
        } else if ch == ' ' || ch == '\t' {
            if !last_was_space {
                mapping2.push((joined_byte, result_byte));
                result.push(' ');
                result_byte += 1;
                last_was_space = true;
            }
            // else: collapsed — map this joined byte to the same result position
            // (no mapping2 entry needed since we'll interpolate)
            joined_byte += ch.len_utf8();
        } else {
            mapping2.push((joined_byte, result_byte));
            result.push(ch);
            joined_byte += ch.len_utf8();
            result_byte += ch.len_utf8();
            last_was_space = false;
        }
    }
    mapping2.push((joined_byte, result_byte));

    // Step 4: trim leading/trailing whitespace and remove spaces before punctuation
    let trimmed_result = result.trim();
    let trim_start = result.len() - result.trim_start().len();
    let trimmed_owned = trimmed_result.to_string();

    let chars_vec: Vec<char> = trimmed_owned.chars().collect();
    let mut cleaned = String::with_capacity(trimmed_owned.len());
    let mut mapping3: Vec<(usize, usize)> = Vec::new();
    let mut trimmed_byte = 0usize;
    let mut cleaned_byte = 0usize;
    let mut ci = 0;
    while ci < chars_vec.len() {
        let skip = if chars_vec[ci] == ' '
            && ci + 1 < chars_vec.len()
            && matches!(chars_vec[ci + 1], '.' | ',' | ';' | ':' | '!' | '?' | '|')
            && (ci == 0 || chars_vec[ci - 1] != ' ')
        {
            true
        } else {
            chars_vec[ci] == ' ' && ci > 0 && chars_vec[ci - 1] == '|'
        };
        if skip {
            trimmed_byte += chars_vec[ci].len_utf8();
            ci += 1;
            continue;
        }
        mapping3.push((trimmed_byte, cleaned_byte));
        cleaned.push(chars_vec[ci]);
        trimmed_byte += chars_vec[ci].len_utf8();
        cleaned_byte += chars_vec[ci].len_utf8();
        ci += 1;
    }
    mapping3.push((trimmed_byte, cleaned_byte));

    // Now compose the three mappings: s -> joined -> result -> trimmed -> cleaned
    // We need a single mapping: s_offset -> cleaned_offset
    // Compose: s -> joined (mapping), joined -> result (mapping2),
    //          result -> trimmed (subtract trim_start), trimmed -> cleaned (mapping3)
    let mut final_mapping: Vec<(usize, usize)> = Vec::new();
    for &(s_off, joined_off) in &mapping {
        let result_off = apply_mapping(&mapping2, joined_off);
        if result_off < trim_start {
            final_mapping.push((s_off, 0));
            continue;
        }
        let trimmed_off = result_off - trim_start;
        let cleaned_off = apply_mapping(&mapping3, trimmed_off);
        final_mapping.push((s_off, cleaned_off));
    }

    (cleaned, final_mapping)
}

/// Look up a byte offset in a sorted mapping using binary search and interpolation.
fn apply_mapping(mapping: &[(usize, usize)], offset: usize) -> usize {
    if mapping.is_empty() {
        return offset;
    }
    match mapping.binary_search_by_key(&offset, |&(old, _)| old) {
        Ok(i) => mapping[i].1,
        Err(0) => mapping[0].1,
        Err(i) if i >= mapping.len() => {
            // Beyond last mapping entry — extrapolate
            let (last_old, last_new) = mapping[mapping.len() - 1];
            if offset >= last_old {
                last_new + (offset - last_old)
            } else {
                last_new
            }
        }
        Err(i) => {
            // Interpolate between mapping[i-1] and mapping[i]
            let (old_lo, new_lo) = mapping[i - 1];
            let delta = offset - old_lo;
            new_lo + delta
        }
    }
}

/// Map a byte offset from the pre-normalized string to the post-normalized string.
pub fn map_offset(mapping: &[(usize, usize)], offset: usize) -> usize {
    apply_mapping(mapping, offset)
}

/// Normalize whitespace in a string.
///
/// - Collapses multiple consecutive spaces/tabs into a single space
/// - Preserves single newlines (paragraph breaks from \par)
/// - Collapses multiple consecutive newlines into a double newline
/// - Trims leading/trailing whitespace from each line
/// - Trims leading/trailing blank lines
pub fn normalize_whitespace(s: &str) -> String {
    // Split into lines, trim each, collapse blank runs
    let mut lines: Vec<&str> = Vec::new();
    let mut last_blank = false;

    for line in s.split('\n') {
        // Collapse internal whitespace on each line
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_blank && !lines.is_empty() {
                lines.push("");
                last_blank = true;
            }
        } else {
            last_blank = false;
            lines.push(trimmed);
        }
    }

    // Trim trailing blank lines
    while lines.last() == Some(&"") {
        lines.pop();
    }

    // Join and collapse internal multi-spaces within each line
    let joined = lines.join("\n");

    // Collapse runs of spaces within lines
    let mut result = String::with_capacity(joined.len());
    let mut last_was_space = false;
    for ch in joined.chars() {
        if ch == '\n' {
            result.push('\n');
            last_was_space = false;
        } else if ch == ' ' || ch == '\t' {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    // Remove spurious spaces before/after punctuation marks that result from RTF group boundaries
    let result = result.trim().to_string();
    let mut cleaned = String::with_capacity(result.len());
    let chars: Vec<char> = result.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' '
            && i + 1 < chars.len()
            && matches!(chars[i + 1], '.' | ',' | ';' | ':' | '!' | '?' | '|')
            && (i == 0 || chars[i - 1] != ' ')
        {
            // Skip the space before punctuation/pipe
            i += 1;
            continue;
        }
        if chars[i] == ' ' && i > 0 && chars[i - 1] == '|' {
            // Skip the space after pipe (table cell separator)
            i += 1;
            continue;
        }
        cleaned.push(chars[i]);
        i += 1;
    }
    cleaned
}
