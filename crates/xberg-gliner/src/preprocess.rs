use ndarray::Array2;

use crate::config::MAX_WORDS_PER_SEQUENCE;
use crate::splitter::Splitter;
use crate::tokenizer::Tokenizer;
use crate::{GlinerError, Result, TextInput, Token};

#[derive(Debug)]
pub(crate) struct Prompt {
    pub(crate) tokens: Vec<String>,
    entities_length: usize,
}

impl Prompt {
    fn new(tokens: Vec<String>, entities_length: usize) -> Self {
        Self {
            tokens,
            entities_length,
        }
    }
}

pub(crate) struct TokenizedInput {
    tokens: Vec<Vec<Token>>,
    texts: Vec<String>,
    entities: Vec<String>,
}

impl TokenizedInput {
    pub(crate) fn from(input: TextInput, splitter: &impl Splitter, max_length: Option<usize>) -> Result<Self> {
        let limit = max_length.unwrap_or(MAX_WORDS_PER_SEQUENCE);
        let tokens = input
            .texts
            .iter()
            .map(|text| splitter.split(text, Some(limit)))
            .collect::<Result<Vec<_>>>()?;
        if let Some(index) = tokens.iter().position(|tokens| tokens.len() > MAX_WORDS_PER_SEQUENCE) {
            return Err(GlinerError::InvalidInput(format!(
                "texts[{index}] exceeds the {MAX_WORDS_PER_SEQUENCE} word limit"
            )));
        }
        Ok(Self {
            tokens,
            texts: input.texts,
            entities: input.entities,
        })
    }
}

pub(crate) struct PromptInput {
    pub(crate) texts: Vec<String>,
    pub(crate) tokens: Vec<Vec<Token>>,
    pub(crate) entities: Vec<String>,
    pub(crate) text_lengths: Vec<usize>,
    pub(crate) num_words: usize,
    pub(crate) prompts: Vec<Prompt>,
}

impl PromptInput {
    pub(crate) fn from(input: TokenizedInput) -> Self {
        let entities_prompt = Self::entities_prompt(&input.entities);
        let mut text_lengths = Vec::with_capacity(input.tokens.len());
        let mut num_words = 0usize;
        let mut prompts = Vec::with_capacity(input.tokens.len());

        for tokens in &input.tokens {
            let mut prompt = Vec::with_capacity(entities_prompt.len() + tokens.len());
            prompt.extend(entities_prompt.clone());
            prompt.extend(tokens.iter().map(|token| token.text().to_string()));
            prompts.push(Prompt::new(prompt, entities_prompt.len()));
            text_lengths.push(tokens.len());
            num_words = num_words.max(tokens.len());
        }

        Self {
            texts: input.texts,
            tokens: input.tokens,
            entities: input.entities,
            text_lengths,
            num_words,
            prompts,
        }
    }

    fn entities_prompt(entities: &[String]) -> Vec<String> {
        const ENTITY_TOKEN: &str = "<<ENT>>";
        const SEP_TOKEN: &str = "<<SEP>>";

        let mut result = Vec::with_capacity(entities.len() * 2 + 1);
        for entity in entities {
            result.push(ENTITY_TOKEN.to_string());
            result.push(entity.clone());
        }
        result.push(SEP_TOKEN.to_string());
        result
    }
}

struct EncodedPrompt {
    encoding: Vec<Vec<u32>>,
    text_offset: usize,
}

pub(crate) struct EncodedInput {
    pub(crate) texts: Vec<String>,
    pub(crate) tokens: Vec<Vec<Token>>,
    pub(crate) entities: Vec<String>,
    pub(crate) num_words: usize,
    pub(crate) input_ids: Array2<i64>,
    pub(crate) attention_masks: Array2<i64>,
    pub(crate) word_masks: Array2<i64>,
    pub(crate) text_lengths: Array2<i64>,
}

impl EncodedInput {
    pub(crate) fn from(input: PromptInput, tokenizer: &impl Tokenizer) -> Result<Self> {
        let mut encodings = Vec::with_capacity(input.prompts.len());
        let mut max_tokens = 0usize;

        for prompt in &input.prompts {
            let mut prompt_tokens = Vec::with_capacity(prompt.tokens.len());
            let mut total_tokens = 2usize;
            let mut total_entity_tokens = 0usize;

            for (position, word) in prompt.tokens.iter().enumerate() {
                let encoding = tokenizer.encode(word)?;
                total_tokens += encoding.len();
                if position < prompt.entities_length {
                    total_entity_tokens += encoding.len();
                }
                prompt_tokens.push(encoding);
            }

            encodings.push(EncodedPrompt {
                encoding: prompt_tokens,
                text_offset: total_entity_tokens + 1,
            });
            max_tokens = max_tokens.max(total_tokens);
        }

        let batch_size = encodings.len();
        let mut input_ids = Array2::<i64>::zeros((batch_size, max_tokens));
        let mut attention_masks = Array2::<i64>::zeros((batch_size, max_tokens));
        let mut word_masks = Array2::<i64>::zeros((batch_size, max_tokens));

        for (row, encoded_prompt) in encodings.into_iter().enumerate() {
            let mut index = 0usize;
            let mut word_id = 0i64;

            input_ids[[row, index]] = 1;
            attention_masks[[row, index]] = 1;
            index += 1;

            for word in encoded_prompt.encoding {
                for (token_index, token) in word.iter().enumerate() {
                    input_ids[[row, index]] = i64::from(*token);
                    attention_masks[[row, index]] = 1;
                    if index >= encoded_prompt.text_offset && token_index == 0 {
                        word_masks[[row, index]] = word_id;
                    }
                    index += 1;
                }
                if index >= encoded_prompt.text_offset {
                    word_id += 1;
                }
            }

            input_ids[[row, index]] = 2;
            attention_masks[[row, index]] = 1;
        }

        let mut text_lengths = Array2::<i64>::zeros((input.text_lengths.len(), 1));
        for (row, text_length) in input.text_lengths.into_iter().enumerate() {
            text_lengths[[row, 0]] = text_length as i64;
        }

        Ok(Self {
            texts: input.texts,
            tokens: input.tokens,
            entities: input.entities,
            num_words: input.num_words,
            input_ids,
            attention_masks,
            word_masks,
            text_lengths,
        })
    }
}
