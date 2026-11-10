#![cfg(feature = "pdf")]

mod helpers;

use xberg::core::config::{ExtractionConfig, OutputFormat};

fn page_lines(name: &str) -> Option<Vec<String>> {
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
    let stripped = document
        .content
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
    assert_one_line("plos-journal.pcbi.1010783-p4", "2 Rnþx is the vector");
}
