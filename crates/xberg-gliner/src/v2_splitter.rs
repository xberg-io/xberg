use regex::Regex;

use crate::{Result, Token};

pub(crate) const V2_SPLITTER_REGEX: &str =
    r"(?i)(?:https?://[^\s]+|www\.[^\s]+)|[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}|@[a-z0-9_]+|\w+(?:[-_]\w+)*|\S";

/// GLiNER2 word splitter. Lowercases the input before matching, mirroring the
/// `fastino/gliner2` reference preprocessing the model was trained against.
///
/// Byte offsets are taken from the lowercased copy and applied back to the
/// original text. This holds for ASCII and most Latin-script text. Characters
/// whose lowercase form changes byte length (e.g. Turkish dotted İ) can yield
/// misaligned spans — the upstream reference implementation has the same
/// limitation, so this preserves parity rather than diverging from it.
pub struct V2Splitter {
    regex: Regex,
}

impl V2Splitter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            regex: Regex::new(V2_SPLITTER_REGEX)?,
        })
    }

    pub fn split(&self, input: &str) -> Vec<Token> {
        let lowered = input.to_lowercase();
        self.regex
            .find_iter(&lowered)
            .map(|m| Token::new(m.start(), m.end(), m.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_lowercases_plain_words() {
        let splitter = V2Splitter::new().expect("valid regex");
        let tokens = splitter.split("Steve Jobs founded Apple Inc.");
        let texts: Vec<&str> = tokens.iter().map(Token::text).collect();
        assert_eq!(texts, vec!["steve", "jobs", "founded", "apple", "inc", "."]);
    }

    #[test]
    fn matches_emails_and_urls_as_single_tokens() {
        let splitter = V2Splitter::new().expect("valid regex");
        let tokens = splitter.split("contact ada@example.com or https://example.com/path now");
        let texts: Vec<&str> = tokens.iter().map(Token::text).collect();
        assert_eq!(
            texts,
            vec!["contact", "ada@example.com", "or", "https://example.com/path", "now"]
        );
    }

    #[test]
    fn preserves_byte_offsets_into_original_text() {
        let splitter = V2Splitter::new().expect("valid regex");
        let text = "Apple Inc. was founded in Cupertino.";
        let tokens = splitter.split(text);
        let cupertino = tokens.iter().find(|token| token.text() == "cupertino").expect("found");
        assert_eq!(&text[cupertino.start()..cupertino.end()], "Cupertino");
    }
}
