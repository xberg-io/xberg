use crate::v2::splitter::V2Splitter;
use crate::v2::tokenizer::{PretokenizedEncoding, PretokenizingTokenizer};
use crate::{GlinerError, Result, Token};

const SCHEMA_TOKEN_P: &str = "[P]";
const SCHEMA_TOKEN_E: &str = "[E]";
const SEP_TEXT_TOKEN: &str = "[SEP_TEXT]";

pub struct V2Encoded {
    pub input_ids: Vec<i64>,
    pub text_positions: Vec<i64>,
    pub schema_positions: Vec<i64>,
    pub words: Vec<Token>,
}

/// Build the GLiNER2 schema-prompt token sequence: `( [P] entities ( [E] label1 [E] label2 ... ) )`.
/// Multi-word labels expand to one schema token per whitespace-separated word, matching
/// the upstream `fastino/gliner2` reference preprocessing.
fn build_schema_tokens(labels: &[String]) -> Vec<String> {
    let mut schema = vec![
        "(".to_string(),
        SCHEMA_TOKEN_P.to_string(),
        "entities".to_string(),
        "(".to_string(),
    ];
    for label in labels {
        schema.push(SCHEMA_TOKEN_E.to_string());
        for part in label.split_whitespace() {
            schema.push(part.to_string());
        }
    }
    schema.push(")".to_string());
    schema.push(")".to_string());
    schema
}

pub fn encode_v2(
    text: &str,
    labels: &[String],
    tokenizer: &impl PretokenizingTokenizer,
    splitter: &V2Splitter,
) -> Result<V2Encoded> {
    let schema_tokens = build_schema_tokens(labels);
    let words = splitter.split(text);
    let num_schema_words = schema_tokens.len() + 1; // +1 for [SEP_TEXT] ~keep

    let mut full_sequence: Vec<&str> = schema_tokens.iter().map(String::as_str).collect();
    full_sequence.push(SEP_TEXT_TOKEN);
    full_sequence.extend(words.iter().map(Token::text));

    let encoding: PretokenizedEncoding = tokenizer.encode_pretokenized(full_sequence)?;

    // One forward pass over word_ids, recording each word's first token
    // position. A per-word `.position()` scan would be O(words x tokens),
    // which is quadratic on exactly the large documents this pipeline
    // targets; this index keeps the whole lookup phase O(tokens). ~keep
    let total_words = num_schema_words + words.len();
    let mut first_token_of_word: Vec<Option<usize>> = vec![None; total_words];
    for (token_pos, word_id) in encoding.word_ids.iter().enumerate() {
        if let Some(id) = word_id {
            let idx = *id as usize;
            if idx < total_words && first_token_of_word[idx].is_none() {
                first_token_of_word[idx] = Some(token_pos);
            }
        }
    }

    let mut text_positions = Vec::with_capacity(words.len());
    for word_index in 0..words.len() {
        let position = first_token_of_word[num_schema_words + word_index].ok_or_else(|| {
            GlinerError::Tokenizer(format!(
                "GLiNER2 tokenizer dropped text word {word_index} during pre-tokenized encoding"
            ))
        })?;
        text_positions.push(position as i64);
    }

    let mut schema_positions = Vec::new();
    for (index, token) in schema_tokens.iter().enumerate() {
        if token == SCHEMA_TOKEN_P || token == SCHEMA_TOKEN_E {
            let position = first_token_of_word[index].ok_or_else(|| {
                GlinerError::Tokenizer(format!(
                    "GLiNER2 tokenizer dropped schema marker '{token}' at schema word {index} during encoding"
                ))
            })?;
            schema_positions.push(position as i64);
        }
    }

    Ok(V2Encoded {
        input_ids: encoding.ids,
        text_positions,
        schema_positions,
        words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One output token per input word, in order; `word_ids[i] == Some(i)`.
    /// Makes position assertions trivial: every schema/text position should
    /// equal the corresponding word's index in `full_sequence`.
    struct FakeTokenizer;

    impl PretokenizingTokenizer for FakeTokenizer {
        fn encode_pretokenized(&self, words: Vec<&str>) -> Result<PretokenizedEncoding> {
            Ok(PretokenizedEncoding {
                ids: (0..words.len() as i64).collect(),
                word_ids: (0..words.len() as u32).map(Some).collect(),
            })
        }
    }

    #[test]
    fn builds_schema_tokens_with_one_entry_per_label_word() {
        let labels = vec!["person".to_string(), "company name".to_string()];
        let schema = build_schema_tokens(&labels);
        assert_eq!(
            schema,
            vec![
                "(", "[P]", "entities", "(", "[E]", "person", "[E]", "company", "name", ")", ")"
            ]
        );
    }

    #[test]
    fn computes_text_and_schema_positions() {
        let labels = vec!["person".to_string(), "city".to_string()];
        // schema_tokens = ["(", "[P]", "entities", "(", "[E]", "person", "[E]", "city", ")", ")"]
        // len = 10, num_schema_words = 11 (+1 for [SEP_TEXT]) ~keep
        let splitter = V2Splitter::new().expect("valid regex");
        let encoded = encode_v2("Ada lives", &labels, &FakeTokenizer, &splitter).expect("encoded");

        assert_eq!(encoded.words.len(), 2);
        // text words start at full_sequence index 11 and 12
        assert_eq!(encoded.text_positions, vec![11, 12]);
        // [P] is schema word 1, [E] tokens are schema words 4 and 6
        assert_eq!(encoded.schema_positions, vec![1, 4, 6]);
        assert_eq!(encoded.input_ids.len(), 13); // 10 schema + 1 sep + 2 words ~keep
    }

    #[test]
    fn errors_when_tokenizer_drops_a_required_word() {
        struct DroppingTokenizer;
        impl PretokenizingTokenizer for DroppingTokenizer {
            fn encode_pretokenized(&self, _words: Vec<&str>) -> Result<PretokenizedEncoding> {
                Ok(PretokenizedEncoding {
                    ids: vec![1, 2, 3],
                    word_ids: vec![Some(0), Some(1), Some(2)],
                })
            }
        }

        let splitter = V2Splitter::new().expect("valid regex");
        let labels = vec!["person".to_string()];
        let result = encode_v2("Ada lives here", &labels, &DroppingTokenizer, &splitter);
        assert!(result.is_err());
    }
}
