use crate::types::Language;
use std::path::Path;

/// A parsed code block extracted from a markdown file or raw source file.
#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub lang: String,
    pub title: Option<String>,
    pub code: String,
    pub start_line: usize,
    pub preceding_comment: Option<String>,
}

/// Extract fenced code blocks from markdown content.
///
/// Recognizes blocks like:
/// ````markdown
/// ```rust title="example"
/// code here
/// ```
/// ````
pub fn extract_fenced_blocks(content: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("```") {
            // Opening fence — parse lang + attributes
            if rest.is_empty() || rest.starts_with('`') {
                // Bare ``` or ````+ — skip
                i += 1;
                continue;
            }

            let (lang, title) = parse_fence_info(rest);
            if lang.is_empty() {
                i += 1;
                continue;
            }

            // Capture preceding HTML comment for annotation
            let preceding_comment = if i > 0 {
                let prev = lines[i - 1].trim();
                if prev.starts_with("<!--") && prev.ends_with("-->") {
                    Some(prev.to_string())
                } else {
                    None
                }
            } else {
                None
            };

            let start_line = i + 1; // 1-indexed
            let mut code_lines = Vec::new();
            i += 1;

            // Collect until closing fence
            while i < lines.len() {
                let cl = lines[i].trim();
                if cl == "```" || cl.starts_with("```") && cl.chars().skip(3).all(|c| c == '`') {
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }

            let code = code_lines.join("\n");
            if !code.trim().is_empty() {
                blocks.push(CodeBlock {
                    lang,
                    title,
                    code,
                    start_line,
                    preceding_comment,
                });
            }
        }
        i += 1;
    }

    blocks
}

/// Parse code blocks from a file. Handles both markdown files and raw source files.
pub fn parse_code_blocks(path: &Path) -> crate::error::Result<Vec<CodeBlock>> {
    let content = std::fs::read_to_string(path).map_err(|e| crate::error::Error::Parse {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    if ext == "md" || ext == "markdown" {
        // Markdown file — extract fenced blocks
        Ok(extract_fenced_blocks(&content))
    } else {
        // Raw source file — check if it contains markdown fences
        let fenced = extract_fenced_blocks(&content);
        if !fenced.is_empty() {
            return Ok(fenced);
        }

        // Treat entire file as a single code block
        let lang = Language::from_extension(&ext);
        if lang == Language::Unknown {
            return Ok(Vec::new());
        }

        Ok(vec![CodeBlock {
            lang: lang.to_string(),
            title: path.file_name().and_then(|n| n.to_str()).map(String::from),
            code: content,
            start_line: 1,
            preceding_comment: None,
        }])
    }
}

/// Parse the info string after ``` to extract language and title.
/// Examples: `rust title="example"`, `python`, `go title="basic_usage.go"`
fn parse_fence_info(info: &str) -> (String, Option<String>) {
    let info = info.trim();

    // Split on whitespace to get lang + rest
    let mut parts = info.splitn(2, char::is_whitespace);
    let lang = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("");

    // Parse title="..." attribute
    let title = parse_title_attr(rest);

    (lang, title)
}

fn parse_title_attr(attrs: &str) -> Option<String> {
    // Match title="..." or title='...'
    let attrs = attrs.trim();
    if let Some(after) = attrs.strip_prefix("title=") {
        let after = after.trim();
        if let Some(stripped) = after.strip_prefix('"') {
            let end = stripped.find('"')?;
            return Some(stripped[..end].to_string());
        }
        if let Some(stripped) = after.strip_prefix('\'') {
            let end = stripped.find('\'')?;
            return Some(stripped[..end].to_string());
        }
        // No quotes — take until whitespace
        let val: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
        if !val.is_empty() {
            return Some(val);
        }
    }

    // Also search for title= anywhere in attributes
    for part in attrs.split_whitespace() {
        if let Some(after) = part.strip_prefix("title=") {
            let after = after.trim_matches(|c| c == '"' || c == '\'');
            if !after.is_empty() {
                return Some(after.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_block() {
        let md = r#"
Some text

```rust title="example"
fn main() {
    println!("hello");
}
```

More text
"#;
        let blocks = extract_fenced_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, "rust");
        assert_eq!(blocks[0].title.as_deref(), Some("example"));
        assert!(blocks[0].code.contains("println!"));
    }

    #[test]
    fn test_extract_multiple_blocks() {
        let md = r#"
```python
import os
```

```rust
fn foo() {}
```
"#;
        let blocks = extract_fenced_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].lang, "python");
        assert_eq!(blocks[1].lang, "rust");
    }

    #[test]
    fn test_extract_with_annotation() {
        let md = r#"
<!-- snippet:skip -->
```rust
fn skipped() {}
```
"#;
        let blocks = extract_fenced_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].preceding_comment.as_deref(), Some("<!-- snippet:skip -->"));
    }

    #[test]
    fn test_parse_fence_info() {
        let (lang, title) = parse_fence_info("rust title=\"my_example\"");
        assert_eq!(lang, "rust");
        assert_eq!(title.as_deref(), Some("my_example"));

        let (lang, title) = parse_fence_info("python");
        assert_eq!(lang, "python");
        assert!(title.is_none());

        let (lang, title) = parse_fence_info("go title=\"basic_usage.go\"");
        assert_eq!(lang, "go");
        assert_eq!(title.as_deref(), Some("basic_usage.go"));
    }

    #[test]
    fn test_bare_fence_skipped() {
        let md = "```\nsome code\n```\n";
        let blocks = extract_fenced_blocks(md);
        assert!(blocks.is_empty());
    }
}
