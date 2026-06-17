use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use tempfile::TempDir;

pub struct PythonValidator;

impl PythonValidator {
    /// Add `...` body to bare function/class signatures and fix indented fragments.
    fn patch_code(code: &str) -> String {
        let trimmed = code.trim();

        // Detect indented fragments (code that starts with whitespace) — dedent
        if trimmed.starts_with(' ') || trimmed.starts_with('\t') {
            let min_indent = trimmed
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.len() - l.trim_start().len())
                .min()
                .unwrap_or(0);
            if min_indent > 0 {
                let dedented: Vec<&str> = trimmed
                    .lines()
                    .map(|l| {
                        if l.trim().is_empty() {
                            ""
                        } else if l.len() > min_indent {
                            &l[min_indent..]
                        } else {
                            l.trim()
                        }
                    })
                    .collect();
                return Self::patch_signatures(&dedented.join("\n"));
            }
        }

        Self::patch_signatures(code)
    }

    /// Add `...` body to bare function/class signatures.
    fn patch_signatures(code: &str) -> String {
        let lines: Vec<&str> = code.lines().collect();
        let mut output = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            output.push(lines[i].to_string());

            // Check if this line or the next ends a function/class signature
            let trimmed = lines[i].trim();
            let is_def_start =
                trimmed.starts_with("def ") || trimmed.starts_with("async def ") || trimmed.starts_with("class ");

            if is_def_start {
                // Find end of signature (might be multi-line)
                let mut sig_end = i;
                let mut has_inline_body = false; // body on the same line as colon
                while sig_end < lines.len() {
                    let t = lines[sig_end].trim();
                    if t.ends_with(':') {
                        break;
                    }
                    // Check for colon followed by body on the same line
                    // e.g. `def foo() -> int: ...` or `def foo(): pass`
                    if let Some(arrow_pos) = t.find("->") {
                        let after_arrow = &t[arrow_pos + 2..];
                        if let Some(colon_pos) = after_arrow.find(':') {
                            let after_colon = after_arrow[colon_pos + 1..].trim();
                            if !after_colon.is_empty() {
                                has_inline_body = true;
                                break;
                            }
                            // Just ends with colon after return type
                            break;
                        }
                        // Bare return type annotation without colon — like `) -> Type`
                        let last = output.len() - 1;
                        if sig_end == i {
                            output[last] = format!("{}:", lines[sig_end]);
                        }
                        break;
                    }
                    // Check for ): body pattern (no return type)
                    if t.contains("): ") || t.contains("):\t") {
                        has_inline_body = true;
                        break;
                    }
                    // Bare signature ending with ) but no colon — like `def foo(x)`
                    if t.ends_with(')') && sig_end > i {
                        let last = output.len() - 1;
                        output[last] = format!("{}:", output[last]);
                        break;
                    }
                    if sig_end > i {
                        output.push(lines[sig_end].to_string());
                    }
                    sig_end += 1;
                }

                // If we ran past the end without finding ':' — it's a bare signature
                if sig_end >= lines.len() {
                    // Ensure last output line ends with ':'
                    let last = output.len() - 1;
                    if !output[last].trim().ends_with(':') {
                        output[last] = format!("{}:", output[last]);
                    }
                    let indent = lines[i].chars().take_while(|c| c.is_whitespace()).count();
                    let body_indent = " ".repeat(indent + 4);
                    output.push(format!("{body_indent}..."));
                    i = sig_end;
                    continue;
                }

                // If the body is inline (on the same line as the colon), skip body generation
                if has_inline_body {
                    i = sig_end + 1;
                    continue;
                }

                // Check if next non-empty line is indented (has a body)
                let next_content = (sig_end + 1..lines.len())
                    .find(|&j| !lines[j].trim().is_empty())
                    .map(|j| lines[j]);

                let has_body = next_content.is_some_and(|l| l.starts_with(' ') || l.starts_with('\t'));

                if !has_body {
                    // Add `...` as body
                    let indent = lines[i].chars().take_while(|c| c.is_whitespace()).count();
                    let body_indent = " ".repeat(indent + 4);
                    // Ensure last line of signature ends with ':'
                    let last = output.len() - 1;
                    if !output[last].trim().ends_with(':') {
                        output[last] = format!("{}:", output[last]);
                    }
                    output.push(format!("{body_indent}..."));
                }

                i = sig_end + 1;
                continue;
            }
            i += 1;
        }

        output.join("\n")
    }
}

impl SnippetValidator for PythonValidator {
    fn language(&self) -> Language {
        Language::Python
    }

    fn is_available(&self) -> bool {
        which::which("python3").is_ok() || which::which("python").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;
        let code = Self::patch_code(&snippet.code);
        let snippet_path = dir.path().join("snippet.py");
        std::fs::write(&snippet_path, &code)?;

        let python = if which::which("python3").is_ok() {
            "python3"
        } else {
            "python"
        };

        let path = snippet_path.to_string_lossy().to_string();

        let mut cmd = match level {
            ValidationLevel::Syntax => {
                let checker_path = dir.path().join("check.py");
                let checker = "\
import ast, sys
try:
    with open(sys.argv[1]) as f:
        ast.parse(f.read())
except SyntaxError as e:
    print(f\"{e}\", file=sys.stderr)
    sys.exit(1)
";
                std::fs::write(&checker_path, checker)?;

                let mut c = std::process::Command::new(python);
                c.args([checker_path.to_string_lossy().as_ref(), &path]);
                c
            }
            ValidationLevel::Compile => {
                let mut c = std::process::Command::new(python);
                c.args(["-m", "py_compile", &path]);
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new(python);
                c.arg(&path);
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
        ValidationLevel::Run
    }

    fn is_dependency_error(&self, output: &str) -> bool {
        // Python syntax errors from ast.parse are always real syntax errors,
        // but some common patterns from incomplete snippets should be treated as dep errors
        output.contains("unexpected indent") || output.contains("was never closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_class_with_inline_methods() {
        let code = r#"class ExtractionResult:
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[ExtractedTable]
    detected_languages: list[str] | None
    chunks: list[Chunk] | None
    images: list[ExtractedImage] | None
    pages: list[PageContent] | None
    elements: list[Element] | None
    djot_content: DjotContent | None
    output_format: str | None
    result_format: str | None
    def get_page_count(self) -> int: ...
    def get_chunk_count(self) -> int: ...
    def get_detected_language(self) -> str | None: ...
    def get_metadata_field(self, field_name: str) -> Any | None: ..."#;

        let patched = PythonValidator::patch_code(code);
        eprintln!("PATCHED OUTPUT:");
        for (i, line) in patched.lines().enumerate() {
            eprintln!("  {:3} | {}", i + 1, line);
        }
        // The patched code should be identical to the input
        // (class has a body, methods have inline bodies)
        assert_eq!(patched.trim(), code.trim());
    }
}
