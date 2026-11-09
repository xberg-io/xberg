//! The arXiv margin stamp (identifier, version, category, date), judged by one rule for every caller.
//! A citation never carries the date after the category, so only the whole shape at the end of a
//! paragraph is a stamp.

use std::sync::LazyLock;

/// The longest tail of a paragraph that can hold a stamp. A stamp is under 60 bytes; the rest is slack
/// for extra spaces between its parts.
const MAX_STAMP_TAIL_BYTES: usize = 160;

/// The three parts of a stamp: the identifier with its version, the category and the date.
const IDENTIFIER: &str = r"arXiv:[0-9]{4}\.[0-9]{4,5}v[0-9]+";
const CATEGORY: &str = r"\[[A-Za-z][A-Za-z0-9._-]*\]";
const DATE: &str = r"[0-9]{1,2}\s+[A-Z][a-z]{2,8}\s+[0-9]{4}";

static STAMP: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(r"{IDENTIFIER}\s*{CATEGORY}\s*{DATE}\s*$")).expect("static regex compiles")
});

#[cfg(feature = "pdf")]
static PARTS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(&format!(r"^(?:{IDENTIFIER})?\s*(?:{CATEGORY})?\s*(?:{DATE})?$")).expect("static regex compiles")
});

/// Return the byte offset where an arXiv stamp starts, when `text` ends with a whole stamp.
///
/// The stamp must start the text or follow white space. Text before the offset is not part of the
/// stamp. A paragraph that is only the stamp returns `0` after trimming.
pub(crate) fn trailing_stamp_start(text: &str) -> Option<usize> {
    let from = text.floor_char_boundary(text.len().saturating_sub(MAX_STAMP_TAIL_BYTES));
    let start = from + STAMP.find(&text[from..])?.start();
    (start == 0 || text[..start].ends_with(char::is_whitespace)).then_some(start)
}

/// Whether `text` is one or two parts of a stamp in stamp order, and not the whole stamp.
#[cfg(feature = "pdf")]
pub(crate) fn is_partial_stamp(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && PARTS.is_match(text) && !STAMP.is_match(text)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "pdf")]
    use super::is_partial_stamp;
    use super::trailing_stamp_start;

    const STAMP: &str = "arXiv:2312.13591v2 [hep-ph] 6 Dec 2023";

    #[test]
    fn a_whole_stamp_is_found_at_the_start_of_the_text() {
        assert_eq!(trailing_stamp_start(STAMP), Some(0));
    }

    #[test]
    fn a_stamp_after_body_words_is_found_after_them_and_not_before() {
        let text = format!("The committee met on the third day. {STAMP}");
        let start = trailing_stamp_start(&text).expect("stamp");
        assert_eq!(&text[start..], STAMP);
        assert_eq!(&text[..start], "The committee met on the third day. ");
    }

    #[test]
    fn trailing_white_space_after_the_stamp_is_allowed() {
        let text = format!("Body words {STAMP}  \n");
        assert_eq!(trailing_stamp_start(&text), Some("Body words ".len()));
    }

    #[test]
    fn real_category_and_date_forms_are_stamps() {
        for stamp in [
            "arXiv:2408.09869v5 [cs.CL] 9 Dec 2024",
            "arXiv:2206.01062v1 [cs.CV] 2 Jun 2022",
            "arXiv:2110.00061v3 [astro-ph.HE] 18 Nov 2021",
            "arXiv:0704.0001v12 [physics.soc-ph] 31 Dec 2009",
            "arXiv:2107.09275v2 [hep-ph] 15 Jul 2021",
            "arXiv:2312.13591v2 [hep-ph] 6 December 2023",
        ] {
            assert_eq!(trailing_stamp_start(stamp), Some(0), "{stamp}");
        }
    }

    #[test]
    fn a_short_versioned_citation_is_not_a_stamp() {
        assert_eq!(
            trailing_stamp_start("[12] A. Author, A title, arXiv:2107.09275v2 [hep-ph]."),
            None
        );
    }

    #[test]
    fn an_unversioned_citation_is_not_a_stamp() {
        assert_eq!(
            trailing_stamp_start("[11] J. Doe, A short title, arXiv:2107.09275 [hep-ph]."),
            None
        );
        assert_eq!(
            trailing_stamp_start(
                "The earlier study by Doe and Smith, Journal of Examples 12, 345 (2021), arXiv:2107.09275"
            ),
            None
        );
    }

    #[test]
    fn an_identifier_without_a_version_is_not_a_stamp_even_with_a_category_and_a_date() {
        // arXiv always prints the version in its stamp; a citation that adds a date does not.
        assert_eq!(trailing_stamp_start("arXiv:2312.13591 [hep-ph] 6 Dec 2023"), None);
        assert_eq!(
            trailing_stamp_start("Body words arXiv:2312.13591 [hep-ph] 6 Dec 2023"),
            None
        );
    }

    #[test]
    fn a_stamp_without_a_category_or_a_date_is_not_a_stamp() {
        assert_eq!(trailing_stamp_start("arXiv:2312.13591v2"), None);
        assert_eq!(trailing_stamp_start("arXiv:2312.13591v2 [hep-ph]"), None);
        assert_eq!(trailing_stamp_start("arXiv:2312.13591v2 6 Dec 2023"), None);
    }

    #[test]
    fn a_stamp_that_is_followed_by_more_text_is_not_trailing() {
        assert_eq!(trailing_stamp_start(&format!("{STAMP} and then more words")), None);
    }

    #[test]
    fn a_stamp_glued_to_a_word_is_not_a_stamp() {
        assert_eq!(trailing_stamp_start(&format!("xx{STAMP}")), None);
    }

    #[test]
    fn a_long_paragraph_is_searched_by_its_tail_only() {
        let text = format!("{} {STAMP}", "word ".repeat(5000));
        let start = trailing_stamp_start(&text).expect("stamp");
        assert_eq!(&text[start..], STAMP);
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn each_part_and_each_pair_of_parts_is_a_partial_stamp() {
        for text in [
            "arXiv:2312.13591v2",
            "[hep-ph]",
            "6 Dec 2023",
            "arXiv:2312.13591v2 [hep-ph]",
            "[hep-ph] 6 Dec 2023",
        ] {
            assert!(is_partial_stamp(text), "{text}");
        }
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn a_whole_stamp_and_body_text_are_not_partial_stamps() {
        for text in [
            STAMP,
            "",
            "6 Dec 2023 was a Wednesday",
            "[12] A. Author, arXiv:2312.13591v2",
            "Body [hep-ph]",
        ] {
            assert!(!is_partial_stamp(text), "{text}");
        }
    }

    #[test]
    fn multibyte_text_before_the_stamp_does_not_split_a_character() {
        let text = format!("{} {STAMP}", "\u{00E9}\u{4E2D}".repeat(200));
        let start = trailing_stamp_start(&text).expect("stamp");
        assert_eq!(&text[start..], STAMP);
    }
}
