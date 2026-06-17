use crate::error::Result;
use crate::types::{RunSummary, Snippet, SnippetAnnotation, SnippetStatus, ValidationLevel, ValidationResult};
use crate::validators::ValidatorRegistry;
use rayon::prelude::*;
use std::time::Instant;

pub struct RunnerConfig {
    pub level: ValidationLevel,
    pub parallelism: usize,
    pub timeout_secs: u64,
    pub fail_fast: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            level: ValidationLevel::Syntax,
            parallelism: num_cpus(),
            timeout_secs: 30,
            fail_fast: false,
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Run validation on all snippets using the registry.
pub fn run_validation(snippets: &[Snippet], registry: &ValidatorRegistry, config: &RunnerConfig) -> Result<RunSummary> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.parallelism)
        .build()
        .map_err(|e| crate::error::Error::Other(format!("failed to build thread pool: {e}")))?;

    let results: Vec<ValidationResult> = pool.install(|| {
        snippets
            .par_iter()
            .map(|snippet| validate_one(snippet, registry, config))
            .collect()
    });

    Ok(RunSummary::from_results(results))
}

fn validate_one(snippet: &Snippet, registry: &ValidatorRegistry, config: &RunnerConfig) -> ValidationResult {
    // Check annotation constraints
    if let Some(annotation) = &snippet.annotation {
        match annotation {
            SnippetAnnotation::Skip => {
                return ValidationResult {
                    snippet: snippet.clone(),
                    status: SnippetStatus::Skip,
                    level: config.level,
                    message: Some("skipped via annotation".to_string()),
                    duration_ms: 0,
                };
            }
            SnippetAnnotation::SyntaxOnly if config.level > ValidationLevel::Syntax => {
                return ValidationResult {
                    snippet: snippet.clone(),
                    status: SnippetStatus::Skip,
                    level: config.level,
                    message: Some("annotation limits to syntax-only".to_string()),
                    duration_ms: 0,
                };
            }
            SnippetAnnotation::CompileOnly if config.level > ValidationLevel::Compile => {
                return ValidationResult {
                    snippet: snippet.clone(),
                    status: SnippetStatus::Skip,
                    level: config.level,
                    message: Some("annotation limits to compile-only".to_string()),
                    duration_ms: 0,
                };
            }
            _ => {}
        }
    }

    let validator = match registry.get(snippet.language) {
        Some(v) => v,
        None => {
            return ValidationResult {
                snippet: snippet.clone(),
                status: SnippetStatus::Unavailable,
                level: config.level,
                message: Some(format!("no validator for {}", snippet.language)),
                duration_ms: 0,
            };
        }
    };

    if !validator.is_available() {
        return ValidationResult {
            snippet: snippet.clone(),
            status: SnippetStatus::Unavailable,
            level: config.level,
            message: Some(format!("{} toolchain not found", snippet.language)),
            duration_ms: 0,
        };
    }

    // Clamp level to validator's max supported level
    let effective_level = config.level.min(validator.max_level());

    let start = Instant::now();
    let (mut status, message) = match validator.validate(snippet, effective_level, config.timeout_secs) {
        Ok((s, m)) => (s, m),
        Err(e) => (SnippetStatus::Error, Some(e.to_string())),
    };
    let duration_ms = start.elapsed().as_millis() as u64;

    // At syntax level, dependency/import errors mean the syntax itself is valid —
    // only the external dependencies are missing. Treat as pass.
    if status == SnippetStatus::Fail
        && effective_level == ValidationLevel::Syntax
        && let Some(ref err_output) = message
        && validator.is_dependency_error(err_output)
    {
        status = SnippetStatus::Pass;
    }

    ValidationResult {
        snippet: snippet.clone(),
        status,
        level: effective_level,
        message,
        duration_ms,
    }
}
