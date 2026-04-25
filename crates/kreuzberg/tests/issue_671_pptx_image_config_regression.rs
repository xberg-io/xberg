//! Regression tests for https://github.com/kreuzberg-dev/kreuzberg/issues/671
//!
//! ImageExtractionConfig.inject_placeholders was silently ignored on PPTX:
//! setting inject_placeholders=False had no effect and all three configs
//! (default / inject_placeholders=False / extract_images=False) produced
//! byte-identical output that still contained 34 `![…](media/imageN.png)` refs.
//!
//! Root cause: inject_placeholders was defined in ImageExtractionConfig but
//! never read. Image references were injected unconditionally in
//! Slide::to_markdown regardless of the flag value.

mod helpers;

use kreuzberg::core::config::{ExtractionConfig, ImageExtractionConfig, OutputFormat};
use kreuzberg::core::extractor::extract_file_sync;
use std::path::Path;

fn extract_md(path: &Path, config: ExtractionConfig) -> String {
    if !path.exists() {
        return String::new();
    }
    extract_file_sync(path, None, &config)
        .expect("extraction must succeed")
        .content
}

/// Core invariant: inject_placeholders=false must produce output with
/// fewer (or equal) `![` occurrences than the default config.
#[test]
fn test_inject_placeholders_false_removes_image_refs() {
    let path = Path::new("test_documents/pptx/powerpoint_with_image.pptx");
    if !path.exists() {
        eprintln!("Skipping: fixture not found at {}", path.display());
        return;
    }

    let default_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let no_placeholder_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        images: Some(ImageExtractionConfig {
            inject_placeholders: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let default_out = extract_md(path, default_config);
    let no_ph_out = extract_md(path, no_placeholder_config);

    // Skip if this PPTX has no images (test would be vacuously true)
    if !default_out.contains("![") {
        eprintln!("Skipping: fixture contains no image references");
        return;
    }

    let default_refs = default_out.matches("![").count();
    let no_ph_refs = no_ph_out.matches("![").count();

    assert!(
        no_ph_refs < default_refs,
        "inject_placeholders=false must reduce image reference count \
         (got {} vs {} with default). The flag is still being ignored.",
        no_ph_refs,
        default_refs,
    );
}

/// inject_placeholders=false must not drop non-image text content.
#[test]
fn test_inject_placeholders_false_preserves_text_content() {
    let path = Path::new("test_documents/pptx/powerpoint_with_image.pptx");
    if !path.exists() {
        eprintln!("Skipping: fixture not found at {}", path.display());
        return;
    }

    let default_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let no_placeholder_config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        images: Some(ImageExtractionConfig {
            inject_placeholders: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let default_out = extract_md(path, default_config);
    let no_ph_out = extract_md(path, no_placeholder_config);

    // Strip image markdown refs from both outputs and compare word counts.
    let strip_img = |s: &str| {
        s.lines()
            .filter(|l| !l.trim_start().starts_with("!["))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let default_words = strip_img(&default_out).split_whitespace().count();
    let no_ph_words = strip_img(&no_ph_out).split_whitespace().count();

    assert!(
        no_ph_words >= default_words.saturating_sub(5),
        "Text content must be preserved when inject_placeholders=false \
         (got {} vs {} words after stripping image lines)",
        no_ph_words,
        default_words,
    );
}

/// The three configs from the issue repro must NOT all be byte-identical when
/// the PPTX has images. Specifically, inject_placeholders=false must differ.
#[test]
fn test_configs_produce_different_output() {
    let path = Path::new("test_documents/pptx/powerpoint_with_image.pptx");
    if !path.exists() {
        eprintln!("Skipping: fixture not found at {}", path.display());
        return;
    }

    let default_out = extract_md(
        path,
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            use_cache: false,
            ..Default::default()
        },
    );
    let no_ph_out = extract_md(
        path,
        ExtractionConfig {
            output_format: OutputFormat::Markdown,
            use_cache: false,
            images: Some(ImageExtractionConfig {
                inject_placeholders: false,
                ..Default::default()
            }),
            ..Default::default()
        },
    );

    if !default_out.contains("![") {
        eprintln!("Skipping: fixture contains no image references");
        return;
    }

    assert_ne!(
        default_out, no_ph_out,
        "default config and inject_placeholders=false must produce different output \
         when the PPTX contains images. The flag is still being ignored."
    );
}
