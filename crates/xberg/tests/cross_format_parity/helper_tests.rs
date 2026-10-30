use super::*;

#[test]
fn test_strip_markdown_headings() {
    let input = "# Heading 1\n## Heading 2\nPlain text\n";
    let stripped = strip_markdown(input);
    assert!(stripped.contains("Heading 1"));
    assert!(stripped.contains("Heading 2"));
    assert!(stripped.contains("Plain text"));
    assert!(!stripped.contains('#'));
}

#[test]
fn test_strip_markdown_links() {
    let input = "See [link text](https://example.com) for details.\n";
    let stripped = strip_markdown(input);
    assert!(stripped.contains("link text"));
    assert!(!stripped.contains("https://example.com"));
    assert!(!stripped.contains('['));
    assert!(!stripped.contains(']'));
}

#[test]
fn test_strip_markdown_bold_italic() {
    let input = "This is **bold** and *italic* text.\n";
    let stripped = strip_markdown(input);
    assert!(stripped.contains("bold"));
    assert!(stripped.contains("italic"));
}

#[test]
fn test_strip_markdown_list() {
    let input = "- item one\n* item two\n1. item three\n";
    let stripped = strip_markdown(input);
    assert!(stripped.contains("item one"));
    assert!(stripped.contains("item two"));
    assert!(stripped.contains("item three"));
}

#[test]
fn test_strip_html_tags() {
    let input = "<h1>Title</h1><p>Hello &amp; goodbye</p>";
    let stripped = strip_html(input);
    assert!(stripped.contains("Title"));
    assert!(stripped.contains("Hello & goodbye"));
    assert!(!stripped.contains('<'));
    assert!(!stripped.contains('>'));
}

#[test]
fn test_strip_html_numeric_entity() {
    let input = "A&#65;B";
    let stripped = strip_html(input);
    assert!(stripped.contains("AAB"));
}

#[test]
fn test_tokenize() {
    let input = "Hello, World! This is a TEST.";
    let tokens = tokenize(input);
    assert!(tokens.contains(&"hello".to_string()));
    assert!(tokens.contains(&"world".to_string()));
    assert!(tokens.contains(&"test".to_string()));
}

#[test]
fn test_token_f1_identical() {
    let a = vec!["hello".to_string(), "world".to_string()];
    let f1 = token_f1(&a, &a);
    assert!((f1 - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_token_f1_no_overlap() {
    let a = vec!["hello".to_string()];
    let b = vec!["world".to_string()];
    let f1 = token_f1(&a, &b);
    assert!(f1.abs() < f64::EPSILON);
}

#[test]
fn test_token_f1_partial_overlap() {
    let a = vec![
        "the".to_string(),
        "quick".to_string(),
        "brown".to_string(),
        "fox".to_string(),
    ];
    let b = vec![
        "the".to_string(),
        "quick".to_string(),
        "red".to_string(),
        "fox".to_string(),
    ];
    let f1 = token_f1(&a, &b);
    assert!((f1 - 0.75).abs() < 0.01);
}

#[test]
fn test_token_f1_empty() {
    let empty: Vec<String> = vec![];
    assert!((token_f1(&empty, &empty) - 1.0).abs() < f64::EPSILON);
    assert!(token_f1(&empty, &["a".to_string()]).abs() < f64::EPSILON);
}

#[test]
fn test_strip_images() {
    let input = "Before ![alt text](image.png) after";
    let stripped = strip_images(input);
    assert!(stripped.contains("alt text"));
    assert!(!stripped.contains("image.png"));
}

#[test]
fn test_gfm_trailing_whitespace() {
    let md = "Hello world  \nNext line\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("trailing whitespace")));
}

#[test]
fn test_gfm_no_trailing_newline() {
    let md = "Hello world";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("does not end with a newline")));
}

#[test]
fn test_gfm_multiple_trailing_newlines() {
    let md = "Hello world\n\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("multiple trailing newlines")));
}

#[test]
fn test_gfm_setext_heading() {
    let md = "Title\n=====\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("setext heading")));
}

#[test]
fn test_gfm_accepts_complete_yaml_front_matter() {
    for newline in ["\n", "\r\n"] {
        let md = [
            "---",
            "title: Metadata",
            "description: |",
            "  a value",
            "---",
            "",
            "# Body",
            "",
        ]
        .join(newline);
        assert_eq!(validate_gfm_basics(&md), Vec::<String>::new());
    }
}

#[test]
fn test_gfm_rejects_setext_in_body_after_yaml_front_matter() {
    let md = "---\ntitle: Metadata\n---\n\nBody\n---\n";
    assert_eq!(
        validate_gfm_basics(md),
        ["line 6: setext heading (--- style), use ATX (# style)"]
    );
}

#[test]
fn test_gfm_does_not_hide_body_after_an_unclosed_front_matter_delimiter() {
    let md = "---\ntitle: Metadata\n\nBody\n===\n";
    assert_eq!(
        validate_gfm_basics(md),
        ["line 5: setext heading (=== style), use ATX (# style)"]
    );
}

#[test]
fn test_gfm_does_not_treat_a_thematic_break_as_yaml_front_matter() {
    let md = "---\n\nBody\n---\n";
    assert_eq!(
        validate_gfm_basics(md),
        ["line 4: setext heading (--- style), use ATX (# style)"]
    );
}

#[test]
fn test_gfm_missing_blank_before_heading() {
    let md = "Some text\n# Heading\n";
    let violations = validate_gfm_basics(md);
    assert!(
        violations
            .iter()
            .any(|v| v.contains("missing blank line before heading"))
    );
}

#[test]
fn test_gfm_escaped_brackets() {
    let md = "Text with \\[escaped\\] brackets\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("escaped bracket")));
}

#[test]
fn test_gfm_escaped_brackets_in_code_ok() {
    let md = "Text with `\\[code\\]` is fine\n";
    let violations = validate_gfm_basics(md);
    assert!(!violations.iter().any(|v| v.contains("escaped bracket")));
}

#[test]
fn test_gfm_indented_code_fence() {
    let md = "  ```rust\ncode\n```\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("indented fenced code block")));
}

#[test]
fn test_gfm_valid_markdown() {
    let md = "# Heading\n\nSome text here.\n\n## Sub heading\n\nMore text.\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.is_empty(), "Expected no violations, got: {:?}", violations);
}

#[test]
fn test_gfm_valid_table() {
    let md = "# Table\n\n| Header | Col |\n| --- | --- |\n| A | B |\n";
    let violations = validate_gfm_basics(md);
    assert!(
        !violations.iter().any(|v| v.contains("table header")),
        "Valid table flagged: {:?}",
        violations
    );
}

#[test]
fn test_gfm_table_missing_separator() {
    let md = "# Table\n\n| Header | Col |\n| A | B |\n";
    let violations = validate_gfm_basics(md);
    assert!(violations.iter().any(|v| v.contains("separator row")));
}

#[test]
fn test_count_blocks_headings() {
    let md = "# H1\n\n## H2\n\n### H3\n\nSome text.\n";
    let counts = count_blocks(md);
    assert_eq!(*counts.get("headings").unwrap_or(&0), 3);
}

#[test]
fn test_count_blocks_list_items() {
    let md = "- one\n- two\n- three\n";
    let counts = count_blocks(md);
    assert_eq!(*counts.get("list_items").unwrap_or(&0), 3);
}

#[test]
fn test_count_blocks_code_blocks() {
    let md = "```rust\nfn main() {}\n```\n\n```\nplain\n```\n";
    let counts = count_blocks(md);
    assert_eq!(*counts.get("code_blocks").unwrap_or(&0), 2);
}

#[test]
fn test_count_blocks_table_rows() {
    let md = "| A | B |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n";
    let counts = count_blocks(md);
    assert_eq!(*counts.get("table_rows").unwrap_or(&0), 3);
}

#[test]
fn test_is_table_separator() {
    assert!(is_table_separator("| --- | --- |"));
    assert!(is_table_separator("|---|---|"));
    assert!(is_table_separator("| :---: | ---: |"));
    assert!(!is_table_separator("| data | here |"));
}

#[test]
fn test_strip_inline_code() {
    assert_eq!(strip_inline_code("hello `world` foo"), "hello  foo");
    assert_eq!(strip_inline_code("no code here"), "no code here");
    assert_eq!(strip_inline_code("`all code`"), "");
}
