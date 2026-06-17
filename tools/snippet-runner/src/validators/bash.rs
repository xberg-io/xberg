use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::NamedTempFile;

pub struct BashValidator;

impl SnippetValidator for BashValidator {
    fn language(&self) -> Language {
        Language::Bash
    }

    fn is_available(&self) -> bool {
        which::which("bash").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let mut tmp = NamedTempFile::with_suffix(".sh")?;
        tmp.write_all(snippet.code.as_bytes())?;
        tmp.flush()?;

        let path = tmp.path().to_string_lossy().to_string();

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("bash");
                c.args(["-n", &path]);
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("bash");
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
