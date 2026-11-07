//! Golden Markdown for single PDF pages: any change to the native Markdown output of a page in
//! `test_documents/pdf/regression_pages/` fails here, naming each page and its first differing
//! line.
//!
//! The pages are cut from public-domain and CC-licensed documents whose sources and licences are
//! recorded in `tests/fixtures/pdf/REGRESSION_PAGES_PROVENANCE.md`. The list
//! `tests/fixtures/pdf/regression_pages.sha256` names every page with its hash and is the source of
//! truth for which pages exist: a missing directory, a missing, unlisted or changed page, or a
//! snapshot set that differs from the list fails the test. Each
//! page's expected output is committed as `tests/fixtures/pdf/regression_pages/<stem>.md`.
//! Regenerate the snapshots only for an intentional change, then review the diff:
//! `XBERG_PDF_PAGES_GOLDEN_WRITE=1 cargo test -p xberg --features full,formula-recognition,bundle-tessdata-eng --test pdf_regression_pages`.
//! Write mode covers exactly the listed pages and refuses a missing or unlisted PDF. To add a page,
//! put its PDF in `test_documents/pdf/regression_pages/` and publish it to the corpus, add its name
//! to the list (any 64-hex hash) and run write mode; to remove one, delete its name and the file
//! and run write mode.
//!
//! The pages are bucket-fetched (`python3 test_documents/scripts/fetch_corpus.py`). A local run
//! without them may opt out with `XBERG_REGRESSION_PAGES_SKIP=1`. Each switch counts only when
//! set to `1`, and either one panics when `CI` is set.

#![allow(clippy::print_stderr)]
// ~keep: test/bench binaries print by design; org logging policy exempts tests
// Some features change native Markdown output, so the snapshots hold only for CI's x86_64 Linux
// and macOS feature list (scripts/ci/rust/run-unit-tests.sh); `full` implies `pdf`.
#![cfg(all(feature = "full", feature = "formula-recognition", feature = "bundle-tessdata-eng"))]

mod helpers;
use helpers::{extract_uri_document_blocking, get_test_documents_dir};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use xberg::{ExtractionConfig, OutputFormat};

const WRITE_VAR: &str = "XBERG_PDF_PAGES_GOLDEN_WRITE";
const SKIP_VAR: &str = "XBERG_REGRESSION_PAGES_SKIP";
const TEST_COMMAND: &str =
    "cargo test -p xberg --features full,formula-recognition,bundle-tessdata-eng --test pdf_regression_pages";
/// CI's `RUST_MIN_STACK`: debug builds with the full feature set overflow the 2 MiB thread default.
const THREAD_STACK_BYTES: usize = 16 * 1024 * 1024;
const MAX_REPORTED: usize = 20;
const MAX_THREADS: usize = 8;
const FETCH_HINT: &str = "The corpus is fetched with: python3 test_documents/scripts/fetch_corpus.py";

/// `(file name, sha256 hex)` pairs, one per listed page.
type PageList = [(String, String)];

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pdf")
}

fn stem(name: &str) -> &str {
    name.strip_suffix(".pdf").unwrap_or(name)
}

fn snapshot_name(name: &str) -> String {
    format!("{}.md", stem(name))
}

fn file_name(path: &Path) -> String {
    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
}

fn sha256_hex(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    hex::encode(Sha256::digest(&bytes))
}

/// File names in `dir` with the given extension.
fn names_in(dir: &Path, extension: &str) -> std::io::Result<BTreeSet<String>> {
    Ok(std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .map(|path| file_name(&path))
        .collect())
}

fn switch(var: &str) -> bool {
    std::env::var_os(var).is_some_and(|value| value == "1")
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `[A-Za-z0-9._-]+\.pdf`, so a listed name can never reach outside the pages directory.
fn is_page_name(text: &str) -> bool {
    text.strip_suffix(".pdf")
        .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)))
}

/// Parses `sha256sum` output, `<64 lowercase hex>  <name>.pdf` per line, rejecting duplicates.
fn parse_list(text: &str) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            let number = index + 1;
            let (hash, name) = line
                .split_once("  ")
                .filter(|(hash, name)| is_hash(hash) && is_page_name(name))
                .unwrap_or_else(|| panic!("page list line {number}: expected `<64 hex>  <name>.pdf`, got {line:?}"));
            assert!(seen.insert(name), "page list line {number}: duplicate {name}");
            (name.to_string(), hash.to_string())
        })
        .collect()
}

fn format_list(list: &PageList) -> String {
    list.iter().map(|(name, hash)| format!("{hash}  {name}\n")).collect()
}

/// The listed pages in list order, or one message per problem: no pages listed, a missing
/// directory, a listed page missing, a page present but not listed, or (with `hashes`) a hash
/// that differs.
fn check_inputs(dir: &Path, expected: &PageList, hashes: bool) -> Result<Vec<PathBuf>, Vec<String>> {
    if expected.is_empty() {
        return Err(vec!["the page list is empty".to_string()]);
    }
    let present = names_in(dir, "pdf")
        .map_err(|e| vec![format!("missing directory {}: {e}; fetch the corpus", dir.display())])?;
    let listed: BTreeSet<&str> = expected.iter().map(|(name, _)| name.as_str()).collect();
    let mut problems = Vec::new();
    for (name, hash) in expected {
        if !present.contains(name) {
            problems.push(format!("{name}: listed but missing; fetch the corpus"));
            continue;
        }
        if !hashes {
            continue;
        }
        let actual = sha256_hex(&dir.join(name));
        if &actual != hash {
            problems.push(format!("{name}: sha256 {actual}, listed {hash}; fetch the corpus"));
        }
    }
    for name in present.iter().filter(|name| !listed.contains(name.as_str())) {
        problems.push(format!(
            "{name}: present but not listed; add it to the list or remove the file"
        ));
    }
    if problems.is_empty() {
        Ok(expected.iter().map(|(name, _)| dir.join(name)).collect())
    } else {
        Err(problems)
    }
}

/// One message per listed page without a snapshot and per snapshot of no listed page.
fn check_snapshots(dir: &Path, expected: &PageList) -> Vec<String> {
    let present = names_in(dir, "md").unwrap_or_default();
    let listed: BTreeSet<String> = expected.iter().map(|(name, _)| snapshot_name(name)).collect();
    let missing = listed
        .difference(&present)
        .map(|name| format!("{name}: snapshot missing"));
    let extra = present
        .difference(&listed)
        .map(|name| format!("{name}: snapshot of no listed page"));
    missing.chain(extra).collect()
}

fn extract(page: &Path) -> Result<String, String> {
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    extract_uri_document_blocking(page, Some("application/pdf"), &config)
        .map(|document| document.content)
        .map_err(|e| format!("extraction failed: {e}"))
}

/// Each page's file name with its Markdown or extraction error, in page order.
fn extract_all(pages: &[PathBuf]) -> Vec<(String, Result<String, String>)> {
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_THREADS);
    let chunk_len = pages.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = pages
            .chunks(chunk_len)
            .map(|chunk| {
                std::thread::Builder::new()
                    .stack_size(THREAD_STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        chunk
                            .iter()
                            .map(|page| (file_name(page), extract(page)))
                            .collect::<Vec<_>>()
                    })
                    .expect("spawn extraction thread")
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("extraction thread panicked"))
            .collect()
    })
}

/// Splits on `\n` rather than `lines()` so a changed trailing newline is a difference too.
fn first_difference(expected: &str, actual: &str) -> String {
    let show = |line: Option<&str>| line.map_or_else(|| "<end of file>".to_string(), |line| format!("{line:?}"));
    let (mut expected_lines, mut actual_lines) = (expected.split('\n'), actual.split('\n'));
    let mut number = 1;
    loop {
        match (expected_lines.next(), actual_lines.next()) {
            (Some(e), Some(a)) if e == a => number += 1,
            (e, a) => return format!("line {number}: expected {}, actual {}", show(e), show(a)),
        }
    }
}

/// One failure per page whose output is an error or differs from its snapshot. Results that do not
/// cover exactly the listed names in order fail by name, before any snapshot is read.
fn compare_outputs(results: &[(String, Result<String, String>)], list: &PageList, snapshots: &Path) -> Vec<String> {
    let listed: Vec<&str> = list.iter().map(|(name, _)| name.as_str()).collect();
    let produced: Vec<&str> = results.iter().map(|(name, _)| name.as_str()).collect();
    if produced != listed {
        let (listed, produced): (BTreeSet<&str>, BTreeSet<&str>) =
            (listed.into_iter().collect(), produced.into_iter().collect());
        let missing = listed.difference(&produced).map(|name| format!("{name}: no output"));
        let extra = produced
            .difference(&listed)
            .map(|name| format!("{name}: output of no listed page"));
        let problems: Vec<String> = missing.chain(extra).collect();
        return if problems.is_empty() {
            vec!["outputs do not follow the list's names in order".to_string()]
        } else {
            problems
        };
    }
    results
        .iter()
        .filter_map(|(name, output)| {
            let path = snapshots.join(snapshot_name(name));
            let failure = match output {
                Err(error) => Some(error.clone()),
                Ok(actual) => match std::fs::read_to_string(&path) {
                    Err(e) => Some(format!("read {}: {e}", path.display())),
                    Ok(expected) if &expected == actual => None,
                    Ok(expected) => Some(first_difference(&expected, actual)),
                },
            };
            failure.map(|failure| format!("{}: {failure}", stem(name)))
        })
        .collect()
}

fn report(header: &str, failures: &[String], hint: &str) -> String {
    let mut message = format!("{header}\n");
    for failure in failures.iter().take(MAX_REPORTED) {
        writeln!(message, "  {failure}").unwrap();
    }
    if failures.len() > MAX_REPORTED {
        writeln!(message, "  ... and {} more", failures.len() - MAX_REPORTED).unwrap();
    }
    message + hint
}

/// Refreshes the snapshots and hashes of exactly the listed pages. Every page is extracted before
/// anything is written, and the list is written last.
fn write_goldens(pages_dir: &Path, list_path: &Path, snapshots: &Path, list: &PageList) {
    let pages = check_inputs(pages_dir, list, false).unwrap_or_else(|problems| {
        let header = "Write mode needs exactly the listed PDFs:";
        panic!("{}", report(header, &problems, FETCH_HINT))
    });
    let hashed: Vec<(String, String)> = pages.iter().map(|page| (file_name(page), sha256_hex(page))).collect();
    let outputs = extract_all(&pages);
    let failures: Vec<String> = outputs
        .iter()
        .filter_map(|(name, output)| output.as_ref().err().map(|e| format!("{}: {e}", stem(name))))
        .collect();
    assert!(
        failures.is_empty(),
        "{}",
        report("Extraction failed, so nothing was written:", &failures, "")
    );
    std::fs::create_dir_all(snapshots).unwrap_or_else(|e| panic!("create {snapshots:?}: {e}"));
    for (name, output) in outputs {
        let path = snapshots.join(snapshot_name(&name));
        let actual = output.expect("every extraction succeeded");
        std::fs::write(&path, actual).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    }
    let live: BTreeSet<String> = list.iter().map(|(name, _)| snapshot_name(name)).collect();
    let present = names_in(snapshots, "md").unwrap_or_else(|e| panic!("read {snapshots:?}: {e}"));
    for stale in present.difference(&live) {
        let path = snapshots.join(stale);
        std::fs::remove_file(&path).unwrap_or_else(|e| panic!("remove {path:?}: {e}"));
    }
    std::fs::write(list_path, format_list(&hashed)).unwrap_or_else(|e| panic!("write {list_path:?}: {e}"));
}

#[test]
fn regression_pages_match_their_markdown_snapshots() {
    let (skip, write) = (switch(SKIP_VAR), switch(WRITE_VAR));
    assert!(!(skip && write), "{SKIP_VAR} and {WRITE_VAR} conflict: set at most one");
    let in_ci = std::env::var_os("CI").is_some();
    assert!(
        !(in_ci && (skip || write)),
        "{SKIP_VAR} and {WRITE_VAR} must not be set in CI"
    );
    if skip {
        eprintln!("Skipping regression pages: {SKIP_VAR}=1 (local opt-out)");
        return;
    }
    let pages_dir = get_test_documents_dir().join("pdf/regression_pages");
    let list_path = fixtures_dir().join("regression_pages.sha256");
    let snapshots = fixtures_dir().join("regression_pages");
    let text = std::fs::read_to_string(&list_path).unwrap_or_else(|e| panic!("read {list_path:?}: {e}"));
    let list = parse_list(&text);
    if write {
        write_goldens(&pages_dir, &list_path, &snapshots, &list);
        return;
    }

    let pages = check_inputs(&pages_dir, &list, true).unwrap_or_else(|problems| {
        let header = "Regression page PDFs do not match regression_pages.sha256:";
        panic!("{}", report(header, &problems, FETCH_HINT))
    });
    let regenerate =
        format!("If the change is intended, regenerate and review the diff:\n  {WRITE_VAR}=1 {TEST_COMMAND}");
    let problems = check_snapshots(&snapshots, &list);
    let header = "Snapshots do not match regression_pages.sha256:";
    assert!(problems.is_empty(), "{}", report(header, &problems, &regenerate));

    let failures = compare_outputs(&extract_all(&pages), &list, &snapshots);
    let header = format!("{} of {} regression pages failed:", failures.len(), list.len());
    assert!(failures.is_empty(), "{}", report(&header, &failures, &regenerate));
}

#[test]
fn native_chart_legend_keeps_the_raised_marker_with_its_label() {
    if switch(SKIP_VAR) {
        assert!(std::env::var_os("CI").is_none(), "{SKIP_VAR}=1 must not be set in CI");
        return;
    }
    let page = get_test_documents_dir().join("pdf/regression_pages/ntrs-19940027557-p18.pdf");
    assert_eq!(
        sha256_hex(&page),
        "76b68c133220c3fc1cc8e2fe2c6a0187d5eae2a154dff57dd6db06c6bd11001b"
    );
    let markdown = extract(&page).expect("native Markdown for verified chart page");
    assert_eq!(
        markdown,
        include_str!("fixtures/pdf/regression_pages/ntrs-19940027557-p18.md")
    );
}

fn assert_regression_page_has_script(file_name: &str, expected: &str) {
    if switch(SKIP_VAR) {
        assert!(std::env::var_os("CI").is_none(), "{SKIP_VAR}=1 must not be set in CI");
        eprintln!("Skipping regression page: {SKIP_VAR}=1 (local opt-out)");
        return;
    }
    let page = get_test_documents_dir().join("pdf/regression_pages").join(file_name);
    let (_, content) = extract_all(&[page]).pop().expect("one extracted page");
    let content = content.unwrap_or_else(|error| panic!("{file_name}: {error}. {FETCH_HINT}"));
    assert_eq!(
        content.matches(expected).count(),
        1,
        "{file_name}: expected one inline {expected:?} in Markdown:\n{content}"
    );
}

#[test]
fn regression_page_keeps_negative_exponent_with_its_prose_base() {
    assert_regression_page_has_script("plos-journal.pcbi.1012696-p8.pdf", "5.0 × 10<sup>−4</sup>.");
}

#[test]
fn regression_page_keeps_squared_suffix_with_its_row_label() {
    assert_regression_page_has_script("plos-journal.pone.0311664-p14.pdf", "R<sup>2</sup> 0.355");
}

#[test]
fn regression_page_keeps_raised_footnote_number_separate_from_ers_definition() {
    assert_regression_page_has_script("fr-2025-04604-nofa-p5.pdf", "\n\n4 The ERS document");
}

#[test]
fn regression_page_keeps_raised_footnote_number_separate_from_cms_definition() {
    assert_regression_page_has_script("fr-2026-08190-hcbs-p18.pdf", "\n\n56 Available at");
}

/// Negative controls: each broken corpus must fail and name its problem.
mod input_checks {
    use super::*;

    /// sha256 of the one-byte files "a" and "b".
    const HASH_A: &str = "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb";
    const HASH_B: &str = "3e23e8160039594a33894f6564e1b1348bbd7a0088d42c4acb73eeaed59c009d";

    fn list(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(name, hash)| (name.to_string(), hash.to_string()))
            .collect()
    }

    fn corpus(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, content) in files {
            std::fs::write(dir.path().join(name), content).expect("write fake page");
        }
        dir
    }

    fn output(name: &str, markdown: &str) -> (String, Result<String, String>) {
        (name.to_string(), Ok(markdown.to_string()))
    }

    fn input_problems(dir: &Path, expected: &PageList) -> String {
        check_inputs(dir, expected, true)
            .expect_err("check should fail")
            .join("\n")
    }

    #[test]
    fn a_complete_set_passes_in_list_order() {
        let dir = corpus(&[("a.pdf", "a"), ("b.pdf", "b"), ("notes.txt", "x")]);
        let pages = check_inputs(dir.path(), &list(&[("b.pdf", HASH_B), ("a.pdf", HASH_A)]), true).expect("complete");
        assert_eq!(pages, vec![dir.path().join("b.pdf"), dir.path().join("a.pdf")]);
    }

    #[test]
    #[should_panic(expected = "line 2: duplicate a.pdf")]
    fn a_duplicate_name_in_the_list_fails() {
        parse_list(&format!("{HASH_A}  a.pdf\n{HASH_B}  a.pdf\n"));
    }

    #[test]
    #[should_panic(expected = "b.pdf: listed but missing")]
    fn write_mode_refuses_a_missing_page_before_writing() {
        let dir = corpus(&[("a.pdf", "changed")]);
        let (list_path, snapshots) = (dir.path().join("list.sha256"), dir.path().join("snapshots"));
        write_goldens(
            dir.path(),
            &list_path,
            &snapshots,
            &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]),
        );
    }

    #[test]
    fn a_missing_directory_fails() {
        let dir = corpus(&[]);
        let absent = dir.path().join("regression_pages");
        assert!(input_problems(&absent, &list(&[("a.pdf", HASH_A)])).starts_with("missing directory"));
    }

    #[test]
    fn an_empty_list_fails() {
        let dir = corpus(&[("a.pdf", "a")]);
        assert_eq!(input_problems(dir.path(), &[]), "the page list is empty");
    }

    #[test]
    fn one_missing_page_fails_and_is_named() {
        let dir = corpus(&[("a.pdf", "a")]);
        let expected = list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]);
        assert_eq!(
            input_problems(dir.path(), &expected),
            "b.pdf: listed but missing; fetch the corpus"
        );
    }

    #[test]
    fn a_changed_page_fails_and_is_named() {
        let dir = corpus(&[("a.pdf", "a"), ("b.pdf", "changed")]);
        let message = input_problems(dir.path(), &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]));
        assert!(
            message.starts_with("b.pdf: sha256 ") && message.contains(&format!("listed {HASH_B}")),
            "{message}"
        );
        let names_only = check_inputs(dir.path(), &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]), false);
        assert!(names_only.is_ok(), "write mode's name-only check must ignore hashes");
    }

    #[test]
    fn an_unlisted_page_fails_and_is_named() {
        let dir = corpus(&[("a.pdf", "a"), ("b.pdf", "b")]);
        assert_eq!(
            input_problems(dir.path(), &list(&[("a.pdf", HASH_A)])),
            "b.pdf: present but not listed; add it to the list or remove the file"
        );
    }

    #[test]
    fn a_missing_or_extra_snapshot_fails_and_is_named() {
        let dir = corpus(&[("a.md", ""), ("old.md", "")]);
        let problems = check_snapshots(dir.path(), &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]));
        assert_eq!(
            problems,
            vec!["b.md: snapshot missing", "old.md: snapshot of no listed page"]
        );
    }

    #[test]
    fn a_differing_output_fails_with_its_first_differing_line() {
        let dir = corpus(&[("a.md", "same"), ("b.md", "x\ny")]);
        let results = [output("a.pdf", "same"), output("b.pdf", "x\nz")];
        let failures = compare_outputs(&results, &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]), dir.path());
        assert_eq!(failures, vec![r#"b: line 2: expected "y", actual "z""#]);
    }

    #[test]
    fn results_missing_a_listed_page_fail_and_name_it() {
        let dir = corpus(&[("a.md", "same"), ("b.md", "same")]);
        let failures = compare_outputs(
            &[output("a.pdf", "same")],
            &list(&[("a.pdf", HASH_A), ("b.pdf", HASH_B)]),
            dir.path(),
        );
        assert_eq!(failures, vec!["b.pdf: no output"]);
    }
}

#[test]
fn regression_page_keeps_raised_note_marker_in_its_sentence_beside_other_columns() {
    assert_regression_page_has_script(
        "fr-2026-08190-hcbs-p2.pdf",
        "Medicaid Director Letter \\# 22–003<sup>2</sup> to release the first official version",
    );
}

#[test]
fn regression_page_keeps_ordinal_suffixes_in_the_author_line() {
    assert_regression_page_has_script(
        "arxiv-2403.01153v1-p1.pdf",
        "1st Zhiyuan He, 2nd Ke Deng, 3rd Jiangchao Gong,",
    );
}

#[test]
fn regression_page_keeps_stacked_scripts_in_the_table_caption() {
    assert_regression_page_has_script(
        "arxiv-2312.13591v2-p5.pdf",
        "in Σ~c~ D , Ξ~c~ D and Σ~c~ D , Ξ~c~ D sys-",
    );
}

#[test]
fn regression_page_keeps_a_lone_squared_marker_in_a_display_formula() {
    assert_regression_page_has_script("gutenberg-33283-p27.pdf", "d<sup>2</sup>y");
}
