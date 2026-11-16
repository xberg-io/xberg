use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::OnceLock;

use aho_corasick::AhoCorasick;

use super::patterns::PatternMatch;
use crate::core::config::redaction::RedactionTerm;
use crate::types::redaction::PiiCategory;

const MIN_SHARED_TERMS: usize = 16;
const MAX_SHARED_TERMS: usize = 1024;
const MAX_SHARED_TERM_BYTES: usize = 256;

pub(super) struct LiteralMatcher<'a> {
    automaton: AhoCorasick,
    terms: &'a [RedactionTerm],
    unicode_fallback: OnceLock<Vec<(String, regex::Regex)>>,
    case_sensitive: bool,
}

impl<'a> LiteralMatcher<'a> {
    pub(super) fn build(terms: &'a [RedactionTerm], main_text: &str) -> Option<Self> {
        if !(MIN_SHARED_TERMS..=MAX_SHARED_TERMS).contains(&terms.len()) {
            return None;
        }
        let case_sensitive = terms[0].case_sensitive;
        if !case_sensitive && !main_text.is_ascii() {
            return None;
        }
        let mut unique = HashSet::with_capacity(terms.len());
        for term in terms {
            let value = term.value.as_str();
            if term.case_sensitive != case_sensitive
                || value.is_empty()
                || value.len() > MAX_SHARED_TERM_BYTES
                || (!case_sensitive && !value.is_ascii())
            {
                return None;
            }
            let key = if case_sensitive {
                Cow::Borrowed(value)
            } else {
                Cow::Owned(value.to_ascii_lowercase())
            };
            if has_self_overlap(key.as_bytes()) || !unique.insert(key) {
                return None;
            }
        }

        let automaton = AhoCorasick::builder()
            .ascii_case_insensitive(!case_sensitive)
            .build(terms.iter().map(|term| &term.value))
            .ok()?;
        Some(Self {
            automaton,
            terms,
            unicode_fallback: OnceLock::new(),
            case_sensitive,
        })
    }

    pub(super) fn append_matches(&self, text: &str, matches: &mut Vec<PatternMatch>) {
        if !self.case_sensitive && !text.is_ascii() {
            // Unicode simple case folding can match ASCII values against non-ASCII text. ~keep
            let regexes = self
                .unicode_fallback
                .get_or_init(|| self.terms.iter().filter_map(compile_literal).collect());
            for (label, regex) in regexes {
                for matched in regex.find_iter(text) {
                    append_match(matches, text, matched.start(), matched.end(), label);
                }
            }
            return;
        }

        for matched in self.automaton.find_overlapping_iter(text) {
            let term = &self.terms[matched.pattern().as_usize()];
            append_match(matches, text, matched.start(), matched.end(), &term.label);
        }
    }
}

fn has_self_overlap(term: &[u8]) -> bool {
    // A term with a common proper prefix and suffix can have overlapping
    // hits that the per-term regex would skip. Keep that term on the regex path. ~keep
    (1..term.len()).any(|start| term.starts_with(&term[start..]))
}

pub(super) fn compile_literal(term: &RedactionTerm) -> Option<(String, regex::Regex)> {
    if term.value.is_empty() {
        return None;
    }
    let escaped = regex::escape(&term.value);
    let pattern = if term.case_sensitive {
        escaped
    } else {
        format!("(?i){escaped}")
    };
    let regex = regex::Regex::new(&pattern).ok()?;
    Some((term.label.clone(), regex))
}

fn append_match(matches: &mut Vec<PatternMatch>, text: &str, start: usize, end: usize, label: &str) {
    matches.push(PatternMatch {
        start,
        end,
        category: PiiCategory::Custom(label.to_string()),
        text: text[start..end].to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TEXT: &str = "xabcde KILO C++";

    fn project_terms(count: usize) -> impl Iterator<Item = RedactionTerm> {
        (0..count).map(|index| RedactionTerm::literal(format!("Project-{index:04}")))
    }

    fn smoke_terms() -> Vec<RedactionTerm> {
        // Keep this in step with the shared-path fixture in pattern_redaction_smoke.rs. ~keep
        let mut terms = vec![
            RedactionTerm::labeled("earlier", "xabc"),
            RedactionTerm::labeled("later", "abcde"),
            RedactionTerm::labeled("code", "kilo"),
            RedactionTerm::labeled("language", "c++"),
        ];
        terms.extend(project_terms(16));
        terms
    }

    #[test]
    fn smoke_fixture_builds_a_shared_literal_matcher() {
        assert!(LiteralMatcher::build(&smoke_terms(), SMOKE_TEXT).is_some());
    }

    #[test]
    fn ineligible_literal_sets_use_the_regex_fallback() {
        let terms = smoke_terms();
        for (value, reason) in [
            ("", "empty term"),
            ("aa", "self-overlap"),
            ("XABC", "duplicate after ASCII case folding"),
            ("çontrol-only", "non-ASCII case-insensitive term"),
        ] {
            let mut excluded = terms.clone();
            excluded[1].value = value.to_string();
            assert!(LiteralMatcher::build(&excluded, SMOKE_TEXT).is_none(), "{reason}");
        }

        let mut mixed = terms.clone();
        mixed[1].case_sensitive = true;
        assert!(
            LiteralMatcher::build(&mixed, SMOKE_TEXT).is_none(),
            "mixed case sensitivity"
        );

        let mut oversized = terms.clone();
        oversized[1].value = format!("+{}", "a".repeat(MAX_SHARED_TERM_BYTES));
        assert!(
            LiteralMatcher::build(&oversized, SMOKE_TEXT).is_none(),
            "oversized term"
        );

        assert!(
            LiteralMatcher::build(&terms, "xabcde KILO C++").is_none(),
            "non-ASCII case-insensitive input"
        );
        assert!(
            LiteralMatcher::build(&terms[..MIN_SHARED_TERMS - 1], SMOKE_TEXT).is_none(),
            "too few terms"
        );
        let too_many = project_terms(MAX_SHARED_TERMS + 1).collect::<Vec<_>>();
        assert!(LiteralMatcher::build(&too_many, SMOKE_TEXT).is_none(), "too many terms");
    }

    #[test]
    fn case_sensitive_literals_can_share_non_ascii_terms_and_input() {
        let mut terms = smoke_terms();
        for term in &mut terms {
            term.case_sensitive = true;
        }
        terms[1].value = "çontrol-only".to_string();
        assert!(LiteralMatcher::build(&terms, "xabcde KILO C++ çontrol-only").is_some());
    }
}
