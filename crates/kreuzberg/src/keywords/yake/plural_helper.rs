// Vendored from yake-rust 1.0.3 (MIT) — https://github.com/quesurifn/yake-rust

pub(crate) trait PluralHelper {
    /// Strip trailing 's'/'S' from words longer than 3 characters.
    fn to_single(self) -> Self;
}

impl<'a> PluralHelper for &'a str {
    #[inline]
    fn to_single(self) -> &'a str {
        if self.len() > 3 && matches!(self.as_bytes().last(), Some(b's' | b'S')) {
            &self[..self.len() - 1]
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_trailing_s() {
        assert_eq!("cats".to_single(), "cat");
        assert_eq!("DOGS".to_single(), "DOG");
    }

    #[test]
    fn preserves_short_words() {
        assert_eq!("as".to_single(), "as");
        assert_eq!("is".to_single(), "is");
        assert_eq!("bus".to_single(), "bus");
    }

    #[test]
    fn preserves_non_s_endings() {
        assert_eq!("hello".to_single(), "hello");
    }
}
