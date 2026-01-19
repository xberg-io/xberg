//! Utility functions for LaTeX parsing.
//!
//! This module contains helper functions for text cleaning, brace extraction,
//! and other common operations used throughout the LaTeX parser.

/// Extracts content from within braces for a given command.
///
/// Example: `\title{Hello World}` with command "title" returns "Hello World"
pub fn extract_braced(text: &str, command: &str) -> Option<String> {
    let pattern = format!("\\{}{{", command);
    if let Some(start) = text.find(&pattern) {
        let after = &text[start + pattern.len()..];
        let mut depth = 1;
        let mut content = String::new();

        for ch in after.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    content.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(clean_text(&content));
                    }
                    content.push(ch);
                }
                _ => content.push(ch),
            }
        }
    }
    None
}

/// Reads braced content from a character iterator.
///
/// Handles nested braces correctly and maintains proper depth tracking.
pub fn read_braced_from_chars(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Option<String> {
    // Skip whitespace before opening brace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    // Check for opening brace
    if chars.peek() != Some(&'{') {
        return None;
    }
    chars.next(); // Consume '{'

    let mut content = String::new();
    let mut depth = 1;

    for c in chars.by_ref() {
        match c {
            '{' => {
                depth += 1;
                content.push(c);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content);
                }
                content.push(c);
            }
            _ => content.push(c),
        }
    }

    Some(content)
}

/// Extracts environment name from a \begin{} statement.
///
/// Example: `\begin{itemize}` returns "itemize"
pub fn extract_env_name(line: &str) -> Option<String> {
    if let Some(start) = line.find("\\begin{") {
        let after = &line[start + 7..];
        if let Some(end) = after.find('}') {
            return Some(after[..end].to_string());
        }
    }
    None
}

/// Cleans LaTeX text by removing escape sequences.
///
/// Handles common LaTeX escape sequences like \\&, \\#, \\\_, etc.
pub fn clean_text(text: &str) -> String {
    text.to_string()
        .replace("\\\\", "\n")
        .replace("\\&", "&")
        .replace("\\#", "#")
        .replace("\\_", "_")
        .replace("\\{", "{")
        .replace("\\}", "}")
        .replace("\\%", "%")
        .trim()
        .to_string()
}

/// Collects content of an environment from begin to end.
///
/// Returns the content and the index of the line after \end{environment}.
pub fn collect_environment(
    lines: &[&str],
    start_idx: usize,
    env_name: &str,
) -> (String, usize) {
    let mut content = String::new();
    let mut i = start_idx + 1;
    let end_marker = format!("\\end{{{}}}", env_name);

    while i < lines.len() {
        let line = lines[i];
        if line.trim().contains(&end_marker) {
            return (content, i + 1);
        }
        content.push_str(line);
        content.push('\n');
        i += 1;
    }

    (content, i)
}
