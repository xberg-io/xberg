use crate::error::Result;
use crate::types::{Language, Snippet, SnippetStatus, ValidationLevel};
use crate::validators::{SnippetValidator, run_command};
use std::io::Write;
use tempfile::TempDir;

pub struct CSharpValidator;

impl SnippetValidator for CSharpValidator {
    fn language(&self) -> Language {
        Language::CSharp
    }

    fn is_available(&self) -> bool {
        which::which("dotnet").is_ok()
    }

    fn validate(
        &self,
        snippet: &Snippet,
        level: ValidationLevel,
        timeout_secs: u64,
    ) -> Result<(SnippetStatus, Option<String>)> {
        let dir = TempDir::new()?;

        // Create minimal .csproj
        let csproj = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net10.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>"#;
        std::fs::write(dir.path().join("snippet.csproj"), csproj)?;

        let mut file = std::fs::File::create(dir.path().join("Program.cs"))?;
        file.write_all(snippet.code.as_bytes())?;

        let mut cmd = match level {
            ValidationLevel::Syntax | ValidationLevel::Compile => {
                let mut c = std::process::Command::new("dotnet");
                c.args(["build", "--nologo", "-v", "quiet"]).current_dir(dir.path());
                c
            }
            ValidationLevel::Run => {
                let mut c = std::process::Command::new("dotnet");
                c.args(["run", "--nologo"]).current_dir(dir.path());
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
            .filter(|l| l.contains("error CS") || l.contains("error MSB"))
            .collect();

        if error_lines.is_empty() {
            // Check for error patterns in non-standard formats
            return output.contains("error CS5001") || output.contains("error CS0106");
        }

        let dep_patterns = [
            "CS0246", // type or namespace name could not be found
            "CS0103", // name does not exist in the current context
            "CS0234", // type or namespace name does not exist in the namespace
            "CS0106", // modifier is not valid (partial class fragments)
            "CS0116", // namespace cannot directly contain members (top-level fragments)
            "CS8802", // only one compilation unit can have top-level statements
            "CS8803", // top-level statements must precede namespace and type declarations
            "CS0029", // Cannot implicitly convert type
            "CS1002", // ; expected (often from partial method signatures)
            "CS1513", // } expected (fragment boundaries)
            "CS5001", // Program does not contain a static 'Main' method
            "CS1003", // Syntax error, ',' expected (from partial expressions)
            "CS1529", // using clause must precede all other elements
            "CS0101", // namespace already contains a definition (conflict from wrapping)
            "CS0161", // not all code paths return a value
            "CS1001", // Identifier expected (from bare signatures)
            "CS0501", // must declare a body (method without body in non-abstract class)
            "CS0535", // does not implement interface member
        ];

        error_lines
            .iter()
            .all(|line| dep_patterns.iter().any(|p| line.contains(p)))
    }
}
