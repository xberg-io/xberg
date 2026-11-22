#![cfg(feature = "html")]

use xberg::ConversionOptions;
use xberg::extraction::convert_html_to_markdown;

#[test]
#[cfg(feature = "url-ingestion")]
fn should_preserve_exhaustive_html_options_and_upstream_type_identity() {
    let defaults = html_to_markdown_rs::ConversionOptions::default();
    let options = ConversionOptions {
        heading_style: defaults.heading_style,
        list_indent_type: defaults.list_indent_type,
        list_indent_width: defaults.list_indent_width,
        bullets: defaults.bullets,
        strong_em_symbol: defaults.strong_em_symbol,
        escape_asterisks: defaults.escape_asterisks,
        escape_underscores: defaults.escape_underscores,
        escape_misc: defaults.escape_misc,
        escape_ascii: defaults.escape_ascii,
        code_language: defaults.code_language,
        autolinks: defaults.autolinks,
        expand_abbreviations: defaults.expand_abbreviations,
        include_blockquote_citations: defaults.include_blockquote_citations,
        default_title: defaults.default_title,
        br_in_tables: defaults.br_in_tables,
        compact_tables: defaults.compact_tables,
        highlight_style: defaults.highlight_style,
        extract_metadata: defaults.extract_metadata,
        whitespace_mode: defaults.whitespace_mode,
        strip_newlines: defaults.strip_newlines,
        wrap: defaults.wrap,
        wrap_width: defaults.wrap_width,
        convert_as_inline: defaults.convert_as_inline,
        sub_symbol: defaults.sub_symbol,
        sup_symbol: defaults.sup_symbol,
        newline_style: defaults.newline_style,
        code_block_style: defaults.code_block_style,
        keep_inline_images_in: defaults.keep_inline_images_in,
        preprocessing: defaults.preprocessing,
        encoding: defaults.encoding,
        debug: defaults.debug,
        strip_tags: defaults.strip_tags,
        preserve_tags: defaults.preserve_tags,
        skip_images: defaults.skip_images,
        inline_data_media: defaults.inline_data_media,
        hidden_content: defaults.hidden_content,
        url_escape_style: defaults.url_escape_style,
        link_style: defaults.link_style,
        output_format: defaults.output_format,
        include_document_structure: defaults.include_document_structure,
        extract_images: defaults.extract_images,
        max_image_size: defaults.max_image_size,
        max_input_size: defaults.max_input_size,
        capture_svg: defaults.capture_svg,
        infer_dimensions: defaults.infer_dimensions,
        max_depth: defaults.max_depth,
        exclude_selectors: defaults.exclude_selectors,
        tier_strategy: defaults.tier_strategy,
        base_url: defaults.base_url,
        data_as_elements: defaults.data_as_elements,
        visitor: defaults.visitor,
    };
    let module: xberg::extraction::html::ConversionOptions = options;
    let config = xberg::ExtractionConfig {
        html_options: Some(module.clone()),
        ..Default::default()
    };
    let file = xberg::FileExtractionConfig {
        html_options: Some(module.clone()),
        ..Default::default()
    };
    let _: html_to_markdown_rs::ConversionOptions = config.html_options.unwrap();
    let _: html_to_markdown_rs::ConversionOptions = file.html_options.unwrap();
    let output = convert_html_to_markdown("<p>Text</p>", Some(module), None).unwrap();
    assert_eq!(output.trim(), "Text");
}

#[test]
fn should_keep_accessible_names_without_adding_a_public_options_field() {
    let html = r#"<p>Before <a href="https://example.com" aria-label="Accessible name"></a> after.</p>
        <svg aria-label="Graphic label"><title>Graphic title</title><text>Drawn words</text></svg>
        <img src="photo.png" alt="Image description">"#;
    let output = convert_html_to_markdown(html, Some(ConversionOptions::default()), None).unwrap();
    for expected in ["Accessible name", "Graphic label", "Drawn words", "Image description"] {
        assert_eq!(output.matches(expected).count(), 1, "{expected}: {output}");
    }
}
