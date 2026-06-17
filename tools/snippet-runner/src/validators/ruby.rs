use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::NamedTempFile;

pub struct RubyValidator;

impl SnippetValidator for RubyValidator {
    fn language(&self) -> Language {
        Language::Ruby
    }

    fn is_available(&self) -> bool {
        which::which("ruby").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        // Detect YARD-style API signatures (not executable Ruby)
        // Pattern: `Module.method(args) -> ReturnType`
        let trimmed = snippet.code.trim();
        if is_api_signature(trimmed) {
            return Ok((SnippetStatus::Pass, None));
        }

        let mut tmp = NamedTempFile::with_suffix(".rb")?;
        tmp.write_all(snippet.code.as_bytes())?;
        tmp.flush()?;

        let path = tmp.path().to_string_lossy().to_string();

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("ruby");
                c.args(["-c", &path]);
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("ruby");
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
}

/// Detect YARD-style method signatures like:
/// `Kreuzberg.extract_file_sync(path, config: nil) -> Kreuzberg::Result`
fn is_api_signature(code: &str) -> bool {
    let lines: Vec<&str> = code.lines().collect();
    // Single-line or very short (1-3 lines) with `->` return type
    if lines.len() <= 3 && code.contains(" -> ") {
        // Looks like: Module.method(...) -> Type
        return true;
    }
    false
}
