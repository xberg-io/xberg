use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    Go,
    Java,
    CSharp,
    Php,
    Ruby,
    Elixir,
    R,
    C,
    Bash,
    Toml,
    Docker,
    Unknown,
}

impl Language {
    pub fn from_fence_tag(tag: &str) -> Self {
        match tag.trim().to_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "python" | "py" | "python3" => Self::Python,
            "typescript" | "ts" | "javascript" | "js" => Self::TypeScript,
            "go" | "golang" => Self::Go,
            "java" => Self::Java,
            "csharp" | "c#" | "cs" => Self::CSharp,
            "php" => Self::Php,
            "ruby" | "rb" => Self::Ruby,
            "elixir" | "ex" | "exs" => Self::Elixir,
            "r" => Self::R,
            "c" | "h" => Self::C,
            "bash" | "sh" | "shell" | "zsh" => Self::Bash,
            "toml" => Self::Toml,
            "dockerfile" | "docker" => Self::Docker,
            _ => Self::Unknown,
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" => Self::Python,
            "ts" | "js" | "mts" | "mjs" => Self::TypeScript,
            "go" => Self::Go,
            "java" => Self::Java,
            "cs" => Self::CSharp,
            "php" => Self::Php,
            "rb" => Self::Ruby,
            "ex" | "exs" => Self::Elixir,
            "r" => Self::R,
            "c" | "h" => Self::C,
            "sh" | "bash" => Self::Bash,
            "toml" => Self::Toml,
            _ => Self::Unknown,
        }
    }

    pub fn from_dir_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "rust" => Self::Rust,
            "python" => Self::Python,
            "typescript" | "wasm" => Self::TypeScript,
            "go" => Self::Go,
            "java" => Self::Java,
            "csharp" => Self::CSharp,
            "php" => Self::Php,
            "ruby" => Self::Ruby,
            "elixir" => Self::Elixir,
            "r" => Self::R,
            "c" => Self::C,
            "docker" => Self::Docker,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::Python => write!(f, "python"),
            Self::TypeScript => write!(f, "typescript"),
            Self::Go => write!(f, "go"),
            Self::Java => write!(f, "java"),
            Self::CSharp => write!(f, "csharp"),
            Self::Php => write!(f, "php"),
            Self::Ruby => write!(f, "ruby"),
            Self::Elixir => write!(f, "elixir"),
            Self::R => write!(f, "r"),
            Self::C => write!(f, "c"),
            Self::Bash => write!(f, "bash"),
            Self::Toml => write!(f, "toml"),
            Self::Docker => write!(f, "docker"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    Syntax,
    Compile,
    Run,
}

impl fmt::Display for ValidationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => write!(f, "syntax"),
            Self::Compile => write!(f, "compile"),
            Self::Run => write!(f, "run"),
        }
    }
}

impl std::str::FromStr for ValidationLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "syntax" => Ok(Self::Syntax),
            "compile" => Ok(Self::Compile),
            "run" => Ok(Self::Run),
            _ => Err(format!("unknown validation level: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetAnnotation {
    Skip,
    CompileOnly,
    SyntaxOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetStatus {
    Pass,
    Fail,
    Skip,
    Error,
    Unavailable,
}

impl fmt::Display for SnippetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Fail => write!(f, "fail"),
            Self::Skip => write!(f, "skip"),
            Self::Error => write!(f, "error"),
            Self::Unavailable => write!(f, "unavailable"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub path: PathBuf,
    pub language: Language,
    pub title: Option<String>,
    pub code: String,
    pub start_line: usize,
    pub block_index: usize,
    pub annotation: Option<SnippetAnnotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub snippet: Snippet,
    pub status: SnippetStatus,
    pub level: ValidationLevel,
    pub message: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub unavailable: usize,
    pub results: Vec<ValidationResult>,
}

impl RunSummary {
    pub fn from_results(results: Vec<ValidationResult>) -> Self {
        let mut summary = Self {
            total: results.len(),
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: 0,
            unavailable: 0,
            results,
        };
        for r in &summary.results {
            match r.status {
                SnippetStatus::Pass => summary.passed += 1,
                SnippetStatus::Fail => summary.failed += 1,
                SnippetStatus::Skip => summary.skipped += 1,
                SnippetStatus::Error => summary.errors += 1,
                SnippetStatus::Unavailable => summary.unavailable += 1,
            }
        }
        summary
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0 || self.errors > 0
    }
}
