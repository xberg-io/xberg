use std::collections::BTreeMap;

use super::{
    Merged, ModelSet, OcrError, Outcome, Pass, RetryStage, TsvWord, VERTICAL_JAPANESE_MODEL, VERTICAL_PSM, mean_conf,
    read_retry_words, script_letter,
};

pub(super) fn read(
    image: &xberg_tesseract::Pix,
    pass: &Pass<'_>,
    models: &ModelSet,
    first: &[TsvWord],
) -> Result<Option<Outcome>, OcrError> {
    if !first_read_candidate(first, pass.config.psm) {
        return Ok(None);
    }
    let Some(words) = read_retry_words(image, pass, models, RetryStage::Han, VERTICAL_PSM)? else {
        return Ok(None);
    };
    Ok(accept(words).and_then(|mut merged| {
        preserve_readable_lines(first, &mut merged).then_some(Outcome {
            script: "Japanese".into(),
            model: VERTICAL_JAPANESE_MODEL,
            merged,
            khmer_supplement: None,
        })
    }))
}

pub(super) fn first_read_candidate(first: &[TsvWord], psm: u8) -> bool {
    if psm == 11 {
        return sparse_glyph_noise(first);
    }
    has_noisy_columns(first)
}

// Sparse segmentation splits vertical columns into mostly square, isolated ink clusters. ~keep
fn sparse_glyph_noise(words: &[TsvWord]) -> bool {
    if words.len() < 30 {
        return false;
    }
    let readable = words.iter().filter(|word| readable(word)).count();
    if readable * 10 >= words.len() {
        return false;
    }
    let near_square = words
        .iter()
        .filter(|word| word.height > 0 && 3 * word.height <= 5 * word.width && 5 * word.width <= 9 * word.height)
        .count();
    near_square * 4 >= words.len() * 3
}

fn has_noisy_columns(first: &[TsvWord]) -> bool {
    let mut groups = BTreeMap::new();
    for word in first {
        groups.entry(word.line_key).or_insert_with(Vec::new).push(word);
    }
    let mut tall = 0;
    for line in groups.values() {
        let left = line.iter().map(|word| word.left).min().unwrap_or(0);
        let right = line.iter().map(|word| word.right()).max().unwrap_or(0);
        let top = line.iter().map(|word| word.top).min().unwrap_or(0);
        let bottom = line.iter().map(|word| word.bottom()).max().unwrap_or(0);
        if right > left && bottom - top >= 2 * (right - left) {
            tall += 1;
        } else if line
            .iter()
            .filter(|word| word.conf >= 80.0 && word.text.chars().all(char::is_alphanumeric))
            .count()
            >= 3
        {
            return false;
        }
    }
    tall >= 6 && tall * 2 > groups.len()
}

fn preserve_readable_lines(first: &[TsvWord], merged: &mut Merged) -> bool {
    let mut protected: Vec<_> = super::lines_of(first.to_vec())
        .into_iter()
        .filter(|line| {
            line.right - line.left >= 2 * (line.bottom - line.top)
                && line.words.iter().filter(|word| readable(word)).count() >= 3
        })
        .collect();
    if protected.is_empty() {
        return true;
    }
    // A retry can hallucinate vertical glyphs on a readable horizontal line. ~keep
    for line in &mut merged.lines {
        line.retain(|word| {
            !protected.iter().any(|primary| {
                primary.left < word.right()
                    && primary.right > word.left
                    && 2 * super::y_overlap(primary.top, primary.bottom, word) >= word.height
            })
        });
    }
    merged.lines.retain(|line| !line.is_empty());
    let Some(top) = merged.words().map(|word| word.top).min() else {
        return false;
    };
    let bottom = merged.words().map(TsvWord::bottom).max().unwrap_or(top);
    let next_block = merged.words().map(|word| word.line_key.0).max().unwrap_or(0) + 1;
    let mut before = Vec::new();
    let mut after = Vec::new();
    for line in &mut protected {
        let target = if line.bottom <= top {
            &mut before
        } else if line.top >= bottom {
            &mut after
        } else {
            return false;
        };
        for word in &mut line.words {
            word.line_key.0 += next_block;
        }
        target.push(std::mem::take(&mut line.words));
    }
    before.append(&mut merged.lines);
    before.append(&mut after);
    merged.lines = before;
    true
}

fn readable(word: &TsvWord) -> bool {
    word.conf >= 80.0 && word.text.chars().count() >= 3 && word.text.chars().all(char::is_alphanumeric)
}

fn kana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}' | '\u{30a1}'..='\u{30fa}')
}

fn accept(words: Vec<TsvWord>) -> Option<Merged> {
    let mut groups = BTreeMap::new();
    for word in words {
        groups.entry(word.line_key).or_insert_with(Vec::new).push(word);
    }
    // The level-5 TSV keys preserve Tesseract's right-to-left column order. ~keep
    let lines: Vec<_> = groups.into_values().collect();
    let mut supported = 0;
    let mut japanese = 0;
    let mut letters = 0;
    for line in &lines {
        let chars = line.iter().flat_map(|word| word.text.chars());
        let count = chars.clone().filter(|c| kana(*c) || script_letter("Han", *c)).count();
        let kana_count = chars.clone().filter(|c| kana(*c)).count();
        let alphabetic = chars.filter(|c| c.is_alphabetic()).count();
        if count >= 6 && kana_count >= 2 && count * 2 > alphabetic && mean_conf(line) >= 40.0 {
            supported += 1;
        }
        japanese += count;
        letters += alphabetic;
    }
    (supported >= 3 && japanese * 4 >= letters * 3).then_some(Merged {
        admitted: lines.len(),
        lines,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(line: u32, text: &str, conf: f64) -> TsvWord {
        TsvWord {
            line_key: (1, 1, line),
            left: 100 - i64::from(line) * 10,
            top: 0,
            width: 8,
            height: 40,
            conf,
            text: text.into(),
        }
    }

    #[test]
    fn requires_a_majority_of_vertical_lines_and_protects_readable_horizontal_text() {
        let mut lines: Vec<_> = (1..=6).map(|line| word(line, "t?", 20.0)).collect();
        assert!(has_noisy_columns(&lines));
        assert!(!has_noisy_columns(&lines[..5]));
        for (index, text) in ["Payment", "received", "today"].into_iter().enumerate() {
            let mut readable = word(7, text, 90.0);
            readable.left = index as i64 * 40;
            readable.width = 36;
            readable.height = 12;
            lines.push(readable);
        }
        assert!(!has_noisy_columns(&lines));
    }

    #[test]
    fn sparse_gate_rejects_readable_words_and_wide_boxes() {
        let mut words: Vec<_> = (1..=30).map(|line| word(line, "t?", 20.0)).collect();
        for word in &mut words {
            word.height = word.width;
        }
        assert!(first_read_candidate(&words, 11));
        for word in &mut words[..3] {
            word.text = "English".into();
            word.conf = 90.0;
        }
        assert!(!first_read_candidate(&words, 11));
        for word in &mut words {
            word.width *= 3;
            word.conf = 20.0;
        }
        assert!(!first_read_candidate(&words, 11));
    }

    #[test]
    fn requires_confident_kana_and_preserves_engine_column_order() {
        let page = |text, confidence| (1..=3).map(|line| word(line, text, confidence)).collect();
        assert!(accept(page("中國產業技術", 95.0)).is_none());
        assert!(accept(page("流産における出血", 30.0)).is_none());
        let mut words = page("流産における出血", 80.0);
        words[0].text = "はじめに流産".into();
        words[2].text = "おわりに出血".into();
        let merged = accept(words).unwrap();
        assert_eq!(merged.text(), "はじめに流産\n流産における出血\nおわりに出血");
    }
}
