//! `--content-format docx` and `--content-format pdf` write the binary document itself to
//! stdout, and batch text output, which joins documents under headers, refuses them. Batch
//! JSON output, the default, carries each document base64-encoded in `content`.

use std::path::PathBuf;
use std::process::Command;

fn xberg_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_xberg"))
}

fn report() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("report.md");
    std::fs::write(&path, "# Quarterly report\n\nRevenue grew in Q3.\n").expect("write the input");
    (dir, path)
}

fn extract(content_format: &str) -> Vec<u8> {
    let (_dir, input) = report();
    let output = Command::new(xberg_bin())
        .args(["extract", &input.to_string_lossy(), "--content-format", content_format])
        .output()
        .expect("run xberg extract");
    assert!(
        output.status.success(),
        "extract exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn extract_writes_a_pdf_file_to_stdout() {
    let stdout = extract("pdf");
    assert!(
        stdout.starts_with(b"%PDF-"),
        "{:?}",
        String::from_utf8_lossy(&stdout[..stdout.len().min(40)])
    );
}

#[test]
fn extract_writes_a_docx_package_to_stdout() {
    let stdout = extract("docx");
    assert!(stdout.starts_with(b"PK\x03\x04"), "not a zip archive");
}

#[test]
fn batch_text_output_refuses_a_binary_content_format() {
    let (_dir, input) = report();
    let output = Command::new(xberg_bin())
        .args([
            "batch",
            &input.to_string_lossy(),
            "--format",
            "text",
            "--content-format",
            "pdf",
        ])
        .output()
        .expect("run xberg batch");
    assert!(!output.status.success(), "batch accepted binary text output");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--content-format pdf"), "{stderr}");
    assert!(stderr.contains("--format json"), "{stderr}");
}
