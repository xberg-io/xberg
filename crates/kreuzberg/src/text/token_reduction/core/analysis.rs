use ahash::AHashMap;

/// Bonus added for sentences at the beginning or end of the document
const SENTENCE_EDGE_POSITION_BONUS: f32 = 0.3;

/// Bonus added for sentences with ideal word count (neither too short nor too long)
const IDEAL_WORD_COUNT_BONUS: f32 = 0.2;

/// Minimum word count for ideal sentence length
const MIN_IDEAL_WORD_COUNT: usize = 3;

/// Maximum word count for ideal sentence length
const MAX_IDEAL_WORD_COUNT: usize = 25;

/// Weight multiplier for numeric content density in sentences
const NUMERIC_CONTENT_WEIGHT: f32 = 0.3;

/// Weight multiplier for capitalized/acronym word density in sentences
const CAPS_ACRONYM_WEIGHT: f32 = 0.25;

/// Weight multiplier for long word density in sentences
const LONG_WORD_WEIGHT: f32 = 0.2;

/// Minimum character length for a word to be considered "long"
const LONG_WORD_THRESHOLD: usize = 8;

/// Weight multiplier for punctuation density in sentences
const PUNCTUATION_DENSITY_WEIGHT: f32 = 0.15;

/// Weight multiplier for word diversity ratio (unique words / total words)
const DIVERSITY_RATIO_WEIGHT: f32 = 0.15;

/// Weight multiplier for character entropy (measure of text randomness/information)
const CHAR_ENTROPY_WEIGHT: f32 = 0.1;

/// Analyzes text characteristics and scores content importance.
pub struct TextAnalyzer;

impl TextAnalyzer {
    /// Scores the importance of a sentence based on various characteristics.
    pub fn score_sentence_importance(sentence: &str, position: usize, total_sentences: usize) -> f32 {
        let mut score = 0.0;

        if position == 0 || position == total_sentences - 1 {
            score += SENTENCE_EDGE_POSITION_BONUS;
        }

        let words: Vec<&str> = sentence.split_whitespace().collect();
        if words.is_empty() {
            return score;
        }

        let word_count = words.len();
        if (MIN_IDEAL_WORD_COUNT..=MAX_IDEAL_WORD_COUNT).contains(&word_count) {
            score += IDEAL_WORD_COUNT_BONUS;
        }

        let mut numeric_count = 0;
        let mut caps_count = 0;
        let mut long_word_count = 0;
        let mut punct_density = 0;

        for word in &words {
            if word.chars().any(|c| c.is_numeric()) {
                numeric_count += 1;
            }

            if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
                caps_count += 1;
            }

            if word.len() > LONG_WORD_THRESHOLD {
                long_word_count += 1;
            }

            punct_density += word.chars().filter(|c| c.is_ascii_punctuation()).count();
        }

        score += (numeric_count as f32 / words.len() as f32) * NUMERIC_CONTENT_WEIGHT;
        score += (caps_count as f32 / words.len() as f32) * CAPS_ACRONYM_WEIGHT;
        score += (long_word_count as f32 / words.len() as f32) * LONG_WORD_WEIGHT;
        score += (punct_density as f32 / sentence.len() as f32) * PUNCTUATION_DENSITY_WEIGHT;

        let estimated_unique = (words.len() as f32 * 0.6).ceil() as usize;
        let mut unique_words: ahash::AHashSet<String> = ahash::AHashSet::with_capacity(estimated_unique.max(10));

        for w in &words {
            let clean = w
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            unique_words.insert(clean);

            if unique_words.len() >= estimated_unique {
                break;
            }
        }

        let final_unique_count = if unique_words.len() >= estimated_unique {
            unique_words.len()
        } else {
            for w in &words {
                let clean = w
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .collect::<String>()
                    .to_lowercase();
                unique_words.insert(clean);
            }
            unique_words.len()
        };

        let diversity_ratio = final_unique_count as f32 / words.len() as f32;
        score += diversity_ratio * DIVERSITY_RATIO_WEIGHT;

        let char_entropy = Self::calculate_char_entropy(sentence);
        score += char_entropy * CHAR_ENTROPY_WEIGHT;

        score
    }

    /// Calculates character entropy (measure of text randomness/information content).
    pub fn calculate_char_entropy(text: &str) -> f32 {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return 0.0;
        }

        let estimated_unique = (chars.len() as f32 * 0.1).ceil() as usize;
        let mut char_freq = AHashMap::with_capacity(estimated_unique.max(26));

        for &ch in &chars {
            let lowercase_ch = ch
                .to_lowercase()
                .next()
                .expect("to_lowercase() must yield at least one character for valid Unicode");
            *char_freq.entry(lowercase_ch).or_insert(0) += 1;
        }

        let total_chars = chars.len() as f32;
        char_freq
            .values()
            .map(|&freq| {
                let p = freq as f32 / total_chars;
                if p > 0.0 { -p * p.log2() } else { 0.0 }
            })
            .sum::<f32>()
            .min(5.0)
    }

    /// Checks if a word has important characteristics that should be preserved.
    pub fn has_important_characteristics(word: &str) -> bool {
        if word.len() > 1 && word.chars().all(|c| c.is_uppercase()) {
            return true;
        }

        if word.chars().any(|c| c.is_numeric()) {
            return true;
        }

        if word.len() > 10 {
            return true;
        }

        let uppercase_count = word.chars().filter(|c| c.is_uppercase()).count();
        if uppercase_count > 1 && uppercase_count < word.len() {
            return true;
        }

        if Self::has_cjk_importance(word) {
            return true;
        }

        false
    }

    /// Checks if a CJK word has important characteristics.
    pub fn has_cjk_importance(word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();

        let has_cjk = chars.iter().any(|&c| c as u32 >= 0x4E00 && (c as u32) <= 0x9FFF);
        if !has_cjk {
            return false;
        }

        let important_radicals = [
            '学', '智', '能', '技', '术', '法', '算', '理', '科', '研', '究', '发', '展', '系', '统', '模', '型', '方',
            '式', '过', '程', '结', '构', '功', '效', '应', '分', '析', '计', '算', '数', '据', '信', '息', '处', '理',
            '语', '言', '文', '生', '成', '产', '用', '作', '为', '成', '变', '化', '转', '换', '提', '高', '网', '络',
            '神', '经', '机', '器', '人', '工', '智', '能', '自', '然', '复',
        ];

        for &char in &chars {
            if important_radicals.contains(&char) {
                return true;
            }
        }

        if chars.len() == 2 && has_cjk {
            let has_technical = chars.iter().any(|&c| {
                let code = c as u32;
                (0x4E00..=0x4FFF).contains(&code)
                    || (0x5000..=0x51FF).contains(&code)
                    || (0x6700..=0x68FF).contains(&code)
                    || (0x7500..=0x76FF).contains(&code)
            });

            if has_technical {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_char_entropy() {
        let low_entropy = TextAnalyzer::calculate_char_entropy("aaaaaaa");
        assert!(low_entropy < 1.0);

        let high_entropy = TextAnalyzer::calculate_char_entropy("abcdefg123");
        assert!(high_entropy > low_entropy);
    }

    #[test]
    fn test_important_word_characteristics() {
        assert!(TextAnalyzer::has_important_characteristics("IMPORTANT"));
        assert!(TextAnalyzer::has_important_characteristics("COVID-19"));
        assert!(TextAnalyzer::has_important_characteristics("PyTorch"));
        assert!(TextAnalyzer::has_important_characteristics("verylongword123"));
    }
}
