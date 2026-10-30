use super::parse_sha256_manifest;

fn entry(path: &str, checksum: char) -> String {
    format!("{}  {path}\n", checksum.to_string().repeat(64))
}

#[test]
fn duplicate_artifact_paths_are_rejected_even_with_identical_hashes() {
    let manifest = entry("model.onnx", 'a').repeat(2);
    let error = parse_sha256_manifest(&manifest).expect_err("duplicate artifact identity");
    assert!(error.contains("duplicate"), "{error}");
}

#[test]
fn normalized_artifact_paths_cannot_hide_conflicting_checksums() {
    let manifest = entry("./model.onnx", 'a') + &entry("model.onnx", 'b');
    let error = parse_sha256_manifest(&manifest).expect_err("conflicting normalized identity");
    assert!(error.contains("duplicate"), "{error}");
}

#[test]
fn extra_fields_cannot_be_silently_ignored() {
    let manifest = format!("{}  model.onnx unexpected\n", "a".repeat(64));
    let error = parse_sha256_manifest(&manifest).expect_err("extra field");
    assert!(error.contains("exactly"), "{error}");
}

#[test]
fn artifact_paths_must_be_portable_relative_files() {
    for path in [
        "/model.onnx",
        "../model.onnx",
        "nested/../model.onnx",
        "nested/./model.onnx",
        "nested//model.onnx",
        "nested/",
        ".",
        "./",
        "C:/model.onnx",
        "C:\\model.onnx",
        "nested\\model.onnx",
        "model.onnx:weights",
        "https://example.com/model.onnx",
    ] {
        let error = parse_sha256_manifest(&entry(path, 'a')).expect_err(path);
        assert!(error.contains("path"), "{path}: {error}");
    }
}

#[test]
fn nested_artifact_paths_retain_order_and_normalized_checksums() {
    let manifest = entry("./encoder/model.onnx", 'A') + &entry("decoder/model.onnx.data", 'b');
    assert_eq!(
        parse_sha256_manifest(&manifest).expect("valid paths"),
        vec![
            ("encoder/model.onnx".to_owned(), "a".repeat(64)),
            ("decoder/model.onnx.data".to_owned(), "b".repeat(64))
        ]
    );
}

#[test]
fn checksum_parser_keeps_comment_only_manifests_available_to_its_callers() {
    assert_eq!(parse_sha256_manifest("# no selected files\n\n").unwrap(), vec![]);
}

#[test]
fn artifact_components_reject_windows_reserved_characters() {
    for path in [
        "model?.onnx",
        "weights*.data",
        "encoder<old>/model.onnx",
        "encoder>new/model.onnx",
        "model\".onnx",
        "encoder|decoder/model.onnx",
    ] {
        let error = parse_sha256_manifest(&entry(path, 'a')).expect_err(path);
        assert!(error.contains("path"), "{path}: {error}");
    }
}

#[test]
fn artifact_components_reject_windows_device_names_with_any_case_or_extension() {
    for path in [
        "CON",
        "prn.onnx",
        "AuX/config.json",
        "nul.tar.gz",
        "COM1.onnx",
        "com9.data",
        "LPT1/model.onnx",
        "lpt9.json",
        "COM¹.bin",
        "com².onnx",
        "COM³.json",
        "lpt¹.onnx",
        "LPT²/data",
        "lpt³.json",
    ] {
        let error = parse_sha256_manifest(&entry(path, 'a')).expect_err(path);
        assert!(error.contains("path"), "{path}: {error}");
    }
}

#[test]
fn trailing_dot_components_cannot_alias_distinct_manifest_entries() {
    for path in ["model.onnx.", "encoder./model.onnx", "encoder../model.onnx"] {
        let manifest = entry("model.onnx", 'a') + &entry(path, 'b');
        let error = parse_sha256_manifest(&manifest).expect_err(path);
        assert!(error.contains("path"), "{path}: {error}");
    }
}

#[test]
fn portable_components_allow_non_device_names_and_leading_dots() {
    let paths = [
        "CONSOLE/model.onnx",
        "COM0.onnx",
        "COM10.data",
        "LPT0.json",
        "LPT10.json",
        ".config.json",
    ];
    let manifest = paths.iter().map(|path| entry(path, 'a')).collect::<String>();
    let entries = parse_sha256_manifest(&manifest).expect("ordinary portable file names");
    assert_eq!(entries.iter().map(|(path, _)| path.as_str()).collect::<Vec<_>>(), paths);
}

#[test]
fn ascii_case_aliases_are_duplicates_even_with_equal_checksums() {
    for (original, alias) in [
        ("Model.onnx", "model.onnx"),
        ("Encoder/model.onnx", "encoder/model.onnx"),
        ("encoder/Model.onnx", "encoder/model.onnx"),
    ] {
        let manifest = entry(original, 'a') + &entry(alias, 'a');
        let error = parse_sha256_manifest(&manifest).expect_err(alias);
        assert!(error.contains("duplicate"), "{alias}: {error}");
    }
}

#[test]
fn ascii_case_aliases_cannot_hide_conflicting_checksums() {
    for (original, alias) in [
        ("Model.onnx", "model.onnx"),
        ("Encoder/model.onnx", "encoder/model.onnx"),
        ("encoder/Model.onnx", "encoder/model.onnx"),
    ] {
        let manifest = entry(original, 'a') + &entry(alias, 'b');
        let error = parse_sha256_manifest(&manifest).expect_err(alias);
        assert!(error.contains("duplicate"), "{alias}: {error}");
    }
}
