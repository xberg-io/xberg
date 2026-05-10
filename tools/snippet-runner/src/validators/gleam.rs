use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct GleamValidator;

impl GleamValidator {
    /// Dedent code that has uniform leading whitespace (from markdown indentation).
    fn dedent(code: &str) -> String {
        let min_indent = code
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        if min_indent == 0 {
            return code.to_string();
        }

        code.lines()
            .map(|l| {
                if l.trim().is_empty() {
                    ""
                } else if l.len() > min_indent {
                    &l[min_indent..]
                } else {
                    l.trim()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Wrap a Gleam snippet so it forms a complete, type-checkable module.
    ///
    /// If the snippet already declares `pub fn main(`, it is returned as-is (after dedent).
    /// Otherwise we emit a minimal module with common imports, a `pub fn main()` containing
    /// only `todo as "snippet"` (so we never need a runtime), and a `pub fn _snippet_body()`
    /// that wraps the original body so the type-checker exercises it.
    fn wrap_if_fragment(code: &str) -> String {
        let code = Self::dedent(code);
        let trimmed = code.trim();

        // Already has a main function — treat as a complete module
        if trimmed.contains("pub fn main(") {
            return code;
        }

        // Separate import lines from body so imports stay at the top of the module.
        let mut imports: Vec<&str> = Vec::new();
        let mut body: Vec<&str> = Vec::new();
        let mut past_imports = false;

        for line in code.lines() {
            let t = line.trim();
            if !past_imports && (t.starts_with("import ") || t.is_empty()) {
                imports.push(line);
            } else {
                past_imports = true;
                body.push(line);
            }
        }

        let body_str = body.join("\n");
        let imports_str = imports.join("\n");

        let mut out = String::new();
        if !imports_str.trim().is_empty() {
            out.push_str(imports_str.trim());
            out.push('\n');
        }
        out.push_str("\npub fn main() {\n  todo as \"snippet\"\n}\n");
        out.push_str("\npub fn _snippet_body() {\n");
        out.push_str(&body_str);
        out.push_str("\n}\n");
        out
    }
}

impl SnippetValidator for GleamValidator {
    fn language(&self) -> Language {
        Language::Gleam
    }

    fn is_available(&self) -> bool {
        which::which("gleam").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        // Scaffold a minimal Gleam project so `gleam check` has something to operate on.
        let gleam_toml = "name = \"snippet_check\"\nversion = \"0.1.0\"\n";
        std::fs::write(dir.path().join("gleam.toml"), gleam_toml)?;

        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir)?;

        let code = Self::wrap_if_fragment(&snippet.code);
        let mut file = std::fs::File::create(src_dir.join("snippet_check.gleam"))?;
        file.write_all(code.as_bytes())?;

        // Gleam has no separate syntax-only mode; `gleam check` covers Syntax and Compile.
        // Run level is intentionally clamped to Compile via max_level().
        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile | ValidationLevel::Run => {
                let mut c = std::process::Command::new("gleam");
                c.args(["check"]).current_dir(dir.path());
                c
            }
        };

        let (success, output) = run_command(&mut cmd, timeout_secs)?;

        if success {
            Ok((SnippetStatus::Pass, None))
        } else {
            Ok((SnippetStatus::Fail, Some(output)))
        }
    }

    fn max_level(&self) -> ValidationLevel {
        ValidationLevel::Compile
    }

    fn is_dependency_error(&self, output: &str) -> bool {
        let error_lines: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty()
                    && (t.contains("error")
                        || t.contains("Error")
                        || t.contains("Unknown")
                        || t.contains("unknown")
                        || t.contains("not found")
                        || t.contains("could not")
                        || t.contains("cannot "))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("Unknown module")
                || line.contains("Unknown type")
                || line.contains("Unknown variable")
                || line.contains("Unknown record")
                || line.contains("Unknown function")
                || line.contains("unknown identifier")
                || line.contains("could not find")
                || line.contains("cannot find")
                || line.contains("not found")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedent_removes_uniform_indent() {
        let input = "    let x = 1\n    let y = 2";
        let output = GleamValidator::dedent(input);
        assert_eq!(output, "let x = 1\nlet y = 2");
    }

    #[test]
    fn test_dedent_preserves_relative_indent() {
        let input = "    pub fn foo() {\n      let x = 1\n    }";
        let output = GleamValidator::dedent(input);
        assert_eq!(output, "pub fn foo() {\n  let x = 1\n}");
    }

    #[test]
    fn test_dedent_handles_blank_lines() {
        let input = "    let x = 1\n\n    let y = 2";
        let output = GleamValidator::dedent(input);
        assert_eq!(output, "let x = 1\n\nlet y = 2");
    }

    #[test]
    fn test_dedent_no_indent_unchanged() {
        let input = "let x = 1\nlet y = 2";
        let output = GleamValidator::dedent(input);
        assert_eq!(output, "let x = 1\nlet y = 2");
    }

    #[test]
    fn test_wrap_if_fragment_preserves_complete_module() {
        let input = "pub fn main() {\n  io.println(\"hi\")\n}";
        let output = GleamValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_wraps_bare_body() {
        let input = "let x = 1";
        let output = GleamValidator::wrap_if_fragment(input);
        assert!(output.contains("pub fn main()"));
        assert!(output.contains("todo as \"snippet\""));
        assert!(output.contains("pub fn _snippet_body()"));
        assert!(output.contains("let x = 1"));
    }

    #[test]
    fn test_wrap_if_fragment_keeps_imports_at_top() {
        let input = "import gleam/io\n\nio.println(\"hi\")";
        let output = GleamValidator::wrap_if_fragment(input);
        let import_pos = output.find("import gleam/io").expect("import present");
        let main_pos = output.find("pub fn main()").expect("main present");
        let body_pos = output.find("pub fn _snippet_body()").expect("body wrapper present");
        assert!(import_pos < main_pos);
        assert!(main_pos < body_pos);
        assert!(output.contains("io.println(\"hi\")"));
    }
}
