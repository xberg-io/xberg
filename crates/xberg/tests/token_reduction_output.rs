//! Regression for xberg-io/xberg#1223: token reduction must take effect for
//! non-plain output formats, not be computed and then discarded when
//! `formatted_content` is swapped in.

#![cfg(feature = "quality")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;
use xberg::{OutputFormat, TokenReductionOptions};

const TEXT: &[u8] = b"The quick brown fox jumps over the lazy dog. It was really actually very quite \
extremely just simply the most incredibly fast fox that anyone had ever seen in the entire history of \
foxes and dogs together. Furthermore, in addition to that, it should be noted that the fox was also \
remarkably agile and nimble in its movements across the wide open field.";

fn extract_len(output_format: OutputFormat) -> usize {
    let cfg = ExtractionConfig {
        output_format,
        token_reduction: Some(TokenReductionOptions {
            mode: "aggressive".parse().unwrap_or_default(),
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(TEXT, "text/plain", &cfg)
        .expect("extraction must succeed")
        .content
        .len()
}

#[test]
fn token_reduction_applies_to_markdown_output() {
    let original = TEXT.len();
    let plain = extract_len(OutputFormat::Plain);
    let markdown = extract_len(OutputFormat::Markdown);

    assert!(plain < original, "plain output must be reduced: {plain} vs {original}");
    // Markdown output must also be reduced — previously it stayed full size
    // because the reduction was discarded by the format swap.
    assert!(
        markdown < original,
        "markdown output must be reduced too, got {markdown} vs original {original}"
    );
}
