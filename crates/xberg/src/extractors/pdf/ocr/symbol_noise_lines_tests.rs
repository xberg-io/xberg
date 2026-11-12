use super::*;

fn noise_filler(count: usize) -> Vec<&'static str> {
    ["a", "oe", "# =", "ae", "\"", "ee", "=|", "rs", "~", "ie"]
        .into_iter()
        .cycle()
        .take(count)
        .collect()
}

fn page(real: &[&str], filler: usize) -> String {
    let mut lines: Vec<&str> = real.to_vec();
    lines.extend(noise_filler(filler));
    lines.join("\n\n")
}

fn pruned(real: &[&str], filler: usize) -> String {
    let text = page(real, filler);
    assert!(
        page_is_dominated_by_noise_lines(text.lines()),
        "gate must pass: {text:?}"
    );
    prune_text(&text).0
}

fn assert_kept_exactly(real: &[&str]) {
    assert_eq!(pruned(real, 12), real.join("\n\n"), "{real:?}");
}

#[test]
fn keeps_word_shaped_lines_numbers_and_titles() {
    for line in [
        "SECTION TITLE",
        "Chapter 1",
        "Page 12",
        "99",
        "NORTH GATE 42",
        "Figure 3 sunset",
        "$5",
        "1.",
    ] {
        assert!(!is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn flags_fragment_and_symbol_lines() {
    for line in [
        "a",
        "mal",
        "# ae",
        "ef \"ae Car Sa % ey Bo ary \"dl",
        "~",
        "I",
        "ie ee",
        "ha >",
        "i. =",
    ] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn keeps_cjk_kana_hangul_and_thai_lines_of_two_or_three_characters() {
    assert_kept_exactly(&[
        "\u{5546}\u{54c1}",
        "\u{5408}\u{8a08}\u{984d}",
        "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}",
        "\u{c548}\u{b155}",
        "\u{0e2a}\u{0e27}\u{0e31}\u{0e2a}\u{0e14}\u{0e35}",
    ]);
}

#[test]
fn keeps_arabic_and_cyrillic_lines() {
    assert_kept_exactly(&[
        "\u{645}\u{631}\u{62d}\u{628}\u{627}",
        "\u{641}\u{64a} \u{627}\u{644}",
        "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}",
        "\u{434}\u{430} \u{43d}\u{435}\u{442}",
        "\u{44f}",
    ]);
}

#[test]
fn keeps_short_codes_and_amounts() {
    assert_kept_exactly(&["A1", "B2", "$5", "1.", "7"]);
}

#[test]
fn keeps_equations_between_variables_and_sums() {
    assert_kept_exactly(&["x = y", "a + b", "n < m", "total = cost"]);
}

#[test]
fn keeps_accented_and_non_english_latin_lines() {
    assert_kept_exactly(&[
        "go f\u{ff}r",
        "Gr\u{fc}n",
        "\u{c9}t\u{e9}",
        "de la gare",
        "Mairie du I",
        "a du LouvRe",
        "Th\u{e9}atre",
    ]);
}

#[test]
fn keeps_single_token_answers_and_unspaced_equations() {
    assert_kept_exactly(&["No.", "Yes.", "Qty?", "x=y", "ABC", "pqr", "LouvRe"]);
}

#[test]
fn prunes_a_lone_capitalized_trigram_without_a_closing_mark_and_keeps_an_accented_one() {
    for line in [
        "Tee",
        "Ces",
        "Ane",
        "Jot",
        "Bid",
        "Gun",
        "Aah",
        "CAS",
        "NOo",
        "Yes",
        "Qty",
        "\u{e9}t\u{e9}",
    ] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
    for line in ["Yes.", "Qty?", "Bid!", "\u{c9}t\u{e9}", "ABC", "pqr"] {
        assert!(!is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn a_lone_capitalized_trigram_is_dropped_from_a_noise_page_but_does_not_make_one() {
    let real = ["alpha beta", "gamma delta", "epsilon zeta", "theta iota"];
    let mut lines = real.to_vec();
    lines.extend(noise_filler(3));
    lines.push("Tee");
    let text = lines.join("\n\n");
    assert!(!page_is_dominated_by_noise_lines(text.lines()), "{text:?}");
    let noisy = page(&["SECTION TITLE"], 12) + "\n\nTee\n\nBid";
    assert_eq!(prune_text(&noisy).0, "SECTION TITLE");
}

#[test]
fn a_lowercase_trigram_or_a_trigram_among_glyphs_still_counts_toward_a_noise_page() {
    for extra in ["mal", "Tee ~ ~", "T~ee"] {
        let mut lines = vec!["alpha beta", "gamma delta", "epsilon zeta", "theta iota"];
        lines.extend(noise_filler(3));
        lines.push(extra);
        let text = lines.join("\n\n");
        assert!(page_is_dominated_by_noise_lines(text.lines()), "{text:?}");
    }
}

#[test]
fn keeps_real_words_beside_stray_glyphs() {
    assert_kept_exactly(&[
        "Annual report q z",
        "Harbour Lights x y",
        "Quarterly budget summary ~ ~",
        "Red box q z",
        "open the north gate ee",
        "the committee met on Tuesday a",
    ]);
}

#[test]
fn a_noise_page_keeps_real_short_names_and_mixed_content() {
    assert_kept_exactly(&["Sade", "Yes.", "Sade Nina ~ q", "Cecil Cecile Cecelia ~"]);
}

#[test]
fn prunes_one_word_beside_stray_glyphs() {
    for line in ["were ee", "Rays ae", "Settee e", "tail) \u{ab}"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn a_list_label_a_contraction_and_an_unspaced_word_each_keep_a_line_of_stray_glyphs() {
    assert_kept_exactly(&["a) b) and c)", "a i'm to ~", "~ \u{5546}\u{54c1} ~ ~"]);
}

#[test]
fn keeps_labels_and_capitalized_short_lines() {
    assert_kept_exactly(&["Go to Al", "b) to go", "c) if so", "Mr X Is In", "X= Y"]);
}

#[test]
fn keeps_assignments_and_calls_with_a_dangling_or_glued_operator() {
    assert_kept_exactly(&["f(x) = x", "X= Y", "a=", "C=", "n=", "Z=", "x=y", "a+b"]);
}

#[test]
fn prunes_a_digit_with_a_stray_quotation_or_section_mark_and_keeps_a_quoted_number() {
    for line in ["\u{201c}9", "\u{ab}5%", "5\u{a7}", "\"4"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
    for line in [
        "\u{201c}9\u{201d}",
        "\"4\"",
        "$5",
        "#3",
        "A1",
        "7",
        "Page 12",
        "we go \u{201c}9",
        "\u{2018}9\u{2019}",
        "\"5 items",
        "\u{201c}3 days\u{201d}",
        "\"10 Downing",
        "\u{a7}12",
        "Siehe \u{a7}12",
    ] {
        assert!(!is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn two_lowercase_trigrams_together_count_as_text_but_one_alone_does_not() {
    assert!(!is_symbol_noise_line("mal cad"));
    assert!(is_symbol_noise_line("mal"));
}

#[test]
fn prunes_lowercase_trigrams_and_symbol_runs_the_photograph_produces() {
    for line in ["\u{201c}P", "ee pe", "ta gM", "r ,", "co FF", "we SY"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
    for line in ["cad", "rtf", "Ne", "He"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
}

#[test]
fn prunes_fragment_operator_fragment_lines() {
    for line in ["ae - ee", "i - l", "oe = ie", "a * e", "= -", "ee + oe"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
    let text = page(&["x = y"], 12) + "\n\nae - ee\n\ni - l\n\noe = ie";
    assert_eq!(prune_text(&text).0, "x = y");
}

#[test]
fn keeps_english_phrases_with_punctuation_case_and_contractions() {
    assert_kept_exactly(&[
        "I am",
        "Go to",
        "I am.",
        "Go to.",
        "I'm",
        "IS IT",
        "We go to",
        "\"Go to\"",
        "it\u{2019}s",
    ]);
}

#[test]
fn prunes_a_function_word_beside_a_fragment() {
    for line in ["ee so", "rs or", "ie it", "oe of", "a a", "zz to"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
    let text = page(&["Go to."], 12) + "\n\nee so\n\nrs or";
    assert_eq!(prune_text(&text).0, "Go to.");
}

#[test]
fn a_page_of_seven_lines_is_never_pruned_and_eight_is_judged() {
    let seven: Vec<&str> = noise_filler(7);
    assert!(!page_is_dominated_by_noise_lines(seven.into_iter()));
    let eight: Vec<&str> = noise_filler(8);
    assert!(page_is_dominated_by_noise_lines(eight.into_iter()));
}

#[test]
fn exactly_half_noise_is_pruned_and_one_fewer_is_not() {
    let real = ["alpha beta", "gamma delta", "epsilon zeta", "theta iota"];
    let half = page(&real, 4);
    assert!(page_is_dominated_by_noise_lines(half.lines()));
    let below = page(&real, 3);
    let below_with_extra_real = format!("{below}\n\nkappa lambda");
    assert!(!page_is_dominated_by_noise_lines(below_with_extra_real.lines()));
    assert_eq!(
        prune_text(&half).0,
        "alpha beta\n\ngamma delta\n\nepsilon zeta\n\ntheta iota"
    );
}

#[test]
fn a_prose_page_with_a_few_short_lines_is_not_dominated() {
    let mut lines = vec![
        "The quick brown fox",
        "jumps over",
        "the lazy dog",
        "and then",
        "runs far away",
    ];
    lines.extend(["a", "b", "c"]);
    let text = lines.join("\n");
    assert!(!page_is_dominated_by_noise_lines(text.lines()));
}

#[test]
fn prunes_a_noise_page_and_keeps_its_real_lines() {
    assert_eq!(pruned(&["SECTION TITLE", "page 12"], 10), "SECTION TITLE\n\npage 12");
}

#[test]
fn paragraph_text_loses_the_same_lines_as_the_page_text() {
    let mut paragraph = crate::pdf::structure::types::PdfParagraph {
        text: "oe\nSECTION TITLE\nae".to_string(),
        lines: vec![],
        dominant_font_size: 12.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count: 4,
    };
    let mut paragraphs = vec![paragraph.clone()];
    prune_paragraphs(&mut paragraphs);
    assert_eq!(paragraphs[0].text, "SECTION TITLE");
    assert_eq!(paragraphs[0].word_count, 2);
    paragraph.text = "oe\nae".to_string();
    let mut emptied = vec![paragraph];
    prune_paragraphs(&mut emptied);
    assert!(emptied.is_empty());
}

#[cfg(all(feature = "ocr", feature = "pdf"))]
#[test]
fn annotations_on_surviving_lines_move_with_their_line() {
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{ElementKind, InternalElement};

    let mut element = InternalElement::text(ElementKind::Paragraph, "oe\nSECTION TITLE\nae\nlast line", 0);
    element.annotations = vec![
        TextAnnotation {
            start: 3,
            end: 10,
            kind: AnnotationKind::Bold,
        },
        TextAnnotation {
            start: 20,
            end: 24,
            kind: AnnotationKind::Bold,
        },
        TextAnnotation {
            start: 0,
            end: 2,
            kind: AnnotationKind::Bold,
        },
    ];
    assert!(prune_element_text(&mut element));
    assert_eq!(element.text, "SECTION TITLE\nlast line");
    let spans: Vec<(u32, u32)> = element.annotations.iter().map(|a| (a.start, a.end)).collect();
    assert_eq!(spans, vec![(0, 7), (14, 18)]);
}

#[test]
fn pruning_a_pruned_page_removes_nothing_more() {
    let text = page(&["SECTION TITLE", "Chapter 1"], 12);
    let (once, removed) = prune_text(&text);
    assert_eq!(removed, 12);
    assert_eq!(prune_text(&once), (once.clone(), 0));
}

#[cfg(all(feature = "ocr", feature = "pdf"))]
#[test]
fn a_document_with_relationships_is_not_prunable_and_keeps_its_elements() {
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };

    let mut document = InternalDocument::new("pdf");
    document.elements = vec![
        InternalElement::text(ElementKind::Paragraph, "oe\nae", 0),
        InternalElement::text(ElementKind::Paragraph, "SECTION TITLE", 0),
    ];
    assert!(elements_are_prunable(&document));
    document.relationships = vec![Relationship {
        source: 1,
        target: RelationshipTarget::Index(0),
        kind: RelationshipKind::FootnoteReference,
    }];
    assert!(!elements_are_prunable(&document));
    prune_document_elements(&mut document);
    assert_eq!(document.elements.len(), 2);
    assert_eq!(document.elements[0].text, "oe\nae");
}

#[test]
fn the_word_rule_needs_two_words_that_are_at_least_half_the_tokens() {
    // Two words of four tokens is the boundary that keeps; one of two and two of five are pruned.
    assert_kept_exactly(&[
        "Annual report q z",
        "north gate",
        "well-known report q ~",
        "I'm ready q ~",
    ]);
    for line in ["were ee", "xyz ~", "Tee ~", "Annual report q z ~", "a~b~c d~e~f ~ ~"] {
        assert!(is_symbol_noise_line(line), "{line}");
    }
}

#[cfg(all(feature = "ocr", feature = "pdf"))]
#[test]
fn the_page_level_pruners_drop_a_lone_capitalized_trigram_and_keep_a_marked_one() {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

    let mut paragraphs = vec![crate::pdf::structure::types::PdfParagraph {
        text: "Tee\nSECTION TITLE\nYes.".to_string(),
        lines: vec![],
        dominant_font_size: 12.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
        word_count: 4,
    }];
    prune_paragraphs(&mut paragraphs);
    assert_eq!(paragraphs[0].text, "SECTION TITLE\nYes.");
    let mut document = InternalDocument::new("pdf");
    document.elements = vec![
        InternalElement::text(ElementKind::Paragraph, "CAS\nSECTION TITLE", 0),
        InternalElement::text(ElementKind::Paragraph, "Bid", 0),
    ];
    prune_document_elements(&mut document);
    let texts: Vec<&str> = document.elements.iter().map(|element| element.text.as_str()).collect();
    assert_eq!(texts, vec!["SECTION TITLE"]);
}
