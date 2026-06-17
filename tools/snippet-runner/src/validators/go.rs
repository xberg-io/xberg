use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct GoValidator;

impl GoValidator {
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

    fn wrap_if_fragment(code: &str) -> String {
        // Dedent first to handle indented markdown snippets
        let code = Self::dedent(code);
        let trimmed = code.trim();
        // Already has a package declaration — complete file
        if trimmed.starts_with("package ") {
            return code;
        }

        // Separate imports from other code
        let mut imports = Vec::new();
        let mut body = Vec::new();
        let mut past_imports = false;

        for line in code.lines() {
            let t = line.trim();
            if !past_imports
                && (t.starts_with("import ")
                    || t.starts_with("import (")
                    || (t.starts_with('"') && !imports.is_empty())
                    || t == ")"
                    || t.is_empty())
            {
                imports.push(line);
            } else {
                past_imports = true;
                body.push(line);
            }
        }

        let body_str = body.join("\n");
        let body_trimmed = body_str.trim();

        // Has a top-level declaration (func, type, var, const) — needs package + imports
        let has_top_level = body_trimmed.starts_with("func ")
            || body_trimmed.starts_with("type ")
            || body_trimmed.starts_with("var ")
            || body_trimmed.starts_with("const ");

        let imports_str = imports.join("\n");
        let has_imports = imports.iter().any(|l| l.trim().starts_with("import"));

        if has_top_level {
            if has_imports {
                format!("package main\n\n{imports_str}\n\n{body_str}")
            } else {
                format!("package main\n\n{body_str}")
            }
        } else {
            // Statement-level code — wrap in func main()
            if has_imports {
                format!("package main\n\n{imports_str}\n\nfunc main() {{\n{body_str}\n}}")
            } else {
                format!("package main\n\nfunc main() {{\n{code}\n}}")
            }
        }
    }
}

impl SnippetValidator for GoValidator {
    fn language(&self) -> Language {
        Language::Go
    }

    fn is_available(&self) -> bool {
        which::which("go").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        // Init go module
        let go_mod = "module snippet-check\n\ngo 1.21\n";
        std::fs::write(dir.path().join("go.mod"), go_mod)?;

        let code = Self::wrap_if_fragment(&snippet.code);
        let mut file = std::fs::File::create(dir.path().join("main.go"))?;
        file.write_all(code.as_bytes())?;

        let mut cmd = match level {
            ValidationLevel::Syntax => {
                let mut c = std::process::Command::new("go");
                c.args(["vet", "."]).current_dir(dir.path());
                c
            }
            ValidationLevel::Compile => {
                let mut c = std::process::Command::new("go");
                c.args(["build", "."]).current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("go");
                c.args(["run", "."]).current_dir(dir.path());
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
                        || t.contains("undefined")
                        || t.contains("cannot ")
                        || t.contains("no required module")
                        || t.contains("could not import")
                        || t.contains("is not in")
                        || t.contains("package ")
                        || t.contains("declared and not used")
                        || t.contains("imported and not used")
                        || t.contains("vet:")
                        || t.starts_with("#"))
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("no required module provides")
                || line.contains("could not import")
                || line.contains("cannot find package")
                || line.contains("is not in std")
                || line.contains("is not in GOROOT")
                || line.contains("undefined:")
                || line.contains("undeclared name")
                || line.contains("not enough arguments")
                || line.contains("too many arguments")
                || line.contains("cannot use")
                || line.contains("declared and not used")
                || line.contains("imported and not used")
                || line.contains("has no field or method")
                || line.contains("# snippet-check") // build output summary
                || line.contains("# [snippet-check]") // vet output summary
                || line.contains("expected '('") // CGo export comments causing parse issues
                || line.contains("expected declaration") // cascading from wrapping
                || line.contains("more errors") // N more errors summary
                || line.contains("expected '}'") // struct { ... } literal syntax in signatures
        })
    }
}
