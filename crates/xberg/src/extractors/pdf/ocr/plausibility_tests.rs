use super::plausibility::*;
use crate::core::config::OcrQualityThresholds;
use crate::types::PageBoundary;

fn t() -> OcrQualityThresholds {
    OcrQualityThresholds::default()
}

/// ROT-`shift` over ASCII letters only; everything else (spaces, punctuation, digits) is
/// left untouched, mirroring how `shifted_to_unicode_pdf` in the top-level extractor tests
/// builds its fixture's `/ToUnicode` mapping.
fn rot(text: &str, shift: u8) -> String {
    let shift = shift % 26;
    text.chars()
        .map(|c| {
            if c.is_ascii_lowercase() {
                ((((c as u8 - b'a') + shift) % 26) + b'a') as char
            } else if c.is_ascii_uppercase() {
                ((((c as u8 - b'A') + shift) % 26) + b'A') as char
            } else {
                c
            }
        })
        .collect()
}

/// Genuine English prose, single line (so the whole passage counts as one prose line), well
/// over 600 characters -- three full [`crate::language_detection::CHUNK_SIZE`] chunks -- so
/// this is never itself the reason a test abstains. Public domain (US Declaration of
/// Independence, opening).
const ENGLISH_PROSE: &str = "When in the course of human events it becomes necessary for one \
    people to dissolve the political bands which have connected them with another and to assume \
    among the powers of the earth the separate and equal station to which the laws of nature and \
    of natures god entitle them a decent respect to the opinions of mankind requires that they \
    should declare the causes which impel them to the separation we hold these truths to be self \
    evident that all men are created equal that they are endowed by their creator with certain \
    unalienable rights that among these are life liberty and the pursuit of happiness that to \
    secure these rights governments are instituted among men deriving their just powers from the \
    consent of the governed";

/// Genuine German prose, single line, well over 600 characters.
const GERMAN_PROSE: &str = "Die Sonne schien hell am blauen Himmel und die Voegel sangen \
    froehliche Lieder in den gruenen Baeumen ein kleines Maedchen ging durch den Wald und \
    sammelte bunte Blumen fuer ihre Grossmutter die im naechsten Dorf wohnte auf dem Weg traf \
    sie einen freundlichen alten Mann der ihr den richtigen Pfad zeigte und ihr eine warme \
    Geschichte erzaehlte am Ende des Tages kam sie gluecklich und zufrieden nach Hause und \
    erzaehlte ihrer Familie von diesem wunderschoenen Abenteuer im Wald das sie nie vergessen \
    wuerde und noch viele jahre spaeter erzaehlte sie ihren eigenen kindern von dieser \
    wunderbaren reise durch den tiefen dunklen wald und von dem freundlichen alten mann der \
    ihr in ihrer groessten not geholfen hatte";

/// Genuine Chinese vocabulary, space-separated the way native PDF text extraction commonly
/// inserts gaps between glyph runs, well over 600 characters. Demonstrates that CJK content
/// needs no special-case handling (design premise: whatlang dispatches single-script text by
/// script with confidence `1.0`).
const CHINESE_PROSE_SEGMENT: &str = "我们 大家 都是 中国 人民 共和国 的 公民 我们 热爱 我们 伟大 的 祖国 和 美丽 的 山河 我们 尊重 历史 文化 传统 \
    我们 追求 和平 发展 与 繁荣 昌盛 我们 相信 未来 会 更加 美好 光明 我们 团结 一心 共同 建设 我们 美好 的 家园 ";

/// A page shaped like a numeric table: dense with digits, few alphabetic words per line.
const NUMERIC_TABLE_PAGE: &str = "\
    2024 1053 8842 7761 3390\n\
    5127 9034 1188 6602 4471\n\
    8890 2231 7765 3140 9982\n\
    1002 4456 7789 3321 6654\n\
    9981 1123 5567 8890 2234";

/// A page shaped like a block of formulas: dense with formula-shaped symbols.
const FORMULA_HEAVY_PAGE: &str = "\
    x = (a + b) / (c - d) ^ 2\n\
    f(x) = a*x^2 + b*x + c\n\
    y = sqrt(x^2 + z^2) / 2\n\
    delta = (b^2 - 4*a*c)\n\
    g(x) = (x - 1) * (x + 1)";

/// A page with far too little content to fill even one prose chunk (`< 600` prose
/// characters): about 30 words of genuine, structurally clean English prose.
const SHORT_PAGE: &str = "This page has only a small number of words on it, well under the \
    minimum amount of prose text this detector requires before it will render a verdict at \
    all, so it must abstain rather than guess.";

#[test]
fn rot3_english_prose_is_implausible() {
    let shifted = rot(ENGLISH_PROSE, 3);
    assert_eq!(
        evaluate_text_plausibility(&shifted, &t()),
        PlausibilityVerdict::Implausible,
        "a ROT-3-shifted English passage must not read as any real language"
    );
}

#[test]
fn english_prose_is_plausible() {
    assert_eq!(
        evaluate_text_plausibility(ENGLISH_PROSE, &t()),
        PlausibilityVerdict::Plausible
    );
}

#[test]
fn german_prose_is_plausible() {
    assert_eq!(
        evaluate_text_plausibility(GERMAN_PROSE, &t()),
        PlausibilityVerdict::Plausible
    );
}

#[test]
fn numeric_table_page_is_not_evaluated() {
    let page = NUMERIC_TABLE_PAGE.repeat(3);
    assert_eq!(
        evaluate_text_plausibility(&page, &t()),
        PlausibilityVerdict::NotEvaluated,
        "a numeric table has no prose lines to judge and must abstain, not guess"
    );
}

#[test]
fn formula_heavy_page_is_not_evaluated() {
    let page = FORMULA_HEAVY_PAGE.repeat(3);
    assert_eq!(
        evaluate_text_plausibility(&page, &t()),
        PlausibilityVerdict::NotEvaluated,
        "a formula-only page has no prose lines to judge and must abstain, not guess"
    );
}

#[test]
fn cjk_prose_abstains_from_implausible_verdict() {
    let page = CHINESE_PROSE_SEGMENT.repeat(5);
    assert_eq!(
        evaluate_text_plausibility(&page, &t()),
        PlausibilityVerdict::Plausible,
        "single-script CJK text is dispatched by script with confidence 1.0; it must never be \
         flagged implausible without a carve-out"
    );
}

#[test]
fn short_page_below_prose_chunk_floor_is_not_evaluated() {
    assert_eq!(
        evaluate_text_plausibility(SHORT_PAGE, &t()),
        PlausibilityVerdict::NotEvaluated,
        "fewer than 3 full prose chunks must abstain rather than judge on too little evidence"
    );
}

#[test]
fn short_trailing_chunk_is_dropped_not_padded() {
    // Three full 200-char chunks (600 chars) plus a 40-char tail: the tail is below
    // `PLAUSIBILITY_MIN_TAIL_CHUNK_CHARS` and must be dropped, leaving exactly 3 chunks --
    // still enough to be evaluated, not bumped down to `NotEvaluated` by a dropped tail that
    // was never counted as a 4th chunk in the first place.
    let mut text = ENGLISH_PROSE.repeat(2);
    text.truncate(640);
    let chunks = prose_chunks(&select_prose_text(&text));
    assert_eq!(
        chunks.len(),
        3,
        "a short trailing chunk must be dropped, not kept as a 4th chunk"
    );
    assert_eq!(
        evaluate_text_plausibility(&text, &t()),
        PlausibilityVerdict::Plausible,
        "3 chunks is still enough evidence to render a verdict"
    );
}

#[test]
fn scan_text_plausibility_skips_a_page_the_character_shape_gate_already_flags() {
    // Far below `OcrQualityThresholds::min_total_non_whitespace`'s default and structurally
    // garbled -- `evaluate_native_text_for_ocr` already flags this page on its own, so the
    // plausibility signal must not redundantly re-flag it.
    let native_text = "a a a a a";
    let boundaries = vec![PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: native_text.len(),
    }];

    let pages = scan_text_plausibility(native_text, Some(&boundaries), Some(1), &t()).implausible;

    assert!(
        pages.is_empty(),
        "a page the character-shape gate already routes to OCR must not also appear here: {pages:?}"
    );
}

#[test]
fn scan_text_plausibility_without_boundaries_flags_every_page() {
    let shifted = rot(ENGLISH_PROSE, 3);
    let pages = scan_text_plausibility(&shifted, None, Some(3), &t()).implausible;
    assert_eq!(
        pages,
        vec![1, 2, 3],
        "without boundaries there is no page subset to attribute the signal to, so an \
         implausible whole document must flag every page"
    );
}

#[test]
fn scan_text_plausibility_returns_empty_when_routing_disabled() {
    let shifted = rot(ENGLISH_PROSE, 3);
    let thresholds = OcrQualityThresholds {
        enable_plausibility_ocr_routing: false,
        ..t()
    };
    let pages = scan_text_plausibility(&shifted, None, Some(1), &thresholds).implausible;
    assert!(
        pages.is_empty(),
        "disabled routing must return no pages regardless of content: {pages:?}"
    );
}

#[test]
fn scan_text_plausibility_flags_only_the_implausible_page_in_a_multi_page_document() {
    let plausible_page = ENGLISH_PROSE;
    let implausible_page = rot(ENGLISH_PROSE, 3);
    let native_text = format!("{plausible_page}{implausible_page}");
    let boundaries = vec![
        PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: plausible_page.len(),
        },
        PageBoundary {
            page_number: 2,
            byte_start: plausible_page.len(),
            byte_end: native_text.len(),
        },
    ];

    let pages = scan_text_plausibility(&native_text, Some(&boundaries), Some(2), &t()).implausible;

    assert_eq!(pages, vec![2], "only the ROT-3-shifted page must be flagged: {pages:?}");
}

/// A page long enough to clear the character-shape gate that still holds no prose line: each
/// row carries five alphabetic words but is over a third ASCII digits, so
/// [`PROSE_LINE_MAX_DIGIT_RATIO`] rejects every one of them. `NUMERIC_TABLE_PAGE` above is the
/// same shape but far too short to be examined at all. This is an invoice or an agenda packet,
/// which is what issue #1709 was reported on. ~keep
fn table_rows_page() -> String {
    (1..=25)
        .map(|row| format!("Item {row:04} Qty 12 Unit 45.00 Tax 3.75 Total 48.75"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn scan_text_plausibility_reports_a_prose_free_page_as_unjudged() {
    let page = table_rows_page();
    let boundaries = vec![PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: page.len(),
    }];

    let scan = scan_text_plausibility(&page, Some(&boundaries), Some(1), &t());

    assert_eq!(scan.judged, 0, "a page holding no prose line cannot be judged");
    assert_eq!(
        scan.unjudged,
        vec![1],
        "the page the check could not judge must be reported as such: {scan:?}"
    );
    assert!(
        scan.implausible.is_empty(),
        "abstaining is not flagging: {:?}",
        scan.implausible
    );
}

#[test]
fn scan_text_plausibility_counts_a_clean_prose_page_as_judged() {
    let boundaries = vec![PageBoundary {
        page_number: 1,
        byte_start: 0,
        byte_end: ENGLISH_PROSE.len(),
    }];

    let scan = scan_text_plausibility(ENGLISH_PROSE, Some(&boundaries), Some(1), &t());

    assert_eq!(scan.judged, 1, "real English prose must produce a verdict: {scan:?}");
    assert!(
        scan.unjudged.is_empty(),
        "a page that was judged is not an abstention: {:?}",
        scan.unjudged
    );
    assert!(
        scan.implausible.is_empty(),
        "real English prose must not be flagged: {:?}",
        scan.implausible
    );
}

/// The distinction the caller needs: one page was read and passed, the other could not be read
/// at all, and `implausible` is empty either way.
#[test]
fn scan_text_plausibility_keeps_a_judged_page_apart_from_an_unjudged_one() {
    let table = table_rows_page();
    let native_text = format!("{ENGLISH_PROSE}\n{table}");
    let boundaries = vec![
        PageBoundary {
            page_number: 1,
            byte_start: 0,
            byte_end: ENGLISH_PROSE.len(),
        },
        PageBoundary {
            page_number: 2,
            byte_start: ENGLISH_PROSE.len() + 1,
            byte_end: native_text.len(),
        },
    ];

    let scan = scan_text_plausibility(&native_text, Some(&boundaries), Some(2), &t());

    assert_eq!(scan.judged, 1, "one of the two pages holds prose: {scan:?}");
    assert_eq!(
        scan.unjudged,
        vec![2],
        "the table page is the one the check could not judge: {scan:?}"
    );
    assert!(
        scan.implausible.is_empty(),
        "neither page is wrongly mapped: {:?}",
        scan.implausible
    );
}
