//! Rejoin words that typesetting split across a line break with a hyphen.
//!
//! The structured (markdown) path dehyphenates paragraph lines in
//! `pdf::structure::pipeline`; the plain page text is assembled from spans without
//! paragraphs, so it needs its own pass over the assembled text. The break may be a
//! single newline or a blank line: double-spaced manuscripts and math-heavy pages put
//! every body line farther apart than the paragraph-gap threshold.

use crate::pdf::structure::pipeline::{HyphenWitnesses, is_word_hyphen, should_preserve_lexical_hyphen};

/// The word hyphens the markdown path dehyphenates, plus U+00AD SOFT HYPHEN, which only
/// ever marks a discretionary break. Dashes and the math minus are excluded.
fn is_line_wrap_hyphen(character: char) -> bool {
    is_word_hyphen(character) || character == '\u{00ad}'
}

/// TeX and most hyphenation dictionaries leave at least two letters before a break, so a
/// single letter before a line-final hyphen is a label or an initial, not a word stem.
const MIN_WRAPPED_STEM_LETTERS: usize = 2;

/// Continuations that mark a suspended hyphen (`pre- and post-processing`), where the
/// hyphen belongs to the word before the break and must not be merged into the next one.
const SUSPENDED_HYPHEN_CONTINUATIONS: [&str; 3] = ["and", "or", "nor"];

/// Rejoin `<word>-` + line break + `<lowercase word>` into one word.
///
/// The hyphen is dropped when the stem ends in a lowercase letter, unless the pair is a
/// known compound or the same text writes it hyphenated mid-line; an uppercase stem
/// (`COVID-` / `related`) keeps its hyphen. A kept hyphen is written as ASCII `-`, as the
/// markdown path writes it. Either way the line break goes.
pub(super) fn rejoin_line_wrap_hyphens(text: String) -> String {
    if !text.contains(is_line_wrap_hyphen) {
        return text;
    }
    let mut witnesses: Option<HyphenWitnesses> = None;
    let mut joined = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(break_at) = rest.find('\n') {
        let (line, after_line) = rest.split_at(break_at);
        let continuation = after_line.trim_start_matches(['\n', '\r', ' ', '\t']);
        match wrapped_word(line.trim_end_matches([' ', '\t', '\r']), continuation) {
            Some(wrap) => {
                let witnesses = witnesses.get_or_insert_with(|| collect_mid_line_hyphen_pairs(&text));
                joined.push_str(wrap.stem);
                if wrap.keeps_hyphen(witnesses) {
                    joined.push('-');
                }
            }
            None => joined.push_str(&rest[..rest.len() - continuation.len()]),
        }
        rest = continuation;
    }
    joined.push_str(rest);
    joined
}

/// A line-final hyphenation: the line without its hyphen and the two word halves.
struct WrappedWord<'a> {
    stem: &'a str,
    left: &'a str,
    right: &'a str,
}

impl WrappedWord<'_> {
    fn keeps_hyphen(&self, witnesses: &HyphenWitnesses) -> bool {
        !self.left.chars().next_back().is_some_and(char::is_lowercase)
            || should_preserve_lexical_hyphen(self.left, self.right, witnesses)
    }
}

/// The hyphenation ending `line` and continuing into `continuation`, or `None` when the
/// break is not a line-wrap hyphenation.
fn wrapped_word<'a>(line: &'a str, continuation: &'a str) -> Option<WrappedWord<'a>> {
    let hyphen = line.chars().next_back().filter(|&c| is_line_wrap_hyphen(c))?;
    let stem = &line[..line.len() - hyphen.len_utf8()];
    let left = trailing_letters(stem);
    if left.chars().count() < MIN_WRAPPED_STEM_LETTERS {
        return None;
    }
    let right = leading_letters(continuation);
    if !right.chars().next().is_some_and(char::is_lowercase) || SUSPENDED_HYPHEN_CONTINUATIONS.contains(&right) {
        return None;
    }
    Some(WrappedWord { stem, left, right })
}

/// Lowercased `(left, right)` letter runs the text itself joins with a hyphen inside a
/// line, the evidence that a hyphen at a break is authored rather than discretionary.
fn collect_mid_line_hyphen_pairs(text: &str) -> HyphenWitnesses {
    let mut witnesses = HyphenWitnesses::default();
    for word in text.split_whitespace() {
        let mut parts = word.split(is_word_hyphen);
        let Some(mut left) = parts.next() else {
            continue;
        };
        for right in parts {
            let left_run = trailing_letters(left);
            let right_run = leading_letters(right);
            if left_run.chars().count() >= MIN_WRAPPED_STEM_LETTERS
                && right_run.chars().count() >= MIN_WRAPPED_STEM_LETTERS
            {
                witnesses.insert((left_run.to_ascii_lowercase(), right_run.to_ascii_lowercase()));
            }
            left = right;
        }
    }
    witnesses
}

fn trailing_letters(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphabetic())
        .last()
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

fn leading_letters(text: &str) -> &str {
    let end = text
        .char_indices()
        .find(|(_, c)| !c.is_alphabetic())
        .map_or(text.len(), |(index, _)| index);
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::rejoin_line_wrap_hyphens;

    fn rejoin(text: &str) -> String {
        rejoin_line_wrap_hyphens(text.to_string())
    }

    #[test]
    fn single_newline_wrap_is_rejoined() {
        assert_eq!(
            rejoin("effects of small molecule per-\nturbagens. In this work"),
            "effects of small molecule perturbagens. In this work"
        );
    }

    #[test]
    fn blank_line_wrap_of_a_double_spaced_page_is_rejoined() {
        assert_eq!(
            rejoin("Jacobi matrix for Jacobi poly-\n\nnomials. The operation"),
            "Jacobi matrix for Jacobi polynomials. The operation"
        );
    }

    #[test]
    fn trailing_spaces_around_the_break_are_absorbed() {
        assert_eq!(rejoin("the like- \n  lihood"), "the likelihood");
    }

    #[test]
    fn soft_hyphen_and_unicode_hyphen_wraps_are_rejoined() {
        assert_eq!(rejoin("approxi\u{00ad}\nmately"), "approximately");
        assert_eq!(rejoin("approxi\u{2010}\nmately"), "approximately");
    }

    #[test]
    fn uppercase_continuation_keeps_the_break() {
        assert_eq!(rejoin("co-\nWorker"), "co-\nWorker");
    }

    #[test]
    fn suspended_hyphen_before_a_conjunction_keeps_the_break() {
        assert_eq!(
            rejoin("first-\nand second-order terms"),
            "first-\nand second-order terms"
        );
        assert_eq!(rejoin("pre-\nor post-processing"), "pre-\nor post-processing");
    }

    #[test]
    fn single_letter_stem_keeps_the_break() {
        assert_eq!(rejoin("item a-\nlabel"), "item a-\nlabel");
    }

    #[test]
    fn dash_minus_and_digit_ends_keep_the_break() {
        assert_eq!(rejoin("value \u{2212}\nx"), "value \u{2212}\nx");
        assert_eq!(rejoin("pages 12-\nfourteen"), "pages 12-\nfourteen");
        assert_eq!(rejoin("a pause \u{2014}\nthen"), "a pause \u{2014}\nthen");
    }

    #[test]
    fn known_compound_keeps_its_hyphen_but_loses_the_break() {
        assert_eq!(rejoin("a well-\nknown result"), "a well-known result");
    }

    #[test]
    fn compound_written_mid_line_elsewhere_keeps_its_hyphen() {
        assert_eq!(
            rejoin("a price-determining rule\nand a price-\ndetermining step"),
            "a price-determining rule\nand a price-determining step"
        );
    }

    #[test]
    fn non_breaking_hyphen_wrap_is_rejoined() {
        assert_eq!(rejoin("random access mem\u{2011}\nory"), "random access memory");
    }

    #[test]
    fn unicode_hyphen_compound_written_mid_line_keeps_an_ascii_hyphen() {
        assert_eq!(
            rejoin("a cross\u{2011}account check\nand a cross\u{2010}\naccount report"),
            "a cross\u{2011}account check\nand a cross-account report"
        );
    }

    #[test]
    fn uppercase_stem_keeps_its_hyphen() {
        assert_eq!(rejoin("COVID-\nrelated cases"), "COVID-related cases");
    }

    #[test]
    fn non_ascii_letters_are_rejoined() {
        assert_eq!(rejoin("Fréchet's théo-\nrème holds"), "Fréchet's théorème holds");
    }

    #[test]
    fn text_without_line_final_hyphens_is_unchanged() {
        let text = "state-of-the-art solution\n\nnext paragraph\n";
        assert_eq!(rejoin(text), text);
    }

    #[test]
    fn every_wrap_on_a_page_is_rejoined() {
        assert_eq!(
            rejoin("we scale it to ap-\nproximately 100K molecules and 600K morpho-\nlogical profiles"),
            "we scale it to approximately 100K molecules and 600K morphological profiles"
        );
    }
}
