use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct KotlinValidator;

impl KotlinValidator {
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

    /// Wrap a Kotlin fragment in `fun main()` if it's not already a complete file.
    ///
    /// Heuristic:
    /// - If the snippet contains `fun main(`, a `class ` or `object ` declaration,
    ///   it is treated as a complete program and returned as-is (after dedent).
    /// - Otherwise, top-level `import` lines are hoisted to the top of the file
    ///   and the remaining body is wrapped. If the body uses the `suspend`
    ///   keyword, the wrap uses `runBlocking { ... }` and adds a
    ///   `kotlinx.coroutines` import.
    fn wrap_if_fragment(code: &str) -> String {
        let code = Self::dedent(code);
        let trimmed = code.trim();

        // Already a complete program — leave as-is.
        if trimmed.contains("fun main(") || trimmed.contains("class ") || trimmed.contains("object ") {
            return code;
        }

        // Separate top-level imports from body.
        let mut imports = Vec::new();
        let mut body = Vec::new();
        for line in code.lines() {
            let t = line.trim();
            if t.starts_with("import ") {
                imports.push(line.to_string());
            } else {
                body.push(line.to_string());
            }
        }

        let body_str = body.join("\n");
        let body_trimmed = body_str.trim();

        // Detect suspending calls — if present, wrap with runBlocking.
        let needs_runblocking = body_trimmed.contains("suspend") || body_trimmed.contains(".await");

        if needs_runblocking {
            let has_coroutines_import = imports.iter().any(|l| l.contains("kotlinx.coroutines"));
            if !has_coroutines_import {
                imports.insert(0, "import kotlinx.coroutines.runBlocking".to_string());
            }
        }

        let imports_str = if imports.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", imports.join("\n"))
        };

        if needs_runblocking {
            format!("{imports_str}fun main() {{\n    runBlocking {{\n{body_str}\n    }}\n}}")
        } else {
            format!("{imports_str}fun main() {{\n{body_str}\n}}")
        }
    }
}

impl SnippetValidator for KotlinValidator {
    fn language(&self) -> Language {
        Language::Kotlin
    }

    fn is_available(&self) -> bool {
        which::which("kotlinc").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;
        let code = Self::wrap_if_fragment(&snippet.code);

        let src_path = dir.path().join("Snippet.kt");
        let mut file = std::fs::File::create(&src_path)?;
        file.write_all(code.as_bytes())?;

        let out_dir = dir.path().join("out");
        std::fs::create_dir_all(&out_dir)?;

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile | ValidationLevel::Run => {
                // Running JVM bytecode is heavy; max_level caps at Compile so we
                // only ever hit this branch for Syntax/Compile in practice. Both
                // use the same kotlinc invocation — produces a JAR but we don't run it.
                let mut c = std::process::Command::new("kotlinc");
                c.arg("-nowarn")
                    .arg("-d")
                    .arg(&out_dir)
                    .arg(&src_path)
                    .current_dir(dir.path());
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
        // Running JVM bytecode is heavy — cap at Compile.
        ValidationLevel::Compile
    }

    fn is_dependency_error(&self, output: &str) -> bool {
        let error_lines: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && (t.contains("error:") || t.contains("error "))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("unresolved reference")
                || line.contains("cannot access")
                || line.contains("cannot find symbol")
                || line.contains("unresolved import")
                || line.contains("error: package")
                || (line.contains("package ") && line.contains("is missing"))
                || line.contains("kotlinx.coroutines")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedent_strips_uniform_indentation() {
        let input = "    val x = 1\n    val y = 2";
        let output = KotlinValidator::dedent(input);
        assert_eq!(output, "val x = 1\nval y = 2");
    }

    #[test]
    fn test_dedent_no_indent_is_noop() {
        let input = "val x = 1\nval y = 2";
        let output = KotlinValidator::dedent(input);
        assert_eq!(output, "val x = 1\nval y = 2");
    }

    #[test]
    fn test_wrap_preserves_complete_program_with_main() {
        let input = "fun main() {\n    println(\"hi\")\n}";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_preserves_complete_program_with_class() {
        let input = "class Foo {\n    fun bar() = 1\n}";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_preserves_complete_program_with_object() {
        let input = "object Foo {\n    fun bar() = 1\n}";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_fragment_in_main() {
        let input = "println(\"hi\")";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert!(output.contains("fun main()"));
        assert!(output.contains("println(\"hi\")"));
        assert!(!output.contains("runBlocking"));
    }

    #[test]
    fn test_wrap_hoists_imports() {
        let input = "import kotlin.math.PI\nval x = PI";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert!(output.starts_with("import kotlin.math.PI"));
        assert!(output.contains("fun main()"));
        assert!(output.contains("val x = PI"));
    }

    #[test]
    fn test_wrap_uses_runblocking_for_suspend() {
        let input = "suspend fun fetch() = 1\nfetch()";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert!(output.contains("import kotlinx.coroutines.runBlocking"));
        assert!(output.contains("runBlocking"));
    }

    #[test]
    fn test_wrap_uses_runblocking_for_await() {
        let input = "deferred.await()";
        let output = KotlinValidator::wrap_if_fragment(input);
        assert!(output.contains("import kotlinx.coroutines.runBlocking"));
        assert!(output.contains("runBlocking"));
    }

    #[test]
    fn test_wrap_does_not_double_import_coroutines() {
        let input = "import kotlinx.coroutines.runBlocking\nsuspend fun fetch() = 1\nfetch()";
        let output = KotlinValidator::wrap_if_fragment(input);
        let count = output.matches("import kotlinx.coroutines").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_language_is_kotlin() {
        assert_eq!(KotlinValidator.language(), Language::Kotlin);
    }

    #[test]
    fn test_max_level_is_compile() {
        assert_eq!(KotlinValidator.max_level(), ValidationLevel::Compile);
    }

    #[test]
    fn test_is_dependency_error_detects_unresolved_reference() {
        let output = "Snippet.kt:3:5: error: unresolved reference: foo";
        assert!(KotlinValidator.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_detects_cannot_access() {
        let output = "Snippet.kt:3:5: error: cannot access 'Foo'";
        assert!(KotlinValidator.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_returns_false_on_empty() {
        assert!(!KotlinValidator.is_dependency_error(""));
    }

    #[test]
    fn test_is_dependency_error_returns_false_on_real_syntax_error() {
        let output = "Snippet.kt:3:5: error: expecting ')'";
        assert!(!KotlinValidator.is_dependency_error(output));
    }
}
