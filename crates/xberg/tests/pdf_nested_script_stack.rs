#![cfg(feature = "pdf")]

mod helpers;

use xberg::core::config::{ExtractionConfig, OutputFormat, PageConfig};

fn skip_regression_pages() -> bool {
    let skip = std::env::var_os("XBERG_REGRESSION_PAGES_SKIP").is_some_and(|value| value == "1");
    if skip {
        assert!(
            std::env::var_os("CI").is_none(),
            "XBERG_REGRESSION_PAGES_SKIP=1 must not be set in CI"
        );
    }
    skip
}

fn page_markdown(name: &str) -> Option<String> {
    if skip_regression_pages() {
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
    assert_one_line("plos-journal.pcbi.1010783-p4", "2 Rþnx is the vector");
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
fn should_keep_the_subscript_of_h_and_the_indexed_exponent_in_separate_runs_in_arxiv_02305_equation_3() {
    assert_markup("arxiv-2305.08917v2-p2", "H~⟨xy⟩,f~ <sup>Z</sup>~2~ = t c s");
    assert_markup(
        "arxiv-2305.08917v2-p2",
        "H~⟨xy⟩,f~ <sup>Z</sup>~2~<sup>,des</sup> = −2t",
    );
    assert_no_markup("arxiv-2305.08917v2-p2", "~2 ⟨xy⟩,f~");
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
    assert_markup("plos-journal.pcbi.1010783-p4", "2 R~þ~ <sup>n</sup>");
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

/// Glyph boxes of plos-journal.pcbi.1010783 p4 (pdftotext -bbox, pt) for the first dimension: `R`
/// x 284.49 to 291.71 y 522.59 to 532.59; `n` x 291.69 to 294.88 y 521.69 to 527.69 (raised); `þ`
/// x 291.69 to 296.30 y 529.05 to 534.45 (lowered), the plus sign of `R`; `x` x 294.92 to 297.28,
/// which starts where `n` ends and is its index. The subscript of `R` is one run and `n` with its
/// index `x` follows it, so the dimension reads `Rþnx`. Every dimension keeps it.
#[test]
fn should_read_the_plus_of_r_as_its_own_run_before_the_exponent_and_its_index_in_plos_1010783() {
    for dimension in [
        "Rþnx is the vector of state",
        "Rþny is the parameter vector",
        "Rþnu is the vector of constant",
    ] {
        assert_one_line("plos-journal.pcbi.1010783-p4", dimension);
    }
    assert_markup(
        "plos-journal.pcbi.1010783-p4",
        "R~þ~ <sup>n</sup>~x~ is the vector of state variables",
    );
    assert_no_markup("plos-journal.pcbi.1010783-p4", "~x þ~");
}

#[test]
fn should_restore_the_script_markup_on_algebra_page_1555() {
    if skip_regression_pages() {
        return;
    }
    let path = helpers::get_test_file_path(concat!(
        "pdf/algebra_topology_differential_calculus_and_optimization_theory_",
        "for_computer_science_and_machine_learning_2019_math_deep.pdf"
    ));
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = helpers::extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("public algebra fixture extracts");
    let page = |number| {
        let marker = format!("<!-- PAGE {number} -->");
        let next_marker = format!("<!-- PAGE {} -->", number + 1);
        document
            .content
            .split_once(&marker)
            .and_then(|(_, rest)| rest.split_once(&next_marker))
            .expect("algebra page has both Markdown markers")
            .0
    };
    let markdown = page(1555);
    let expression = "γ<sup>j+</sup> ~k~<sup>−</sup>";
    assert!(markdown.contains(expression), "page 1555 is missing {expression:?}");
    assert!(!markdown.contains("<sup>− j"), "page 1555 fused distinct script runs");
    let equation = "θ<sup>+</sup> = u~k~<sup>−</sup>/γ<sup>j+</sup> ~k~<sup>−</sup> ≥ 0,";
    assert!(
        markdown.lines().any(|line| line == equation),
        "page 1555 split the denominator or inequality from the θ equation"
    );
    let markers = markdown.matches("<sup>").count()
        + markdown.matches("<sub>").count()
        + regex::Regex::new(r"~[^~\n]+~").unwrap().find_iter(markdown).count();
    assert!(markers >= 59, "page 1555 has {markers} script markers");
    let neighbor = page(1524);
    assert!(neighbor.contains("γ<sup>j</sup> ~K~<sup>+</sup>"));
    assert!(neighbor.contains("j<sup>+</sup> is the index of the incoming basis vector"));
    assert!(page(1514).contains("A<sup>j</sup> = γ<sup>j+</sup> ~k~<sup>−</sup>A<sup>k</sup> + γ~k~ A<sup>k</sup>,"));
    assert!(page(1521).contains("determine which index k for which γ<sup>j+</sup>"));
    assert!(page(1522).trim_start().starts_with("1522\n\nSince "));
    assert!(page(1523).contains("The entry γ<sup>j+</sup> ~k~<sup>−</sup> is called the pivot element."));
}
