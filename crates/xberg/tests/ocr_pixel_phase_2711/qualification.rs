use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Mode {
    Default,
    Denoise,
}

impl Mode {
    pub(super) fn selected() -> Self {
        if std::env::var_os("XBERG_2711_DENOISE").is_some() {
            Self::Denoise
        } else {
            Self::Default
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub name: String,
    pub sha256: String,
    pub expected: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<xberg::PageContent>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<xberg::types::ExtractionMethod>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    pub mode: Mode,
    pub font_sha256: String,
    pub tessdata_eng_sha256: String,
    pub rows: Vec<Row>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    font_sha256: String,
    tessdata_eng_sha256: String,
    cases: Vec<CasePin>,
    reference_reports: Vec<ReferencePin>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CasePin {
    name: String,
    pdf_sha256: String,
    expected_words: usize,
    source_text_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferencePin {
    mode: Mode,
    baseline_ocr_revision: String,
    native_revision: String,
    recorded_ocr_revision: String,
    baseline_sha256: String,
    recorded_sha256: String,
}

fn manifest() -> Result<Manifest, String> {
    let pins: Manifest = serde_json::from_str(include_str!("qualification-pins.json"))
        .map_err(|error| format!("read qualification manifest: {error}"))?;
    if pins.cases.len() != 13 || pins.reference_reports.len() != 2 {
        return Err("manifest must retain thirteen cases and both preprocessing modes".to_owned());
    }
    let names: BTreeSet<_> = pins.cases.iter().map(|pin| &pin.name).collect();
    if names.len() != pins.cases.len() {
        return Err("manifest has duplicate case identities".to_owned());
    }
    Ok(pins)
}

fn reference(mode: Mode, baseline: bool) -> &'static str {
    match (mode, baseline) {
        (Mode::Default, true) => include_str!("baseline-default.json"),
        (Mode::Denoise, true) => include_str!("baseline-denoise.json"),
        (Mode::Default, false) => include_str!("recorded-default.json"),
        (Mode::Denoise, false) => include_str!("recorded-denoise.json"),
    }
}

fn retained(mode: Mode, baseline: bool, pins: &Manifest) -> Result<Report, String> {
    let pin = pins
        .reference_reports
        .iter()
        .find(|pin| pin.mode == mode)
        .ok_or_else(|| format!("missing retained reference for {mode:?}"))?;
    let revisions = [
        &pin.baseline_ocr_revision,
        &pin.native_revision,
        &pin.recorded_ocr_revision,
    ];
    if revisions
        .iter()
        .any(|revision| revision.len() != 40 || !revision.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("retained reference has invalid revision provenance".to_owned());
    }
    let json = reference(mode, baseline);
    let expected_hash = if baseline {
        &pin.baseline_sha256
    } else {
        &pin.recorded_sha256
    };
    if hex::encode(Sha256::digest(json.as_bytes())) != *expected_hash {
        return Err(format!("retained {mode:?} reference SHA-256 mismatch"));
    }
    let report: Report = serde_json::from_str(json).map_err(|error| format!("read retained reference: {error}"))?;
    if report.mode != mode {
        return Err("retained reference preprocessing mode mismatch".to_owned());
    }
    validate_report(&report, pins)?;
    Ok(report)
}

pub(super) fn recorded(mode: Mode) -> Report {
    retained(mode, false, &manifest().expect("pinned qualification manifest")).expect("pinned OCR recording")
}

pub(super) fn report(rows: Vec<Row>) -> Report {
    Report {
        mode: Mode::selected(),
        font_sha256: super::fixture::FONT_SHA256.to_owned(),
        tessdata_eng_sha256: "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2".to_owned(),
        rows,
    }
}

pub(super) fn finish_report(report: &Report, destination: &Path) -> Result<(), String> {
    qualify(report)?;
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| format!("serialize report: {error}"))?;
    std::fs::write(destination, bytes).map_err(|error| format!("write report {}: {error}", destination.display()))?;
    println!("quality gate passed: 13 pinned cases, no lost source occurrences or new false words");
    Ok(())
}

fn validate_report(report: &Report, pins: &Manifest) -> Result<(), String> {
    if report.font_sha256 != pins.font_sha256 || report.tessdata_eng_sha256 != pins.tessdata_eng_sha256 {
        return Err("report font or English model SHA-256 mismatch".to_owned());
    }
    if report.rows.len() != pins.cases.len() {
        return Err("report does not contain the thirteen pinned cases".to_owned());
    }
    let mut names = BTreeSet::new();
    for row in &report.rows {
        let pin = pins
            .cases
            .iter()
            .find(|pin| pin.name == row.name)
            .ok_or_else(|| format!("unknown qualification case {}", row.name))?;
        if !names.insert(&row.name) {
            return Err(format!("duplicate qualification case {}", row.name));
        }
        if row.sha256 != pin.pdf_sha256 {
            return Err(format!("{}: fixture SHA-256 mismatch", row.name));
        }
        if hex::encode(Sha256::digest(row.expected.as_bytes())) != pin.source_text_sha256
            || row.expected.split_whitespace().count() != pin.expected_words
        {
            return Err(format!("{}: independent source-text pin mismatch", row.name));
        }
        if row.text.trim().is_empty() {
            return Err(format!("{}: empty OCR read", row.name));
        }
    }
    Ok(())
}

fn occurrences(text: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c| ".,;:!?()[]{}\"".contains(c)).to_lowercase();
        if !word.is_empty() {
            *counts.entry(word).or_default() += 1;
        }
    }
    counts
}

fn compare_row(before: &Row, after: &Row) -> Result<(), String> {
    let source = occurrences(&before.expected);
    let baseline = occurrences(&before.text);
    let candidate = occurrences(&after.text);
    for (word, expected) in &source {
        let old = baseline.get(word).copied().unwrap_or_default().min(*expected);
        let new = candidate.get(word).copied().unwrap_or_default().min(*expected);
        if new < old {
            return Err(format!(
                "{}: lost source occurrence {word:?}: {old} -> {new}",
                after.name
            ));
        }
    }
    for (word, count) in &candidate {
        let expected = source.get(word).copied().unwrap_or_default();
        let old = baseline.get(word).copied().unwrap_or_default().saturating_sub(expected);
        if count.saturating_sub(expected) > old {
            return Err(format!("{}: new false word occurrence {word:?}", after.name));
        }
    }
    Ok(())
}

pub(super) fn qualify(report: &Report) -> Result<(), String> {
    let pins = manifest()?;
    let baseline = retained(report.mode, true, &pins)?;
    validate_report(report, &pins)?;
    for before in &baseline.rows {
        let after = report
            .rows
            .iter()
            .find(|row| row.name == before.name)
            .ok_or_else(|| format!("missing qualification case {}", before.name))?;
        compare_row(before, after)?;
    }
    Ok(())
}
