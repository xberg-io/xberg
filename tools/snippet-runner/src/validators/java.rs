use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct JavaValidator;

impl JavaValidator {
    /// Extract the public type name from Java source, or default to "Snippet".
    /// Handles: class, interface, enum, record declarations.
    fn class_name(code: &str) -> String {
        for line in code.lines() {
            let trimmed = line.trim();
            // Try all public type declarations
            for prefix in [
                "public class ",
                "public abstract class ",
                "public final class ",
                "public interface ",
                "public enum ",
                "public record ",
            ] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
        "Snippet".to_string()
    }

    /// Detect bare method signatures (API reference style).
    /// e.g. `public static ExtractionResult extractFile(String path) throws IOException`
    fn is_api_signature(code: &str) -> bool {
        let trimmed = code.trim();
        // Multiple method signatures listed (no braces at all)
        if !trimmed.contains('{') {
            let has_method = trimmed.contains('(') && trimmed.contains(')');
            let has_modifier = trimmed.starts_with("public ")
                || trimmed.starts_with("static ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("protected ");
            if has_method && has_modifier {
                return true;
            }
        }
        false
    }

    /// Detect if code is a complete Java file, or if it needs wrapping.
    fn wrap_if_fragment(code: &str) -> (String, String) {
        let trimmed = code.trim();

        // Bare API signatures — treat as pass-through (will fail and be caught by is_dependency_error)
        if Self::is_api_signature(trimmed) {
            // Wrap in interface (methods are implicitly abstract)
            let methods: String = trimmed
                .lines()
                .map(|l| {
                    let t = l.trim();
                    if t.is_empty() || t.starts_with("//") || t.starts_with("import ") {
                        l.to_string()
                    } else {
                        // Strip modifiers that aren't allowed in interfaces
                        let cleaned = t
                            .replace("public static ", "")
                            .replace("public ", "")
                            .replace("static ", "");
                        if cleaned.ends_with(';') {
                            format!("    {cleaned}")
                        } else {
                            format!("    {cleaned};")
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            return ("Snippet".to_string(), format!("interface Snippet {{\n{methods}\n}}"));
        }

        // Has a class/interface/enum/record declaration — use as-is
        if trimmed.contains("class ")
            || trimmed.contains("interface ")
            || trimmed.contains("enum ")
            || trimmed.contains("record ")
        {
            let name = Self::class_name(code);
            return (name, code.to_string());
        }

        // Separate imports from body
        let mut imports = Vec::new();
        let mut body_lines = Vec::new();
        let mut past_imports = false;
        for line in code.lines() {
            if !past_imports && (line.trim().starts_with("import ") || line.trim().is_empty()) {
                imports.push(line);
            } else {
                past_imports = true;
                body_lines.push(line);
            }
        }

        let imports_str = imports.join("\n");
        let body_str = body_lines.join("\n");
        let body_trimmed = body_str.trim();

        // Check if body contains method declarations (should be class members, not in main)
        let has_method_decl = body_trimmed.starts_with("public ")
            || body_trimmed.starts_with("private ")
            || body_trimmed.starts_with("protected ")
            || body_trimmed.starts_with("static ")
            || body_trimmed.starts_with("@");

        if has_method_decl && !body_trimmed.contains("class ") {
            // Method-level code — wrap in a class without main
            let has_imports = !imports_str.trim().is_empty();
            if has_imports {
                return (
                    "Snippet".to_string(),
                    format!("{imports_str}\n\npublic class Snippet {{\n{body_str}\n}}"),
                );
            }
            return (
                "Snippet".to_string(),
                format!("public class Snippet {{\n{body_str}\n}}"),
            );
        }

        // Statement-level code — wrap in class + main
        let has_imports = !imports_str.trim().is_empty();
        if has_imports {
            (
                "Snippet".to_string(),
                format!(
                    "{imports_str}\n\npublic class Snippet {{\n    public static void main(String[] args) throws Exception {{\n{body_str}\n    }}\n}}"
                ),
            )
        } else {
            (
                "Snippet".to_string(),
                format!(
                    "public class Snippet {{\n    public static void main(String[] args) throws Exception {{\n{code}\n    }}\n}}"
                ),
            )
        }
    }
}

impl SnippetValidator for JavaValidator {
    fn language(&self) -> Language {
        Language::Java
    }

    fn is_available(&self) -> bool {
        which::which("javac").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;
        let (class_name, code) = Self::wrap_if_fragment(&snippet.code);
        let file_path = dir.path().join(format!("{class_name}.java"));

        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(code.as_bytes())?;

        let path_str = file_path.to_string_lossy().to_string();

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("javac");
                c.args(["-d", &dir.path().to_string_lossy(), &path_str]);
                c
            }
            ValidationLevel::Run => {
                // First compile
                let mut compile = std::process::Command::new("javac");
                compile.args(["-d", &dir.path().to_string_lossy(), &path_str]);
                let (ok, output) = run_command(&mut compile, timeout_secs)?;
                if !ok {
                    return Ok((SnippetStatus::Fail, Some(output)));
                }

                let mut c = std::process::Command::new("java");
                c.args(["-cp", &dir.path().to_string_lossy(), &class_name]);
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
                // Match javac error lines (file:line: error: message) or summary lines (N errors)
                l.contains(": error:")
                    || l.contains(": error ")
                    || l.trim().ends_with("error")
                    || l.trim().ends_with("errors")
            })
            .collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines.iter().all(|line| {
            line.contains("cannot find symbol")
                || (line.contains("package") && line.contains("does not exist"))
                || line.contains("cannot access")
                || line.contains("class, interface, enum, or record expected")
                || line.contains("illegal start of expression") // from wrapping artifacts
                || line.contains("reached end of file while parsing")
                || line.contains("not a statement")
                || line.contains("should be declared in a file named") // filename mismatch
                || line.contains("illegal combination of modifiers") // abstract static
                || line.contains("unreported exception") // checked exceptions
                || line.contains("incompatible types") // cascading type errors
                || (line.contains(" error") && line.trim().ends_with("errors")) // N errors summary
                || (line.contains(" error") && line.trim().ends_with("error")) // 1 error summary
                || line.contains("class, interface, annotation type") // top-level statement after class
                || line.contains("method does not override") // missing interface
                || line.contains("is abstract; cannot be instantiated") // abstract type usage
                || line.contains("illegal start of type") // bare method sigs in interface
                || line.contains("<identifier> expected") // cascading from type wrapping
                || line.contains("= expected") // cascading from interface wrapping
                || line.trim().ends_with("errors") // "N errors" summary line
                || line.contains("implicitly declared classes") // preview feature error (Java 21+)
                || line.contains("preview feature") // general preview feature errors
                || line.contains("missing method body, or declare abstract") // bare method signatures
        })
    }
}
