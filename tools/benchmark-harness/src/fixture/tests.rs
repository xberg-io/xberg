use super::*;
use tempfile::TempDir;
use xberg_child_test::{child_test_root, run_child_test};

const CHILD_CASE: &str = "XBERG_BENCHMARK_FIXTURE_CHILD_CASE";
const PROFILING_FIXTURES: &str = "PROFILING_FIXTURES";

fn run_fixture_env_case(case: &str, filter: Option<&str>) {
    let module = module_path!()
        .strip_prefix("benchmark_harness::")
        .expect("benchmark harness crate module prefix");
    run_child_test(&format!("{module}::{case}::child"), CHILD_CASE, case, |command| {
        if let Some(filter) = filter {
            command.env(PROFILING_FIXTURES, filter);
        } else {
            command.env_remove(PROFILING_FIXTURES);
        }
    });
}

macro_rules! isolated_fixture_test {
    (fn $case:ident [$filter:expr] $body:block) => {
        #[test]
        fn $case() {
            run_fixture_env_case(stringify!($case), $filter);
        }

        mod $case {
            use super::*;

            #[test]
            #[ignore = "run in an isolated subprocess"]
            fn child() {
                let Some(case) = child_test_root(CHILD_CASE) else {
                    return;
                };
                assert_eq!(case.as_os_str(), std::ffi::OsStr::new(stringify!($case)));
                $body
            }
        }
    };
}

#[test]
fn test_fixture_validation() {
    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec!["xberg".to_string()],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.validate(Path::new("fixture.json")).is_ok());
}

#[test]
fn file_type_matching_the_document_extension_exactly_is_accepted() {
    let fixture = Fixture {
        document: PathBuf::from("simple.typ"),
        file_type: "typ".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.validate(Path::new("fixture.json")).is_ok());
}

#[test]
fn file_type_naming_a_recognized_alias_of_the_document_extension_is_accepted() {
    // The corpus deliberately normalizes docbook4/docbook5 documents to the umbrella
    // "docbook" label (see fixtures/docbook_tables4.json), and xberg's MIME table lists
    // dbk/docbook/docbook4/docbook5 as legal aliases of the same format. That normalization
    // must keep working even though "docbook" isn't literally the document's extension.
    let fixture = Fixture {
        document: PathBuf::from("tables.docbook4"),
        file_type: "docbook".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.validate(Path::new("fixture.json")).is_ok());
}

#[test]
fn file_type_naming_an_unrelated_format_is_rejected_with_the_recognized_aliases() {
    let fixture = Fixture {
        document: PathBuf::from("photo.jpg"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    let error = fixture.validate(Path::new("fixture.json")).unwrap_err().to_string();
    assert!(
        error.contains("file_type 'pdf' does not match document extension '.jpg'"),
        "{error}"
    );
    assert!(error.contains("jpg"), "{error}");
    assert!(error.contains("jpeg"), "{error}");
}

#[test]
fn file_type_for_an_extension_unknown_to_the_mime_table_is_not_rejected() {
    // Extensions xberg's MIME table has never heard of (a genuinely new or synthetic
    // format) must not be blocked by this guard — only extensions the table *does*
    // recognize, but recognizes as a different format, are rejected.
    let fixture = Fixture {
        document: PathBuf::from("data.totally-unknown-extension"),
        file_type: "something-else".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.validate(Path::new("fixture.json")).is_ok());
}

#[test]
fn fixture_validation_accepts_apple_preview_ground_truth() {
    let root = TempDir::new().unwrap();
    let fixture_path = root.path().join("fixture.json");
    std::fs::write(root.path().join("expected.txt"), "cell value").unwrap();
    let fixture = Fixture {
        document: PathBuf::from("test.numbers"),
        file_type: "numbers".to_string(),
        file_size: 1024,
        expected_frameworks: vec!["xberg".to_string()],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("expected.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "apple_preview".to_string(),
        }),
    };

    assert!(fixture.validate(&fixture_path).is_ok());
}

#[test]
fn fixture_ocr_language_validation_accepts_codes_and_rejects_paths() {
    let make_fixture = |language: &str| Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec!["xberg".to_string()],
        metadata: HashMap::from([("ocr_language".to_string(), serde_json::json!(language))]),
        ground_truth: None,
    };

    assert!(make_fixture(" deu + eng ").validate(Path::new("fixture.json")).is_ok());
    assert!(make_fixture("+").validate(Path::new("fixture.json")).is_err());
    assert!(make_fixture("../deu").validate(Path::new("fixture.json")).is_err());
}

#[test]
fn test_absolute_path_rejected() {
    #[cfg(windows)]
    let absolute_path = PathBuf::from("C:\\absolute\\path\\test.pdf");
    #[cfg(not(windows))]
    let absolute_path = PathBuf::from("/absolute/path/test.pdf");

    let fixture = Fixture {
        document: absolute_path,
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.validate(Path::new("fixture.json")).is_err());
}

#[test]
fn fixture_paths_cannot_escape_a_standalone_fixture_tree() {
    let root = TempDir::new().unwrap();
    let fixture_dir = root.path().join("fixtures");
    std::fs::create_dir(&fixture_dir).unwrap();
    std::fs::write(root.path().join("secret.txt"), "not fixture data").unwrap();
    let fixture_path = fixture_dir.join("escape.json");
    let fixture = Fixture {
        document: PathBuf::from("../secret.txt"),
        file_type: "txt".to_string(),
        file_size: 16,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    let error = fixture.validate(&fixture_path).unwrap_err().to_string();
    assert!(error.contains("document escapes the fixture trust boundary"), "{error}");
}

#[test]
fn repository_fixture_boundary_is_derived_from_the_runtime_path() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("fixtures/pdf").canonicalize().unwrap();
    let expected_root = manifest_dir.join("../..").canonicalize().unwrap();

    assert_eq!(repository_fixture_root(&fixture_dir), Some(expected_root));
}

#[test]
fn ground_truth_paths_cannot_escape_a_standalone_fixture_tree() {
    let root = TempDir::new().unwrap();
    let fixture_dir = root.path().join("fixtures");
    std::fs::create_dir(&fixture_dir).unwrap();
    std::fs::write(root.path().join("secret.txt"), "not fixture data").unwrap();
    let fixture_path = fixture_dir.join("escape.json");
    let fixture = Fixture {
        document: PathBuf::from("document.txt"),
        file_type: "txt".to_string(),
        file_size: 16,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("../secret.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "manual".to_string(),
        }),
    };

    let error = fixture.validate(&fixture_path).unwrap_err().to_string();
    assert!(
        error.contains("ground_truth.text_file escapes the fixture trust boundary"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn fixture_paths_cannot_escape_through_symlinks() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let fixture_dir = root.path().join("fixtures");
    std::fs::create_dir(&fixture_dir).unwrap();
    std::fs::write(root.path().join("secret.txt"), "not fixture data").unwrap();
    symlink(root.path().join("secret.txt"), fixture_dir.join("document.txt")).unwrap();
    let fixture_path = fixture_dir.join("escape.json");
    let fixture = Fixture {
        document: PathBuf::from("document.txt"),
        file_type: "txt".to_string(),
        file_size: 16,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    let error = fixture.validate(&fixture_path).unwrap_err().to_string();
    assert!(
        error.contains("document resolves outside the fixture trust boundary"),
        "{error}"
    );
}

#[test]
fn test_fixture_manager_load() {
    let temp_dir = TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("test.json");

    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

    let mut manager = FixtureManager::new();
    assert!(manager.load_fixture(&fixture_path).is_ok());
    assert_eq!(manager.len(), 1);
}

isolated_fixture_test! {
    fn test_profiling_fixtures_with_env_var [Some("pdf_small,docx_simple")] {
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple", "html_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }


        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 2);

        let loaded_names: Vec<String> = manager
            .fixtures()
            .iter()
            .filter_map(|(path, _)| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .collect();

        assert!(loaded_names.contains(&"pdf_small".to_string()));
        assert!(loaded_names.contains(&"docx_simple".to_string()));
        assert!(!loaded_names.contains(&"pdf_medium".to_string()));
        assert!(!loaded_names.contains(&"html_simple".to_string()));
    }
}

isolated_fixture_test! {
    fn test_profiling_fixtures_all_when_env_not_set [None] {
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }


        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 3, "all fixtures load with no profiling filter");
    }
}

#[test]
#[should_panic(expected = "all fixtures load with no profiling filter")]
fn test_wrong_child_filter_fails_the_no_filter_case() {
    run_fixture_env_case("test_profiling_fixtures_all_when_env_not_set", Some("pdf_small"));
}

isolated_fixture_test! {
    fn test_fixture_discovery_is_sorted [None] {
        let temp_dir = TempDir::new().unwrap();

        for fixture_name in ["zeta", "alpha", "middle"] {
            let fixture = Fixture {
                document: PathBuf::from(format!("{fixture_name}.pdf")),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(
                temp_dir.path().join(format!("{fixture_name}.json")),
                serde_json::to_string(&fixture).unwrap(),
            )
            .unwrap();
        }

        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();
        let loaded_names: Vec<_> = manager
            .fixtures()
            .iter()
            .map(|(path, _)| path.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(loaded_names, ["alpha", "middle", "zeta"]);
    }
}

isolated_fixture_test! {
    fn test_profiling_fixtures_with_whitespace [Some("pdf_small , pdf_medium , docx_simple")] {
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple", "html_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }


        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 3);
    }
}

isolated_fixture_test! {
    fn test_profiling_fixtures_partial_match [Some("pdf_small,nonexistent_fixture")] {
        let temp_dir = TempDir::new().unwrap();

        let fixtures = vec!["pdf_small", "pdf_medium", "docx_simple"];
        for fixture_name in &fixtures {
            let fixture_path = temp_dir.path().join(format!("{}.json", fixture_name));
            let fixture = Fixture {
                document: PathBuf::from(format!("{}.pdf", fixture_name)),
                file_type: "pdf".to_string(),
                file_size: 1024,
                expected_frameworks: vec![],
                metadata: HashMap::new(),
                ground_truth: None,
            };
            std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
        }


        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 1);

        let loaded_names: Vec<String> = manager
            .fixtures()
            .iter()
            .filter_map(|(path, _)| path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
            .collect();

        assert!(loaded_names.contains(&"pdf_small".to_string()));
    }
}

#[test]
fn test_requires_ocr_for_image_types() {
    let image_types = vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"];

    for file_type in image_types {
        let fixture = Fixture {
            document: PathBuf::from(format!("test.{}", file_type)),
            file_type: file_type.to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        assert!(
            fixture.requires_ocr(),
            "Expected file type {} to require OCR",
            file_type
        );
    }
}

#[test]
fn test_requires_ocr_for_non_image_types() {
    let non_image_types = vec!["pdf", "docx", "txt", "html", "md"];

    for file_type in non_image_types {
        let fixture = Fixture {
            document: PathBuf::from(format!("test.{}", file_type)),
            file_type: file_type.to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };

        assert!(
            !fixture.requires_ocr(),
            "Expected file type {} to not require OCR",
            file_type
        );
    }
}

#[test]
fn test_requires_ocr_explicit_metadata_true() {
    let mut metadata = HashMap::new();
    metadata.insert("requires_ocr".to_string(), serde_json::json!(true));

    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata,
        ground_truth: None,
    };

    assert!(fixture.requires_ocr());
}

#[test]
fn test_requires_ocr_explicit_metadata_false() {
    let mut metadata = HashMap::new();
    metadata.insert("requires_ocr".to_string(), serde_json::json!(false));

    let fixture = Fixture {
        document: PathBuf::from("test.png"),
        file_type: "png".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata,
        ground_truth: None,
    };

    assert!(!fixture.requires_ocr());
}

#[test]
fn test_requires_ocr_case_insensitive() {
    let fixture = Fixture {
        document: PathBuf::from("test.JPG"),
        file_type: "JPG".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: None,
    };

    assert!(fixture.requires_ocr());
}

#[test]
fn test_ground_truth_file_existence_validation() {
    let temp_dir = TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("test.json");

    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("nonexistent_ground_truth.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "manual".to_string(),
        }),
    };

    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

    let result = Fixture::from_file(&fixture_path);
    assert!(result.is_err());
    match result {
        Err(Error::InvalidFixture { reason, .. }) => {
            assert!(reason.contains("unable to read ground truth text file"));
        }
        _ => panic!("Expected InvalidFixture error for unreadable ground truth"),
    }
}

#[test]
fn test_markdown_only_ground_truth_is_validated() {
    let temp_dir = TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("test.json");
    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: None,
            markdown_file: Some(PathBuf::from("missing.md")),
            fields_json: None,
            formulas_json: None,
            source: "markdown_file".to_string(),
        }),
    };
    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

    let error = Fixture::from_file(&fixture_path).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidFixture { reason, .. }
            if reason.contains("unable to read ground truth markdown file")
    ));
}

#[test]
fn test_non_utf8_ground_truth_is_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("test.json");
    std::fs::write(temp_dir.path().join("ground_truth.txt"), [0xff, 0xfe]).unwrap();
    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("ground_truth.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "manual".to_string(),
        }),
    };
    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

    let error = Fixture::from_file(&fixture_path).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidFixture { reason, .. }
            if reason.contains("unable to read ground truth text file")
    ));
}

#[test]
fn test_ground_truth_file_existence_validation_success() {
    let temp_dir = TempDir::new().unwrap();
    let fixture_path = temp_dir.path().join("test.json");
    let ground_truth_path = temp_dir.path().join("ground_truth.txt");

    std::fs::write(&ground_truth_path, "Sample ground truth text").unwrap();

    let fixture = Fixture {
        document: PathBuf::from("test.pdf"),
        file_type: "pdf".to_string(),
        file_size: 1024,
        expected_frameworks: vec![],
        metadata: HashMap::new(),
        ground_truth: Some(GroundTruth {
            text_file: Some(PathBuf::from("ground_truth.txt")),
            markdown_file: None,
            fields_json: None,
            formulas_json: None,
            source: "manual".to_string(),
        }),
    };

    std::fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();

    let result = Fixture::from_file(&fixture_path);
    assert!(result.is_ok());
}

isolated_fixture_test! {
    fn test_fixture_load_with_mixed_success_and_failure [None] {
        let temp_dir = TempDir::new().unwrap();

        let valid_fixture_path = temp_dir.path().join("valid.json");
        let valid_fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: None,
        };
        std::fs::write(&valid_fixture_path, serde_json::to_string(&valid_fixture).unwrap()).unwrap();

        let invalid_fixture_path = temp_dir.path().join("invalid.json");
        let invalid_fixture = Fixture {
            document: PathBuf::from("test.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1024,
            expected_frameworks: vec![],
            metadata: HashMap::new(),
            ground_truth: Some(GroundTruth {
                text_file: Some(PathBuf::from("nonexistent.txt")),
                markdown_file: None,
                fields_json: None,
                formulas_json: None,
                source: "manual".to_string(),
            }),
        };
        std::fs::write(&invalid_fixture_path, serde_json::to_string(&invalid_fixture).unwrap()).unwrap();


        let mut manager = FixtureManager::new();
        let result = manager.load_fixtures_from_dir(temp_dir.path());
        assert!(result.is_err());

        assert_eq!(manager.len(), 1);
    }
}

isolated_fixture_test! {
    fn nested_invalid_fixture_fails_the_entire_directory_load [None] {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested");
        std::fs::create_dir(&nested_dir).unwrap();
        std::fs::write(
            temp_dir.path().join("valid.json"),
            serde_json::json!({
                "document": "valid.pdf",
                "file_type": "pdf",
                "file_size": 1
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(nested_dir.join("malformed.json"), "{not valid json").unwrap();

        let mut manager = FixtureManager::new();
        let error = manager.load_fixtures_from_dir(temp_dir.path()).unwrap_err();

        assert!(error.to_string().contains("malformed.json"));
        assert_eq!(
            manager.len(),
            0,
            "nested failure must abort before the parent corpus loads"
        );
    }
}

isolated_fixture_test! {
    fn directory_load_skips_split_sidecars [None] {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("fixture.json"),
            serde_json::json!({
                "document": "document.pdf",
                "file_type": "pdf",
                "file_size": 1
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            temp_dir.path().join("document.split.json"),
            serde_json::json!({
                "document": "document.pdf",
                "boundaries": [{"start_page": 1, "end_page": 2}]
            })
            .to_string(),
        )
        .unwrap();
        let mut manager = FixtureManager::new();
        manager.load_fixtures_from_dir(temp_dir.path()).unwrap();

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.fixtures()[0].0.file_name().unwrap(), "fixture.json");
    }
}

isolated_fixture_test! {
    fn directory_load_rejects_partial_fixture_schema [None] {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("incomplete.json"),
            serde_json::json!({"document": "document.pdf"}).to_string(),
        )
        .unwrap();

        let mut manager = FixtureManager::new();
        let error = manager.load_fixtures_from_dir(temp_dir.path()).unwrap_err();

        assert!(error.to_string().contains("incomplete.json"));
    }
}
