#![cfg(feature = "pdf")]

mod helpers;

use xberg::core::config::{ExtractionConfig, OutputFormat};

/// `None` only for the local opt-out `XBERG_REGRESSION_PAGES_SKIP=1`, which panics when `CI` is set;
/// a skipped suite therefore fails loudly in CI and nowhere else.
fn page_markdown(name: &str) -> Option<String> {
    if std::env::var_os("XBERG_REGRESSION_PAGES_SKIP").is_some_and(|value| value == "1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "XBERG_REGRESSION_PAGES_SKIP=1 must not be set in CI"
        );
        return None;
    }
    let path = helpers::get_test_file_path(&format!("pdf/regression_pages/{name}.pdf"));
    assert!(
        path.exists(),
        "{name}.pdf is missing. The corpus is fetched with: python3 test_documents/scripts/fetch_corpus.py"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let document = helpers::extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("regression page extracts");
    Some(document.content)
}

fn page_lines(name: &str) -> Option<Vec<String>> {
    let content = page_markdown(name)?;
    let stripped = content
        .replace("<sup>", "")
        .replace("</sup>", "")
        .replace("<sub>", "")
        .replace("</sub>", "")
        .replace(['*', '~', '\\'], "");
    Some(
        stripped
            .lines()
            .map(|line| line.split_whitespace().collect::<String>())
            .collect(),
    )
}

fn assert_one_line(name: &str, expected: &str) {
    let Some(lines) = page_lines(name) else {
        return;
    };
    let needle: String = expected.split_whitespace().collect();
    assert!(
        lines.iter().any(|line| line.contains(&needle)),
        "{expected:?} is not on one line of {name}:\n{}",
        lines.join("\n")
    );
}

#[test]
fn should_keep_a_subscripted_exponent_with_its_base_in_arxiv_00970_equation_7() {
    assert_one_line("arxiv-2403.00970v1-p3", "(τ)e c1 τ dτ");
}

#[test]
fn should_keep_a_two_member_exponent_with_its_base_in_arxiv_01153_equation_2() {
    assert_one_line("arxiv-2403.01153v1-p3", "ejθR = α(");
}

#[test]
fn should_keep_a_series_term_exponent_with_its_base_in_gutenberg_38769_exercise_3() {
    assert_one_line("gutenberg-38769-p412", "(a1 ns1 + a2 ns2");
}

#[test]
fn should_keep_a_subscripted_exponent_with_its_base_in_plos_1010783_state_space() {
    assert_one_line("plos-journal.pcbi.1010783-p4", "2 Rnxþ is the vector");
}

fn assert_markup(name: &str, fragment: &str) {
    let Some(markdown) = page_markdown(name) else {
        return;
    };
    assert!(
        markdown.contains(fragment),
        "{fragment:?} is not in the Markdown of {name}:\n{markdown}"
    );
}

fn assert_no_markup(name: &str, fragment: &str) {
    let Some(markdown) = page_markdown(name) else {
        return;
    };
    assert!(
        !markdown.contains(fragment),
        "{fragment:?} is in the Markdown of {name}:\n{markdown}"
    );
}

#[test]
fn should_keep_the_superscript_and_subscript_of_a_stack_in_separate_runs_in_arxiv_02305_equation_3() {
    assert_markup("arxiv-2305.08917v2-p2", "H<sup>Z</sup>~2 ⟨xy⟩,f~ = t c s");
    assert_markup(
        "arxiv-2305.08917v2-p2",
        "H<sup>Z</sup>~2~<sup>,des</sup> ~⟨xy⟩,f~ = −2t",
    );
}

#[test]
fn should_mark_the_scripts_of_s_and_sigma_in_arxiv_02305() {
    assert_markup(
        "arxiv-2305.08917v2-p2",
        "s<sup>1</sup> = σ<sup>1</sup>/2 couples to fermions",
    );
}

#[test]
fn should_not_take_the_closing_bracket_piece_for_a_superscript_in_openstax_business_statistics() {
    assert_no_markup("openstax-introductory-business-statistics-1e-p362", "<sup>⎞");
}

#[test]
fn should_hold_the_exponent_n_in_the_superscript_of_r_in_plos_1010783() {
    assert_markup("plos-journal.pcbi.1010783-p4", "2 R<sup>n</sup>");
    assert_no_markup("plos-journal.pcbi.1010783-p4", "\nn~x~ n~y~\n");
}

#[test]
fn should_keep_the_stack_scripts_of_arxiv_01153_equations_2_and_3_on_their_base() {
    assert_markup("arxiv-2403.01153v1-p3", "· e<sup>iθ</sup> (2)");
    assert_markup("arxiv-2403.01153v1-p3", "∥H(f, t)∥ · e<sup>jθ</sup>\n");
    assert_markup("arxiv-2403.01153v1-p3", "= ∥H~s~(f, t)∥ · e<sup>jθs</sup>");
    assert_markup("arxiv-2403.01153v1-p3", "\\+ ∥H~m~(f, t)∥ · e<sup>jθm</sup> (3)");
    assert_no_markup("arxiv-2403.01153v1-p3", "\njθs\n\n∥H");
}

#[test]
fn should_keep_the_subscript_of_psi_in_nist_sp800_22_p49() {
    assert_markup("nist-sp800-22r1a-p49", "- (3) Compute: ψ~*m*~ = ∑");
}

/// The sources set a fraction in each exponent or index: the glyph stream of gutenberg-33283-p84 reads
/// `x`, `−`, `1`, `2` for x^{−1/2}, p161 `z`, `−`, `2`, `3` for z^{−2/3}, and arxiv-2405.00569 p4 `f`,
/// `j`, `+`, `3`, `2` for f_{j+3/2}. A numerator over a denominator is no script stack, so none of
/// them may read as an integer power or index.
#[test]
fn should_not_flatten_the_fractional_powers_of_gutenberg_33283_p84_and_p161_into_integers() {
    assert_no_markup("gutenberg-33283-p84", "x<sup>−");
    assert_no_markup("gutenberg-33283-p84", "<sup>− 2</sup>");
    assert_one_line("gutenberg-33283-p84", "θ=3a2x");
    assert_no_markup("gutenberg-33283-p161", "z<sup>−");
    assert_no_markup("gutenberg-33283-p161", "<sup>− 3</sup>");
}

#[test]
fn should_not_move_a_denominator_into_the_half_cell_index_of_arxiv_00569_p4() {
    assert_no_markup("arxiv-2405.00569v2-p4", "f~j+2~ 3");
    assert_no_markup("arxiv-2405.00569v2-p4", "f~j+2~ 1");
    assert_one_line("arxiv-2405.00569v2-p4", "(2.5)f");
}

#[test]
fn should_not_flatten_the_half_power_of_the_root_in_gutenberg_38769_p500() {
    assert_no_markup("gutenberg-38769-p500", "2π</sup>");
}

/// The glyph stream of plos-journal.pcbi.1010783 p4 reads `R`, `n`, `x`, `þ`: the exponent `n_x`
/// first, then the `þ` below the line. Every dimension keeps that order.
#[test]
fn should_keep_the_glyph_stream_order_of_the_dimension_in_plos_1010783() {
    for dimension in [
        "Rnxþ is the vector of state",
        "Rnyþ is the parameter vector",
        "Rnuþ is the vector of constant",
    ] {
        assert_one_line("plos-journal.pcbi.1010783-p4", dimension);
    }
    assert_markup(
        "plos-journal.pcbi.1010783-p4",
        "R<sup>n</sup>~x þ~ is the vector of state variables",
    );
}
