pub mod bash;
pub mod c;
pub mod csharp;
pub mod elixir;
pub mod go;
pub mod java;
pub mod php;
pub mod python;
pub mod r;
pub mod ruby;
pub mod rust;
pub mod toml_validator;
pub mod typescript;

use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use std::collections::HashMap;

/// Trait for language-specific snippet validators.
pub trait SnippetValidator: Send + Sync {
    fn language(&self) -> Language;
    fn is_available(&self) -> bool;
    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)>;
    fn max_level(&self) -> ValidationLevel;

    /// Returns true if the error output indicates only dependency/import resolution
    /// failures (not actual syntax errors). Used at syntax level to pass snippets
    /// that are syntactically correct but reference project-specific types/modules.
    fn is_dependency_error(&self, _error_output: &str) -> bool {
        false
    }
}

/// Registry of validators keyed by language.
pub struct ValidatorRegistry {
    validators: HashMap<Language, Box<dyn SnippetValidator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            validators: HashMap::new(),
        };

        reg.register(Box::new(rust::RustValidator));
        reg.register(Box::new(python::PythonValidator));
        reg.register(Box::new(typescript::TypeScriptValidator));
        reg.register(Box::new(go::GoValidator));
        reg.register(Box::new(java::JavaValidator));
        reg.register(Box::new(csharp::CSharpValidator));
        reg.register(Box::new(php::PhpValidator));
        reg.register(Box::new(ruby::RubyValidator));
        reg.register(Box::new(elixir::ElixirValidator));
        reg.register(Box::new(r::RValidator));
        reg.register(Box::new(c::CValidator));
        reg.register(Box::new(bash::BashValidator));
        reg.register(Box::new(toml_validator::TomlValidator));

        reg
    }

    fn register(&mut self, validator: Box<dyn SnippetValidator>) {
        self.validators.insert(validator.language(), validator);
    }

    pub fn get(&self, lang: Language) -> Option<&dyn SnippetValidator> {
        self.validators.get(&lang).map(|v| v.as_ref())
    }

    pub fn available_languages(&self) -> Vec<Language> {
        let mut langs: Vec<_> = self
            .validators
            .iter()
            .filter(|(_, v)| v.is_available())
            .map(|(l, _)| *l)
            .collect();
        langs.sort_by_key(|l| l.to_string());
        langs
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip ANSI escape codes from a string.
/// Removes CSI color/style sequences matching \x1b[...m
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC[...m sequences
            if let Some('[') = chars.next() {
                // Consume until 'm' or end
                for c2 in chars.by_ref() {
                    if c2 == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Run a command with timeout. Returns (success, stdout+stderr).
pub fn run_command(cmd: &mut std::process::Command, timeout_secs: u64) -> Result<(bool, String)> {
    use std::io::Read;

    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| crate::error::Error::Other(format!("spawn failed: {e}")))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    // Wait with timeout
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => {
            let mut output = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                let _ = stdout.read_to_string(&mut output);
            }
            if let Some(mut stderr) = child.stderr.take() {
                let _ = stderr.read_to_string(&mut output);
            }
            // Strip ANSI codes to ensure clean output for validators
            let clean_output = strip_ansi_codes(&output);
            Ok((status.success(), clean_output))
        }
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(crate::error::Error::Timeout {
                command: format!("{cmd:?}"),
                timeout_secs,
            })
        }
        Err(e) => Err(crate::error::Error::Other(format!("wait failed: {e}"))),
    }
}

/// Trait extension for Command to add timeout support.
trait WaitTimeout {
    fn wait_timeout(&mut self, timeout: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, timeout: std::time::Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        let poll_interval = std::time::Duration::from_millis(50);

        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes_color_sequences() {
        // Test basic color code stripping
        let input = "\x1b[1m\x1b[91merror\x1b[0m: something went wrong";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "error: something went wrong");
    }

    #[test]
    fn test_strip_ansi_codes_multiple_sequences() {
        // Test multiple color sequences in one string
        let input = "\x1b[1mBold\x1b[0m \x1b[32mGreen\x1b[0m text";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "Bold Green text");
    }

    #[test]
    fn test_strip_ansi_codes_no_codes() {
        // Test string with no ANSI codes
        let input = "plain text without codes";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "plain text without codes");
    }

    #[test]
    fn test_strip_ansi_codes_empty_string() {
        // Test empty string
        let input = "";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_ansi_codes_only_codes() {
        // Test string with only ANSI codes
        let input = "\x1b[1m\x1b[91m\x1b[0m";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_ansi_codes_rust_error_pattern() {
        // Test the specific pattern from Rust compiler error output
        let input = "\x1b[1m\x1b[91merror\x1b[0m[E0432]: unresolved import `foo`";
        let output = strip_ansi_codes(input);
        assert_eq!(output, "error[E0432]: unresolved import `foo`");
    }
}
