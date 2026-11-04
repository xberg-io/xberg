#[path = "../build_support/output.rs"]
mod output;

use std::path::Path;
use std::process::Command;

const BUILD_SCRIPT: &str = include_str!("../build.rs");
const FIXTURE_MODE: &str = "XBERG_BUILD_OUTPUT_FIXTURE";

#[test]
fn should_keep_static_build_progress_out_of_cargo_warnings() {
    assert_build_output("static");
}

#[test]
fn should_keep_dynamic_build_progress_out_of_cargo_warnings() {
    assert_build_output("dylib");
}

fn assert_build_output(mode: &str) {
    let captured = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "output_fixture",
            "--ignored",
            "--nocapture",
            "--format=terse",
        ])
        .env(FIXTURE_MODE, mode)
        .output()
        .expect("run build output fixture");
    assert!(captured.status.success(), "fixture must execute successfully");
    let stdout = String::from_utf8(captured.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(captured.stderr).expect("UTF-8 stderr");
    assert!(
        !stdout.contains("cargo:warning="),
        "routine status became a Cargo warning: {stdout}"
    );
    assert!(!stderr.contains("cargo:warning="));
    let directives: Vec<_> = stdout.lines().filter(|line| line.starts_with("cargo:")).collect();
    assert_eq!(
        directives,
        [
            format!("cargo:rustc-link-lib={mode}=tesseract"),
            format!("cargo:rustc-link-lib={mode}=leptonica"),
        ]
    );
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        [
            "Library already available at expected location: leptonica.a".to_owned(),
            "Library already available at expected location: tesseract.a".to_owned(),
            format!("Linking with tesseract ({mode} linking): tesseract"),
            format!("Linking with leptonica ({mode} linking): leptonica"),
        ]
    );
}

#[test]
#[ignore = "child process fixture invoked by the output tests"]
fn output_fixture() {
    let mode = std::env::var(FIXTURE_MODE).expect("fixture link mode");
    output::available_library(Path::new("leptonica.a"));
    output::available_library(Path::new("tesseract.a"));
    output::link_library("tesseract", &mode, "tesseract");
    output::link_library("leptonica", &mode, "leptonica");
}

#[test]
fn should_route_production_progress_through_the_tested_output_helpers() {
    assert!(BUILD_SCRIPT.contains("#[path = \"build_support/output.rs\"]"));
    assert_eq!(BUILD_SCRIPT.matches("output::link_library(").count(), 2);
    assert_eq!(BUILD_SCRIPT.matches("output::available_library(&out_path)").count(), 1);
    assert_eq!(BUILD_SCRIPT.matches("output::status(format_args!(").count(), 2);
    assert_eq!(BUILD_SCRIPT.matches("cargo:warning=").count(), 8);
    for diagnostic in [
        "Could not resolve absolute path for MinGW",
        "Failed to create cache dir",
        "All {} attempts",
        "Download attempt",
        "Tesseract WASM patch not found",
        "Patch command failed",
        "patch command not available",
        "Library {} not found!",
    ] {
        assert!(
            BUILD_SCRIPT.contains(&format!("cargo:warning={diagnostic}")),
            "lost warning: {diagnostic}"
        );
    }
}
