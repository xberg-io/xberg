//! Rejoin words that typesetting split across a line break with a hyphen.
//!
//! The structured (markdown) path dehyphenates paragraph lines in
//! `pdf::structure::pipeline`; the plain page text is assembled from spans without
//! paragraphs, so it needs its own pass over the assembled text. The break may be a
//! single newline or a blank line: double-spaced manuscripts and math-heavy pages put
//! every body line farther apart than the paragraph-gap threshold.

use crate::pdf::structure::pipeline::{
    HyphenWitnesses, WrapHyphen, classify_wrap, is_wrap_hyphen, record_hyphen_witnesses,
};

/// Rejoin `<word>-` + line break + `<lowercase word>` into one word, by the policy of
/// [`classify_wrap`] that the markdown path applies too. A kept hyphen is written as ASCII
/// `-`, as the markdown path writes it. Either way the line break goes.
///
/// Witnesses come from this page's text alone: the plain path assembles one page at a time.
pub(super) fn rejoin_line_wrap_hyphens(text: String) -> String {
    rejoin_with(text, |_| {})
}

/// [`rejoin_line_wrap_hyphens`] with the map from source byte offsets to offsets in the
/// rejoined text.
#[cfg(feature = "layout-detection")]
pub(super) fn rejoin_line_wrap_hyphens_mapped(text: String) -> (String, RejoinOffsets) {
    let mut joins = Vec::new();
    let joined = rejoin_with(text, |join| joins.push(join));
    (joined, RejoinOffsets(joins))
}

/// One rejoined break: source bytes `start..end` (the hyphen and the line break) became
/// `inserted` bytes (a kept `-`, or nothing).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(feature = "layout-detection"), allow(dead_code))]
struct Join {
    start: usize,
    end: usize,
    inserted: usize,
}

/// Source-to-rejoined byte offsets of one [`rejoin_line_wrap_hyphens_mapped`] call.
#[cfg(feature = "layout-detection")]
pub(super) struct RejoinOffsets(Vec<Join>);

#[cfg(feature = "layout-detection")]
impl RejoinOffsets {
    /// The rejoined offset of source byte `offset`; an offset inside a removed break maps
    /// into what replaced it.
    pub(super) fn map(&self, offset: usize) -> usize {
        let (mut removed, mut inserted) = (0, 0);
        for join in &self.0 {
            if offset <= join.start {
                break;
            }
            if offset < join.end {
                return join.start - removed + inserted + (offset - join.start).min(join.inserted);
            }
            removed += join.end - join.start;
            inserted += join.inserted;
        }
        offset - removed + inserted
    }
}

fn rejoin_with(text: String, mut on_join: impl FnMut(Join)) -> String {
    if !text.contains(is_wrap_hyphen) {
        return text;
    }
    let mut witnesses: Option<HyphenWitnesses> = None;
    let mut joined = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(break_at) = rest.find('\n') {
        let (line, after_line) = rest.split_at(break_at);
        let continuation = after_line.trim_start_matches(['\n', '\r', ' ', '\t']);
        let line = line.trim_end_matches([' ', '\t', '\r']);
        let wrap = if line.ends_with(is_wrap_hyphen) {
            let witnesses = witnesses.get_or_insert_with(|| {
                let mut witnesses = HyphenWitnesses::default();
                record_hyphen_witnesses(&text, &mut witnesses);
                witnesses
            });
            classify_wrap(line, continuation, witnesses)
        } else {
            WrapHyphen::NotApplicable
        };
        if wrap == WrapHyphen::NotApplicable {
            joined.push_str(&rest[..rest.len() - continuation.len()]);
        } else {
            let hyphen_len = line.chars().next_back().map_or(0, char::len_utf8);
            let stem = &line[..line.len() - hyphen_len];
            joined.push_str(stem);
            let keeps_hyphen = wrap == WrapHyphen::KeepHyphen;
            if keeps_hyphen {
                joined.push('-');
            }
            on_join(Join {
                start: text.len() - rest.len() + stem.len(),
                end: text.len() - continuation.len(),
                inserted: usize::from(keeps_hyphen),
            });
        }
        rest = continuation;
    }
    joined.push_str(rest);
    joined
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
    fn single_letter_stem_keeps_its_hyphen_but_loses_the_break() {
        assert_eq!(rejoin("a k-\ndimensional space"), "a k-dimensional space");
    }

    #[test]
    fn digit_and_bracket_stems_keep_their_hyphen_but_lose_the_break() {
        assert_eq!(rejoin("The 24- \nhour requirement"), "The 24-hour requirement");
        assert_eq!(
            rejoin("an (m − n)-\ndimensional sphere"),
            "an (m − n)-dimensional sphere"
        );
    }

    #[test]
    fn all_caps_halves_keep_the_hyphen_but_lose_the_break() {
        assert_eq!(
            rejoin("HISTORICAL INTER-\nACTIONS WITH"),
            "HISTORICAL INTER-ACTIONS WITH"
        );
        assert_eq!(rejoin("work on MU\u{2011}\nNIT."), "work on MU-NIT.");
    }

    #[test]
    fn dash_minus_and_number_continuations_keep_the_break() {
        assert_eq!(rejoin("value \u{2212}\nx"), "value \u{2212}\nx");
        assert_eq!(rejoin("pages 12-\n14"), "pages 12-\n14");
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

    #[cfg(feature = "layout-detection")]
    #[test]
    fn mapped_offsets_follow_each_rejoined_break() {
        let source = "the per-\nturbagens and a well-\n\nknown COVID-\nrelated end";
        let (joined, offsets) = super::rejoin_line_wrap_hyphens_mapped(source.to_string());
        assert_eq!(joined, "the perturbagens and a well-known COVID-related end");
        let word = |start: &str, end: &str| {
            let start = source.find(start).expect("start word");
            let end = source.find(end).expect("end word") + end.len();
            &joined[offsets.map(start)..offsets.map(end)]
        };
        assert_eq!(word("the", "the"), "the");
        assert_eq!(word("per-", "turbagens"), "perturbagens");
        assert_eq!(word("and", "and"), "and");
        assert_eq!(word("well-", "known"), "well-known");
        assert_eq!(word("COVID-", "related"), "COVID-related");
        assert_eq!(word("end", "end"), "end");
        assert_eq!(offsets.map(source.len()), joined.len());
    }

    #[test]
    fn every_wrap_on_a_page_is_rejoined() {
        assert_eq!(
            rejoin("we scale it to ap-\nproximately 100K molecules and 600K morpho-\nlogical profiles"),
            "we scale it to approximately 100K molecules and 600K morphological profiles"
        );
    }
}
