//! Comprehensive fixture validation integration tests
//!
//! This module ensures the fixture corpus maintains quality and consistency by:
//! - Validating JSON parsing
//! - Verifying fixture structure and required fields
//! - Checking document file existence
//! - Verifying file size metadata matches actual files
//! - Validating ground truth files exist
//! - Detecting duplicate document references
//! - Ensuring format coverage for core formats

use benchmark_harness::Fixture;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Find all fixture JSON files recursively from the fixtures directory
fn discover_fixture_files() -> Vec<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixtures_dir = Path::new(manifest_dir).join("fixtures");

    let mut fixtures = Vec::new();
    if let Ok(entries) = fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_fixtures_recursive(&path, &mut fixtures);
            } else if is_json_fixture(&path) {
                fixtures.push(path);
            }
        }
    }

    fixtures.sort();
    fixtures
}

/// Recursively discover fixture JSON files in a directory
fn discover_fixtures_recursive(dir: &Path, fixtures: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_fixtures_recursive(&path, fixtures);
            } else if is_json_fixture(&path) {
                fixtures.push(path);
            }
        }
    }
}

/// Check if a path is a JSON fixture file (ends with .json)
fn is_json_fixture(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("json")
        && !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".split.json"))
}

#[test]
fn all_fixtures_parse_as_valid_json() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut parse_errors = Vec::new();

    for fixture_path in &fixtures {
        match fs::read_to_string(fixture_path) {
            Ok(contents) => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&contents) {
                    parse_errors.push(format!("{}: Invalid JSON: {}", fixture_path.display(), e));
                }
            }
            Err(e) => {
                parse_errors.push(format!("{}: Cannot read file: {}", fixture_path.display(), e));
            }
        }
    }

    if !parse_errors.is_empty() {
        panic!(
            "JSON parsing failures ({}):\n{}",
            parse_errors.len(),
            parse_errors.join("\n")
        );
    }
}

#[test]
fn all_fixtures_deserialize_and_validate() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut validation_errors = Vec::new();

    for fixture_path in &fixtures {
        match Fixture::from_file(fixture_path) {
            Ok(fixture) => {
                if fixture.file_type.is_empty() {
                    validation_errors.push(format!("{}: file_type cannot be empty", fixture_path.display()));
                }

                if fixture.document.is_absolute() {
                    validation_errors.push(format!(
                        "{}: document path must be relative, got {}",
                        fixture_path.display(),
                        fixture.document.display()
                    ));
                }
            }
            Err(e) => {
                validation_errors.push(format!(
                    "{}: Deserialization/validation failed: {}",
                    fixture_path.display(),
                    e
                ));
            }
        }
    }

    if !validation_errors.is_empty() {
        panic!(
            "Fixture validation failures ({}):\n{}",
            validation_errors.len(),
            validation_errors.join("\n")
        );
    }
}

#[test]
fn all_fixture_documents_exist_on_disk() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut missing_files = Vec::new();

    for fixture_path in &fixtures {
        match Fixture::from_file(fixture_path) {
            Ok(fixture) => {
                let fixture_dir = fixture_path
                    .parent()
                    .expect("fixture path should have parent directory");
                let document_path = fixture_dir.join(&fixture.document);

                if !document_path.exists() {
                    missing_files.push(format!(
                        "{}: Document not found at {} (resolved from {})",
                        fixture_path.display(),
                        document_path.display(),
                        fixture.document.display()
                    ));
                }
            }
            Err(e) => {
                missing_files.push(format!(
                    "{}: Cannot validate document existence: {}",
                    fixture_path.display(),
                    e
                ));
            }
        }
    }

    if !missing_files.is_empty() {
        panic!(
            "Missing fixture documents ({}):\n{}",
            missing_files.len(),
            missing_files.join("\n")
        );
    }
}

#[test]
fn all_fixture_file_sizes_match() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut size_mismatches = Vec::new();

    for fixture_path in &fixtures {
        match Fixture::from_file(fixture_path) {
            Ok(fixture) => {
                let fixture_dir = fixture_path
                    .parent()
                    .expect("fixture path should have parent directory");
                let document_path = fixture_dir.join(&fixture.document);

                if document_path.exists() {
                    match fs::metadata(&document_path) {
                        Ok(metadata) => {
                            let actual_size = metadata.len();
                            if actual_size != fixture.file_size {
                                size_mismatches.push(format!(
                                    "{}: file_size mismatch - expected {} bytes, actual {} bytes ({})",
                                    fixture_path.display(),
                                    fixture.file_size,
                                    actual_size,
                                    fixture.document.display()
                                ));
                            }
                        }
                        Err(e) => {
                            size_mismatches.push(format!(
                                "{}: Cannot read file metadata: {}",
                                fixture_path.display(),
                                e
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                size_mismatches.push(format!("{}: Cannot validate file sizes: {}", fixture_path.display(), e));
            }
        }
    }

    if !size_mismatches.is_empty() {
        panic!(
            "File size mismatches ({}):\n{}",
            size_mismatches.len(),
            size_mismatches.join("\n")
        );
    }
}

#[test]
fn all_ground_truth_files_exist() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut missing_ground_truth = Vec::new();

    for fixture_path in &fixtures {
        match Fixture::from_file(fixture_path) {
            Ok(fixture) => {
                if let Some(ground_truth) = &fixture.ground_truth
                    && let Some(ref tf) = ground_truth.text_file
                {
                    let fixture_dir = fixture_path
                        .parent()
                        .expect("fixture path should have parent directory");
                    let ground_truth_path = fixture_dir.join(tf);

                    if !ground_truth_path.exists() {
                        missing_ground_truth.push(format!(
                            "{}: Ground truth file not found at {} (resolved from {})",
                            fixture_path.display(),
                            ground_truth_path.display(),
                            tf.display()
                        ));
                    }
                }
            }
            Err(e) => {
                missing_ground_truth.push(format!(
                    "{}: Cannot validate ground truth: {}",
                    fixture_path.display(),
                    e
                ));
            }
        }
    }

    if !missing_ground_truth.is_empty() {
        panic!(
            "Missing ground truth files ({}):\n{}",
            missing_ground_truth.len(),
            missing_ground_truth.join("\n")
        );
    }
}

#[test]
fn no_duplicate_document_references() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let mut document_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    let mut duplicates = Vec::new();

    for fixture_path in &fixtures {
        match Fixture::from_file(fixture_path) {
            Ok(fixture) => {
                let fixture_dir = fixture_path
                    .parent()
                    .expect("fixture path should have parent directory");
                let document_path = fixture_dir.join(&fixture.document);

                let canonical_path = if document_path.exists() {
                    match document_path.canonicalize() {
                        Ok(p) => p,
                        Err(_) => document_path.clone(),
                    }
                } else {
                    document_path.clone()
                };

                document_map
                    .entry(canonical_path)
                    .or_default()
                    .push(fixture_path.clone());
            }
            Err(e) => {
                duplicates.push(format!(
                    "{}: Cannot check for duplicates: {}",
                    fixture_path.display(),
                    e
                ));
            }
        }
    }

    for (doc_path, fixture_paths) in document_map {
        if fixture_paths.len() > 1 {
            duplicates.push(format!(
                "Document {} is referenced by {} fixtures:\n{}",
                doc_path.display(),
                fixture_paths.len(),
                fixture_paths
                    .iter()
                    .map(|p| format!("  - {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    if !duplicates.is_empty() {
        panic!(
            "Duplicate document references found ({}):\n{}",
            duplicates.len(),
            duplicates.join("\n\n")
        );
    }
}

#[test]
fn core_formats_have_fixture_coverage() {
    let fixtures = discover_fixture_files();
    assert!(
        !fixtures.is_empty(),
        "No fixture JSON files found in fixtures directory"
    );

    let required_formats = vec![
        "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "html", "csv", "json", "xml", "yaml", "md", "txt", "eml",
        "epub", "rtf", "odt", "png", "jpg", "gif", "bmp", "tiff", "webp",
    ];

    let mut covered_formats: HashSet<String> = HashSet::new();
    let mut format_examples: HashMap<String, Vec<String>> = HashMap::new();

    for fixture_path in &fixtures {
        if let Ok(fixture) = Fixture::from_file(fixture_path) {
            let file_type_lower = fixture.file_type.to_lowercase();

            if required_formats.contains(&file_type_lower.as_str()) {
                covered_formats.insert(file_type_lower.clone());
            }

            format_examples.entry(file_type_lower).or_default().push(
                fixture_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }

    let mut missing_formats = Vec::new();
    for format in &required_formats {
        if !covered_formats.contains(*format) {
            missing_formats.push(format.to_string());
        }
    }

    if !missing_formats.is_empty() {
        panic!(
            "Missing format coverage for core formats ({}):\n\
             Required: {}\n\
             Missing: {}\n\
             Covered: {}",
            missing_formats.len(),
            required_formats.join(", "),
            missing_formats.join(", "),
            covered_formats.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    eprintln!("\nFormat Coverage Summary:");
    eprintln!("========================");
    for format in required_formats.iter().copied() {
        let count = format_examples.get(format).map(|v| v.len()).unwrap_or(0);
        eprintln!("  {}: {} fixture(s)", format, count);
    }
}

/// Test individual fixture structure and content
/// This is a helper that can be used to validate a specific fixture
#[test]
fn fixture_structure_is_valid() {
    let sample_json = json!({
        "document": "relative/path/to/document.pdf",
        "file_type": "pdf",
        "file_size": 1024,
        "expected_frameworks": ["xberg"],
        "metadata": {
            "description": "Test document",
            "category": "sample"
        },
        "ground_truth": {
            "text_file": "relative/path/to/ground_truth.txt",
            "source": "manual"
        }
    });

    let result: Result<Fixture, _> = serde_json::from_value(sample_json);
    assert!(
        result.is_ok(),
        "Sample fixture structure should deserialize: {:?}",
        result.err()
    );

    let fixture = result.unwrap();
    assert_eq!(fixture.file_type, "pdf");
    assert_eq!(fixture.file_size, 1024);
    assert_eq!(fixture.expected_frameworks.len(), 1);
    assert!(fixture.ground_truth.is_some());
}
