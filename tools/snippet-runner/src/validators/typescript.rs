use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct TypeScriptValidator;

impl TypeScriptValidator {
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

    /// Detect bare function/interface declarations (API reference signatures).
    fn is_api_signature(code: &str) -> bool {
        let trimmed = code.trim();
        let lines: Vec<&str> = trimmed.lines().collect();

        // Short snippets that are just function signatures without bodies
        if lines.len() <= 6 {
            let has_fn_decl = trimmed.starts_with("function ")
                || trimmed.starts_with("async function ")
                || trimmed.starts_with("export function ")
                || trimmed.starts_with("export async function ");
            let has_body = trimmed.contains('{');
            if has_fn_decl && !has_body {
                return true;
            }
        }

        false
    }
}

impl SnippetValidator for TypeScriptValidator {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn is_available(&self) -> bool {
        which::which("tsc").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        // Skip bare function signatures (API docs)
        if Self::is_api_signature(&snippet.code) {
            return Ok((SnippetStatus::Pass, None));
        }

        // Skip markdown admonitions/content that isn't code
        let trimmed_code = snippet.code.trim();
        if trimmed_code.starts_with("!!!") || trimmed_code.starts_with("???") {
            return Ok((SnippetStatus::Pass, None));
        }

        let dir = TempDir::new()?;

        // Write tsconfig.json for type checking
        let tsconfig = r#"{
  "compilerOptions": {
    "strict": true,
    "noEmit": true,
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "skipLibCheck": true
  },
  "include": ["*.ts"]
}"#;
        std::fs::write(dir.path().join("tsconfig.json"), tsconfig)?;

        // Dedent indented snippets (from markdown indentation)
        let code = Self::dedent(&snippet.code);
        let file_path = dir.path().join("snippet.ts");
        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(code.as_bytes())?;

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("tsc");
                c.args(["--noEmit", "--pretty", "false"]).current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("tsx");
                c.args([file_path.to_string_lossy().as_ref()]);
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
        let dep_patterns = [
            "TS2307",  // Cannot find module
            "TS2304",  // Cannot find name
            "TS2305",  // has no exported member
            "TS2306",  // not a module
            "TS2322",  // Type is not assignable (from unresolved types)
            "TS2345",  // Argument type not assignable (from unresolved types)
            "TS2339",  // Property does not exist
            "TS2351",  // Cannot use 'new'
            "TS2552",  // Cannot find name, did you mean
            "TS2314",  // Generic type requires N arguments
            "TS2391",  // Function implementation is missing
            "TS2693",  // only refers to a type
            "TS7016",  // Could not find a declaration file
            "TS2371",  // Parameter initializer only allowed in function (cascades from bare sig)
            "TS2580",  // Cannot find name 'module' / 'require'
            "TS1375",  // 'await' only allowed at top level of module
            "TS2792",  // Cannot find module (different form)
            "TS2503",  // Cannot find namespace
            "TS7006",  // Parameter implicitly has an 'any' type
            "TS2769",  // No overload matches this call
            "TS1128",  // Declaration or statement expected (bare static methods)
            "TS1005",  // ',' expected (partial expressions from signatures)
            "TS18046", // is of type 'unknown' (cascading from missing types)
            "TS18047", // is possibly 'null' (strict null checks)
            "TS2531",  // Object is possibly 'null'
            "TS2532",  // Object is possibly 'undefined'
            "TS2451",  // Cannot redeclare block-scoped variable
            "TS2591",  // Cannot find name (needs @types/node for fs, process, module, etc.)
            "TS2390",  // Constructor implementation is missing (bare class signatures)
        ];

        let error_lines: Vec<&str> = output.lines().filter(|l| l.contains("error TS")).collect();

        if error_lines.is_empty() {
            return false;
        }

        error_lines
            .iter()
            .all(|line| dep_patterns.iter().any(|p| line.contains(p)))
    }
}
