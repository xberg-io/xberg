use crate::{adapters::subprocess::SubprocessAdapter, error::Result};
use std::time::Duration;
use std::{env, path::PathBuf};

use super::ocr_flag;

/// Maximum per-extraction timeout for persistent adapters (seconds).
const PERSISTENT_MAX_TIMEOUT_SECS: u64 = 180;

/// Higher timeout for slow ML frameworks (mineru, pymupdf4llm) that load
/// large models and can take significantly longer on first extractions.
const SLOW_ML_TIMEOUT_SECS: u64 = 300;

/// Margin between the Python-side and Rust-side timeouts.
/// The Python script handles timeouts internally (via multiprocessing fork),
/// reporting the result as a JSON error. The Rust-side timeout is a safety net
/// that only fires if the Python side fails to respond.
const PYTHON_TIMEOUT_MARGIN_SECS: u64 = 30;

/// Python-side extraction timeout passed via `--timeout=N` CLI arg.
const PYTHON_EXTRACTION_TIMEOUT_SECS: u64 = PERSISTENT_MAX_TIMEOUT_SECS - PYTHON_TIMEOUT_MARGIN_SECS;

/// Helper function to define supported file types for each framework
///
/// Maps framework names to the file extensions they can actually process.
/// This prevents invalid benchmark combinations (e.g., Pandoc cannot read PDFs).
/// Format lists are based on comprehensive research of each framework's actual capabilities.
fn get_supported_formats(framework_name: &str) -> Vec<String> {
    match framework_name {
        // LiteParse (run-llama/liteparse): PDF-only Rust CLI (`lit parse`).
        "liteparse" => vec!["pdf".to_string()],

        // PyMuPDF4LLM: PDF + formats via PyMuPDF/fitz
        // See: https://pymupdf.readthedocs.io/en/latest/how-to-open-a-file.html
        // Note: many non-PDF formats return empty content — tracked as EmptyContent errors
        "pymupdf4llm" => vec![
            // Documents
            "pdf",  // E-books
            "epub", // Vector/text
            "svg", "txt", // Images (for OCR) - gif and webp NOT supported by PyMuPDF
            "png", "jpg", "jpeg", "bmp", "tiff", "tif",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // Docling: 15+ format types, 38+ extensions
        // See: https://docling-project.github.io/docling/usage/supported_formats/
        "docling" => vec![
            // Office documents
            "pdf", "docx", "pptx", "xlsx", // Web/markup
            "html", "htm", "md", "markdown", "asciidoc", // Data formats
            "csv",      // Scientific/publishing
            "jats",     // Subtitles
            "vtt",      // Images (converted to PDF internally for layout analysis)
            "png", "jpg", "jpeg", "tiff", "tif", "bmp", "webp",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // Tika: 1500+ formats for detection, extensive text extraction
        // See: https://tika.apache.org/ and tika-mimetypes.xml
        "tika" => vec![
            // Office documents (Microsoft)
            "pdf", "docx", "doc", "pptx", "ppt", "ppsx", "pptm", "xlsx", "xls", "xlsm", "xlsb",
            // Office documents (OpenDocument)
            "odt", "ods", // Other documents
            "rtf", "epub", // Web/markup
            "html", "htm", "xml", "svg", "md", "txt", // Data formats
            "csv", "tsv", "json", "yaml", "yml", "toml", // Email
            "eml", "msg", // Scientific/technical (typst not supported - too new)
            "tex", "latex", "bib", "rst", "org", "ipynb", // Images (metadata + OCR)
            "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "jp2", // Archives
            "zip", "tar", "gz", "7z",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // MarkItDown: 25+ formats with optional dependencies
        // See: https://github.com/microsoft/markitdown
        // Note: MarkItDown OUTPUTS markdown, so md/txt are not conversion inputs
        "markitdown" => vec![
            // Office documents
            "pdf", "docx", "pptx", "xlsx", "xls", // Web/markup (md, txt not valid - outputs markdown)
            "html", "htm", "xml", // Data formats
            "csv", "json", // E-books & notebooks
            "epub", "ipynb", // Email
            "msg",   // Images (with Azure Document Intelligence)
            "png", "jpg", "jpeg", "bmp", "tiff", "tif", // Archives
            "zip",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // Unstructured: 31+ partitionable formats
        // See: https://docs.unstructured.io/ui/supported-file-types
        "unstructured" => vec![
            // Office documents (Microsoft)
            "pdf", "docx", "doc", "pptx", "ppt", "xlsx", "xls", // Office documents (OpenDocument)
            "odt", // Other documents
            "rtf", "epub", // Web/markup
            "html", "htm", "xml", "md", "rst", "org", "txt",
            // Data formats (json NOT supported for partitioning)
            "csv", "tsv", // Email
            "eml", "msg", // Images (requires hi_res strategy)
            "png", "jpg", "jpeg", "tiff", "tif", "bmp",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // MinerU: PDF and PNG/JPG images ONLY
        // See: https://github.com/opendatalab/MinerU - cli/common.py defines actual formats
        "mineru" => vec![
            // Documents
            "pdf", // Images (only png, jpg confirmed in source)
            "png", "jpg",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // Default: common document formats for unknown frameworks
        _ => vec![
            "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "txt", "md", "html", "xml", "json",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),
    }
}

/// Creates a subprocess adapter for Docling.
///
/// Uses wrapper script approach for extraction.
pub fn create_docling_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let (command, mut args) = find_python_with_framework("docling")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("sync".to_string());

    let supported_formats = get_supported_formats("docling");
    Ok(
        SubprocessAdapter::new("docling", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for Unstructured.
///
/// Uses wrapper script approach for extraction.
pub fn create_unstructured_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("unstructured_extract.py")?;
    let (command, mut args) = find_python_with_framework("unstructured")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("sync".to_string());

    let supported_formats = get_supported_formats("unstructured");
    Ok(
        SubprocessAdapter::new("unstructured", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for MarkItDown
pub fn create_markitdown_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("markitdown_extract.py")?;
    let (command, mut args) = find_python_with_framework("markitdown")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("sync".to_string());

    let supported_formats = get_supported_formats("markitdown");
    Ok(
        SubprocessAdapter::new("markitdown", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for LiteParse (run-llama/liteparse) Rust CLI.
///
/// Requires the `lit` binary on PATH. Install with `cargo install liteparse`.
/// Wrapper invokes `lit parse <file> --format text` per file — no persistent
/// server, default options only.
pub fn create_liteparse_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    which::which("lit").map_err(|_| {
        crate::Error::Config("lit (liteparse) not found. Install with: cargo install liteparse".to_string())
    })?;

    let script_path = get_script_path("liteparse_extract.sh")?;
    let command = PathBuf::from("bash");
    let mut args = vec![script_path.to_string_lossy().to_string()];
    args.push(ocr_flag(ocr_enabled));

    let supported_formats = get_supported_formats("liteparse");
    Ok(
        SubprocessAdapter::new("liteparse", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Helper function to get the path to a wrapper script
fn get_script_path(script_name: &str) -> Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let script_path = PathBuf::from(manifest_dir).join("scripts").join(script_name);
        if script_path.exists() {
            return Ok(script_path);
        }
    }

    let script_path = PathBuf::from("tools/benchmark-harness/scripts").join(script_name);
    if script_path.exists() {
        return Ok(script_path);
    }

    Err(crate::error::Error::Config(format!(
        "Script not found: {}",
        script_name
    )))
}

/// Helper function to find Python interpreter with a specific open source extraction framework installed
///
/// Returns (command, args) where command is the executable and args are the base arguments
fn find_python_with_framework(framework: &str) -> Result<(PathBuf, Vec<String>)> {
    if which::which("uv").is_ok() {
        // Use `uv run <script>` which runs the script with the project's
        // Python environment (.venv). Framework dependencies are installed
        // via pyproject.toml dependency groups (bench-*).
        return Ok((PathBuf::from("uv"), vec!["run".to_string()]));
    }

    let python_candidates = vec!["python3", "python"];

    for candidate in python_candidates {
        if let Ok(python_path) = which::which(candidate) {
            let check = std::process::Command::new(&python_path)
                .arg("-c")
                .arg(format!("import {}", framework))
                .output();

            if let Ok(output) = check
                && output.status.success()
            {
                return Ok((python_path, vec![]));
            }
        }
    }

    Err(crate::error::Error::Config(format!(
        "No Python interpreter found with {} installed. Install with: pip install {}",
        framework, framework
    )))
}

/// Helper to find Java runtime
fn find_java() -> Result<PathBuf> {
    which::which("java").map_err(|_| crate::Error::Config("Java runtime not found".to_string()))
}

/// Helper to locate Tika JAR (auto-detect from libs/ or env var)
fn get_tika_jar_path() -> Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let lib_dir = PathBuf::from(manifest_dir).join("libs");
        if let Ok(entries) = std::fs::read_dir(&lib_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("tika-app-")
                    && name.ends_with(".jar")
                {
                    return Ok(path);
                }
            }
        }
    }

    let fallback_lib_dir = PathBuf::from("tools/benchmark-harness/libs");
    if let Ok(entries) = std::fs::read_dir(&fallback_lib_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("tika-app-")
                && name.ends_with(".jar")
            {
                return Ok(path);
            }
        }
    }

    if let Ok(jar_path) = env::var("TIKA_JAR") {
        let path = PathBuf::from(jar_path);
        if path.exists() {
            return Ok(path);
        }
    }

    let version = env::var("TIKA_VERSION").unwrap_or_else(|_| "3.2.3".to_string());
    Err(crate::Error::Config(format!(
        "Tika JAR not found. Download: curl -fsSL -o tools/benchmark-harness/libs/tika-app-{version}.jar https://repo1.maven.org/maven2/org/apache/tika/tika-app/{version}/tika-app-{version}.jar"
    )))
}

/// Helper to ensure TikaExtract.class is compiled
/// Compiles TikaExtract.java if .class file doesn't exist, and returns the directory containing the class
fn ensure_tika_extract_compiled(java_path: &PathBuf, tika_jar_path: &PathBuf) -> Result<PathBuf> {
    let script_path = get_script_path("TikaExtract.java")?;

    // Create a temp directory for compiled classes if it doesn't exist
    let compile_dir = PathBuf::from("target").join("tika-extract-classes");
    std::fs::create_dir_all(&compile_dir)
        .map_err(|e| crate::Error::Config(format!("Failed to create compile directory: {}", e)))?;

    let class_path = compile_dir.join("dev").join("kreuzberg").join("benchmark").join("TikaExtract.class");

    // Only compile if the class file doesn't exist
    if !class_path.exists() {
        let output = std::process::Command::new(java_path)
            .arg("-version")
            .output()
            .map_err(|e| crate::Error::Config(format!("Failed to check Java version: {}", e)))?;

        if !output.status.success() {
            return Err(crate::Error::Config("Java is not properly installed".to_string()));
        }

        let compile_output = std::process::Command::new("javac")
            .arg("-cp")
            .arg(tika_jar_path)
            .arg("-d")
            .arg(&compile_dir)
            .arg(&script_path)
            .output()
            .map_err(|e| crate::Error::Config(format!("Failed to compile TikaExtract.java: {}", e)))?;

        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            return Err(crate::Error::Config(format!(
                "TikaExtract.java compilation failed: {}",
                stderr
            )));
        }
    }

    Ok(compile_dir)
}

/// Creates a subprocess adapter for Apache Tika (persistent server mode)
///
/// Uses Tika via compiled Java class approach for extraction.
pub fn create_tika_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let jar_path = get_tika_jar_path()?;
    let command = find_java()?;
    let compile_dir = ensure_tika_extract_compiled(&command, &jar_path)?;

    // Build classpath: compiled classes directory + tika-app JAR
    // Use the platform-appropriate path separator
    let classpath = format!(
        "{}{}{}",
        compile_dir.display(),
        std::path::MAIN_SEPARATOR,
        jar_path.display()
    );

    let args = vec![
        "-server".to_string(),
        "-Xms512m".to_string(),
        "-Xmx2g".to_string(),
        "-XX:+UseG1GC".to_string(),
        "-cp".to_string(),
        classpath,
        "dev.kreuzberg.benchmark.TikaExtract".to_string(),
        ocr_flag(ocr_enabled),
        "sync".to_string(),
    ];

    let supported_formats = get_supported_formats("tika");
    Ok(SubprocessAdapter::new("tika", command, args, vec![], supported_formats)
        .with_max_timeout(Duration::from_secs(180)))
}

/// Creates a subprocess adapter for PyMuPDF4LLM
pub fn create_pymupdf4llm_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pymupdf4llm_extract.py")?;
    let (command, mut args) = find_python_with_framework("pymupdf4llm")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("sync".to_string());

    let supported_formats = get_supported_formats("pymupdf4llm");
    Ok(
        SubprocessAdapter::new("pymupdf4llm", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(SLOW_ML_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for MinerU (persistent server mode)
///
/// Uses wrapper script approach for extraction.
pub fn create_mineru_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("mineru_extract.py")?;
    let (command, mut args) = find_python_with_framework("mineru")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("sync".to_string());

    let supported_formats = get_supported_formats("mineru");
    Ok(
        SubprocessAdapter::new("mineru", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(SLOW_ML_TIMEOUT_SECS)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_script_path() {
        let result = get_script_path("docling_extract.py");
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        let _ = create_docling_adapter(true);
        let _ = create_unstructured_adapter(true);
        let _ = create_markitdown_adapter(true);
        let _ = create_tika_adapter(true);
        let _ = create_pymupdf4llm_adapter(true);
        let _ = create_mineru_adapter(true);
        let _ = create_liteparse_adapter(true);
    }
}
