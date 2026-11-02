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
        if bytes[pos] == b'\n' {
            let nl_start = pos;
            pos += 1;
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
                return Some(scan);
            }
            pos = nl_start + 1;
            continue;
        }

        {
            let offset = memchr3(b'.', b'!', b'?', &bytes[pos..])?;
            let terminal_pos = pos + offset;
            let mut end = terminal_pos + 1;
            while end < bytes.len() && (bytes[end] == b'.' || bytes[end] == b'!' || bytes[end] == b'?') {
                end += 1;
            }

            while end < bytes.len() && matches!(bytes[end], b'"' | b'\'' | b')' | b']' | b'}') {
                end += 1;
            }

            if is_sentence_boundary(text, terminal_pos, end) {
                return Some(end);
            }

            pos = end;
        }
    }

    None
}

/// Determine if a terminal at `terminal_pos` is a real sentence boundary.
fn is_sentence_boundary(text: &str, terminal_pos: usize, after_terminal: usize) -> bool {
    let bytes = text.as_bytes();

    if bytes[terminal_pos] != b'.' {
        return has_content_after(bytes, after_terminal);
    }

    if is_abbreviation(text, terminal_pos) {
        return false;
    }

    if terminal_pos >= 1
        && bytes[terminal_pos - 1].is_ascii_alphabetic()
        && (terminal_pos < 2 || !bytes[terminal_pos - 2].is_ascii_alphabetic())
    {
        return false;
    }

    let mut next = after_terminal;
    while next < bytes.len() && bytes[next].is_ascii_whitespace() {
        next += 1;
    }

    if next >= bytes.len() {
        return false;
    }

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
    let bytes = text.as_bytes();
    let mut word_start = dot_pos;
    while word_start > 0 && bytes[word_start - 1].is_ascii_alphabetic() {
        word_start -= 1;
    }

    if word_start == dot_pos {
        return false;
    }

    let word = &text[word_start..dot_pos];
    let lower: Cow<'_, str> = if word.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(word.to_ascii_lowercase())
    } else {
        Cow::Borrowed(word)
    };

    ABBREVIATIONS.contains(&lower.as_ref())
}

pub(super) fn canonical_term(word: &str) -> String {
    let mut term = word.to_lowercase();
    if let Some(base) = term.strip_suffix("'s").or_else(|| term.strip_suffix("’s"))
        && !base.is_empty()
        && !NONPOSSESSIVE_BASES.contains(&base)
    {
        term.truncate(base.len());
    }
    term
}

// These contractions share the possessive suffix but retain their lexical identity. ~keep
const NONPOSSESSIVE_BASES: &[&str] = &[
    "he", "she", "it", "that", "what", "who", "where", "when", "why", "how", "there", "here", "let",
];

/// Split a sentence into word tokens. No regex, handles contractions.
pub(crate) fn split_into_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if is_punctuation_byte(b) && b != b'\'' && b != b'-' {
            words.push(String::from(b as char));
            i += 1;
            continue;
        }

        let word_start = i;
        i = scan_word_end(text, word_start);

        if i > word_start {
            let word = &text[word_start..i];
            if !word.is_empty() {
                words.push(word.to_string());
            }
        } else if i < len {
            let ch_len = text[i..].chars().next().map_or(1, |c| c.len_utf8());
            words.push(text[i..i + ch_len].to_string());
            i += ch_len;
        }
    }

    words
}

/// Scan forward from `word_start` to the end of one word token, returning that offset.
fn scan_word_end(text: &str, word_start: usize) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = word_start;

    while i < len {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c > 127 {
            i += 1;
            while i < len && bytes[i] > 127 && bytes[i] < 192 {
                i += 1;
            }
        } else if c == b'-' && i + 1 < len && (bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] > 127) {
            i += 1;
        } else if c == b'\'' && i > word_start && i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
            i += 1;
            while i < len && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
        } else {
            break;
        }
    }

    i
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
        let paragraph = "This is a test sentence with some words. ";
        let large_text = paragraph.repeat(250_000);
        let sentences = split_into_sentences(&large_text);
        assert!(!sentences.is_empty());
    }
}

#[cfg(test)]
mod possessive_regression_tests {
    use super::super::{Yake, YakeConfig};
    use super::*;

    #[test]
    fn apostrophe_words_emit_one_original_token_without_invented_prefixes() {
        let sources = [
            "COMMISSION'S",
            "CONSULTANT'S",
            "can't",
            "we're",
            "it's",
            "O'Connor",
            "O'Sullivan",
        ];
        for source in sources {
            assert_eq!(source.split_whitespace().count(), 1);
        }
        let actual: Vec<_> = sources.iter().map(|source| split_into_words(source)).collect();
        let expected: Vec<_> = sources.iter().map(|source| vec![source.to_string()]).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn curly_words_and_real_repeated_names_conserve_original_tokens() {
        for (source, expected) in [
            ("commission’s", vec!["commission’s"]),
            ("CONSULTANT’S", vec!["CONSULTANT’S"]),
            ("can’t", vec!["can’t"]),
            ("O’Connor O’Connor", vec!["O’Connor", "O’Connor"]),
            ("Baden Baden", vec!["Baden", "Baden"]),
        ] {
            assert_eq!(split_into_words(source), expected);
        }
    }

    #[test]
    fn possessive_canonical_terms_agree_before_candidate_scoring_and_deduplication() {
        let stopwords = ahash::AHashSet::new();
        let yake = Yake {
            config: YakeConfig::default(),
            stopwords: &stopwords,
        };
        let cases = [
            ("cat", "cat"),
            ("cat's", "cat"),
            ("cat’s", "cat"),
            ("CAT’S", "cat"),
            ("Commission", "commission"),
            ("COMMISSION'S", "commission"),
            ("commission’s", "commission"),
            ("CONSULTANT’S", "consultant"),
        ];
        let actual: Vec<_> = cases
            .iter()
            .map(|(source, _)| {
                assert_eq!(source.split_whitespace().count(), 1);
                let sentences = yake.preprocess_text(source);
                assert_eq!(sentences.len(), 1);
                let candidates = yake.ngram_selection(1, &sentences);
                let keys: Vec<_> = candidates.keys().map(|key| key.to_vec()).collect();
                (sentences[0].lc_terms.clone(), sentences[0].uq_terms.clone(), keys)
            })
            .collect();
        let expected: Vec<_> = cases
            .iter()
            .map(|(_, term)| {
                let term = term.to_string();
                (vec![term.clone()], vec![term.clone()], vec![vec![term]])
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn nonpossessive_contractions_keep_their_canonical_text() {
        let stopwords = ahash::AHashSet::new();
        let yake = Yake {
            config: YakeConfig::default(),
            stopwords: &stopwords,
        };
        let sources = [
            "can't",
            "we're",
            "they've",
            "I'm",
            "it's",
            "that's",
            "he's",
            "it’s",
            "HE’S",
            "O'Connor",
            "O’Connor",
        ];
        let actual: Vec<_> = sources
            .iter()
            .map(|source| {
                let sentences = yake.preprocess_text(source);
                assert_eq!(sentences.len(), 1);
                sentences[0].lc_terms.clone()
            })
            .collect();
        let expected: Vec<_> = sources.iter().map(|source| vec![source.to_lowercase()]).collect();
        assert_eq!(actual, expected);
    }
}
