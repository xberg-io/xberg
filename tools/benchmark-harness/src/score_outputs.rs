//! Score precomputed Markdown outputs without running extraction.

use crate::fixture::Fixture;
use crate::quality::{compute_f1, structural_sidecar, tokenize};
use crate::{Error, Result};
use rayon::prelude::*;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

pub const SCORE_OUTPUTS_SCHEMA_VERSION: u32 = 1;
pub const SF1_VERSION: u32 = 2;
pub const TF1_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOutputsReport {
    pub schema_version: u32,
    pub sf1_version: u32,
    pub tf1_version: u32,
    pub document_count: usize,
    pub mean_sf1: f64,
    pub mean_tf1: f64,
    pub documents: Vec<ScoredOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoredOutput {
    pub fixture_id: String,
    pub sf1: f64,
    pub tf1: f64,
    pub structural: structural_sidecar::StructuralScore,
}

#[derive(Debug)]
struct GroundTruth {
    text: String,
    markdown: String,
}

#[derive(Debug)]
struct OutputRecord {
    fixture_id: String,
    content: String,
}

#[derive(Debug)]
enum JsonOutputs {
    Records(Vec<OutputRecord>),
    ByFixture(UniqueOutputMap),
}

#[derive(Debug)]
struct UniqueOutputMap(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for OutputRecord {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct OutputRecordVisitor;

        impl<'de> Visitor<'de> for OutputRecordVisitor {
            type Value = OutputRecord;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an output record with an ID and Markdown content")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut seen = BTreeSet::new();
                let mut fixture_id = None;
                let mut content = None;

                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(de::Error::custom(format!("duplicate output record field '{key}'")));
                    }
                    match key.as_str() {
                        "fixture_id" | "id" => {
                            if fixture_id.is_some() {
                                return Err(de::Error::custom("duplicate output record fixture ID field"));
                            }
                            fixture_id = Some(map.next_value()?);
                        }
                        "content" | "markdown" => {
                            if content.is_some() {
                                return Err(de::Error::custom("duplicate output record content field"));
                            }
                            content = Some(map.next_value()?);
                        }
                        _ => {
                            map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(OutputRecord {
                    fixture_id: fixture_id.ok_or_else(|| de::Error::missing_field("fixture_id or id"))?,
                    content: content.ok_or_else(|| de::Error::missing_field("content or markdown"))?,
                })
            }
        }

        deserializer.deserialize_map(OutputRecordVisitor)
    }
}

impl<'de> Deserialize<'de> for JsonOutputs {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JsonOutputsVisitor;

        impl<'de> Visitor<'de> for JsonOutputsVisitor {
            type Value = JsonOutputs;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an output-record array or a map of fixture IDs to Markdown content")
            }

            fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut records = Vec::new();
                while let Some(record) = sequence.next_element()? {
                    records.push(record);
                }
                Ok(JsonOutputs::Records(records))
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                let mut seen = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(de::Error::custom(format!("duplicate output fixture ID '{key}'")));
                    }
                    entries.push((key, map.next_value::<Value>()?));
                }

                if looks_like_output_record(&entries) {
                    return output_record_from_entries(entries)
                        .map(|record| JsonOutputs::Records(vec![record]))
                        .map_err(de::Error::custom);
                }

                let mut outputs = BTreeMap::new();
                for (fixture_id, value) in entries {
                    let content = value.as_str().ok_or_else(|| {
                        de::Error::custom(format!("output map value for fixture '{fixture_id}' must be a string"))
                    })?;
                    outputs.insert(fixture_id, content.to_string());
                }
                Ok(JsonOutputs::ByFixture(UniqueOutputMap(outputs)))
            }
        }

        deserializer.deserialize_any(JsonOutputsVisitor)
    }
}

fn looks_like_output_record(entries: &[(String, Value)]) -> bool {
    let has_id = entries
        .iter()
        .any(|(key, _)| matches!(key.as_str(), "fixture_id" | "id"));
    let has_content = entries
        .iter()
        .any(|(key, _)| matches!(key.as_str(), "content" | "markdown"));
    has_id && has_content
}

fn output_record_from_entries(entries: Vec<(String, Value)>) -> std::result::Result<OutputRecord, String> {
    let mut fixture_id = None;
    let mut content = None;
    for (key, value) in entries {
        match key.as_str() {
            "fixture_id" | "id" => {
                if fixture_id.is_some() {
                    return Err("duplicate output record fixture ID field".to_string());
                }
                fixture_id = Some(json_string_field(value, &key)?);
            }
            "content" | "markdown" => {
                if content.is_some() {
                    return Err("duplicate output record content field".to_string());
                }
                content = Some(json_string_field(value, &key)?);
            }
            _ => {}
        }
    }

    Ok(OutputRecord {
        fixture_id: fixture_id.ok_or_else(|| "output record is missing fixture_id or id".to_string())?,
        content: content.ok_or_else(|| "output record is missing content or markdown".to_string())?,
    })
}

fn json_string_field(value: Value, field: &str) -> std::result::Result<String, String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("output record field '{field}' must be a string"))
}

/// Score an exact set of precomputed Markdown outputs against fixture ground truth.
///
/// The fixture IDs and output IDs must be identical. Missing ground truth,
/// missing outputs, unexpected outputs, and duplicate IDs are all errors.
pub fn score_outputs(fixtures: &Path, outputs: &Path) -> Result<ScoreOutputsReport> {
    let ground_truth = load_ground_truth(fixtures)?;
    let extracted = load_outputs(outputs)?;
    validate_exact_ids(ground_truth.keys(), extracted.keys())?;

    let truths = ground_truth.into_iter().collect::<Vec<_>>();
    let mut documents = truths
        .into_par_iter()
        .map(|(fixture_id, truth)| {
            let content = extracted
                .get(&fixture_id)
                .expect("output IDs were validated against fixture IDs");
            let structural = structural_sidecar::score_structural(
                &structural_sidecar::StructuralSidecar::from_markdown(content),
                &structural_sidecar::StructuralSidecar::from_markdown(&truth.markdown),
            );
            let tf1 = compute_f1(&tokenize(content), &tokenize(&truth.text));

            ScoredOutput {
                fixture_id,
                sf1: structural.sf1_prime,
                tf1,
                structural,
            }
        })
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.fixture_id.cmp(&right.fixture_id));

    let document_count = documents.len();
    let divisor = document_count as f64;
    let mean_sf1 = documents.iter().map(|document| document.sf1).sum::<f64>() / divisor;
    let mean_tf1 = documents.iter().map(|document| document.tf1).sum::<f64>() / divisor;

    Ok(ScoreOutputsReport {
        schema_version: SCORE_OUTPUTS_SCHEMA_VERSION,
        sf1_version: SF1_VERSION,
        tf1_version: TF1_VERSION,
        document_count,
        mean_sf1,
        mean_tf1,
        documents,
    })
}

fn load_ground_truth(fixtures: &Path) -> Result<BTreeMap<String, GroundTruth>> {
    let mut fixture_paths = Vec::new();
    collect_files(fixtures, Some("json"), &mut fixture_paths)?;
    fixture_paths.sort();
    if fixture_paths.is_empty() {
        return Err(Error::Benchmark(format!(
            "no fixture JSON files found at {}",
            fixtures.display()
        )));
    }

    let mut ground_truth = BTreeMap::new();
    for fixture_path in fixture_paths {
        let fixture = Fixture::from_file(&fixture_path)?;
        let fixture_id = file_id(&fixture_path)?;
        let fixture_dir = fixture_path.parent().ok_or_else(|| {
            Error::Benchmark(format!(
                "fixture path has no parent directory: {}",
                fixture_path.display()
            ))
        })?;
        let markdown_path = fixture
            .resolve_ground_truth_markdown_path(fixture_dir)
            .ok_or_else(|| Error::Benchmark(format!("fixture '{fixture_id}' is missing Markdown ground truth")))?;
        let markdown = read_with_context(&markdown_path, &fixture_id, "Markdown ground truth")?;
        let text = match fixture.resolve_ground_truth_path(fixture_dir) {
            Some(text_path) => read_with_context(&text_path, &fixture_id, "text ground truth")?,
            None => markdown.clone(),
        };
        let truth = GroundTruth { text, markdown };
        if ground_truth.insert(fixture_id.clone(), truth).is_some() {
            return Err(Error::Benchmark(format!("duplicate fixture ID '{fixture_id}'")));
        }
    }

    Ok(ground_truth)
}

fn load_outputs(outputs: &Path) -> Result<BTreeMap<String, String>> {
    if outputs.is_dir() {
        return load_output_directory(outputs);
    }
    if !outputs.is_file() {
        return Err(Error::Benchmark(format!(
            "output path is not a file or directory: {}",
            outputs.display()
        )));
    }

    match outputs.extension().and_then(|extension| extension.to_str()) {
        Some("json") => load_json_outputs(outputs),
        Some("jsonl" | "ndjson") => load_jsonl_outputs(outputs),
        Some("md" | "markdown") => {
            let fixture_id = file_id(outputs)?;
            let content = read_with_context(outputs, &fixture_id, "extracted Markdown")?;
            let mut extracted = BTreeMap::new();
            insert_output(&mut extracted, fixture_id, content)?;
            Ok(extracted)
        }
        _ => Err(Error::Benchmark(format!(
            "unsupported output format for {} (expected a directory, .md, .json, or .jsonl)",
            outputs.display()
        ))),
    }
}

fn load_output_directory(outputs: &Path) -> Result<BTreeMap<String, String>> {
    let mut paths = Vec::new();
    collect_output_files(outputs, &mut paths)?;
    paths.sort();
    let mut extracted = BTreeMap::new();

    for path in paths {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("md" | "markdown") => {
                let fixture_id = file_id(&path)?;
                let content = read_with_context(&path, &fixture_id, "extracted Markdown")?;
                insert_output(&mut extracted, fixture_id, content)?;
            }
            Some("json") => merge_outputs(&mut extracted, load_json_outputs(&path)?)?,
            Some("jsonl" | "ndjson") => merge_outputs(&mut extracted, load_jsonl_outputs(&path)?)?,
            _ => {}
        }
    }

    Ok(extracted)
}

fn load_json_outputs(path: &Path) -> Result<BTreeMap<String, String>> {
    let input = std::fs::read_to_string(path)
        .map_err(|error| Error::Benchmark(format!("failed to read output JSON {}: {error}", path.display())))?;
    let parsed: JsonOutputs = serde_json::from_str(&input)
        .map_err(|error| Error::Benchmark(format!("failed to parse output JSON {}: {error}", path.display())))?;

    match parsed {
        JsonOutputs::ByFixture(outputs) => validated_output_map(outputs.0),
        JsonOutputs::Records(records) => records_to_map(records),
    }
}

fn load_jsonl_outputs(path: &Path) -> Result<BTreeMap<String, String>> {
    let input = std::fs::read_to_string(path)
        .map_err(|error| Error::Benchmark(format!("failed to read output JSONL {}: {error}", path.display())))?;
    let mut records = Vec::new();
    for (index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(line).map_err(|error| {
            Error::Benchmark(format!(
                "failed to parse output JSONL {} at line {}: {error}",
                path.display(),
                index + 1
            ))
        })?;
        records.push(record);
    }
    records_to_map(records)
}

fn records_to_map(records: Vec<OutputRecord>) -> Result<BTreeMap<String, String>> {
    let mut outputs = BTreeMap::new();
    for record in records {
        insert_output(&mut outputs, record.fixture_id, record.content)?;
    }
    Ok(outputs)
}

fn validated_output_map(outputs: BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    let mut validated = BTreeMap::new();
    merge_outputs(&mut validated, outputs)?;
    Ok(validated)
}

fn merge_outputs(target: &mut BTreeMap<String, String>, source: BTreeMap<String, String>) -> Result<()> {
    for (fixture_id, content) in source {
        insert_output(target, fixture_id, content)?;
    }
    Ok(())
}

fn insert_output(outputs: &mut BTreeMap<String, String>, fixture_id: String, content: String) -> Result<()> {
    if fixture_id.trim().is_empty() {
        return Err(Error::Benchmark("output fixture ID cannot be empty".to_string()));
    }
    if content.trim().is_empty() {
        return Err(Error::Benchmark(format!(
            "output content for fixture '{fixture_id}' cannot be empty"
        )));
    }
    if outputs.insert(fixture_id.clone(), content).is_some() {
        return Err(Error::Benchmark(format!("duplicate output fixture ID '{fixture_id}'")));
    }
    Ok(())
}

fn validate_exact_ids<'a>(
    expected: impl Iterator<Item = &'a String>,
    actual: impl Iterator<Item = &'a String>,
) -> Result<()> {
    let expected = expected.cloned().collect::<BTreeSet<_>>();
    let actual = actual.cloned().collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();

    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }

    let mut problems = Vec::new();
    if !missing.is_empty() {
        problems.push(format!("missing output fixture IDs: {}", missing.join(", ")));
    }
    if !unexpected.is_empty() {
        problems.push(format!("unexpected output fixture IDs: {}", unexpected.join(", ")));
    }
    Err(Error::Benchmark(problems.join("; ")))
}

fn collect_files(path: &Path, extension: Option<&str>, paths: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if extension.is_none_or(|expected| path.extension().and_then(|value| value.to_str()) == Some(expected)) {
            paths.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(Error::Benchmark(format!(
            "path is not a file or directory: {}",
            path.display()
        )));
    }

    for entry in std::fs::read_dir(path)
        .map_err(|error| Error::Benchmark(format!("failed to read directory {}: {error}", path.display())))?
    {
        let child = entry
            .map_err(|error| Error::Benchmark(format!("failed to read entry in {}: {error}", path.display())))?
            .path();
        if child.is_dir() {
            collect_files(&child, extension, paths)?;
        } else if extension.is_none_or(|expected| child.extension().and_then(|value| value.to_str()) == Some(expected))
        {
            paths.push(child);
        }
    }
    Ok(())
}

fn collect_output_files(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(path)
        .map_err(|error| Error::Benchmark(format!("failed to read output directory {}: {error}", path.display())))?
    {
        let child = entry
            .map_err(|error| Error::Benchmark(format!("failed to read entry in {}: {error}", path.display())))?
            .path();
        if child.is_dir() {
            collect_output_files(&child, paths)?;
        } else if matches!(
            child.extension().and_then(|extension| extension.to_str()),
            Some("md" | "markdown" | "json" | "jsonl" | "ndjson")
        ) {
            paths.push(child);
        }
    }
    Ok(())
}

fn file_id(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            Error::Benchmark(format!(
                "path does not have a valid UTF-8 fixture ID: {}",
                path.display()
            ))
        })
}

fn read_with_context(path: &Path, fixture_id: &str, description: &str) -> Result<String> {
    std::fs::read_to_string(path).map_err(|error| {
        Error::Benchmark(format!(
            "failed to read {description} for fixture '{fixture_id}' from {}: {error}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::GroundTruth as FixtureGroundTruth;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn write_fixture_with_text(root: &Path, id: &str, markdown: &str, text: Option<&str>) {
        let fixture = Fixture {
            document: PathBuf::from(format!("{id}.pdf")),
            file_type: "pdf".to_string(),
            file_size: 1,
            expected_frameworks: Vec::new(),
            metadata: HashMap::new(),
            ground_truth: Some(FixtureGroundTruth {
                text_file: text.map(|_| PathBuf::from(format!("{id}.txt"))),
                markdown_file: Some(PathBuf::from(format!("{id}.md"))),
                fields_json: None,
                formulas_json: None,
                source: "manual".to_string(),
            }),
        };
        std::fs::write(root.join(format!("{id}.pdf")), b"pdf").unwrap();
        if let Some(text) = text {
            std::fs::write(root.join(format!("{id}.txt")), text).unwrap();
        }
        std::fs::write(root.join(format!("{id}.md")), markdown).unwrap();
        std::fs::write(root.join(format!("{id}.json")), serde_json::to_vec(&fixture).unwrap()).unwrap();
    }

    fn write_fixture(root: &Path, id: &str, markdown: &str) {
        write_fixture_with_text(root, id, markdown, Some(markdown));
    }

    #[test]
    fn scores_exact_directory_with_canonical_versions() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "# Alpha\n\nBody");
        write_fixture(fixtures.path(), "beta", "- one\n- two");
        std::fs::write(outputs.path().join("beta.md"), "- one\n- two").unwrap();
        std::fs::write(outputs.path().join("alpha.md"), "# Alpha\n\nBody").unwrap();

        let report = score_outputs(fixtures.path(), outputs.path()).unwrap();

        assert_eq!(report.schema_version, SCORE_OUTPUTS_SCHEMA_VERSION);
        assert_eq!(report.sf1_version, SF1_VERSION);
        assert_eq!(report.tf1_version, TF1_VERSION);
        assert_eq!(report.document_count, 2);
        assert_eq!(report.mean_sf1, 1.0);
        assert_eq!(report.mean_tf1, 1.0);
        assert_eq!(report.documents[0].fixture_id, "alpha");
        assert_eq!(report.documents[1].fixture_id, "beta");
    }

    #[test]
    fn missing_and_unexpected_ids_fail_closed() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "Alpha");
        write_fixture(fixtures.path(), "beta", "Beta");
        std::fs::write(outputs.path().join("alpha.md"), "Alpha").unwrap();
        std::fs::write(outputs.path().join("gamma.md"), "Gamma").unwrap();

        let error = score_outputs(fixtures.path(), outputs.path()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("missing output fixture IDs: beta"));
        assert!(message.contains("unexpected output fixture IDs: gamma"));
    }

    #[test]
    fn duplicate_output_ids_fail_closed() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "Alpha");
        std::fs::create_dir(outputs.path().join("nested")).unwrap();
        std::fs::write(outputs.path().join("alpha.md"), "Alpha").unwrap();
        std::fs::write(outputs.path().join("nested/alpha.markdown"), "Alpha").unwrap();

        let error = score_outputs(fixtures.path(), outputs.path()).unwrap_err();
        assert!(error.to_string().contains("duplicate output fixture ID 'alpha'"));
    }

    #[test]
    fn duplicate_fixture_ids_fail_closed() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        let first = fixtures.path().join("first");
        let second = fixtures.path().join("second");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        write_fixture(&first, "alpha", "Alpha");
        write_fixture(&second, "alpha", "Alpha");
        std::fs::write(outputs.path().join("alpha.md"), "Alpha").unwrap();

        let error = score_outputs(fixtures.path(), outputs.path()).unwrap_err();

        assert!(error.to_string().contains("duplicate fixture ID 'alpha'"));
    }

    #[test]
    fn jsonl_records_are_supported_and_sorted_by_fixture_id() {
        let fixtures = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "Alpha");
        write_fixture(fixtures.path(), "beta", "Beta");
        let outputs = fixtures.path().join("outputs.jsonl");
        std::fs::write(
            &outputs,
            "{\"id\":\"beta\",\"markdown\":\"Beta\"}\n{\"fixture_id\":\"alpha\",\"content\":\"Alpha\"}\n",
        )
        .unwrap();

        let report = score_outputs(fixtures.path(), &outputs).unwrap();

        assert_eq!(report.documents[0].fixture_id, "alpha");
        assert_eq!(report.documents[1].fixture_id, "beta");
    }

    #[test]
    fn runner_record_is_supported_as_single_file_and_directory() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "# Alpha\n\nBody");
        let output = outputs.path().join("alpha.json");
        std::fs::write(
            &output,
            r##"{"id":"alpha","input_sha256":"input","output_sha256":"output","elapsed_ms":7,"content":"# Alpha\n\nBody"}"##,
        )
        .unwrap();

        let single_report = score_outputs(fixtures.path(), &output).unwrap();
        let directory_report = score_outputs(fixtures.path(), outputs.path()).unwrap();

        assert_eq!(single_report, directory_report);
        assert_eq!(single_report.mean_sf1, 1.0);
        assert_eq!(single_report.mean_tf1, 1.0);
    }

    #[test]
    fn markdown_ground_truth_is_tf1_fallback_when_text_is_absent() {
        let fixtures = TempDir::new().unwrap();
        let outputs = TempDir::new().unwrap();
        write_fixture_with_text(fixtures.path(), "alpha", "# Alpha\n\nBody", None);
        std::fs::write(outputs.path().join("alpha.md"), "# Alpha\n\nBody").unwrap();

        let report = score_outputs(fixtures.path(), outputs.path()).unwrap();

        assert_eq!(report.mean_sf1, 1.0);
        assert_eq!(report.mean_tf1, 1.0);
    }

    #[test]
    fn whitespace_ids_and_content_fail_closed_for_all_output_forms() {
        let markdown_dir = TempDir::new().unwrap();
        let whitespace_id = markdown_dir.path().join("   .md");
        std::fs::write(&whitespace_id, "content").unwrap();
        assert!(
            load_outputs(&whitespace_id)
                .unwrap_err()
                .to_string()
                .contains("valid UTF-8 fixture ID")
        );

        let blank_markdown = markdown_dir.path().join("alpha.md");
        std::fs::write(&blank_markdown, " \n\t").unwrap();
        assert!(
            load_outputs(&blank_markdown)
                .unwrap_err()
                .to_string()
                .contains("content")
        );

        let json_map = markdown_dir.path().join("map.json");
        std::fs::write(&json_map, r#"{"   ":"content"}"#).unwrap();
        assert!(load_outputs(&json_map).unwrap_err().to_string().contains("fixture ID"));
        std::fs::write(&json_map, r#"{"alpha":"  "}"#).unwrap();
        assert!(load_outputs(&json_map).unwrap_err().to_string().contains("content"));

        let json_record = markdown_dir.path().join("record.json");
        std::fs::write(&json_record, r#"{"id":"  ","content":"content"}"#).unwrap();
        assert!(
            load_outputs(&json_record)
                .unwrap_err()
                .to_string()
                .contains("fixture ID")
        );
        std::fs::write(&json_record, r#"{"id":"alpha","content":"\n "}"#).unwrap();
        assert!(load_outputs(&json_record).unwrap_err().to_string().contains("content"));

        let jsonl = markdown_dir.path().join("records.jsonl");
        std::fs::write(&jsonl, "{\"id\":\" \",\"content\":\"content\"}\n").unwrap();
        assert!(load_outputs(&jsonl).unwrap_err().to_string().contains("fixture ID"));
        std::fs::write(&jsonl, "{\"id\":\"alpha\",\"content\":\" \"}\n").unwrap();
        assert!(load_outputs(&jsonl).unwrap_err().to_string().contains("content"));
    }

    #[test]
    fn duplicate_json_map_ids_fail_closed() {
        let fixtures = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();
        write_fixture(fixtures.path(), "alpha", "Alpha");
        let outputs = output_dir.path().join("outputs.json");
        std::fs::write(&outputs, "{\"alpha\":\"first\",\"alpha\":\"second\"}").unwrap();

        let error = score_outputs(fixtures.path(), &outputs).unwrap_err();

        assert!(error.to_string().contains("duplicate output fixture ID 'alpha'"));
    }

    #[test]
    fn duplicate_json_record_fields_fail_closed() {
        let outputs = TempDir::new().unwrap();
        let exact_duplicate = outputs.path().join("exact.json");
        std::fs::write(&exact_duplicate, r#"[{"id":"alpha","id":"beta","content":"Alpha"}]"#).unwrap();
        assert!(
            load_outputs(&exact_duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );

        let alias_duplicate = outputs.path().join("alias.json");
        std::fs::write(
            &alias_duplicate,
            r#"{"id":"alpha","fixture_id":"beta","content":"Alpha"}"#,
        )
        .unwrap();
        assert!(
            load_outputs(&alias_duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
    }
}
