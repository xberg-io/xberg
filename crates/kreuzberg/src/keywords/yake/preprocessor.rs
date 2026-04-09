// Custom sentence splitter and word tokenizer replacing segtok.
// Fixes #676: segtok's regex-based splitter panics with BacktrackLimitExceeded on large files.
// Uses memchr for O(n) scanning with no backtracking.

use std::borrow::Cow;

use memchr::memchr3;

/// Common abbreviations that should not trigger sentence splits.
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "ave", "blvd", "gen", "gov", "sgt", "cpl", "pvt", "capt", "lt",
    "col", "maj", "cmdr", "adm", "dept", "univ", "assn", "bros", "inc", "ltd", "co", "corp", "vs", "al", "approx",
    "appt", "apt", "dept", "dpt", "est", "etc", "fig", "figs", "ft", "hr", "hrs", "min", "mins", "misc", "mt", "no",
    "nos", "nr", "oz", "ph", "pp", "sec", "vol", "rev", "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct",
    "nov", "dec", "mon", "tue", "wed", "thu", "fri", "sat", "sun",
];

/// Split text into sentences. O(n) with no regex.
pub(crate) fn split_into_sentences(text: &str) -> Vec<Cow<'_, str>> {
    if text.is_empty() {
        return Vec::new();
    }

    let bytes = text.as_bytes();
    let mut sentences: Vec<Cow<'_, str>> = Vec::new();
    let mut start = 0;

    while start < bytes.len() {
        // Skip leading whitespace
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }
        if start >= bytes.len() {
            break;
        }

        match find_sentence_end(text, start) {
            Some(end) => {
                let s = text[start..end].trim();
                if !s.is_empty() {
                    sentences.push(Cow::Borrowed(s));
                }
                start = end;
            }
            None => {
                // Rest of text is one sentence
                let s = text[start..].trim();
                if !s.is_empty() {
                    sentences.push(Cow::Borrowed(s));
                }
                break;
            }
        }
    }

    sentences
}

/// Find the end position of the current sentence starting at `from`.
/// Returns the byte index *after* the sentence boundary, or None if no boundary found.
fn find_sentence_end(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut pos = from;

    while pos < bytes.len() {
        // Check for paragraph boundary (\n\n)
        if bytes[pos] == b'\n' {
            let nl_start = pos;
            pos += 1;
            // Consume whitespace-only chars looking for another newline
            let mut found_second_nl = false;
            let mut scan = pos;
            while scan < bytes.len() {
                if bytes[scan] == b'\n' {
                    found_second_nl = true;
                    scan += 1;
                    break;
                } else if bytes[scan] == b' ' || bytes[scan] == b'\t' || bytes[scan] == b'\r' {
                    scan += 1;
                } else {
                    break;
                }
            }
            if found_second_nl {
                // Split at paragraph boundary
                return Some(scan);
            }
            pos = nl_start + 1;
            continue;
        }

        // Look for sentence terminals: . ! ?
        match memchr3(b'.', b'!', b'?', &bytes[pos..]) {
            None => return None,
            Some(offset) => {
                let terminal_pos = pos + offset;
                // Consume consecutive terminals (e.g., "..." or "?!")
                let mut end = terminal_pos + 1;
                while end < bytes.len() && (bytes[end] == b'.' || bytes[end] == b'!' || bytes[end] == b'?') {
                    end += 1;
                }

                // Consume closing quotes/brackets after terminal
                while end < bytes.len() && matches!(bytes[end], b'"' | b'\'' | b')' | b']' | b'}') {
                    end += 1;
                }

                // Check if this is a real sentence boundary
                if is_sentence_boundary(text, terminal_pos, end) {
                    return Some(end);
                }

                pos = end;
            }
        }
    }

    None
}

/// Determine if a terminal at `terminal_pos` is a real sentence boundary.
fn is_sentence_boundary(text: &str, terminal_pos: usize, after_terminal: usize) -> bool {
    let bytes = text.as_bytes();

    // Only '.' needs abbreviation checks; '!' and '?' are always sentence-ending
    if bytes[terminal_pos] != b'.' {
        // For ! and ?, check that next non-space char exists and starts new content
        return has_content_after(bytes, after_terminal);
    }

    // Check if it's an abbreviation
    if is_abbreviation(text, terminal_pos) {
        return false;
    }

    // Single letter followed by dot (initial like "A." or "J.")
    if terminal_pos >= 1
        && bytes[terminal_pos - 1].is_ascii_alphabetic()
        && (terminal_pos < 2 || !bytes[terminal_pos - 2].is_ascii_alphabetic())
    {
        return false;
    }

    // Check what follows: if next non-whitespace char is lowercase, probably not a new sentence
    let mut next = after_terminal;
    while next < bytes.len() && bytes[next].is_ascii_whitespace() {
        next += 1;
    }

    if next >= bytes.len() {
        return false; // End of text, no need to split
    }

    // If next char is lowercase, likely not a sentence boundary (e.g., "3.14 meters", "e.g. something")
    if bytes[next].is_ascii_lowercase() {
        return false;
    }

    true
}

/// Check if there's meaningful content after position.
fn has_content_after(bytes: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i < bytes.len()
}

/// Check if the word before the dot at `dot_pos` is a known abbreviation.
fn is_abbreviation(text: &str, dot_pos: usize) -> bool {
    // Walk backwards to find the start of the word
    let bytes = text.as_bytes();
    let mut word_start = dot_pos;
    while word_start > 0 && bytes[word_start - 1].is_ascii_alphabetic() {
        word_start -= 1;
    }

    if word_start == dot_pos {
        return false; // No word before the dot
    }

    let word = &text[word_start..dot_pos];
    // Check case-insensitively
    let lower: Cow<'_, str> = if word.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(word.to_ascii_lowercase())
    } else {
        Cow::Borrowed(word)
    };

    ABBREVIATIONS.contains(&lower.as_ref())
}

/// Split a sentence into word tokens. No regex, handles contractions.
pub(crate) fn split_into_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Punctuation as single token (but not apostrophe in middle of word, not hyphen in middle of word)
        if is_punctuation_byte(b) && b != b'\'' && b != b'-' {
            // Emit single punctuation token
            words.push(String::from(b as char));
            i += 1;
            continue;
        }

        // Start of a word: collect word characters (letters, digits, hyphens, apostrophes within words)
        let word_start = i;
        while i < len {
            let c = bytes[i];
            if c.is_ascii_alphanumeric() || c > 127 {
                // Letter, digit, or non-ASCII (multibyte UTF-8 start)
                i += 1;
                // Skip continuation bytes for multi-byte UTF-8
                while i < len && bytes[i] > 127 && bytes[i] < 192 {
                    i += 1;
                }
            } else if c == b'-' && i + 1 < len && (bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] > 127) {
                // Hyphen within word (e.g., "high-tech")
                i += 1;
            } else if c == b'\'' && i > word_start && i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
                // Apostrophe within word (e.g., "don't")
                // Split contraction: emit the part before apostrophe, then the part with apostrophe
                let before = &text[word_start..i];
                if !before.is_empty() {
                    words.push(before.to_string());
                }
                // Now collect the contraction part (e.g., "'t", "'s", "'ll")
                let cont_start = i;
                i += 1; // skip apostrophe
                while i < len && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let contraction = &text[cont_start..i];
                // Skip contractions that are just a lone apostrophe
                if contraction.len() > 1 && !contraction.starts_with("'") || contraction.len() > 1 {
                    // Filter out tokens starting with ' that are longer than 1 char (matching original behavior)
                    // Original: filter(|word| !(word.len() > 1 && word.starts_with("'")))
                    // Actually we should NOT add these per original yake-rust behavior
                }
                // Original yake-rust filters out tokens that start with ' and len > 1
                // So we skip the contraction suffix
                continue;
            } else {
                break;
            }
        }

        if i > word_start {
            let word = &text[word_start..i];
            if !word.is_empty() {
                words.push(word.to_string());
            }
        } else {
            // Single non-word character (punctuation we didn't catch above)
            if i < len {
                let ch_len = text[i..].chars().next().map_or(1, |c| c.len_utf8());
                words.push(text[i..i + ch_len].to_string());
                i += ch_len;
            }
        }
    }

    words
}

#[inline]
fn is_punctuation_byte(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'"'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b'-'
            | b'.'
            | b'/'
            | b':'
            | b';'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'['
            | b'\\'
            | b']'
            | b'^'
            | b'_'
            | b'`'
            | b'{'
            | b'|'
            | b'}'
            | b'~'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple_sentences() {
        let text = "One smartwatch. One phone. Many phones.";
        let result = split_into_sentences(text);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "One smartwatch.");
        assert_eq!(result[1], "One phone.");
        assert_eq!(result[2], "Many phones.");
    }

    #[test]
    fn split_exclamation_sentences() {
        let text = "This is your weekly newsletter! Hundreds of great deals - everything from men's fashion to high-tech drones!";
        let result = split_into_sentences(text);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "This is your weekly newsletter!");
    }

    #[test]
    fn split_paragraph_boundary() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let result = split_into_sentences(text);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "First paragraph.");
        assert_eq!(result[1], "Second paragraph.");
    }

    #[test]
    fn abbreviation_no_split() {
        let text = "Dr. Smith went to Washington.";
        let result = split_into_sentences(text);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn split_hyphenated_words() {
        let text = "Truly high-tech!";
        let words = split_into_words(text);
        assert_eq!(words, vec!["Truly", "high-tech", "!"]);
    }

    #[test]
    fn empty_text() {
        assert!(split_into_sentences("").is_empty());
        assert!(split_into_words("").is_empty());
    }

    #[test]
    fn large_input_no_panic() {
        // Regression test for #676: large inputs must not panic
        let paragraph = "This is a test sentence with some words. ";
        let large_text = paragraph.repeat(250_000); // ~10 MB
        let sentences = split_into_sentences(&large_text);
        assert!(!sentences.is_empty());
    }
}
