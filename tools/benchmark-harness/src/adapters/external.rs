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
        // Pandoc: 45+ input formats, but CANNOT read PDF (output only)
        // See: pandoc --list-input-formats
        // Only list formats that pandoc can auto-detect from file extension
        // and reliably convert to plain text via --to=plain.
        // Excluded: pptx, xlsx (return empty text), bib (needs explicit --from=biblatex),
        //           ris (returns empty text), dbk (unreliable auto-detection)
        "pandoc" => vec![
            "docx", "odt", // Office documents
            "md", "markdown", "rst", "org", "typst", // Markup languages
            "html", "htm", // Web formats
            "csv", "tsv", // Data formats
            "tex", "latex", "ipynb", // Scientific/technical
            "epub",  // E-books
            "rtf", "txt", // Other documents
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        // pdfplumber: PDF-only (built on pdfminer.six)
        "pdfplumber" => vec!["pdf".to_string()],

        // pypdf: PDF-only (pure Python PDF library)
        "pypdf" => vec!["pdf".to_string()],

        // playa-pdf: PDF-only (pure Python PDF library)
        "playa-pdf" => vec!["pdf".to_string()],

        // pdfminer.six: PDF-only (Python PDF text extraction)
        "pdfminer" => vec!["pdf".to_string()],

        // pdftotext: PDF-only (Python binding for poppler's pdftotext)
        "pdftotext" => vec!["pdf".to_string()],

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

/// Creates a subprocess adapter for Docling (persistent server mode)
///
/// Uses persistent mode to avoid repeated Python startup and heavy ML model
/// loading on every file.
pub fn create_docling_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("docling_extract.py")?;
    let (command, mut args) = find_python_with_framework("docling")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("docling");
    Ok(
        SubprocessAdapter::with_persistent_mode("docling", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for Unstructured (persistent server mode)
///
/// Uses persistent mode to keep the Python process alive, avoiding repeated
/// import overhead for the heavy unstructured ML library on each file.
pub fn create_unstructured_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("unstructured_extract.py")?;
    let (command, mut args) = find_python_with_framework("unstructured")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("unstructured");
    Ok(
        SubprocessAdapter::with_persistent_mode("unstructured", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for MarkItDown (persistent server mode)
pub fn create_markitdown_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("markitdown_extract.py")?;
    let (command, mut args) = find_python_with_framework("markitdown")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("markitdown");
    Ok(
        SubprocessAdapter::with_persistent_mode("markitdown", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for Pandoc (universal document converter)
pub fn create_pandoc_adapter() -> Result<SubprocessAdapter> {
    which::which("pandoc").map_err(|_| {
        crate::Error::Config(
            "pandoc not found. Install with: brew install pandoc (macOS) or apt install pandoc (Linux)".to_string(),
        )
    })?;

    let script_path = get_script_path("pandoc_extract.sh")?;
    let command = PathBuf::from("bash");
    let args = vec![script_path.to_string_lossy().to_string()];

    let supported_formats = get_supported_formats("pandoc");
    Ok(
        SubprocessAdapter::new("pandoc", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(180)),
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

    Err(crate::Error::Config(
        "Tika JAR not found. Download: curl -LO https://repo1.maven.org/maven2/org/apache/tika/tika-app/3.2.3/tika-app-3.2.3.jar && mv tika-app-3.2.3.jar tools/benchmark-harness/libs/".to_string()
    ))
}

/// Creates a subprocess adapter for Apache Tika (persistent server mode)
///
/// Uses Tika's built-in server mode which keeps the JVM alive and accepts
/// file paths via stdin, eliminating per-file JVM startup overhead.
pub fn create_tika_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let jar_path = get_tika_jar_path()?;
    let script_path = get_script_path("TikaExtract.java")?;
    let command = find_java()?;

    let args = vec![
        "-server".to_string(),
        "-Xms512m".to_string(),
        "-Xmx2g".to_string(),
        "-XX:+UseG1GC".to_string(),
        "-cp".to_string(),
        jar_path.to_string_lossy().to_string(),
        script_path.to_string_lossy().to_string(),
        ocr_flag(ocr_enabled),
        "server".to_string(),
    ];

    let supported_formats = get_supported_formats("tika");
    Ok(
        SubprocessAdapter::with_persistent_mode("tika", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(180)),
    )
}

/// Creates a subprocess adapter for PyMuPDF4LLM (persistent server mode)
pub fn create_pymupdf4llm_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pymupdf4llm_extract.py")?;
    let (command, mut args) = find_python_with_framework("pymupdf4llm")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("pymupdf4llm");
    Ok(
        SubprocessAdapter::with_persistent_mode("pymupdf4llm", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(SLOW_ML_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for pdfplumber (persistent server mode)
pub fn create_pdfplumber_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pdfplumber_extract.py")?;
    let (command, mut args) = find_python_with_framework("pdfplumber")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("pdfplumber");
    Ok(
        SubprocessAdapter::with_persistent_mode("pdfplumber", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for pypdf (persistent server mode)
pub fn create_pypdf_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pypdf_extract.py")?;
    let (command, mut args) = find_python_with_framework("pypdf")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("pypdf");
    Ok(
        SubprocessAdapter::with_persistent_mode("pypdf", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for playa-pdf (persistent server mode)
pub fn create_playa_pdf_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("playa_pdf_extract.py")?;
    let (command, mut args) = find_python_with_framework("playa")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("playa-pdf");
    Ok(
        SubprocessAdapter::with_persistent_mode("playa-pdf", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for pdfminer.six (persistent server mode)
pub fn create_pdfminer_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pdfminer_extract.py")?;
    let (command, mut args) = find_python_with_framework("pdfminer")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("pdfminer");
    Ok(
        SubprocessAdapter::with_persistent_mode("pdfminer", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for pdftotext (persistent server mode)
///
/// Requires poppler-utils system package for the Python pdftotext binding.
pub fn create_pdftotext_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("pdftotext_extract.py")?;
    let (command, mut args) = find_python_with_framework("pdftotext")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("pdftotext");
    Ok(
        SubprocessAdapter::with_persistent_mode("pdftotext", command, args, vec![], supported_formats)
            .with_max_timeout(Duration::from_secs(PERSISTENT_MAX_TIMEOUT_SECS)),
    )
}

/// Creates a subprocess adapter for MinerU (persistent server mode)
///
/// Uses persistent mode to avoid repeated Python startup and heavy ML model
/// loading on every file.
pub fn create_mineru_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("mineru_extract.py")?;
    let (command, mut args) = find_python_with_framework("mineru")?;
    args.push(script_path.to_string_lossy().to_string());
    args.push(format!("--timeout={}", PYTHON_EXTRACTION_TIMEOUT_SECS));
    args.push(ocr_flag(ocr_enabled));
    args.push("server".to_string());

    let supported_formats = get_supported_formats("mineru");
    Ok(
        SubprocessAdapter::with_persistent_mode("mineru", command, args, vec![], supported_formats)
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
        let _ = create_pandoc_adapter();
        let _ = create_tika_adapter(true);
        let _ = create_pymupdf4llm_adapter(true);
        let _ = create_pdfplumber_adapter(true);
        let _ = create_mineru_adapter(true);
        let _ = create_pypdf_adapter(true);
        let _ = create_pdfminer_adapter(true);
        let _ = create_pdftotext_adapter(true);
        let _ = create_playa_pdf_adapter(true);
    }
}
