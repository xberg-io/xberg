use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct SwiftValidator;

impl SwiftValidator {
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

    /// Wrap snippet if it is a fragment.
    ///
    /// Returns `(code, needs_parse_as_library)`:
    /// - If the body contains `await`/`try await`, wrap in `@main struct ... async throws` and
    ///   request `-parse-as-library` (top-level `await` is not allowed in script mode).
    /// - If the snippet already declares `@main`, leave it as-is and request `-parse-as-library`.
    /// - Otherwise, leave the snippet unwrapped — Swift's script mode permits top-level
    ///   statements alongside imports and type declarations.
    fn wrap_if_fragment(code: &str) -> (String, bool) {
        let code = Self::dedent(code);
        let trimmed = code.trim();

        // Already has @main — caller-provided complete file with an entry point.
        if trimmed.contains("@main") {
            return (code, true);
        }

        // Detect async usage in the body so we can wrap it in a proper async entry point.
        let has_async = code
            .lines()
            .map(|l| l.trim_start())
            .any(|l| l.starts_with("await ") || l.starts_with("try await ") || l.contains(" await "));

        if !has_async {
            // Sync top-level code is fine in Swift script mode.
            return (code, false);
        }

        // Split imports from body — imports must remain at the top level of the file.
        let mut imports = Vec::new();
        let mut body = Vec::new();
        for line in code.lines() {
            let t = line.trim();
            if t.starts_with("import ") {
                imports.push(line);
            } else {
                body.push(line);
            }
        }

        let imports_str = imports.join("\n");
        let body_str = body.join("\n");

        let wrapped = if imports.is_empty() {
            format!("@main\nstruct SnippetMain {{\n  static func main() async throws {{\n{body_str}\n  }}\n}}")
        } else {
            format!(
                "{imports_str}\n\n@main\nstruct SnippetMain {{\n  static func main() async throws {{\n{body_str}\n  }}\n}}"
            )
        };

        (wrapped, true)
    }
}

impl SnippetValidator for SwiftValidator {
    fn language(&self) -> Language {
        Language::Swift
    }

    fn is_available(&self) -> bool {
        which::which("swift").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        let (code, parse_as_library) = Self::wrap_if_fragment(&snippet.code);
        let file_path = dir.path().join("snippet.swift");
        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(code.as_bytes())?;

        let mut cmd = match level {
            // Both Syntax and Compile use `swift -typecheck`. `swift run` is too heavy
            // (would require building a SwiftPM package), so we cap at Compile.
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("swift");
                c.arg("-typecheck");
                if parse_as_library {
                    c.arg("-parse-as-library");
                }
                c.arg(&file_path).current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("swift");
                c.arg("-typecheck");
                if parse_as_library {
                    c.arg("-parse-as-library");
                }
                c.arg(&file_path).current_dir(dir.path());
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
                !t.is_empty() && (t.contains("error:") || t.contains("error "))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("cannot find")
                || line.contains("no such module")
                || line.contains("could not find module")
                || line.contains("cannot find type")
                || line.contains("is not a member of")
                || line.contains("unresolved identifier")
                || line.contains("use of unresolved identifier")
                || line.contains("module not found")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedent_uniform_indent() {
        let input = "    let x = 1\n    print(x)";
        let output = SwiftValidator::dedent(input);
        assert_eq!(output, "let x = 1\nprint(x)");
    }

    #[test]
    fn test_dedent_no_indent_unchanged() {
        let input = "let x = 1\nprint(x)";
        let output = SwiftValidator::dedent(input);
        assert_eq!(output, "let x = 1\nprint(x)");
    }

    #[test]
    fn test_dedent_preserves_relative_indent() {
        let input = "    func foo() {\n        return 1\n    }";
        let output = SwiftValidator::dedent(input);
        assert_eq!(output, "func foo() {\n    return 1\n}");
    }

    #[test]
    fn test_wrap_sync_fragment_unwrapped() {
        let input = "let x = 1\nprint(x)";
        let (out, parse_as_library) = SwiftValidator::wrap_if_fragment(input);
        assert_eq!(out, input);
        assert!(!parse_as_library);
    }

    #[test]
    fn test_wrap_async_fragment_wraps_with_main() {
        let input = "let x = await fetchValue()";
        let (out, parse_as_library) = SwiftValidator::wrap_if_fragment(input);
        assert!(out.contains("@main"));
        assert!(out.contains("struct SnippetMain"));
        assert!(out.contains("async throws"));
        assert!(out.contains("await fetchValue()"));
        assert!(parse_as_library);
    }

    #[test]
    fn test_wrap_async_fragment_with_imports_separates_them() {
        let input = "import Foundation\nlet x = try await fetchValue()";
        let (out, parse_as_library) = SwiftValidator::wrap_if_fragment(input);
        assert!(parse_as_library);
        assert!(out.starts_with("import Foundation"));
        assert!(out.contains("@main"));
        // Imports must precede the @main struct.
        let import_pos = out.find("import Foundation").unwrap();
        let main_pos = out.find("@main").unwrap();
        assert!(import_pos < main_pos);
    }

    #[test]
    fn test_wrap_existing_main_left_as_is_with_parse_as_library() {
        let input = "@main\nstruct App {\n  static func main() {}\n}";
        let (out, parse_as_library) = SwiftValidator::wrap_if_fragment(input);
        assert_eq!(out, input);
        assert!(parse_as_library);
    }

    #[test]
    fn test_language_is_swift() {
        let v = SwiftValidator;
        assert_eq!(v.language(), Language::Swift);
    }

    #[test]
    fn test_max_level_is_compile() {
        let v = SwiftValidator;
        assert_eq!(v.max_level(), ValidationLevel::Compile);
    }

    #[test]
    fn test_is_dependency_error_no_such_module() {
        let v = SwiftValidator;
        let output = "snippet.swift:1:8: error: no such module 'KreuzbergSwift'";
        assert!(v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_cannot_find_in_scope() {
        let v = SwiftValidator;
        let output = "snippet.swift:3:5: error: cannot find 'someValue' in scope";
        assert!(v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_cannot_find_type() {
        let v = SwiftValidator;
        let output = "snippet.swift:2:10: error: cannot find type 'Foo' in scope";
        assert!(v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_real_syntax_error_returns_false() {
        let v = SwiftValidator;
        let output = "snippet.swift:1:1: error: expected expression";
        assert!(!v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_empty_returns_false() {
        let v = SwiftValidator;
        assert!(!v.is_dependency_error(""));
    }
}
