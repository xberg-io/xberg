use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct ZigValidator;

impl ZigValidator {
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

    /// Wrap a Zig fragment into a complete program if it isn't already a complete file.
    ///
    /// A snippet is considered "complete" if it already contains a `pub fn main(`,
    /// any `pub fn ` declaration, or starts with a top-level `const`/`var` declaration.
    /// Otherwise it is treated as a body to be wrapped inside `pub fn main() !void { ... }`,
    /// with `const std = @import("std");` prepended (and a default GeneralPurposeAllocator
    /// declared inside main if the body references `allocator`).
    fn wrap_if_fragment(code: &str) -> String {
        // Dedent first to handle indented markdown snippets
        let code = Self::dedent(code);
        let trimmed = code.trim();

        // Detect "complete" snippets that don't need wrapping.
        let has_pub_main = trimmed.contains("pub fn main(");
        let has_pub_fn = trimmed.contains("pub fn ");
        // `try X` is only valid inside error-returning functions, so a fragment
        // starting with `const Y = try ...` is statement-level and must be wrapped.
        let has_try_outside_fn =
            (trimmed.starts_with("const ") || trimmed.starts_with("var ")) && trimmed.contains(" try ");
        let starts_with_top_level = !has_try_outside_fn
            && (trimmed.starts_with("const ")
                || trimmed.starts_with("var ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("comptime ")
                || trimmed.starts_with("test ")
                || trimmed.starts_with("@import"));

        if has_pub_main || has_pub_fn || starts_with_top_level {
            // Ensure std import exists if referenced by the snippet.
            if trimmed.contains("std.") && !trimmed.contains("@import(\"std\")") {
                return format!("const std = @import(\"std\");\n\n{code}");
            }
            return code;
        }

        // Statement-level fragment: wrap in pub fn main() !void { ... }
        let needs_allocator = code.contains("allocator") && !code.contains("const allocator");
        let allocator_prelude = if needs_allocator {
            "    var gpa = std.heap.GeneralPurposeAllocator(.{}){};\n    defer _ = gpa.deinit();\n    const allocator = gpa.allocator();\n"
        } else {
            ""
        };

        format!("const std = @import(\"std\");\n\npub fn main() !void {{\n{allocator_prelude}{code}\n}}\n")
    }
}

impl SnippetValidator for ZigValidator {
    fn language(&self) -> Language {
        Language::Zig
    }

    fn is_available(&self) -> bool {
        which::which("zig").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        let code = Self::wrap_if_fragment(&snippet.code);
        let file_path = dir.path().join("snippet.zig");
        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(code.as_bytes())?;

        let mut cmd = match level {
            ValidationLevel::Syntax => {
                let mut c = std::process::Command::new("zig");
                c.args(["ast-check", "snippet.zig"]).current_dir(dir.path());
                c
            }
            ValidationLevel::Compile => {
                // Type-check / build without producing a binary.
                let mut c = std::process::Command::new("zig");
                c.args(["build-exe", "-fno-emit-bin", "snippet.zig"])
                    .current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("zig");
                c.args(["run", "snippet.zig"]).current_dir(dir.path());
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
        // `zig run` is heavy (full build + execute); cap at Compile by default.
        ValidationLevel::Compile
    }

    fn is_dependency_error(&self, output: &str) -> bool {
        let error_lines: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty()
                    && (t.contains("error:") || t.contains("error[") || t.contains("note:") || t.starts_with("error"))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("unable to find")
                || line.contains("no such file")
                || line.contains("import file not found")
                || line.contains("unresolved")
                || line.contains("use of undeclared identifier")
                || line.contains("file not found")
                || line.contains("expected type")
                || line.contains("unable to evaluate constant expression")
                || line.contains("unknown builtin")
                || line.contains("container 'std' has no member")
                || line.contains("root struct of file")
                || line.contains("note:")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedent_uniform_indentation() {
        let input = "    const x = 1;\n    const y = 2;";
        let output = ZigValidator::dedent(input);
        assert_eq!(output, "const x = 1;\nconst y = 2;");
    }

    #[test]
    fn test_dedent_no_indentation() {
        let input = "const x = 1;\nconst y = 2;";
        let output = ZigValidator::dedent(input);
        assert_eq!(output, "const x = 1;\nconst y = 2;");
    }

    #[test]
    fn test_dedent_preserves_relative_indent() {
        let input = "    if (true) {\n        const x = 1;\n    }";
        let output = ZigValidator::dedent(input);
        assert_eq!(output, "if (true) {\n    const x = 1;\n}");
    }

    #[test]
    fn test_wrap_if_fragment_complete_program_unchanged() {
        let input = "const std = @import(\"std\");\n\npub fn main() !void {\n    std.debug.print(\"hi\\n\", .{});\n}";
        let output = ZigValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_top_level_const_adds_std_import_if_needed() {
        let input = "const x: i32 = 42;\nconst y = std.math.pi;";
        let output = ZigValidator::wrap_if_fragment(input);
        assert!(output.contains("const std = @import(\"std\");"));
        assert!(output.contains("const x: i32 = 42;"));
    }

    #[test]
    fn test_wrap_if_fragment_top_level_const_without_std_left_alone() {
        let input = "const x: i32 = 42;";
        let output = ZigValidator::wrap_if_fragment(input);
        assert_eq!(output, "const x: i32 = 42;");
    }

    #[test]
    fn test_wrap_if_fragment_pub_fn_unchanged_modulo_std() {
        let input = "pub fn add(a: i32, b: i32) i32 { return a + b; }";
        let output = ZigValidator::wrap_if_fragment(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_wrap_if_fragment_statement_fragment_wrapped_in_main() {
        let input = "std.debug.print(\"hi\\n\", .{});";
        let output = ZigValidator::wrap_if_fragment(input);
        assert!(output.contains("const std = @import(\"std\");"));
        assert!(output.contains("pub fn main() !void {"));
        assert!(output.contains("std.debug.print(\"hi\\n\", .{});"));
    }

    #[test]
    fn test_wrap_if_fragment_allocator_use_adds_gpa() {
        let input = "const buf = try allocator.alloc(u8, 16);\nallocator.free(buf);";
        let output = ZigValidator::wrap_if_fragment(input);
        assert!(output.contains("pub fn main() !void {"));
        assert!(output.contains("GeneralPurposeAllocator"));
        assert!(output.contains("const allocator = gpa.allocator();"));
    }

    #[test]
    fn test_wrap_if_fragment_allocator_already_declared_no_gpa() {
        let input = "const allocator = std.testing.allocator;\nconst buf = try allocator.alloc(u8, 16);";
        let output = ZigValidator::wrap_if_fragment(input);
        // Already starts with `const ` so treated as top-level; should not inject GPA.
        assert!(!output.contains("GeneralPurposeAllocator"));
    }

    #[test]
    fn test_wrap_if_fragment_dedents_indented_input() {
        let input = "    std.debug.print(\"hi\\n\", .{});";
        let output = ZigValidator::wrap_if_fragment(input);
        assert!(output.contains("pub fn main() !void {"));
        assert!(output.contains("std.debug.print(\"hi\\n\", .{});"));
    }

    #[test]
    fn test_language_returns_zig() {
        let v = ZigValidator;
        assert_eq!(v.language(), Language::Zig);
    }

    #[test]
    fn test_max_level_is_compile() {
        let v = ZigValidator;
        assert_eq!(v.max_level(), ValidationLevel::Compile);
    }

    #[test]
    fn test_is_dependency_error_detects_import_failure() {
        let v = ZigValidator;
        let output = "snippet.zig:1:23: error: unable to find 'foo.zig'";
        assert!(v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_detects_undeclared_identifier() {
        let v = ZigValidator;
        let output = "snippet.zig:3:5: error: use of undeclared identifier 'Bar'";
        assert!(v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_rejects_real_syntax_error() {
        let v = ZigValidator;
        let output = "snippet.zig:2:10: error: expected ';' after statement";
        assert!(!v.is_dependency_error(output));
    }

    #[test]
    fn test_is_dependency_error_empty_output_returns_false() {
        let v = ZigValidator;
        assert!(!v.is_dependency_error(""));
    }
}
