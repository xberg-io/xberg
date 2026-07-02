//! Heading classification utilities for PDF layout detection.

/// Check if text is a bare URL rather than a real heading.
///
/// Some documents render URLs in a larger typeface than the surrounding body
/// text, which the font-size clustering then promotes to a heading. Bare URLs
/// are semantically body text, never section headings.
pub(in crate::pdf::structure) fn looks_like_bare_url(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www."))
        && !t.contains(char::is_whitespace)
}

/// Check if text looks like a figure/diagram label rather than a real heading.
///
/// Catches concatenated figure labels (e.g., "Tightened nut Flexloc nut
/// Fiber locknut Elastic stop nut") and pure single-letter sequences ("A B C").
pub(in crate::pdf::structure) fn looks_like_figure_label(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();

    // All single-character words (3+): "A B C", "D E F"
    if words.len() >= 3 && words.iter().all(|w| w.len() <= 1) {
        return true;
    }

    // Concatenated labels: same word appears 3+ times (e.g., "nut" in figure parts)
    if words.len() >= 5 {
        for w in &words {
            let lw = w.to_lowercase();
            if words.iter().filter(|x| x.to_lowercase() == lw).count() >= 3 {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_letter_sequence() {
        assert!(looks_like_figure_label("A B C"));
        assert!(looks_like_figure_label("D E F G"));
    }

    #[test]
    fn test_repeated_word() {
        assert!(looks_like_figure_label(
            "Tightened nut Flexloc nut Fiber locknut Elastic stop nut"
        ));
    }

    #[test]
    fn test_normal_heading() {
        assert!(!looks_like_figure_label("Introduction"));
        assert!(!looks_like_figure_label("3.1 PDF backends"));
        assert!(!looks_like_figure_label("Abstract"));
    }

    #[test]
    fn test_bare_url_detected() {
        use super::looks_like_bare_url;
        assert!(looks_like_bare_url("https://dell-research-harvard.github.io/HJDataset/"));
        assert!(looks_like_bare_url("http://example.com/page"));
        assert!(looks_like_bare_url("www.example.org"));
    }

    #[test]
    fn test_url_with_prose_is_not_bare_url() {
        use super::looks_like_bare_url;
        assert!(!looks_like_bare_url("https://example.com is the project homepage"));
        assert!(!looks_like_bare_url("Introduction"));
        assert!(!looks_like_bare_url("3.1 PDF backends"));
    }
}
