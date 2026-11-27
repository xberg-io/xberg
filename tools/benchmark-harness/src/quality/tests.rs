use super::*;

#[test]
fn test_identical_text() {
    let text = "Hello world this is a test";
    let result = compute_quality(text, text);
    assert!((result.f1_score_text - 1.0).abs() < 0.001);
    assert!((result.quality_score - 1.0).abs() < 0.01);
}

#[test]
fn test_completely_different() {
    let result = compute_quality("alpha beta gamma", "one two three");
    assert_eq!(result.f1_score_text, 0.0);
}

#[test]
fn test_partial_overlap() {
    let result = compute_quality("hello world foo", "hello world bar");
    assert!((result.f1_score_text - 2.0 / 3.0).abs() < 0.001);
}

#[test]
fn test_numeric_scoring() {
    let result = compute_quality("page 42 section 7", "page 42 section 7");
    assert!((result.f1_score_numeric - 1.0).abs() < 0.001);
}

#[test]
fn test_empty_inputs() {
    let result = compute_quality("", "");
    assert!((result.f1_score_text - 1.0).abs() < 0.001);
}

#[test]
fn test_empty_extracted() {
    let result = compute_quality("", "some ground truth");
    assert_eq!(result.f1_score_text, 0.0);
}

#[test]
fn test_punctuation_stripped() {
    let result = compute_quality("hello, world!", "hello world");
    assert!((result.f1_score_text - 1.0).abs() < 0.001);
}

#[test]
fn test_case_insensitive() {
    let result = compute_quality("Hello World", "hello world");
    assert!((result.f1_score_text - 1.0).abs() < 0.001);
}

#[test]
fn should_lose_only_the_four_true_boundary_bigrams_when_japanese_ocr_lines_are_reordered() {
    // Same fixture as before the CJK-bigram-boundary fix: a continuous two-sentence ground
    // truth vs. an extraction that OCR split into 5 lines and reassembled OUT OF their
    // true order. Before the fix, one global CJK accumulator spanned every line break, so
    // reassembling the (still internally-intact) chunks in the WRONG order fabricated 4
    // cross-line "seam" bigrams that happened to land as `extra_tokens`, while the metric
    // otherwise looked almost perfect (0.9444...) — reading-order damage was nearly
    // invisible. After the fix, no bigram is ever fabricated across a line break: precision
    // is now exactly 1.0 (every extracted bigram genuinely occurs somewhere in the ground
    // truth) because the extraction never claims a cross-line adjacency it doesn't have.
    // The 4 real seam bigrams that a correctly-ordered reassembly WOULD have produced
    // ("上か", "進め", "名の", "り横" — see below) are honestly reported as
    // `missing_tokens` (recall loss) instead of being silently smuggled in as a false
    // match. Bag-of-bigram F1 remains inherently insensitive to reordering whole intact
    // chunks (that is a property of any bag/multiset metric, not fixable by boundary
    // flushing alone), but the fix removes the specific defect: false-positive credit for
    // adjacencies that were never actually extracted. ~keep
    let ground_truth = concat!(
        "元来日本語は漢文に倣い、文字を上から下へ、また行を右から左へと進めて表記を行っていた。",
        "漢字と仮名の筆順も縦書きを前提としており、横書き不能な書体も存在する。"
    );
    let extracted = concat!(
        "から下へまた行を右から左へと進\n",
        "の筆順も縦書きを前提としており、\n",
        "めて表記を行っていた。漢字と仮名\n",
        "横書き不能な書体も存在する。\n",
        "元来日本語は漢文に倣い、文字を上"
    );

    let result = compute_quality(extracted, ground_truth);

    assert_eq!(result.f1_score_text, 0.9714285714285714);
    assert_eq!(result.quality_score, 0.9714285714285714);
    assert_eq!(
        result.extra_tokens.len(),
        0,
        "no bigram is ever fabricated across a line break"
    );
    assert_eq!(
        result.missing_tokens.len(),
        4,
        "the 4 true seam bigrams are honestly reported as missing, not smuggled in as extra"
    );
    // Bag-of-bigram F1 is inherently insensitive to reordering whole intact chunks — that
    // ceiling is a property of any bag/multiset metric, not something boundary-flushing
    // alone can fix (see `char_error_rate`/`normalized_edit_similarity` for the
    // order-sensitive diagnostics that do catch this). What the fix DOES guarantee is that
    // the score is no longer inflated by fabricated cross-line matches, above. ~keep
    assert!(result.correct, "0.9714... clears the 0.95 threshold; see comment above");
}

#[test]
fn should_score_one_for_a_correctly_ordered_multiline_japanese_document() {
    // Regression guard for the fix above: when the extraction's line structure genuinely
    // matches the ground truth (i.e. nothing was reordered or reassembled differently),
    // flushing CJK bigrams at line boundaries must not introduce any spurious penalty. ~keep
    let text = concat!(
        "元来日本語は漢文に倣い、文字を上から下へ、また行を右から左へと進めて表記を行っていた。\n",
        "漢字と仮名の筆順も縦書きを前提としており、横書き不能な書体も存在する。"
    );

    let result = compute_quality(text, text);

    assert_eq!(result.f1_score_text, 1.0);
    assert_eq!(result.quality_score, 1.0);
    assert!(result.correct);
}

#[test]
fn should_not_award_partial_credit_for_unrelated_cjk_strings() {
    let result = compute_quality("日本語の文書", "本日の歴史");

    assert_eq!(result.f1_score_text, 0.0);
}

#[test]
fn should_use_cjk_bigrams_with_single_character_fallback() {
    assert_eq!(tokenize("日本語"), vec!["日本", "本語"]);
    assert_eq!(tokenize("日"), vec!["日"]);
}

#[test]
fn should_treat_a_line_break_between_cjk_characters_as_a_bigram_boundary() {
    // Previously named `should_ignore_line_wrapping_between_cjk_characters` and asserted
    // `wrapped == continuous` — that "decorative OCR line wrap" bridging is exactly the D2
    // defect: it let a single CJK accumulator span the whole document, silently absorbing
    // reordered/reassembled line breaks along with genuine mid-word OCR wraps. A CJK
    // bigram must never cross a line break, full stop; this fixture's wrapped form now
    // tokenizes to fewer, shorter runs than the continuous form. ~keep
    let continuous = tokenize("日本語の文書");
    let wrapped = tokenize("日本\n語の\n文書");

    assert_eq!(continuous, vec!["日本", "本語", "語の", "の文", "文書"]);
    assert_eq!(wrapped, vec!["日本", "語の", "文書"]);
    assert_ne!(wrapped, continuous);

    let result = compute_quality("日本\n語の\n文書", "日本語の文書");
    assert_eq!(result.f1_score_text, 0.7499999999999999);
    assert_eq!(result.quality_score, 0.7499999999999999);
}

#[test]
fn should_ignore_decorative_punctuation_between_cjk_characters() {
    assert_eq!(tokenize("日本,語"), tokenize("日本語"));
    assert_eq!(tokenize("日本。語"), tokenize("日本語"));
}

#[test]
fn should_tokenize_cjk_text_identically_with_or_without_spaces_between_characters() {
    let spaced = "日 本 語 の 文 書";
    let unspaced = "日本語の文書";

    assert_eq!(tokenize(spaced), tokenize(unspaced));
    assert_eq!(tokenize(spaced), vec!["日本", "本語", "語の", "の文", "文書"]);
}

#[test]
fn should_score_spaced_cjk_output_equal_to_unspaced_output_on_either_side() {
    let spaced = "元 来 日 本 語 は 漢 文 に 倣 い 、 文 字 を 上 か ら 下 へ";
    let unspaced = "元来日本語は漢文に倣い、文字を上から下へ";

    assert_eq!(compute_quality(spaced, unspaced).f1_score_text, 1.0);
    assert_eq!(compute_quality(unspaced, spaced).f1_score_text, 1.0);
    assert_eq!(
        compute_quality(spaced, unspaced).quality_score,
        compute_quality(unspaced, unspaced).quality_score
    );
}

#[test]
fn should_remove_tab_no_break_and_ideographic_spaces_between_cjk_characters() {
    assert_eq!(tokenize("日\t本\u{00a0}語\u{3000}の"), tokenize("日本語の"));
}

#[test]
fn should_keep_a_line_break_between_spaced_cjk_characters_as_a_bigram_boundary() {
    assert_eq!(tokenize("日 本\n語 の"), vec!["日本", "語の"]);
    assert_eq!(tokenize("日 本 \n 語 の"), vec!["日本", "語の"]);
}

#[test]
fn should_remove_spaces_between_hangul_syllables() {
    assert_eq!(tokenize("대한 민국"), tokenize("대한민국"));
    assert_eq!(tokenize("대한 민국"), vec!["대한", "한민", "민국"]);
}

#[test]
fn should_keep_spaces_between_latin_words() {
    assert_eq!(
        tokenize("alpha beta  gamma\tdelta"),
        vec!["alpha", "beta", "gamma", "delta"]
    );
    assert_eq!(compute_quality("alpha beta", "alphabeta").f1_score_text, 0.0);
}

#[test]
fn should_keep_spaces_between_cjk_and_latin_or_digits() {
    assert_eq!(
        tokenize("東 京 Tokyo 大 阪 2024 年"),
        vec!["東京", "tokyo", "大阪", "2024", "年"]
    );
    assert_eq!(tokenize("東京 Tokyo"), vec!["東京", "tokyo"]);
}

#[test]
fn should_tokenize_cjk_around_embedded_latin_and_numeric_runs() {
    assert_eq!(
        tokenize("日本語HS令和5年"),
        vec!["日本", "本語", "hs", "令和", "5", "年"]
    );
}

#[test]
fn should_keep_iteration_marks_inside_whitespace_free_japanese_bigrams() {
    assert_eq!(tokenize("時々刻々"), vec!["時々", "々刻", "刻々"]);
}

#[test]
fn should_preserve_latin_whitespace_tokenization() {
    assert_eq!(tokenize("Hello, world! 15.0"), vec!["hello", "world", "15"]);
}

#[test]
fn should_normalize_fullwidth_digits_to_ascii_via_nfkc() {
    // D3: fullwidth digits are outside `is_cjk_character`'s ranges and outside the
    // ASCII-digit count `normalize_numeric_token` checks, so without NFKC they never
    // matched their ASCII-equivalent form. ~keep
    assert_eq!(tokenize("１２３"), tokenize("123"));
    assert_eq!(tokenize("１２３"), vec!["123"]);
}

#[test]
fn should_normalize_ligatures_to_ascii_via_nfkc() {
    // D3: "ﬁ" is alphabetic so it silently survived the alphanumeric filter as its own
    // literal glyph instead of matching "fi", corrupting f1_score_text for a correct
    // extraction. ~keep
    assert_eq!(tokenize("ﬁle"), tokenize("file"));
    assert_eq!(tokenize("ﬁle"), vec!["file"]);
}

#[test]
fn should_ignore_a_soft_hyphen_within_a_word() {
    assert_eq!(tokenize("exam\u{ad}ple"), tokenize("example"));
    assert_eq!(tokenize("exam\u{ad}ple"), vec!["example"]);
}

#[test]
fn test_tokenize_number_normalization() {
    let tokens_a = tokenize("15.0");
    let tokens_b = tokenize("15");
    assert_eq!(tokens_a, tokens_b, "15.0 and 15 should normalize to the same token");
    assert_eq!(tokens_a, vec!["15"]);

    assert_eq!(tokenize("100.00"), vec!["100"]);
}

#[test]
fn test_compute_f1_number_equivalence() {
    let extracted = tokenize("price 15.0 dollars");
    let truth = tokenize("price 15 dollars");
    let f1 = compute_f1(&extracted, &truth);
    assert!(
        (f1 - 1.0).abs() < 0.001,
        "F1 should be 1.0 for semantically equivalent numeric tokens, got {f1}"
    );
}

#[test]
fn test_tokenize_preserves_decimals() {
    assert_eq!(tokenize("3.14"), vec!["3.14"]);
    assert_eq!(tokenize("0.5"), vec!["0.5"]);
    assert_eq!(tokenize("12.345"), vec!["12.345"]);
}

#[test]
fn test_no_numbers_no_boost() {
    let result = compute_quality("hello world foo", "hello world bar");
    let expected_text_f1 = 2.0 / 3.0;
    assert!(
        (result.f1_score_text - expected_text_f1).abs() < 0.001,
        "text F1 should be 2/3, got {}",
        result.f1_score_text
    );
    assert!(
        (result.quality_score - expected_text_f1).abs() < 0.001,
        "quality_score should equal text F1 ({expected_text_f1}) when no numbers, got {}",
        result.quality_score
    );
}

#[test]
fn should_score_text_only_when_ground_truth_has_no_numeric_tokens_despite_a_stray_extracted_digit() {
    // D1: the ground truth has zero digits, but the extraction emits a stray page number.
    // Numeric fidelity cannot be measured against a reference with no numbers, so
    // quality_score must equal the text-only F1 — not collapse through the 0.6/0.4 split,
    // which would apply a 40% penalty for a header/footer convention mismatch. ~keep
    let ground_truth = "revenue grew significantly year over year";
    let extracted = "revenue grew significantly year over year Page 3";

    let result = compute_quality(extracted, ground_truth);

    assert_eq!(
        result.f1_score_numeric, 0.0,
        "no numeric tokens in GT ⇒ numeric F1 is vacuously 0"
    );
    assert_eq!(
        result.quality_score, result.f1_score_text,
        "quality_score must equal text-only F1 when the ground truth has no numeric tokens"
    );
    assert_eq!(result.f1_score_text, 0.8571428571428571);
    // The stray digit is not unpunished: it still costs precision inside f1_score_text.
    assert!(result.quality_score < 1.0);
}

#[test]
fn should_still_score_numeric_zero_when_ground_truth_has_digits_extraction_drops() {
    // D1's asymmetry: when the ground truth DOES contain numerics and the extraction
    // dropped them, that is a genuine failure and must still cost the full 40% numeric
    // weight, unlike the no-numeric-in-GT case above. ~keep
    let ground_truth = "revenue was 42 million dollars";
    let extracted = "revenue was million dollars";

    let result = compute_quality(extracted, ground_truth);

    assert_eq!(result.f1_score_numeric, 0.0);
    assert_eq!(result.quality_score, 0.6 * result.f1_score_text);
    assert_ne!(
        result.quality_score, result.f1_score_text,
        "unlike the no-GT-numerics case, this must NOT equal the text-only score"
    );
}

#[test]
fn should_gate_combined_scoring_numeric_weight_on_ground_truth_only() {
    // Same D1 gating principle applied to `compute_quality_with_structure`'s combined
    // 0.5/0.2/0.3 formula: a stray extracted digit with no ground-truth numerics must not
    // pull in the numeric-weighted branch either. ~keep
    let ground_truth = "# Title\n\nParagraph text here.";
    let extracted = "# Title\n\nParagraph text here. 7";

    let metrics = compute_quality_with_structure(extracted, ground_truth, Some(ground_truth), OutputFormat::Markdown);
    let structural_f1 = structural_sidecar::score_markdown(extracted, ground_truth).sf1;
    let expected = 0.625 * metrics.f1_score_text + 0.375 * structural_f1;

    assert_eq!(metrics.f1_score_numeric, 0.0);
    assert_eq!(metrics.quality_score, expected);
}

#[test]
fn test_url_stripped_from_tokens() {
    let tokens = tokenize("[link text](https://example.com)");
    assert_eq!(tokens, vec!["link", "text"]);

    let tokens = tokenize("![alt text](https://example.com/image.png)");
    assert_eq!(tokens, vec!["alt", "text"]);

    let tokens = tokenize("See [docs](https://example.com/docs) for details");
    assert_eq!(tokens, vec!["see", "docs", "for", "details"]);
}

#[test]
fn should_strip_bare_urls_same_as_markdown_linked_urls() {
    // D4: a bare url used to survive the alphanumeric filter as junk
    // (`https://example.com` -> `httpsexamplecom`), so a framework emitting bare links took
    // a precision hit that a framework emitting markdown links did not. Both must discard
    // the url identically — compared here against an empty-anchor-text markdown link, which
    // discards the same amount of surrounding link text (none) as the bare form. ~keep
    let bare = tokenize("See https://example.com for details");
    let linked = tokenize("See [](https://example.com) for details");

    assert_eq!(bare, vec!["see", "for", "details"]);
    assert_eq!(bare, linked);
    assert_eq!(tokenize("Visit www.example.com today"), vec!["visit", "today"]);

    let ground_truth = tokenize("See for details");
    assert_eq!(compute_f1(&bare, &ground_truth), 1.0);
}

#[test]
fn should_score_pipe_table_cell_padding_identically() {
    // D4: `|` must act as a token separator so markdown table cell padding style cannot
    // change the token stream — `|a|b|` and `| a | b |` are semantically identical tables
    // and must tokenize (and therefore score) identically. ~keep
    assert_eq!(tokenize("|a|b|"), tokenize("| a | b |"));
    assert_eq!(tokenize("|a|b|"), vec!["a", "b"]);

    let ground_truth = "a b";
    let tight = compute_quality("|a|b|", ground_truth);
    let padded = compute_quality("| a | b |", ground_truth);
    assert_eq!(tight.f1_score_text, padded.f1_score_text);
    assert_eq!(tight.quality_score, padded.quality_score);
    assert_eq!(tight.f1_score_text, 1.0);
}

#[test]
fn test_large_number_preserved() {
    let tokens = tokenize("10000000000000001");
    assert_eq!(
        tokens,
        vec!["10000000000000001"],
        "17-digit number should be preserved as-is, not rounded by f64"
    );

    let tokens = tokenize("12345678901234.0");
    assert_eq!(
        tokens,
        vec!["12345678901234"],
        "15-digit number with trailing .0 should still normalize"
    );
}

#[test]
fn test_thousands_separators_normalize_to_bare_number() {
    // "1,000" and "1000" must tokenize identically (previously "1,000" failed f64 parse). ~keep
    assert_eq!(tokenize("1,000"), tokenize("1000"));
    assert_eq!(tokenize("12,345,678"), tokenize("12345678"));
    assert_eq!(tokenize("1,234.56"), tokenize("1234.56"));
    // A European-decimal comma (2-digit group) must NOT be treated as a thousands separator. ~keep
    assert_eq!(tokenize("3,14"), vec!["3,14"]);
}

#[test]
fn structured_quality_uses_canonical_sf1() {
    let extracted = "# Title\n\nParagraph.\n\n- first\n- second";
    let ground_truth = "## Title\n\nParagraph.\n\n1. first\n2. second";
    let expected = structural_sidecar::score_markdown(extracted, ground_truth).sf1;

    let metrics = compute_quality_with_structure(
        extracted,
        "Title Paragraph first second",
        Some(ground_truth),
        OutputFormat::Markdown,
    );

    assert_eq!(metrics.f1_score_layout, Some(expected));
}

#[test]
fn edit_distance_counts_single_edits() {
    let chars = |s: &str| s.chars().collect::<Vec<_>>();
    assert_eq!(edit_distance(&chars("kitten"), &chars("sitting")), 3);
    assert_eq!(edit_distance(&chars(""), &chars("abc")), 3);
    assert_eq!(edit_distance(&chars("abc"), &chars("abc")), 0);
}

#[test]
fn normalized_edit_similarity_is_order_sensitive() {
    assert!((normalized_edit_similarity("hello world", "hello world") - 1.0).abs() < 1e-9);
    let sim = normalized_edit_similarity("ab", "ba");
    assert!(
        (0.0..1.0).contains(&sim),
        "transposition should lower similarity, got {sim}"
    );
    assert!((normalized_edit_similarity("abcdefghij", "abcdefghiX") - 0.9).abs() < 1e-9);
}

#[test]
fn char_metrics_guard_empty_and_oversized_inputs() {
    assert!(char_error_rate("", "anything").is_nan(), "empty reference ⇒ NaN CER");
    assert!(
        normalized_edit_similarity("", "").is_nan(),
        "empty pair ⇒ NaN similarity"
    );
    let huge = "a".repeat(CER_MAX_CHARS + 1);
    assert!(
        normalized_edit_similarity(&huge, &huge).is_nan(),
        "oversized input is skipped"
    );
    assert!(
        (char_error_rate("abcd", "abXd") - 0.25).abs() < 1e-9,
        "one of four chars wrong ⇒ 0.25"
    );
}

// --- reading_order_score (report-only) ---

#[test]
fn min_reading_order_anchors_starts_at_eight() {
    assert_eq!(MIN_READING_ORDER_ANCHORS, 8);
}

#[test]
fn should_score_reading_order_one_for_identical_documents() {
    let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    assert_eq!(reading_order_score(text, text), Some(1.0));
}

#[test]
fn should_score_reading_order_near_zero_for_fully_reversed_token_order() {
    // 10 unique anchors in fully reversed order. Any strictly decreasing sequence has
    // a Longest Increasing Subsequence of length exactly 1, so the score is exactly
    // 1/10 = 0.1.
    let ground_truth = "one two three four five six seven eight nine ten";
    let extracted = "ten nine eight seven six five four three two one";
    assert_eq!(reading_order_score(extracted, ground_truth), Some(0.1));
}

#[test]
fn should_score_reading_order_one_for_correctly_ordered_japanese_document() {
    // Same fixture as `should_score_one_for_a_correctly_ordered_multiline_japanese_document`.
    let text = concat!(
        "元来日本語は漢文に倣い、文字を上から下へ、また行を右から左へと進めて表記を行っていた。\n",
        "漢字と仮名の筆順も縦書きを前提としており、横書き不能な書体も存在する。"
    );
    assert_eq!(reading_order_score(text, text), Some(1.0));
}

#[test]
fn should_score_reading_order_far_below_bag_of_tokens_f1_for_scrambled_japanese_lines() {
    // Same 5-line-reordered fixture as
    // `should_lose_only_the_four_true_boundary_bigrams_when_japanese_ocr_lines_are_reordered`,
    // where f1_score_text scores 0.9714285714285714 on this scrambled document — bag-of-
    // tokens F1 is order-insensitive by construction and cannot see the reordering at all.
    // reading_order_score exists precisely to catch what f1_score_text cannot: on the exact
    // same input it must be, and is, dramatically lower. ~keep
    let ground_truth = concat!(
        "元来日本語は漢文に倣い、文字を上から下へ、また行を右から左へと進めて表記を行っていた。",
        "漢字と仮名の筆順も縦書きを前提としており、横書き不能な書体も存在する。"
    );
    let extracted = concat!(
        "から下へまた行を右から左へと進\n",
        "の筆順も縦書きを前提としており、\n",
        "めて表記を行っていた。漢字と仮名\n",
        "横書き不能な書体も存在する。\n",
        "元来日本語は漢文に倣い、文字を上"
    );

    let score = reading_order_score(extracted, ground_truth);

    assert_eq!(score, Some(0.578125));
    let f1_text_on_same_fixture = 0.9714285714285714;
    assert!(
        score.unwrap() < f1_text_on_same_fixture - 0.3,
        "reading_order_score ({score:?}) must be dramatically lower than the bag-of-tokens \
             f1 ({f1_text_on_same_fixture}) on this scrambled fixture — that contrast is the \
             entire point of this metric"
    );
}

#[test]
fn should_return_none_when_fewer_than_minimum_anchors_are_found() {
    // "hello" and "world" each occur exactly once on both sides, but 2 anchors is below
    // MIN_READING_ORDER_ANCHORS (8) — insufficient signal to report a number.
    assert_eq!(reading_order_score("hello world", "hello world"), None);
}

#[test]
fn should_return_none_for_empty_extraction_or_empty_ground_truth() {
    let has_tokens = "some ground truth with plenty of unique tokens listed right here";
    assert_eq!(reading_order_score("", has_tokens), None);
    assert_eq!(reading_order_score(has_tokens, ""), None);
    assert_eq!(reading_order_score("", ""), None);
}

#[test]
fn should_not_panic_and_return_none_for_heavily_repetitive_tokens() {
    // Every token repeats, so there are zero anchors (an anchor requires an exactly-once
    // occurrence on both sides) — must return None cleanly rather than panic or divide by
    // zero.
    let repetitive = "the the the the the the the the the the the the";
    assert_eq!(reading_order_score(repetitive, repetitive), None);
}

#[test]
fn should_expose_reading_order_score_via_compute_quality_without_moving_quality_score() {
    // The whole point of this metric: bag-of-tokens F1 is 1.0 (every token survives, just
    // reordered), while reading_order_score correctly flags the reordering. quality_score
    // must stay exactly what it was before this metric existed — REPORT-ONLY.
    let ground_truth = "one two three four five six seven eight nine ten";
    let extracted = "ten nine eight seven six five four three two one";

    let result = compute_quality(extracted, ground_truth);

    assert_eq!(result.f1_score_text, 1.0);
    assert_eq!(result.reading_order_score, Some(0.1));
    assert_eq!(
        result.quality_score, 1.0,
        "reading_order_score is report-only and must not move quality_score"
    );
}

#[test]
fn should_exclude_leading_front_matter_from_markdown_text_and_structure_scores() {
    let ground_truth = "Report\n\nBody text here.";
    let ground_truth_markdown = "# Report\n\nBody text here.\n";
    let body = "# Report\n\nBody text here.\n";
    let with_front_matter = format!("---\ntitle: Complex Table Test\nmeta-viewport: width=device-width\n---\n\n{body}");

    let stripped = compute_quality_with_structure(
        &with_front_matter,
        ground_truth,
        Some(ground_truth_markdown),
        OutputFormat::Markdown,
    );
    let plain_body =
        compute_quality_with_structure(body, ground_truth, Some(ground_truth_markdown), OutputFormat::Markdown);

    assert_eq!(stripped.f1_score_text, 1.0);
    assert_eq!(stripped.f1_score_layout, plain_body.f1_score_layout);
    assert_eq!(stripped.quality_score, plain_body.quality_score);
    assert_eq!(
        structural_sidecar::score_markdown(&with_front_matter, ground_truth_markdown).sf1,
        structural_sidecar::score_markdown(body, ground_truth_markdown).sf1
    );
}

#[test]
fn should_score_markdown_without_front_matter_exactly_as_before() {
    let ground_truth = "Title\n\nOne two three.\n\nFour five.";
    let extracted = "# Title\n\nOne two three.\n\n-----\n\nFour five six.\n";

    let metrics = compute_quality_with_structure(extracted, ground_truth, None, OutputFormat::Markdown);

    assert_eq!(
        metrics.f1_score_text,
        compute_quality(extracted, ground_truth).f1_score_text
    );
}
