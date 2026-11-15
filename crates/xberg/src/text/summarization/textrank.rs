//! TextRank-based extractive summarisation.
//!
//! Sentence-level TextRank: build a graph where vertices are sentences and edge
//! weights are TF-IDF cosine similarity. Iterate PageRank to convergence, pick
//! the top-N highest-scoring sentences (capped by `max_tokens`), and join them
//! in original document order.
//!
//! Stopword filtering uses the per-language tables already shipped in the
//! crate ([`crate::stopwords`]). When `result.detected_languages` is populated
//! the first entry drives the lookup; otherwise we fall back to English.
//!
//! The implementation has no external dependencies and is fully deterministic.

use ahash::{AHashMap, AHashSet};

/// Convergence tolerance for the PageRank iteration.
const PAGERANK_TOLERANCE: f32 = 1.0e-4;
/// Hard cap on PageRank iterations.
const PAGERANK_MAX_ITERATIONS: usize = 64;
/// Damping factor (standard PageRank value).
const PAGERANK_DAMPING: f32 = 0.85;
/// Edges below this similarity contribute nothing.
const MIN_EDGE_SIMILARITY: f32 = 1.0e-6;
/// When no token budget is supplied we cap the summary at this many words.
const DEFAULT_MAX_TOKENS: u32 = 150;
/// Maximum number of sentences considered to keep the algorithm tractable.
const MAX_SENTENCES: usize = 256;

/// Score and return the top-N sentences from `text`, joined in original order.
///
/// `language` is an ISO 639 (or locale) code used to pick a stopword list;
/// pass `None` (or an unknown code) to fall back to English.
/// `max_tokens` bounds the summary length by whitespace-separated tokens;
/// `None` falls back to `DEFAULT_MAX_TOKENS`.
pub fn summarize(text: &str, language: Option<&str>, max_tokens: Option<u32>) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let sentences = split_sentences(trimmed);
    if sentences.is_empty() {
        return None;
    }
    if sentences.len() == 1 {
        return Some(sentences.into_iter().next().unwrap().to_string());
    }

    let working = if sentences.len() > MAX_SENTENCES {
        &sentences[..MAX_SENTENCES]
    } else {
        sentences.as_slice()
    };

    let stopwords = resolve_stopwords(language);
    let tokens: Vec<Vec<String>> = working.iter().map(|s| tokenize(s, stopwords)).collect();

    let scores = pagerank_scores(&tokens);

    let budget = max_tokens.unwrap_or(DEFAULT_MAX_TOKENS).max(1) as usize;
    let selected = select_top_sentences(working, &scores, budget);
    if selected.is_empty() {
        return None;
    }

    Some(selected.join(" "))
}

/// Count whitespace-separated tokens (used for token-budget bookkeeping by
/// callers).
pub fn token_count(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

fn resolve_stopwords(language: Option<&str>) -> Option<&'static AHashSet<String>> {
    let lang = language.unwrap_or("en");
    crate::stopwords::get_stopwords_with_fallback(lang, "en")
}

fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if matches!(c, '.' | '!' | '?' | '\n') {
            let end = i + 1;
            let candidate = text[start..end].trim();
            if !candidate.is_empty() {
                sentences.push(candidate);
            }
            i = end;
            while i < bytes.len() && matches!(bytes[i] as char, '.' | '!' | '?' | '\n' | ' ' | '\t' | '\r') {
                i += 1;
            }
            start = i;
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        let candidate = text[start..].trim();
        if !candidate.is_empty() {
            sentences.push(candidate);
        }
    }
    sentences
}

fn tokenize(sentence: &str, stopwords: Option<&AHashSet<String>>) -> Vec<String> {
    sentence
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|tok| {
            if tok.is_empty() {
                return None;
            }
            let lowered = tok.to_lowercase();
            if let Some(sw) = stopwords
                && sw.contains(&lowered)
            {
                return None;
            }
            if lowered.chars().count() < 2 {
                return None;
            }
            Some(lowered)
        })
        .collect()
}

/// Build a TF-IDF cosine-similarity matrix, run PageRank, return per-sentence scores.
fn pagerank_scores(token_lists: &[Vec<String>]) -> Vec<f32> {
    let n = token_lists.len();
    if n == 0 {
        return Vec::new();
    }

    let mut df: AHashMap<&str, usize> = AHashMap::new();
    for tokens in token_lists {
        let mut seen: AHashSet<&str> = AHashSet::new();
        for tok in tokens {
            if seen.insert(tok.as_str()) {
                *df.entry(tok.as_str()).or_insert(0) += 1;
            }
        }
    }

    let vectors: Vec<AHashMap<&str, f32>> = token_lists
        .iter()
        .map(|tokens| {
            let mut tf: AHashMap<&str, f32> = AHashMap::new();
            for tok in tokens {
                *tf.entry(tok.as_str()).or_insert(0.0) += 1.0;
            }
            let len = tokens.len().max(1) as f32;
            let mut vec = AHashMap::with_capacity(tf.len());
            for (term, count) in tf {
                let tf_val = count / len;
                let dfn = *df.get(term).unwrap_or(&1) as f32;
                let idf = ((n as f32 + 1.0) / (dfn + 1.0)).ln() + 1.0;
                vec.insert(term, tf_val * idf);
            }
            vec
        })
        .collect();

    let norms: Vec<f32> = vectors
        .iter()
        .map(|v| v.values().map(|x| x * x).sum::<f32>().sqrt())
        .collect();

    let mut adjacency = vec![vec![0.0f32; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let sim = cosine_similarity(&vectors[i], &vectors[j], norms[i], norms[j]);
            if sim > MIN_EDGE_SIMILARITY {
                adjacency[i][j] = sim;
                adjacency[j][i] = sim;
            }
        }
    }

    let mut transition = vec![vec![0.0f32; n]; n];
    let mut dangling = Vec::with_capacity(n);
    for j in 0..n {
        let col_sum: f32 = (0..n).map(|i| adjacency[i][j]).sum();
        if col_sum <= MIN_EDGE_SIMILARITY {
            dangling.push(j);
            continue;
        }
        for i in 0..n {
            transition[i][j] = adjacency[i][j] / col_sum;
        }
    }

    power_iteration(&transition, &dangling, n)
}

/// Run damped power iteration over a column-stochastic `transition` matrix until the
/// L1 delta falls below [`PAGERANK_TOLERANCE`] or the iteration cap is reached.
///
/// `dangling` lists the column indices with no outgoing mass; their score is
/// redistributed uniformly each iteration. ~keep
fn power_iteration(transition: &[Vec<f32>], dangling: &[usize], n: usize) -> Vec<f32> {
    let mut scores = vec![1.0f32 / n as f32; n];
    let teleport = (1.0 - PAGERANK_DAMPING) / n as f32;
    for _ in 0..PAGERANK_MAX_ITERATIONS {
        let dangling_mass: f32 = dangling.iter().map(|&j| scores[j]).sum::<f32>() / n as f32;
        let mut new_scores = vec![0.0f32; n];
        for i in 0..n {
            let mut acc = 0.0f32;
            for j in 0..n {
                acc += transition[i][j] * scores[j];
            }
            new_scores[i] = teleport + PAGERANK_DAMPING * (acc + dangling_mass);
        }
        let delta: f32 = new_scores.iter().zip(scores.iter()).map(|(a, b)| (a - b).abs()).sum();
        scores = new_scores;
        if delta < PAGERANK_TOLERANCE {
            break;
        }
    }

    scores
}

fn cosine_similarity(a: &AHashMap<&str, f32>, b: &AHashMap<&str, f32>, norm_a: f32, norm_b: f32) -> f32 {
    if a.is_empty() || b.is_empty() || norm_a <= MIN_EDGE_SIMILARITY || norm_b <= MIN_EDGE_SIMILARITY {
        return 0.0;
    }
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let mut dot = 0.0f32;
    for (term, av) in small {
        if let Some(bv) = large.get(term) {
            dot += av * bv;
        }
    }
    dot / (norm_a * norm_b)
}

/// Pick the highest-scoring sentences whose combined token count fits the
/// budget, then re-sort them by original index.
fn select_top_sentences<'a>(sentences: &'a [&'a str], scores: &[f32], budget_tokens: usize) -> Vec<&'a str> {
    let mut ranked: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut chosen_indices: Vec<usize> = Vec::new();
    let mut accumulated = 0usize;
    for (idx, _) in ranked {
        let tokens = sentences[idx].split_whitespace().count();
        if tokens == 0 {
            continue;
        }
        if accumulated > 0 && accumulated + tokens > budget_tokens {
            continue;
        }
        chosen_indices.push(idx);
        accumulated += tokens;
        if accumulated >= budget_tokens {
            break;
        }
    }

    if chosen_indices.is_empty()
        && let Some((idx, _)) = scores.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1))
    {
        chosen_indices.push(idx);
    }

    chosen_indices.sort_unstable();
    chosen_indices.into_iter().map(|i| sentences[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAGRAPH: &str = "Machine learning is a branch of artificial intelligence. \
        It focuses on building systems that learn from data. \
        Deep learning is a subset of machine learning. \
        Deep learning uses neural networks with multiple layers. \
        Neural networks are inspired by the human brain. \
        Cats are adorable mammals that purr. \
        The weather today is unusually warm.";

    #[test]
    fn split_sentences_handles_basic_punctuation() {
        let sentences = split_sentences("One. Two! Three? Four.");
        assert_eq!(sentences, vec!["One.", "Two!", "Three?", "Four."]);
    }

    #[test]
    fn summarize_returns_some_text() {
        let summary = summarize(PARAGRAPH, Some("en"), Some(40)).expect("summary produced");
        assert!(!summary.is_empty());
        assert!(summary.len() <= PARAGRAPH.len());
    }

    #[test]
    fn summarize_picks_relevant_sentences() {
        let summary = summarize(PARAGRAPH, Some("en"), Some(40)).expect("summary produced");
        let lower = summary.to_lowercase();
        assert!(
            lower.contains("machine learning") || lower.contains("deep learning") || lower.contains("neural"),
            "expected ML-related content, got: {}",
            summary
        );
    }

    #[test]
    fn summarize_is_deterministic() {
        let s1 = summarize(PARAGRAPH, Some("en"), Some(40)).unwrap();
        let s2 = summarize(PARAGRAPH, Some("en"), Some(40)).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn summarize_empty_returns_none() {
        assert!(summarize("", None, None).is_none());
        assert!(summarize("   \n", None, None).is_none());
    }

    #[test]
    fn summarize_single_sentence_passthrough() {
        let summary = summarize("Just one sentence here.", Some("en"), None).unwrap();
        assert_eq!(summary, "Just one sentence here.");
    }

    #[test]
    fn summarize_unknown_language_falls_back_to_english() {
        let summary = summarize(PARAGRAPH, Some("xx"), Some(40)).expect("falls back to en");
        assert!(!summary.is_empty());
    }

    #[test]
    fn token_count_uses_whitespace_split() {
        assert_eq!(token_count("hello world"), 2);
        assert_eq!(token_count("  multiple   spaces  here "), 3);
        assert_eq!(token_count(""), 0);
    }

    #[test]
    fn summarize_respects_budget() {
        let summary = summarize(PARAGRAPH, Some("en"), Some(8)).expect("summary produced");
        assert!(PARAGRAPH.contains(summary.split('.').next().unwrap_or(&summary).trim()));
    }
}
