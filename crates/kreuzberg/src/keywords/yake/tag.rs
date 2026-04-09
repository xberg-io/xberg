// Vendored from yake-rust 1.0.3 (MIT) — https://github.com/quesurifn/yake-rust
// Replaced std::collections::HashSet with inline checks for punctuation.

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum Tag {
    /// Numeric token
    Digit,
    /// Pure punctuation
    Punctuation,
    /// Unparsable (mixed digits+alpha, or multiple punctuation symbols)
    Unparsable,
    /// All-uppercase acronym (e.g., USA, USSR)
    Acronym,
    /// Starts with uppercase (not first word of sentence)
    Uppercase,
    /// Normal parsable word
    Parsable,
}

impl Tag {
    pub fn classify(word: &str, is_first_word: bool, punctuation: &[u8; 256], strict_capital: bool) -> Tag {
        if Self::is_numeric(word) {
            Tag::Digit
        } else if Self::is_punctuation(word, punctuation) {
            Tag::Punctuation
        } else if Self::is_unparsable(word, punctuation) {
            Tag::Unparsable
        } else if Self::is_acronym(word) {
            Tag::Acronym
        } else if Self::is_uppercase(word, is_first_word, strict_capital) {
            Tag::Uppercase
        } else {
            Tag::Parsable
        }
    }

    #[inline]
    fn is_numeric(word: &str) -> bool {
        // Check if word is a number (possibly with commas)
        let s = word.as_bytes();
        if s.is_empty() {
            return false;
        }
        let mut has_digit = false;
        let mut has_dot = false;
        for &b in s {
            match b {
                b'0'..=b'9' => has_digit = true,
                b',' => {}
                b'.' if !has_dot => has_dot = true,
                b'-' | b'+' if !has_digit => {}
                _ => return false,
            }
        }
        has_digit
    }

    #[inline]
    fn is_acronym(word: &str) -> bool {
        !word.is_empty() && word.chars().all(char::is_uppercase)
    }

    #[inline]
    fn is_uppercase(word: &str, is_first_word: bool, strict_capital: bool) -> bool {
        if is_first_word {
            return false;
        }
        if strict_capital {
            is_strict_capitalized(word)
        } else {
            is_capitalized(word)
        }
    }

    #[inline]
    fn is_punctuation(word: &str, punctuation: &[u8; 256]) -> bool {
        !word.is_empty() && word.bytes().all(|b| punctuation[b as usize] != 0)
    }

    fn is_unparsable(word: &str, punctuation: &[u8; 256]) -> bool {
        has_multiple_punctuation_symbols(word, punctuation) || {
            let has_digits = word.chars().any(|w| w.is_ascii_digit());
            let has_alphas = word.chars().any(|w| w.is_alphabetic());
            has_alphas == has_digits
        }
    }
}

#[inline]
fn is_capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

#[inline]
fn is_strict_capitalized(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(char::is_uppercase) && !chars.any(char::is_uppercase)
}

/// Check if a word contains more than one distinct punctuation symbol.
fn has_multiple_punctuation_symbols(word: &str, punctuation: &[u8; 256]) -> bool {
    let mut first_punct: u8 = 0;
    for &b in word.as_bytes() {
        if b < 128 && punctuation[b as usize] != 0 {
            if first_punct == 0 {
                first_punct = b;
            } else if b != first_punct {
                return true;
            }
        }
    }
    false
}

/// Build a fast 256-byte lookup table from a punctuation character set.
pub(crate) fn build_punctuation_table(chars: &str) -> [u8; 256] {
    let mut table = [0u8; 256];
    for ch in chars.bytes() {
        if (ch as usize) < 256 {
            table[ch as usize] = 1;
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_punct() -> [u8; 256] {
        build_punctuation_table(r##"!"#$%&'()*+,-./:,<=>?@[\]^_`{|}~"##)
    }

    #[test]
    fn tag_digit() {
        let p = default_punct();
        assert_eq!(Tag::classify("42", false, &p, true), Tag::Digit);
        assert_eq!(Tag::classify("3.14", false, &p, true), Tag::Digit);
        assert_eq!(Tag::classify("1,000", false, &p, true), Tag::Digit);
    }

    #[test]
    fn tag_punctuation() {
        let p = default_punct();
        assert_eq!(Tag::classify("...", false, &p, true), Tag::Punctuation);
        assert_eq!(Tag::classify("!", false, &p, true), Tag::Punctuation);
    }

    #[test]
    fn tag_acronym() {
        let p = default_punct();
        assert_eq!(Tag::classify("USA", false, &p, true), Tag::Acronym);
    }

    #[test]
    fn tag_uppercase_strict() {
        let p = default_punct();
        assert_eq!(Tag::classify("Paypal", false, &p, true), Tag::Uppercase);
        // PayPal has intermediate uppercase, so strict mode rejects it
        assert_eq!(Tag::classify("PayPal", false, &p, true), Tag::Parsable);
    }

    #[test]
    fn tag_first_word_not_uppercase() {
        let p = default_punct();
        assert_eq!(Tag::classify("Hello", true, &p, true), Tag::Parsable);
    }

    #[test]
    fn tag_unparsable() {
        let p = default_punct();
        assert_eq!(Tag::classify("B2C", false, &p, true), Tag::Unparsable);
    }
}
