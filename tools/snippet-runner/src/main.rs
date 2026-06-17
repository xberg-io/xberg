use clap::{Parser, Subcommand};
use snippet_runner::discovery;
use snippet_runner::output;
use snippet_runner::runner::{RunnerConfig, run_validation};
use snippet_runner::types::{Language, ValidationLevel};
use snippet_runner::validators::ValidatorRegistry;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "snippet-runner")]
#[command(about = "Validate documentation code snippets across languages")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all discovered documentation snippets
    List {
        /// Snippet directories to scan
        #[arg(short, long, required = true, num_args = 1..)]
        snippets: Vec<PathBuf>,

        /// Additional directories to scan (e.g., docs/reference)
        #[arg(short, long, num_args = 0..)]
        reference: Vec<PathBuf>,

        /// Filter by languages (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        languages: Option<Vec<String>>,
    },

    /// Validate snippets by running language-specific checks
    Validate {
        /// Snippet directories to scan
        #[arg(short, long, required = true, num_args = 1..)]
        snippets: Vec<PathBuf>,

        /// Additional directories to scan (e.g., docs/reference)
        #[arg(short, long, num_args = 0..)]
        reference: Vec<PathBuf>,

        /// Validation level
        #[arg(short = 'L', long, default_value = "syntax")]
        level: ValidationLevel,

        /// Filter by languages (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        languages: Option<Vec<String>>,

        /// Write JSON results to file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Number of parallel jobs
        #[arg(short = 'j', long, default_value = "4")]
        jobs: usize,

        /// Per-snippet timeout in seconds
        #[arg(short = 't', long, default_value = "30")]
        timeout: u64,

        /// Stop on first failure
        #[arg(long)]
        fail_fast: bool,

        /// Glob pattern to filter snippet paths
        #[arg(long)]
        include: Option<String>,

        /// Show snippet source code for failures
        #[arg(long)]
        show_code: bool,
    },

    /// Debug: parse and display code blocks from a file
    Parse {
        /// File to parse
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::List {
            snippets,
            reference,
            languages,
        } => {
            let filter = parse_language_filter(languages.as_deref());
            let mut dirs = snippets;
            dirs.extend(reference);

            match discovery::discover_snippets(&dirs, filter.as_deref()) {
                Ok(found) => {
                    output::print_snippet_list(&found);

                    // Print language breakdown
                    println!();
                    let counts = discovery::count_by_language(&found);
                    for (lang, count) in &counts {
                        println!("  {lang:<12} {count}");
                    }
                    println!();

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Error discovering snippets: {e}");
                    ExitCode::FAILURE
                }
            }
        }

        Commands::Validate {
            snippets,
            reference,
            level,
            languages,
            output: output_path,
            jobs,
            timeout,
            fail_fast,
            include,
            show_code,
        } => {
            let filter = parse_language_filter(languages.as_deref());
            let mut dirs = snippets;
            dirs.extend(reference);

            let mut found = match discovery::discover_snippets(&dirs, filter.as_deref()) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error discovering snippets: {e}");
                    return ExitCode::FAILURE;
                }
            };

            // Apply include glob filter
            if let Some(pattern) = &include {
                found.retain(|s| {
                    let path_str = s.path.to_string_lossy();
                    path_str.contains(pattern.as_str())
                });
            }

            if found.is_empty() {
                println!("No snippets found.");
                return ExitCode::SUCCESS;
            }

            println!("Validating {} snippets at level '{level}'...", found.len());

            let registry = ValidatorRegistry::new();
            let config = RunnerConfig {
                level,
                parallelism: jobs,
                timeout_secs: timeout,
                fail_fast,
            };

            match run_validation(&found, &registry, &config) {
                Ok(summary) => {
                    output::print_summary(&summary, show_code);

                    if let Some(path) = output_path {
                        if let Err(e) = output::write_json(&summary.results, &path) {
                            eprintln!("Error writing JSON output: {e}");
                        } else {
                            println!("Results written to {}", path.display());
                        }
                    }

                    if summary.has_failures() {
                        ExitCode::FAILURE
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Error running validation: {e}");
                    ExitCode::FAILURE
                }
            }
        }

        Commands::Parse { file } => match snippet_runner::parser::parse_code_blocks(&file) {
            Ok(blocks) => {
                if blocks.is_empty() {
                    println!("No code blocks found in {}", file.display());
                } else {
                    for (i, block) in blocks.iter().enumerate() {
                        println!("--- Block {} (line {}) ---", i + 1, block.start_line);
                        println!("Language: {}", block.lang);
                        if let Some(title) = &block.title {
                            println!("Title: {title}");
                        }
                        if let Some(comment) = &block.preceding_comment {
                            println!("Annotation: {comment}");
                        }
                        println!("Code ({} lines):", block.code.lines().count());
                        println!("{}", block.code);
                        println!();
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error parsing {}: {e}", file.display());
                ExitCode::FAILURE
            }
        },
    }
}

fn parse_language_filter(languages: Option<&[String]>) -> Option<Vec<Language>> {
    languages.map(|langs| {
        langs
            .iter()
            .map(|l| Language::from_fence_tag(l))
            .filter(|l| *l != Language::Unknown)
            .collect()
    })
}
