//! Kreuzberg Rust extraction subprocess for fair benchmarking.
//!
//! This binary runs kreuzberg extraction in a subprocess, matching the same
//! protocol used by Python/Node/Ruby extraction scripts. This ensures fair
//! timing comparisons by including subprocess overhead equally for all frameworks.
//!
//! Protocol:
//! - Prints "READY" on startup
//! - Reads JSON requests from stdin: {"path": "/path/to/file", "force_ocr": true}
//!   (also accepts plain file paths for backward compatibility)
//! - Outputs JSON to stdout: {"content": "...", "_extraction_time_ms": 123.4, "_ocr_used": false}
//! - On error: {"error": "message"}

use kreuzberg::{
    ExtractionConfig, FileExtractionConfig, FormatMetadata, OcrConfig, batch_extract_file_sync, extract_file_sync,
};
use serde_json::json;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let batch_mode = args.iter().any(|a| a == "batch");
    let ocr_enabled = args.iter().any(|a| a == "--ocr");

    // Parse --ocr-backend <backend> (default: tesseract)
    let ocr_backend = args
        .windows(2)
        .find(|w| w[0] == "--ocr-backend")
        .map(|w| w[1].as_str())
        .unwrap_or("tesseract");

    // Parse --layout-preset <preset> (e.g., "fast" or "accurate")
    let layout_preset = args
        .windows(2)
        .find(|w| w[0] == "--layout-preset")
        .map(|w| w[1].clone());

    let layout_config = layout_preset.map(|preset| kreuzberg::core::config::layout::LayoutDetectionConfig {
        preset,
        ..Default::default()
    });

    let config = ExtractionConfig {
        use_cache: false,
        ocr: if ocr_enabled {
            Some(OcrConfig {
                backend: ocr_backend.to_string(),
                language: "eng".to_string(),
                ..Default::default()
            })
        } else {
            None
        },
        layout: layout_config,
        ..Default::default()
    };

    // Warmup: validate that the configured OCR backend is available and trigger
    // lazy initialization (plugin discovery, allocator warmup, etc.).
    // If the backend isn't registered (e.g., PaddleOCR without ONNX Runtime),
    // exit early so the harness reports an initialization failure instead of
    // running N extractions that all fail with "not registered".
    {
        let warmup_dir = std::env::temp_dir();
        let warmup_path = warmup_dir.join("kreuzberg-benchmark-warmup.pdf");
        // Minimal valid PDF for warmup
        let _ = std::fs::write(&warmup_path, b"%PDF-1.0\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/MediaBox[0 0 3 3]/Parent 2 0 R/Resources<<>>>>endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \ntrailer<</Size 4/Root 1 0 R>>\nstartxref\n206\n%%EOF");
        if let Err(e) = extract_file_sync(warmup_path.to_str().unwrap_or(""), None, &config) {
            let err_str = format!("{}", e);
            if err_str.contains("not registered") || err_str.contains("not available") {
                eprintln!("Fatal: OCR backend '{}' not available: {}", ocr_backend, e);
                std::process::exit(1);
            }
            // Other errors (e.g., empty PDF) are fine for warmup
        }
        let _ = std::fs::remove_file(&warmup_path);
    }

    // Batch mode: receive file paths as positional args, output JSON array
    if batch_mode {
        // Collect file paths: everything after "batch" that isn't a flag
        let batch_idx = args.iter().position(|a| a == "batch").unwrap();
        let file_paths: Vec<PathBuf> = args[batch_idx + 1..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .map(PathBuf::from)
            .collect();

        let items: Vec<(PathBuf, Option<FileExtractionConfig>)> = file_paths.into_iter().map(|p| (p, None)).collect();

        let start = Instant::now();
        match batch_extract_file_sync(items, &config) {
            Ok(results) => {
                let total_ms = start.elapsed().as_secs_f64() * 1000.0;
                let per_file_ms = total_ms / results.len().max(1) as f64;
                let output: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|r| {
                        json!({
                            "content": r.content,
                            "_extraction_time_ms": per_file_ms,
                            "_batch_total_ms": total_ms,
                            "_ocr_used": ocr_enabled,
                        })
                    })
                    .collect();
                println!("{}", serde_json::Value::Array(output));
                io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("Batch extraction failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Signal readiness (persistent server mode)
    println!("READY");
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let raw_line = match line {
            Ok(l) => l.trim().to_string(),
            Err(_) => break,
        };

        if raw_line.is_empty() {
            continue;
        }

        // Parse JSON request or fall back to plain file path
        let (file_path, force_ocr) = if raw_line.starts_with('{') {
            match serde_json::from_str::<serde_json::Value>(&raw_line) {
                Ok(req) => {
                    let path = req.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let fo = req.get("force_ocr").and_then(|v| v.as_bool()).unwrap_or(false);
                    (path, fo)
                }
                Err(_) => (raw_line, false),
            }
        } else {
            (raw_line, false)
        };

        if file_path.is_empty() {
            continue;
        }

        // Apply force_ocr override to config for this request
        let effective_config = if force_ocr && !config.force_ocr {
            let mut c = config.clone();
            c.force_ocr = true;
            c
        } else {
            config.clone()
        };

        let start = Instant::now();
        match extract_file_sync(&file_path, None, &effective_config) {
            Ok(result) => {
                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                let ocr_used = (ocr_enabled || force_ocr)
                    && matches!(
                        &result.metadata.format,
                        Some(FormatMetadata::Ocr(_)) | Some(FormatMetadata::Image(_)) | Some(FormatMetadata::Pdf(_))
                    );

                let output = json!({
                    "content": result.content,
                    "_extraction_time_ms": duration_ms,
                    "_ocr_used": ocr_used,
                });
                println!("{}", output);
                io::stdout().flush().unwrap();
            }
            Err(e) => {
                let output = json!({
                    "error": format!("{}", e),
                });
                println!("{}", output);
                io::stdout().flush().unwrap();
            }
        }
    }
}
