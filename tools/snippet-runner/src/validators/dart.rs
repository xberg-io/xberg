use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct DartValidator;

impl DartValidator {
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

    /// Wrap a snippet as a complete Dart program if it appears to be a fragment.
    ///
    /// A snippet is considered complete if it contains a `main` function or any
    /// import directive. Otherwise it is wrapped in `Future<void> main() async { ... }`.
    fn wrap_if_fragment(code: &str) -> String {
        // Dedent first to handle indented markdown snippets
        let code = Self::dedent(code);
        let trimmed = code.trim();

        // Already a complete program — has a main() function
        let has_main = trimmed.contains("void main(")
            || trimmed.contains("Future<void> main(")
            || trimmed.contains("Future main(")
            || trimmed.contains("dynamic main(");

        // Has import directives — treat as complete file
        let has_import = code.lines().any(|l| {
            let t = l.trim();
            t.starts_with("import 'dart:")
                || t.starts_with("import \"dart:")
                || t.starts_with("import 'package:")
                || t.starts_with("import \"package:")
        });

        if has_main || has_import {
            return code;
        }

        format!("Future<void> main() async {{\n{code}\n}}")
    }
}

impl SnippetValidator for DartValidator {
    fn language(&self) -> Language {
        Language::Dart
    }

    fn is_available(&self) -> bool {
        which::which("dart").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        // Scaffold a minimal pubspec so `dart analyze` and `dart run` work without
        // requiring external dependency resolution for snippets that don't import packages.
        let pubspec = "name: snippet_check\nenvironment:\n  sdk: '>=3.0.0 <4.0.0'\n";
        std::fs::write(dir.path().join("pubspec.yaml"), pubspec)?;

        let bin_dir = dir.path().join("bin");
        std::fs::create_dir_all(&bin_dir)?;

        let code = Self::wrap_if_fragment(&snippet.code);
        let main_path = bin_dir.join("main.dart");
        let mut file = std::fs::File::create(&main_path)?;
        file.write_all(code.as_bytes())?;

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("dart");
                c.args(["analyze", "--fatal-infos", "--fatal-warnings"])
                    .arg(&main_path)
                    .current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("dart");
                c.args(["run", "bin/main.dart"]).current_dir(dir.path());
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
        let error_lines: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty()
                    && (t.contains("error")
                        || t.contains("warning")
                        || t.contains("info")
                        || t.contains("Error")
                        || t.contains("Warning")
                        || t.contains("URI")
                        || t.contains("Undefined")
                        || t.contains("isn't defined")
                        || t.contains("not found")
                        || t.contains("Couldn't resolve"))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("URI doesn't exist")
                || line.contains("Target of URI doesn't exist")
                || line.contains("Undefined name")
                || line.contains("Undefined class")
                || line.contains("Undefined identifier")
                || line.contains("isn't defined")
                || line.contains("not found")
                || line.contains("Couldn't resolve")
                || line.contains("uri_does_not_exist")
                || line.contains("unused_import")
                || line.contains("unused_local_variable")
                || line.contains("unused_element")
                || line.contains("depend_on_referenced_packages")
                || line.contains("avoid_print")
                || line.contains("issue found")
                || line.contains("issues found")
                || line.contains("Analyzing")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedent_removes_uniform_indentation() {
        let input = "    line1\n    line2\n    line3";
        let output = DartValidator::dedent(input);
        assert_eq!(output, "line1\nline2\nline3");
    }

    #[test]
    fn test_dedent_preserves_relative_indentation() {
        let input = "  outer\n    inner\n  outer";
        let output = DartValidator::dedent(input);
        assert_eq!(output, "outer\n  inner\nouter");
    }

    #[test]
    fn test_dedent_no_indent_returns_unchanged() {
        let input = "line1\nline2\nline3";
        let output = DartValidator::dedent(input);
        assert_eq!(output, "line1\nline2\nline3");
    }

    #[test]
    fn test_dedent_handles_empty_lines() {
        let input = "    line1\n\n    line2";
        let output = DartValidator::dedent(input);
        assert_eq!(output, "line1\n\nline2");
    }

    #[test]
    fn test_wrap_if_fragment_complete_with_void_main() {
        let input = "void main() {\n  print('hello');\n}";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_complete_with_future_main() {
        let input = "Future<void> main() async {\n  await foo();\n}";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_complete_with_dart_import() {
        let input = "import 'dart:io';\n\nvoid foo() {}";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_complete_with_package_import() {
        let input = "import 'package:kreuzberg/kreuzberg.dart';\n\nfinal x = 1;";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_wraps_statement() {
        let input = "print('hello');";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, "Future<void> main() async {\nprint('hello');\n}");
    }

    #[test]
    fn test_wrap_if_fragment_dedents_then_wraps() {
        let input = "    final x = 42;\n    print(x);";
        let output = DartValidator::wrap_if_fragment(input);
        assert_eq!(output, "Future<void> main() async {\nfinal x = 42;\nprint(x);\n}");
    }
}
