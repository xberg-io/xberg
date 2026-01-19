//! LaTeX command processing.
//!
//! This module handles inline LaTeX commands like formatting (\textbf, \emph, etc.),
//! math mode ($...$), and other inline elements.

use super::utilities::read_braced_from_chars;

/// Processes a line of LaTeX, handling commands and inline math.
///
/// Recursively processes nested commands and preserves math mode content.
pub fn process_line(line: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let mut cmd = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphabetic() {
                    cmd.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            process_command(&cmd, &mut chars, &mut result);
        } else if ch == '$' {
            // Handle inline math
            result.push(ch);
            while let Some(&c) = chars.peek() {
                result.push(chars.next().unwrap());
                if c == '$' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Processes a single LaTeX command.
///
/// Handles formatting commands (\textbf, \emph, etc.) and extracts their content.
fn process_command(
    cmd: &str,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
) {
    match cmd {
        "textbf" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "textit" | "emph" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "texttt" => {
            if let Some(content) = read_braced_from_chars(chars) {
                result.push_str(&content);
            }
        }
        "underline" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "font" => {
            // Skip font commands
            while let Some(&c) = chars.peek() {
                if c == '\\' {
                    break;
                }
                chars.next();
            }
        }
        "usepackage" => {
            // Skip package declarations
            read_braced_from_chars(chars);
        }
        _ => {
            // For unknown commands, try to extract and process content
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
    }
}
