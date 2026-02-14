mod csharp;
mod elixir;
mod fixtures;
mod go;
mod java;
mod php;
mod python;
mod ruby;
mod rust;
mod typescript;
mod wasm_deno;
mod wasm_workers;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand, ValueEnum};
use fixtures::load_fixtures;

#[derive(Parser)]
#[command(author, version, about = "Generate language-specific E2E suites from fixtures")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate test assets for a language.
    Generate {
        /// Target language.
        #[arg(long, value_enum)]
        lang: Language,
        /// Fixture directory (defaults to workspace fixtures/).
        #[arg(long, default_value = "fixtures")]
        fixtures: Utf8PathBuf,
        /// Output directory (defaults to workspace e2e/).
        #[arg(long, default_value = "e2e")]
        output: Utf8PathBuf,
    },
    /// List fixtures (for quick inspection).
    List {
        /// Fixture directory (defaults to workspace fixtures/).
        #[arg(long, default_value = "fixtures")]
        fixtures: Utf8PathBuf,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Language {
    Rust,
    Python,
    Typescript,
    Ruby,
    Java,
    Go,
    Csharp,
    Php,
    Elixir,
    WasmDeno,
    WasmWorkers,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { lang, fixtures, output } => {
            let fixtures = load_fixtures(fixtures.as_path())?;
            match lang {
                Language::Rust => {
                    rust::generate(&fixtures, output.as_path())?;
                    run_cargo_fmt(&output.join("rust"));
                }
                Language::Python => {
                    python::generate(&fixtures, output.as_path())?;
                    run_ruff_format(&output.join("python/tests"));
                }
                Language::Typescript => {
                    typescript::generate(&fixtures, output.as_path())?;
                    run_biome_format(&output.join("typescript"));
                }
                Language::Ruby => {
                    ruby::generate(&fixtures, output.as_path())?;
                    run_rubocop_format(&output.join("ruby"));
                }
                Language::Java => {
                    java::generate(&fixtures, output.as_path())?;
                    run_google_java_format(&output.join("java"));
                }
                Language::Go => {
                    go::generate(&fixtures, output.as_path())?;
                    run_go_format(&output.join("go"));
                }
                Language::Csharp => {
                    csharp::generate(&fixtures, output.as_path())?;
                    run_dotnet_format(&output.join("csharp"));
                }
                Language::Php => {
                    php::generate(&fixtures, output.as_path())?;
                    run_php_format(&output.join("php"));
                }
                Language::Elixir => {
                    elixir::generate(&fixtures, output.as_path())?;
                    run_mix_format(&output.join("elixir"));
                }
                Language::WasmDeno => {
                    wasm_deno::generate(&fixtures, output.as_path())?;
                    run_biome_format(&output.join("wasm-deno"));
                }
                Language::WasmWorkers => {
                    wasm_workers::generate(&fixtures, output.as_path())?;
                    run_biome_format(&output.join("wasm-workers"));
                }
            };
        }
        Commands::List { fixtures } => {
            let fixtures = load_fixtures(fixtures.as_path())?;
            for fixture in fixtures {
                if fixture.is_document_extraction() {
                    println!(
                        "{:<24} {:<12} [doc] {}",
                        fixture.id,
                        fixture.category(),
                        fixture.document().path
                    );
                } else if fixture.is_plugin_api() {
                    println!(
                        "{:<24} {:<12} [api] {} -> {}",
                        fixture.id,
                        fixture.category(),
                        fixture.api_category.as_deref().unwrap_or("N/A"),
                        fixture.api_function.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }
    }

    Ok(())
}

fn run_biome_format(dir: &Utf8Path) {
    // Fix lint issues (import ordering)
    let status = std::process::Command::new("npx")
        .args(["biome", "check", "--fix", "--unsafe"])
        .arg(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: biome check --fix returned non-zero for {dir}"),
        Err(e) => {
            eprintln!("Warning: failed to run biome: {e}");
            return;
        }
    }
    // Apply formatting (tabs, line width, trailing commas)
    let status = std::process::Command::new("npx")
        .args(["biome", "format", "--write"])
        .arg(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: biome format returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run biome format: {e}"),
    }
}

fn run_cargo_fmt(dir: &Utf8Path) {
    let status = std::process::Command::new("cargo")
        .args(["fmt", "--manifest-path"])
        .arg(dir.join("Cargo.toml").as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: cargo fmt returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run cargo fmt: {e}"),
    }
}

fn run_rubocop_format(dir: &Utf8Path) {
    let status = std::process::Command::new("bundle")
        .args(["exec", "rubocop", "--autocorrect-all"])
        .current_dir(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: rubocop --autocorrect-all returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run rubocop: {e}"),
    }
}

fn run_google_java_format(dir: &Utf8Path) {
    let status = std::process::Command::new("google-java-format")
        .args(["--replace"])
        .arg(dir.join("src/test/java").as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: google-java-format returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run google-java-format: {e}"),
    }
}

fn run_go_format(dir: &Utf8Path) {
    let status = std::process::Command::new("gofmt").args(["-w", dir.as_str()]).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: gofmt returned non-zero for {dir}"),
        Err(e) => {
            eprintln!("Warning: failed to run gofmt: {e}");
            return;
        }
    }
    let status = std::process::Command::new("goimports")
        .args(["-w", dir.as_str()])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: goimports returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run goimports: {e}"),
    }
}

fn run_dotnet_format(dir: &Utf8Path) {
    let status = std::process::Command::new("dotnet")
        .args(["format", "--verbosity", "quiet"])
        .current_dir(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: dotnet format returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run dotnet format: {e}"),
    }
}

fn run_php_format(dir: &Utf8Path) {
    let status = std::process::Command::new("php-cs-fixer")
        .args(["fix", dir.as_str(), "--rules=@PSR12"])
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: php-cs-fixer returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run php-cs-fixer: {e}"),
    }
}

fn run_mix_format(dir: &Utf8Path) {
    let status = std::process::Command::new("mix")
        .arg("format")
        .current_dir(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: mix format returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run mix format: {e}"),
    }
}

fn run_ruff_format(dir: &Utf8Path) {
    // Fix lint issues
    let status = std::process::Command::new("uv")
        .args(["run", "ruff", "check", "--fix"])
        .arg(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: ruff check --fix returned non-zero for {dir}"),
        Err(e) => {
            eprintln!("Warning: failed to run ruff: {e}");
            return;
        }
    }
    // Apply formatting
    let status = std::process::Command::new("uv")
        .args(["run", "ruff", "format"])
        .arg(dir.as_str())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => eprintln!("Warning: ruff format returned non-zero for {dir}"),
        Err(e) => eprintln!("Warning: failed to run ruff format: {e}"),
    }
}
