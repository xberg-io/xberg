//! Vendored from text-splitter v0.30.1 (MIT, © 2023 Benjamin Brandt). See ATTRIBUTIONS.md.

use tokenizers::{Encoding, Tokenizer};

use super::super::ChunkSizer;

/// Compute the number of tokens that exist within an entire [`Encoding`] object.
///
/// Take into account [`Encoding::get_overflowing`] for cases where the [`Tokenizer`] producing the [`Encoding`] has truncation parameters set.
fn num_tokens_with_overflow(encoding: &Encoding) -> usize {
    let base = encoding
        .get_attention_mask()
        .iter()
        // Padding has a zero mask; real content may share its token ID.
        .filter(|&&attention| attention != 0)
        .count();

    let overflow: usize = encoding.get_overflowing().iter().map(num_tokens_with_overflow).sum();

    base + overflow
}

impl ChunkSizer for Tokenizer {
    /// Returns the number of tokens in a given text after tokenization.
    ///
    /// # Panics
    ///
    /// Will panic if you don't have a byte-level tokenizer and the splitter
    /// encounters text it can't tokenize.
    fn size(&self, chunk: &str) -> usize {
        let encoding = self
            .encode_fast(chunk, false)
            .expect("Unable to tokenize the following string {chunk}");

        num_tokens_with_overflow(&encoding)
    }
}

#[cfg(test)]
mod tests {
    use tokenizers::{
        PaddingDirection, PaddingParams, PaddingStrategy, Token, models::wordlevel::WordLevel,
        pre_tokenizers::whitespace::WhitespaceSplit,
    };

    use super::*;

    fn tokenizer(repo: &str, revision: &str) -> Tokenizer {
        let path = crate::model_download::hf_resolve_file(repo, "tokenizer.json", Some(revision), None, None)
            .unwrap_or_else(|error| panic!("Could not resolve tokenizer '{repo}@{revision}': {error}"));
        Tokenizer::from_file(&path)
            .unwrap_or_else(|error| panic!("Could not load tokenizer '{}': {error}", path.display()))
    }

    fn local_tokenizer(direction: PaddingDirection) -> Tokenizer {
        let model = WordLevel::builder()
            .vocab(
                [
                    ("[PAD]".into(), 0),
                    ("[UNK]".into(), 1),
                    ("first".into(), 2),
                    ("second".into(), 3),
                    ("third".into(), 4),
                ]
                .into_iter()
                .collect(),
            )
            .unk_token("[UNK]".into())
            .build()
            .unwrap();
        let mut tokenizer = Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(WhitespaceSplit));
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(8),
            direction,
            pad_id: 0,
            pad_token: "[PAD]".into(),
            ..Default::default()
        }));
        tokenizer
    }

    #[test]
    fn content_padding_ids_are_counted() {
        for direction in [PaddingDirection::Left, PaddingDirection::Right] {
            let mut tokenizer = local_tokenizer(direction);
            for padding in [true, false] {
                if !padding {
                    tokenizer.with_padding(None);
                }
                for (text, expected) in [
                    ("", 0),
                    ("first second third", 3),
                    ("[PAD]", 1),
                    ("[PAD] first second third", 4),
                    ("first [PAD] second third", 4),
                    ("first second third [PAD]", 4),
                ] {
                    assert_eq!(
                        tokenizer.size(text),
                        expected,
                        "{direction:?}, padding={padding}, {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn content_padding_ids_in_overflow_are_counted() {
        let mut encoding = Encoding::from_tokens(vec![Token::new(0, "[PAD]".into(), (0, 5))], 0);
        let mut overflow = Encoding::from_tokens(
            vec![
                Token::new(0, "[PAD]".into(), (0, 5)),
                Token::new(2, "first".into(), (6, 11)),
            ],
            0,
        );
        overflow.pad(8, 0, 0, "[PAD]", PaddingDirection::Left);
        encoding.set_overflowing(vec![overflow]);
        encoding.pad(8, 0, 0, "[PAD]", PaddingDirection::Right);
        assert_eq!(num_tokens_with_overflow(&encoding), 3);
    }

    #[test]
    fn returns_size() {
        let tokenizer = tokenizer("bert-base-cased", "cd5ef92a9fb2f889e972770a36d4ed042daf221e");
        let size = tokenizer.size(" An apple a");
        assert_eq!(size, 3);
    }

    #[test]
    fn returns_size_handles_prefix() {
        let tokenizer = tokenizer("thenlper/gte-small", "17e1f347d17fe144873b1201da91788898c639cd");

        let size = tokenizer.size("An apple a");
        assert_eq!(size, 3);
    }

    #[test]
    fn handles_padding() {
        let tokenizer = tokenizer("thenlper/gte-small", "17e1f347d17fe144873b1201da91788898c639cd");
        let size = tokenizer.size("An apple a");
        assert_eq!(size, 3);
    }

    #[test]
    fn handle_truncation() {
        let tokenizer = tokenizer(
            "sentence-transformers/all-MiniLM-L6-v2",
            "1110a243fdf4706b3f48f1d95db1a4f5529b4d41",
        );

        assert_eq!(
            tokenizer.size("An apple a day keeps the doctor away.".repeat(100).as_str()),
            128
        );
    }
}
