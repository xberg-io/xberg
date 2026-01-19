use ahash::AHashMap;
use once_cell::sync::Lazy;
use regex::Regex;

/// Regular expression for matching Markdown code blocks.
/// Matches triple-backtick code blocks: ```...```
static MARKDOWN_CODE_BLOCK_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"```[\s\S]*?```").expect("Markdown code block regex pattern is valid and should compile"));

/// Regular expression for matching Markdown inline code.
/// Matches single-backtick inline code: `code`
static MARKDOWN_INLINE_CODE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"`[^`\n]+`").expect("Markdown inline code regex pattern is valid and should compile"));

/// Regular expression for matching Markdown headers.
/// Matches headers like `# Header`, `## Header`, etc.
static MARKDOWN_HEADERS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#{1,6}\s+").expect("Markdown headers regex pattern is valid and should compile"));

/// Regular expression for matching Markdown list items.
/// Matches list markers: `- `, `* `, `+ ` at the start of lines
static MARKDOWN_LISTS_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[ \t]*[-*+]\s+").expect("Markdown lists regex pattern is valid and should compile"));

/// Extracts and preserves Markdown code blocks and inline code by replacing them with placeholders.
///
/// This function scans the input text for Markdown code blocks (``` ... ```) and inline code (` ... `),
/// replaces them with unique placeholders, and stores the original content in a hashmap.
///
/// # Arguments
/// * `text` - The input text containing Markdown code
/// * `preserved` - A mutable hashmap to store the preserved code blocks
///
/// # Returns
/// A new `String` with code blocks replaced by placeholders
pub fn extract_and_preserve_code(text: &str, preserved: &mut AHashMap<String, String>) -> String {
    let mut result = text.to_string();
    let mut code_block_id = 0;
    let mut inline_code_id = 0;

    // Extract code blocks first
    result = MARKDOWN_CODE_BLOCK_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let code_block = caps[0].to_string();
            let placeholder = format!("__CODEBLOCK_{}__", code_block_id);
            code_block_id += 1;
            preserved.insert(placeholder.clone(), code_block);
            placeholder
        })
        .to_string();

    // Extract inline code
    result = MARKDOWN_INLINE_CODE_REGEX
        .replace_all(&result, |caps: &regex::Captures| {
            let inline_code = caps[0].to_string();
            let placeholder = format!("__INLINECODE_{}__", inline_code_id);
            inline_code_id += 1;
            preserved.insert(placeholder.clone(), inline_code);
            placeholder
        })
        .to_string();

    result
}

/// Restores preserved code blocks by replacing placeholders with their original content.
///
/// # Arguments
/// * `text` - The text containing placeholders
/// * `preserved` - The hashmap containing the original code blocks
///
/// # Returns
/// A new `String` with placeholders replaced by their original content
pub fn restore_preserved_blocks(text: &str, preserved: &AHashMap<String, String>) -> String {
    if preserved.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    for (placeholder, original_content) in preserved {
        result = result.replace(placeholder, original_content);
    }

    result
}

/// Preserves Markdown structure elements like headers, lists, and tables.
///
/// This function processes text line-by-line and preserves lines that contain
/// Markdown structural elements without modification.
///
/// # Arguments
/// * `text` - The input text with Markdown structure
///
/// # Returns
/// A new `String` with Markdown structure preserved
pub fn preserve_markdown_structure(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut processed_lines = Vec::with_capacity(lines.len());

    for line in lines {
        // Preserve headers
        if MARKDOWN_HEADERS_REGEX.is_match(line) {
            processed_lines.push(line);
            continue;
        }

        // Preserve list items
        if MARKDOWN_LISTS_REGEX.is_match(line) {
            processed_lines.push(line);
            continue;
        }

        processed_lines.push(line);
    }

    processed_lines.join("\n")
}

/// Checks if a line is a Markdown header.
///
/// # Arguments
/// * `line` - The line to check
///
/// # Returns
/// `true` if the line is a Markdown header, `false` otherwise
#[inline]
pub fn is_markdown_header(line: &str) -> bool {
    MARKDOWN_HEADERS_REGEX.is_match(line)
}

/// Checks if a line is a Markdown list item.
///
/// # Arguments
/// * `line` - The line to check
///
/// # Returns
/// `true` if the line is a Markdown list item, `false` otherwise
#[inline]
pub fn is_markdown_list(line: &str) -> bool {
    MARKDOWN_LISTS_REGEX.is_match(line)
}

/// Checks if a line is a Markdown table row.
///
/// # Arguments
/// * `line` - The line to check
///
/// # Returns
/// `true` if the line appears to be a Markdown table row, `false` otherwise
#[inline]
pub fn is_markdown_table(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_block() {
        let mut preserved = AHashMap::new();
        let input = "Text before\n```rust\nfn main() {}\n```\nText after";
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v.contains("fn main()")));
        assert!(result.contains("__CODEBLOCK_0__"));
    }

    #[test]
    fn test_extract_inline_code() {
        let mut preserved = AHashMap::new();
        let input = "Use the `println!` macro";
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 1);
        assert!(preserved.values().any(|v| v == "`println!`"));
        assert!(result.contains("__INLINECODE_0__"));
    }

    #[test]
    fn test_multiple_code_blocks() {
        let input =
            "Start ```rust\nlet x = 1;\n``` middle `inline1` text ```python\nprint('hi')\n``` and `inline2` end";
        let mut preserved = AHashMap::new();
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 4);
        assert!(preserved.contains_key("__CODEBLOCK_0__"));
        assert!(preserved.contains_key("__CODEBLOCK_1__"));
        assert!(preserved.contains_key("__INLINECODE_0__"));
        assert!(preserved.contains_key("__INLINECODE_1__"));

        assert_eq!(preserved.get("__CODEBLOCK_0__").unwrap(), "```rust\nlet x = 1;\n```");
        assert_eq!(preserved.get("__CODEBLOCK_1__").unwrap(), "```python\nprint('hi')\n```");
        assert_eq!(preserved.get("__INLINECODE_0__").unwrap(), "`inline1`");
        assert_eq!(preserved.get("__INLINECODE_1__").unwrap(), "`inline2`");

        let restored = restore_preserved_blocks(&result, &preserved);
        assert!(restored.contains("```rust\nlet x = 1;\n```"));
        assert!(restored.contains("```python\nprint('hi')\n```"));
        assert!(restored.contains("`inline1`"));
        assert!(restored.contains("`inline2`"));
        assert!(!restored.contains("__CODEBLOCK_"));
        assert!(!restored.contains("__INLINECODE_"));
    }

    #[test]
    fn test_restore_preserved_blocks() {
        let mut preserved = AHashMap::new();
        preserved.insert("__CODEBLOCK_0__".to_string(), "```code```".to_string());
        preserved.insert("__INLINECODE_0__".to_string(), "`inline`".to_string());
        let input = "Text __CODEBLOCK_0__ and __INLINECODE_0__ here";
        let result = restore_preserved_blocks(input, &preserved);

        assert!(result.contains("```code```"));
        assert!(result.contains("`inline`"));
        assert!(!result.contains("__CODEBLOCK_0__"));
        assert!(!result.contains("__INLINECODE_0__"));
    }

    #[test]
    fn test_hashmap_order_independence() {
        let input = "Text `a` and `b` and `c` here";
        let mut preserved = AHashMap::new();
        let result = extract_and_preserve_code(input, &mut preserved);

        assert_eq!(preserved.len(), 3);
        let restored = restore_preserved_blocks(&result, &preserved);

        assert!(restored.contains("`a`"));
        assert!(restored.contains("`b`"));
        assert!(restored.contains("`c`"));
        assert_eq!(restored, "Text `a` and `b` and `c` here");
    }

    #[test]
    fn test_preserve_markdown_structure() {
        let input = "# Header 1\n## Header 2\n### Header 3\nRegular text";
        let result = preserve_markdown_structure(input);

        assert!(result.contains("# Header 1"));
        assert!(result.contains("## Header 2"));
        assert!(result.contains("### Header 3"));
    }

    #[test]
    fn test_is_markdown_header() {
        assert!(is_markdown_header("# Header 1"));
        assert!(is_markdown_header("## Header 2"));
        assert!(is_markdown_header("### Header 3"));
        assert!(!is_markdown_header("Regular text"));
        assert!(!is_markdown_header("- List item"));
    }

    #[test]
    fn test_is_markdown_list() {
        assert!(is_markdown_list("- Item 1"));
        assert!(is_markdown_list("* Item 2"));
        assert!(is_markdown_list("+ Item 3"));
        assert!(is_markdown_list("  - Indented item"));
        assert!(!is_markdown_list("Regular text"));
        assert!(!is_markdown_list("# Header"));
    }

    #[test]
    fn test_is_markdown_table() {
        assert!(is_markdown_table("| Header 1 | Header 2 |"));
        assert!(is_markdown_table("|----------|----------|"));
        assert!(is_markdown_table("| Cell 1   | Cell 2   |"));
        assert!(!is_markdown_table("Regular text"));
        assert!(!is_markdown_table("- List item"));
    }

    #[test]
    fn test_lazy_regex_initialization() {
        let _ = &*MARKDOWN_CODE_BLOCK_REGEX;
        let _ = &*MARKDOWN_INLINE_CODE_REGEX;
        let _ = &*MARKDOWN_HEADERS_REGEX;
        let _ = &*MARKDOWN_LISTS_REGEX;
    }
}
