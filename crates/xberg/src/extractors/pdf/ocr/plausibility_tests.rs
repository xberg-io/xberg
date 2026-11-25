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

/// The GH#1767 carrier: Cyrillic written at cp1251 code points and read as Latin-1. Supplied
/// by the reporter; fictitious marketing copy naming no real individual or organisation.
const CP1251_CYRILLIC_AS_LATIN1: &str = "\u{C2}\u{E5}\u{F1}\u{E5}\u{ED}\u{ED}\u{FF}\u{FF} \u{EA}\u{E0}\u{EC}\u{EF}\u{E0}\u{ED}\u{E8}\u{FF} \u{E1}\u{F0}\u{E5}\u{ED}\u{E4}\u{E0} \u{D1}\u{E5}\u{E2}\u{E5}\u{F0}\u{ED}\u{FB}\u{E9} \u{E2}\u{E5}\u{F2}\u{E5}\u{F0} \u{F1}\u{F2}\u{F0}\u{EE}\u{E8}\u{F2}\u{F1}\u{FF} \u{E2}\u{EE}\u{EA}\u{F0}\u{F3}\u{E3} \u{E8}\u{F1}\u{F2}\u{EE}\u{F0}\u{E8}\u{E9} \u{EC}\u{E5}\u{F1}\u{F2}\u{ED}\u{FB}\u{F5} \u{EC}\u{E0}\u{F1}\u{F2}\u{E5}\u{F0}\u{EE}\u{E2}. \u{CC}\u{FB} \u{EF}\u{EE}\u{EA}\u{E0}\u{E7}\u{FB}\u{E2}\u{E0}\u{E5}\u{EC}, \u{EA}\u{E0}\u{EA} \u{ED}\u{E5}\u{E1}\u{EE}\u{EB}\u{FC}\u{F8}\u{E8}\u{E5} \u{EC}\u{E0}\u{F1}\u{F2}\u{E5}\u{F0}\u{F1}\u{EA}\u{E8}\u{E5} \u{EF}\u{F0}\u{E5}\u{E2}\u{F0}\u{E0}\u{F9}\u{E0}\u{FE}\u{F2} \u{EF}\u{F0}\u{EE}\u{F1}\u{F2}\u{FB}\u{E5} \u{EC}\u{E0}\u{F2}\u{E5}\u{F0}\u{E8}\u{E0}\u{EB}\u{FB} \u{E2} \u{E2}\u{E5}\u{F9}\u{E8}, \u{EA}\u{EE}\u{F2}\u{EE}\u{F0}\u{FB}\u{E5} \u{F1}\u{EB}\u{F3}\u{E6}\u{E0}\u{F2} \u{E3}\u{EE}\u{E4}\u{E0}\u{EC}\u{E8}. \u{C3}\u{EB}\u{E0}\u{E2}\u{ED}\u{E0}\u{FF} \u{E7}\u{E0}\u{E4}\u{E0}\u{F7}\u{E0} \u{EA}\u{E0}\u{EC}\u{EF}\u{E0}\u{ED}\u{E8}\u{E8} \u{F1}\u{EE}\u{F1}\u{F2}\u{EE}\u{E8}\u{F2} \u{E2} \u{F2}\u{EE}\u{EC}, \u{F7}\u{F2}\u{EE}\u{E1}\u{FB} \u{EF}\u{EE}\u{E2}\u{FB}\u{F1}\u{E8}\u{F2}\u{FC} \u{F3}\u{E7}\u{ED}\u{E0}\u{E2}\u{E0}\u{E5}\u{EC}\u{EE}\u{F1}\u{F2}\u{FC} \u{E1}\u{F0}\u{E5}\u{ED}\u{E4}\u{E0} \u{F1}\u{F0}\u{E5}\u{E4}\u{E8} \u{EC}\u{EE}\u{EB}\u{EE}\u{E4}\u{FB}\u{F5} \u{F1}\u{E5}\u{EC}\u{E5}\u{E9} \u{E8} \u{EF}\u{F0}\u{E8}\u{E2}\u{E5}\u{F1}\u{F2}\u{E8} \u{ED}\u{EE}\u{E2}\u{FB}\u{F5} \u{EF}\u{EE}\u{EA}\u{F3}\u{EF}\u{E0}\u{F2}\u{E5}\u{EB}\u{E5}\u{E9} \u{E2} \u{F4}\u{E8}\u{F0}\u{EC}\u{E5}\u{ED}\u{ED}\u{FB}\u{E5} \u{EC}\u{E0}\u{E3}\u{E0}\u{E7}\u{E8}\u{ED}\u{FB} \u{E3}\u{EE}\u{F0}\u{EE}\u{E4}\u{E0} \u{D0}\u{E5}\u{F7}\u{ED}\u{EE}\u{E9}. \u{CE}\u{F1}\u{ED}\u{EE}\u{E2}\u{ED}\u{EE}\u{E9} \u{E2}\u{E8}\u{E7}\u{F3}\u{E0}\u{EB}\u{FC}\u{ED}\u{FB}\u{E9} \u{EE}\u{E1}\u{F0}\u{E0}\u{E7} \u{EE}\u{E1}\u{FA}\u{E5}\u{E4}\u{E8}\u{ED}\u{FF}\u{E5}\u{F2} \u{F2}\u{E5}\u{EF}\u{EB}\u{FB}\u{E9} \u{F1}\u{E2}\u{E5}\u{F2}, \u{ED}\u{E0}\u{F2}\u{F3}\u{F0}\u{E0}\u{EB}\u{FC}\u{ED}\u{EE}\u{E5} \u{E4}\u{E5}\u{F0}\u{E5}\u{E2}\u{EE} \u{E8} \u{F1}\u{EF}\u{EE}\u{EA}\u{EE}\u{E9}\u{ED}\u{FB}\u{E5} \u{F6}\u{E2}\u{E5}\u{F2}\u{E0}. \u{CD}\u{E0}\u{F0}\u{F3}\u{E6}\u{ED}\u{E0}\u{FF} \u{F0}\u{E5}\u{EA}\u{EB}\u{E0}\u{EC}\u{E0} \u{F0}\u{E0}\u{E7}\u{EC}\u{E5}\u{F9}\u{E0}\u{E5}\u{F2}\u{F1}\u{FF} \u{F0}\u{FF}\u{E4}\u{EE}\u{EC} \u{F1} \u{EF}\u{E0}\u{F0}\u{EA}\u{E0}\u{EC}\u{E8}, \u{F0}\u{FB}\u{ED}\u{EA}\u{E0}\u{EC}\u{E8} \u{E8} \u{EE}\u{F1}\u{F2}\u{E0}\u{ED}\u{EE}\u{E2}\u{EA}\u{E0}\u{EC}\u{E8} \u{EE}\u{E1}\u{F9}\u{E5}\u{F1}\u{F2}\u{E2}\u{E5}\u{ED}\u{ED}\u{EE}\u{E3}\u{EE} \u{F2}\u{F0}\u{E0}\u{ED}\u{F1}\u{EF}\u{EE}\u{F0}\u{F2}\u{E0}. \u{C4}\u{EB}\u{FF} \u{F1}\u{EE}\u{F6}\u{E8}\u{E0}\u{EB}\u{FC}\u{ED}\u{FB}\u{F5} \u{F1}\u{E5}\u{F2}\u{E5}\u{E9} \u{E3}\u{EE}\u{F2}\u{EE}\u{E2}\u{E8}\u{F2}\u{F1}\u{FF} \u{F1}\u{E5}\u{F0}\u{E8}\u{FF} \u{EA}\u{EE}\u{F0}\u{EE}\u{F2}\u{EA}\u{E8}\u{F5} \u{F0}\u{EE}\u{EB}\u{E8}\u{EA}\u{EE}\u{E2}, \u{E3}\u{E4}\u{E5} \u{EA}\u{E0}\u{E6}\u{E4}\u{FB}\u{E9} \u{EC}\u{E0}\u{F1}\u{F2}\u{E5}\u{F0} \u{F0}\u{E0}\u{F1}\u{F1}\u{EA}\u{E0}\u{E7}\u{FB}\u{E2}\u{E0}\u{E5}\u{F2} \u{EE} \u{F1}\u{E2}\u{EE}\u{E5}\u{E9} \u{F0}\u{E0}\u{E1}\u{EE}\u{F2}\u{E5}.";

#[test]
fn cp1251_cyrillic_read_as_latin1_is_implausible_gh1767() {
    assert_eq!(
        evaluate_text_plausibility(CP1251_CYRILLIC_AS_LATIN1, &t()),
        PlausibilityVerdict::Implausible,
        "a single-byte code page's alphabet read as Latin-1 must route to OCR"
    );
}

/// Pins WHY the GH#1696 confidence signal cannot catch GH#1767, so it is not rediscovered:
/// whatlang finds a real language in this mojibake and is confident and reliable about it.
/// It is telling the truth about character shape and nothing about content, so a check on
/// confidence alone can only ever read this page as plausible. Both clauses of that check
/// are asserted to be satisfied here -- if either ever stops being, this fix's whole
/// premise needs rechecking. ~keep
#[test]
fn the_gh1767_carrier_defeats_the_confidence_signal_on_both_clauses() {
    let prose = select_prose_text(CP1251_CYRILLIC_AS_LATIN1);
    let chunks = prose_chunks(&prose);
    let reliability =
        crate::language_detection::chunk_reliability(&chunks, usize::MAX).expect("an unbounded read never stops early");

    assert!(
        reliability.reliable_ratio() >= t().min_reliable_language_chunk_ratio,
        "carrier clears the reliability bar at {}, so that clause cannot flag it",
        reliability.reliable_ratio()
    );
    assert!(
        reliability.mean_confidence() >= PLAUSIBILITY_MAX_MEAN_CONFIDENCE,
        "carrier clears the confidence bar at {}, so that clause cannot flag it either",
        reliability.mean_confidence()
    );
}

/// The separation the GH#1767 threshold rests on. The Latin-1 Supplement block holds no
/// unaccented ASCII letters, and every Latin-script orthography needs them, so genuine prose
/// cannot approach a ratio of 1.0 however diacritic-dense it is. Measured against the densest
/// real cases available and against 229 corpus documents, whose maximum was 0.0065 -- but the
/// corpus is predominantly English and so cannot speak for the population actually at risk,
/// which is why these samples are pinned here instead. ~keep
#[test]
fn diacritic_dense_real_prose_stays_far_below_the_mojibake_threshold() {
    const ICELANDIC: &str = "\u{DE}etta er \u{ED}slenskur texti me\u{F0} m\u{F6}rgum s\u{E9}rst\u{F6}kum st\u{F6}fum \u{FE}ar sem \u{E1}hersla er l\u{F6}g\u{F0} \u{E1} a\u{F0} s\u{FD}na hvernig tungum\u{E1}li\u{F0} l\u{ED}tur \u{FA}t \u{ED} venjulegu prentu\u{F0}u m\u{E1}li.";
    const PORTUGUESE: &str = "A informa\u{E7}\u{E3}o n\u{E3}o est\u{E1} dispon\u{ED}vel porque a pr\u{F3}xima reuni\u{E3}o de avalia\u{E7}\u{E3}o foi adiada at\u{E9} \u{E0} conclus\u{E3}o das obras de manuten\u{E7}\u{E3}o no edif\u{ED}cio hist\u{F3}rico da c\u{E2}mara municipal.";
    const FRENCH: &str = "Les h\u{F4}tels tr\u{E8}s pris\u{E9}s de la r\u{E9}gion c\u{F4}ti\u{E8}re accueillent chaque \u{E9}t\u{E9} des millions de visiteurs \u{E9}trangers qui d\u{E9}couvrent une cuisine r\u{E9}put\u{E9}e et des paysages pr\u{E9}serv\u{E9}s.";
    const WELSH: &str = "Mae'r tywydd yng Nghymru yn newid yn gyflym iawn yn ystod misoedd yr hydref, ac mae llawer o ymwelwyr yn dod i weld y mynyddoedd a'r traethau hardd.";

    for (language, sample) in [
        ("Icelandic", ICELANDIC),
        ("Portuguese", PORTUGUESE),
        ("French", FRENCH),
        ("Welsh", WELSH),
    ] {
        let ratio = latin1_supplement_alpha_ratio(sample);
        assert!(
            ratio < 0.25,
            "{language} measured {ratio}, too close to the {MOJIBAKE_LATIN1_SUPPLEMENT_ALPHA_RATIO} threshold"
        );
    }
    assert_eq!(
        latin1_supplement_alpha_ratio(CP1251_CYRILLIC_AS_LATIN1),
        1.0,
        "every letter of the carrier sits inside the block"
    );
}

/// The chunking before it sliced in place: the prose cut into owned strings of
/// [`crate::language_detection::CHUNK_SIZE`] characters through a `Vec<char>`, a short tail
/// dropped. The reference [`prose_chunks`] must reproduce.
fn legacy_prose_chunks(prose_text: &str) -> Vec<String> {
    let chars: Vec<char> = prose_text.chars().collect();
    let mut chunks: Vec<String> = chars
        .chunks(crate::language_detection::CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect())
        .collect();
    if chunks.last().is_some_and(|tail| tail.chars().count() < 100) {
        chunks.pop();
    }
    chunks
}

/// The verdict before detection could stop early: every chunk read, both clauses evaluated.
fn legacy_verdict(text: &str, thresholds: &OcrQualityThresholds) -> PlausibilityVerdict {
    let prose_text = select_prose_text(text);
    let chunks = legacy_prose_chunks(&prose_text);
    if chunks.len() < PLAUSIBILITY_MIN_PROSE_CHUNKS {
        return PlausibilityVerdict::NotEvaluated;
    }
    let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    let reliability = crate::language_detection::chunk_reliability(&chunk_refs, usize::MAX)
        .expect("an unbounded read never stops early");
    let low_confidence = reliability.reliable_ratio() < thresholds.min_reliable_language_chunk_ratio
        && reliability.mean_confidence() < PLAUSIBILITY_MAX_MEAN_CONFIDENCE;
    let mojibake = latin1_supplement_alpha_ratio(&prose_text) >= MOJIBAKE_LATIN1_SUPPLEMENT_ALPHA_RATIO;
    if low_confidence || mojibake {
        PlausibilityVerdict::Implausible
    } else {
        PlausibilityVerdict::Plausible
    }
}

#[test]
fn prose_chunks_slices_exactly_the_legacy_owned_chunks() {
    let alphabets = ["a", "\u{E9}", "\u{4E2D}", "\u{1F600}", "ab\u{E9}\u{4E2D} "];
    for alphabet in alphabets {
        for repeat in 0..=700 {
            let text = alphabet.repeat(repeat);
            let sliced = prose_chunks(&text);
            assert_eq!(
                sliced,
                legacy_prose_chunks(&text),
                "chunking diverged for {repeat} repeats of {alphabet:?}"
            );
        }
    }
}

#[test]
fn reliable_chunks_clearing_ratio_is_the_least_count_that_fails_the_ratio_clause() {
    assert_eq!(reliable_chunks_clearing_ratio(10, 0.1), 1);
    assert_eq!(reliable_chunks_clearing_ratio(20, 0.1), 2);
    assert_eq!(reliable_chunks_clearing_ratio(22, 0.1), 3);
    assert_eq!(reliable_chunks_clearing_ratio(3, 0.0), 0);
    assert_eq!(reliable_chunks_clearing_ratio(5, 1.0), 5);
    assert_eq!(reliable_chunks_clearing_ratio(5, 1.5), 6);
    for chunks in 1..=64usize {
        for min_ratio in [0.0, 0.05, 0.1, 0.25, 1.0 / 3.0, 0.5, 0.9, 1.0] {
            let least = reliable_chunks_clearing_ratio(chunks, min_ratio);
            for reliable in 0..=chunks {
                let clause_fails = (reliable as f64 / chunks as f64) >= min_ratio;
                assert_eq!(
                    clause_fails,
                    reliable >= least,
                    "{reliable}/{chunks} against {min_ratio}: least count {least}"
                );
            }
        }
    }
}

#[test]
fn chunk_reliability_stops_at_the_requested_reliable_count() {
    let english = select_prose_text(&ENGLISH_PROSE.repeat(3));
    let chunks = prose_chunks(&english);
    assert!(chunks.len() >= 3, "fixture must hold several chunks");
    assert!(
        crate::language_detection::chunk_reliability(&chunks, 1).is_none(),
        "English prose holds a reliable chunk, so a read asked to stop at one must stop"
    );
    assert!(crate::language_detection::chunk_reliability(&chunks, 0).is_none());

    let shifted = select_prose_text(&rot(ENGLISH_PROSE, 3).repeat(3));
    let shifted_chunks = prose_chunks(&shifted);
    let full = crate::language_detection::chunk_reliability(&shifted_chunks, usize::MAX)
        .expect("an unbounded read never stops early");
    assert_eq!(full.chunks, shifted_chunks.len());
    let reached = full.reliable_chunks + 1;
    let unstopped = crate::language_detection::chunk_reliability(&shifted_chunks, reached)
        .expect("a count above every reliable chunk on the page is never reached");
    assert_eq!(unstopped.reliable_chunks, full.reliable_chunks);
    assert_eq!(unstopped.confidence_sum.to_bits(), full.confidence_sum.to_bits());
}

/// Pages spanning every verdict: real prose in three scripts, every ROT shift of English, a
/// mojibake page, prose-free pages, a short tail at each length near the chunk floor, and
/// English and ROT-3 text mixed in every proportion up to 2:6, so the reliable ratio takes many
/// values between 0 and 1.
fn equivalence_pages() -> Vec<String> {
    let mut pages = vec![
        ENGLISH_PROSE.to_string(),
        GERMAN_PROSE.to_string(),
        CHINESE_PROSE_SEGMENT.repeat(5),
        CP1251_CYRILLIC_AS_LATIN1.to_string(),
        SHORT_PAGE.to_string(),
        NUMERIC_TABLE_PAGE.repeat(3),
        FORMULA_HEAVY_PAGE.repeat(3),
        table_rows_page(),
        "Мы хотим видеть дальше чем окна дома напротив и слышать музыку ветра ".repeat(12),
    ];
    pages.extend((1..26).map(|shift| rot(ENGLISH_PROSE, shift)));
    pages.extend([3, 13].map(|shift| rot(GERMAN_PROSE, shift)));
    let long = ENGLISH_PROSE.repeat(2);
    pages.extend((590..=900).step_by(7).map(|length| long[..length].to_string()));
    let shifted = rot(ENGLISH_PROSE, 3);
    for english in 0..=2 {
        for rotated in 0..=6 {
            pages.push(format!(
                "{}\n{}",
                ENGLISH_PROSE.repeat(english),
                shifted.repeat(rotated)
            ));
            let lines: Vec<&str> = std::iter::repeat_n(ENGLISH_PROSE, english)
                .chain(std::iter::repeat_n(shifted.as_str(), rotated))
                .collect();
            pages.push(lines.join("\n"));
        }
    }
    pages
}

/// The early stop must never change a verdict. Thresholds include, per page, the page's own
/// full-read reliable ratio and its neighbours on either side, the borderline values where
/// stopping one chunk too soon or too late would flip the verdict.
#[test]
fn early_stopping_verdict_equals_the_full_read_verdict() {
    let mut verdicts = std::collections::HashMap::new();
    let mut boundary_cases = 0usize;
    for page in equivalence_pages() {
        let prose = select_prose_text(&page);
        let chunks = prose_chunks(&prose);
        let mut thresholds_to_try = vec![0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 1.0 / 3.0, 0.5, 0.75, 0.9, 1.0];
        if chunks.len() >= PLAUSIBILITY_MIN_PROSE_CHUNKS {
            let full = crate::language_detection::chunk_reliability(&chunks, usize::MAX)
                .expect("an unbounded read never stops early");
            let own = full.reliable_ratio();
            thresholds_to_try.extend([own, own - 1e-12, own + 1e-12]);
            boundary_cases += 1;
        }
        for min_ratio in thresholds_to_try {
            let thresholds = OcrQualityThresholds {
                min_reliable_language_chunk_ratio: min_ratio,
                ..t()
            };
            let expected = legacy_verdict(&page, &thresholds);
            assert_eq!(
                evaluate_text_plausibility(&page, &thresholds),
                expected,
                "verdict changed at min_reliable_language_chunk_ratio {min_ratio} for page {:?}",
                page.chars().take(80).collect::<String>()
            );
            *verdicts.entry(format!("{expected:?}")).or_insert(0usize) += 1;
        }
    }
    for verdict in ["Plausible", "Implausible", "NotEvaluated"] {
        assert!(
            verdicts.get(verdict).copied().unwrap_or(0) > 0,
            "the equivalence pages must reach every verdict, missing {verdict}: {verdicts:?}"
        );
    }
    assert!(
        boundary_cases >= 40,
        "too few judged pages to probe the boundary: {boundary_cases}"
    );
}
