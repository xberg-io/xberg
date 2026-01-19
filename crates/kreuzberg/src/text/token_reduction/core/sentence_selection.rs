use super::analysis::TextAnalyzer;

/// Handles sentence selection and filtering based on importance scoring.
pub struct SentenceSelector;

impl SentenceSelector {
    /// Applies sentence selection to keep only the most important sentences.
    pub fn apply_sentence_selection(text: &str) -> String {
        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.len() <= 2 {
            return text.to_string();
        }

        let mut scored_sentences: Vec<(usize, f32, &str)> = sentences
            .iter()
            .enumerate()
            .map(|(i, sentence)| {
                let score = TextAnalyzer::score_sentence_importance(sentence, i, sentences.len());
                (i, score, *sentence)
            })
            .collect();

        scored_sentences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let keep_count = ((sentences.len() as f32 * 0.4).ceil() as usize).max(1);
        let mut selected_indices: Vec<usize> = scored_sentences[..keep_count].iter().map(|(i, _, _)| *i).collect();

        selected_indices.sort();

        let selected_sentences: Vec<&str> = selected_indices
            .iter()
            .filter_map(|&i| sentences.get(i))
            .copied()
            .collect();

        if selected_sentences.is_empty() {
            text.to_string()
        } else {
            selected_sentences.join(". ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_selection() {
        let input = "First sentence here. Second sentence with more words. Third one. Fourth sentence is even longer than the others.";
        let result = SentenceSelector::apply_sentence_selection(input);

        assert!(result.len() < input.len());
        assert!(result.split(". ").count() < 4);
    }

    #[test]
    fn test_sentence_selection_short_text() {
        let input = "Only one sentence.";
        let result = SentenceSelector::apply_sentence_selection(input);
        assert_eq!(result, input);
    }
}
