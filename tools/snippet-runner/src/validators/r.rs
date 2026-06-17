use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::NamedTempFile;

pub struct RValidator;

impl SnippetValidator for RValidator {
    fn language(&self) -> Language {
        Language::R
    }

    fn is_available(&self) -> bool {
        which::which("Rscript").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        // Detect roxygen-style signatures: `func(args) -> Type`
        let trimmed = snippet.code.trim();
        if trimmed.lines().count() <= 3 && trimmed.contains(" -> ") {
            return Ok((SnippetStatus::Pass, None));
        }

        let mut tmp = NamedTempFile::with_suffix(".R")?;
        tmp.write_all(snippet.code.as_bytes())?;
        tmp.flush()?;

        let path = tmp.path().to_string_lossy().to_string();

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("Rscript");
                c.args(["-e", &format!("parse(file='{path}')")]);
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("Rscript");
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
