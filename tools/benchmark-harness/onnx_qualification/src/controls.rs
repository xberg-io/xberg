//! Negative controls. Each control mutates a valid fixture in one way and requires the real
//! validator or scorer to reject it with a specific message. A control that "passes" because the
//! operation succeeded is reported as a failed control — that is the zero-work pass this file
//! exists to catch.

use crate::caption::{CaptionPlan, CaptionReport};
use crate::corpus::{CaptionManifest, CorpusManifest};
use crate::metrics::{Bootstrap, CaptionGeneration};
use crate::prereg::Preregistration;
use crate::score::{score_captioning, score_classification};
use crate::{Result, digest, require};
use nli_qualification::{FrozenPlan, RunReport};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlResult {
    pub id: String,
    pub description: String,
    pub expected_failure_contains: String,
    /// The error the real code produced, or `None` when it wrongly accepted the input.
    pub observed: Option<String>,
    pub control_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlReport {
    pub controls: Vec<ControlResult>,
    pub passed: usize,
    pub failed: usize,
}

impl ControlReport {
    pub fn require_all_passed(&self) -> Result<()> {
        require(
            self.failed == 0 && !self.controls.is_empty(),
            "a negative control was accepted; the run cannot be trusted",
        )
    }
}

fn expect_failure<T>(id: &str, description: &str, marker: &str, result: Result<T>) -> ControlResult {
    let observed = result.err().map(|error| error.to_string());
    let control_passed = observed.as_ref().is_some_and(|message| message.contains(marker));
    ControlResult {
        id: id.into(),
        description: description.into(),
        expected_failure_contains: marker.into(),
        observed,
        control_passed,
    }
}

const MANIFEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub fn run() -> ControlReport {
    let mut controls = corpus_controls();
    controls.extend(prereg_controls());
    controls.extend(report_controls());
    controls.extend(plan_controls());
    controls.extend(caption_controls());
    controls.extend(scoring_controls());
    let passed = controls.iter().filter(|control| control.control_passed).count();
    ControlReport {
        failed: controls.len() - passed,
        passed,
        controls,
    }
}

fn corpus_controls() -> Vec<ControlResult> {
    let mut controls = Vec::new();
    let mut empty = corpus_value();
    empty["documents"] = json!([]);
    empty["digest"] = json!(digest(b"[]"));
    controls.push(expect_failure(
        "empty-corpus",
        "a corpus with no documents cannot authorize anything",
        "must not be empty",
        manifest(&empty).and_then(|manifest| manifest.validate()),
    ));

    let mut training_only = corpus_value();
    for document in training_only["documents"].as_array_mut().unwrap() {
        document["split"] = json!("training");
    }
    training_only["digest"] = json!(crate::canonical_digest(&training_only["documents"]).unwrap());
    controls.push(expect_failure(
        "no-held-out-documents",
        "a corpus without held-out work has nothing to qualify on",
        "no held-out documents",
        manifest(&training_only).and_then(|manifest| manifest.validate()),
    ));

    let mut tampered = corpus_value();
    tampered["documents"][0]["text_sha256"] = json!("b".repeat(64));
    controls.push(expect_failure(
        "corpus-digest-tamper",
        "editing a committed document without a new corpus digest is refused",
        "digest mismatch",
        manifest(&tampered).and_then(|manifest| manifest.validate()),
    ));

    let mut leaky = corpus_value();
    leaky["documents"][2]["family"] = leaky["documents"][0]["family"].clone();
    leaky["digest"] = json!(crate::canonical_digest(&leaky["documents"]).unwrap());
    controls.push(expect_failure(
        "family-leakage",
        "one source family may not span two splits",
        "family spans two splits",
        manifest(&leaky).and_then(|manifest| manifest.validate()),
    ));
    controls
}

fn prereg_controls() -> Vec<ControlResult> {
    let wrong_total = prereg_value_with(|value| {
        value["classification"]["expected"]["total"] = json!(7);
    });
    vec![expect_failure(
        "wrong-expected-count",
        "a preregistered total that disagrees with its factors is refused",
        "does not equal its factors",
        prereg(&wrong_total).and_then(|prereg| prereg.validate()),
    )]
}

fn report_controls() -> Vec<ControlResult> {
    let mut controls = Vec::new();
    let skipped = report_value_with(|value| {
        value["rows"][0]["inference"] = json!("skipped");
    });
    controls.push(expect_failure(
        "skipped-inference",
        "a skipped row is never counted as completed work",
        "inference must complete",
        score(&skipped),
    ));

    let missing = report_value_with(|value| {
        value["rows"].as_array_mut().unwrap().pop();
    });
    controls.push(expect_failure(
        "missing-prediction",
        "a report missing one document x model x arm x repeat row is refused",
        "row count",
        score(&missing),
    ));

    let unexpected = report_value_with(|value| {
        value["rows"][0]["document"] = json!("not-in-plan");
    });
    controls.push(expect_failure(
        "unexpected-document",
        "a prediction for a document the plan never froze is refused",
        "unexpected document",
        score(&unexpected),
    ));

    let unknown_backend = report_value_with(|value| {
        value["rows"][0]["model"]["effective_backend"] = json!("provider");
    });
    controls.push(expect_failure(
        "unknown-backend",
        "a completed CPU row cannot claim an unknown backend",
        "unknown variant",
        nli_qualification::files::parse_json::<RunReport>(&serde_json::to_vec(&unknown_backend).unwrap()),
    ));

    let corrupt_artifact = report_value_with(|value| {
        value["rows"][0]["model"]["model_sha256"] = json!("0".repeat(64));
    });
    controls.push(expect_failure(
        "corrupt-artifact",
        "a report claiming a different graph digest than the frozen pin is refused",
        "model identity",
        score(&corrupt_artifact),
    ));
    controls
}

fn plan_controls() -> Vec<ControlResult> {
    let mut missing_companion = plan_value();
    missing_companion["candidates"][0]["variants"]["int8"]["config"]["model"]["sha256_manifest"] =
        json!(format!("{MANIFEST}  model.onnx\n"));
    vec![expect_failure(
        "missing-companion-artifact",
        "a frozen plan without tokenizer/configuration pins is refused",
        "companion",
        plan(&missing_companion).and_then(|plan| plan.validate()),
    )]
}

fn caption_controls() -> Vec<ControlResult> {
    let empty_generations = crate::metrics::score_captions(
        "smolvlm",
        "int8",
        &[] as &[CaptionGeneration],
        &Bootstrap::new(0.95, 200),
    );
    let skipped = caption_report_value_with(|value| {
        value["rows"][0]["inference"] = json!("skipped");
    });
    vec![
        expect_failure(
            "caption-empty-generation-set",
            "an empty caption set yields unmeasured metrics, never a zero that reads as a pass",
            "no held-out generation",
            match empty_generations.invented_number_image_rate.value {
                None => Err(empty_generations
                    .invented_number_image_rate
                    .reason
                    .unwrap_or_default()
                    .into()),
                Some(_) => Ok(()),
            },
        ),
        expect_failure(
            "caption-skipped-inference",
            "the caption runner cannot report a skipped row as completed",
            "must complete",
            caption_score(&skipped),
        ),
    ]
}

fn scoring_controls() -> Vec<ControlResult> {
    let mut missing_document = corpus_value();
    missing_document["documents"].as_array_mut().unwrap().remove(0);
    missing_document["digest"] = json!(crate::canonical_digest(&missing_document["documents"]).unwrap());
    let corpus: CorpusManifest = serde_json::from_value(missing_document).expect("fixture corpus parses");
    let outcome = score_classification(
        &plan(&plan_value()).expect("fixture plan parses"),
        &report(&report_value_with(|_| {})).expect("fixture report parses"),
        &corpus,
        &prereg(&prereg_value())
            .expect("fixture preregistration parses")
            .classification
            .expect("fixture classification prereg"),
    );
    vec![expect_failure(
        "plan-corpus-disagreement",
        "scoring refuses a plan document the committed corpus does not contain",
        "absent from the committed corpus",
        outcome.map(|_| ()),
    )]
}

fn manifest(value: &Value) -> Result<CorpusManifest> {
    Ok(serde_json::from_value(value.clone())?)
}

fn prereg(value: &Value) -> Result<Preregistration> {
    Ok(serde_json::from_value(value.clone())?)
}

fn plan(value: &Value) -> Result<FrozenPlan> {
    Ok(serde_json::from_value(value.clone())?)
}

fn report(value: &Value) -> Result<RunReport> {
    Ok(serde_json::from_value(value.clone())?)
}

fn score(report_value: &Value) -> Result<()> {
    let corpus = manifest(&corpus_value())?;
    let frozen = plan(&plan_value())?;
    let prereg = prereg(&prereg_value())?;
    let outcome = score_classification(
        &frozen,
        &report(report_value)?,
        &corpus,
        prereg.classification.as_ref().expect("classification prereg"),
    )?;
    require(!outcome.variants.is_empty(), "scoring produced no variant")?;
    Ok(())
}

fn caption_score(report_value: &Value) -> Result<()> {
    let corpus: CaptionManifest = serde_json::from_value(caption_corpus_value())?;
    let frozen: CaptionPlan = serde_json::from_value(caption_plan_value())?;
    let prereg = prereg(&prereg_value())?;
    score_captioning(
        &frozen,
        &serde_json::from_value::<CaptionReport>(report_value.clone())?,
        &corpus,
        prereg.captioning.as_ref().expect("captioning prereg"),
    )
    .map(|_| ())
}

fn corpus_value() -> Value {
    let value = json!({
        "schema": 1, "purpose": "control fixture", "source": {}, "selection": {}, "counts": {},
        "documents": [
            {"id": "doc-a", "family": "family-a", "subfamily": "family-a", "split": "held_out",
             "type": "alpha", "type_code": "RULE", "known_class": true, "document_number": "1",
             "publication_date": "2026-01-01", "agency": "A", "source_url": "https://example.test/1",
             "source_sha256": "e".repeat(64), "text_sha256": digest(b"Alpha body."),
             "text_bytes": 11, "removed_metadata_lines": []},
            {"id": "doc-b", "family": "family-b", "subfamily": "family-b", "split": "held_out",
             "type": "beta", "type_code": "NOTICE", "known_class": true, "document_number": "2",
             "publication_date": "2026-01-01", "agency": "B", "source_url": "https://example.test/2",
             "source_sha256": "f".repeat(64), "text_sha256": digest(b"Beta body."),
             "text_bytes": 10, "removed_metadata_lines": []},
            {"id": "doc-c", "family": "family-c", "subfamily": "family-c", "split": "calibration",
             "type": "alpha", "type_code": "RULE", "known_class": true, "document_number": "3",
             "publication_date": "2026-01-01", "agency": "C", "source_url": "https://example.test/3",
             "source_sha256": "9".repeat(64), "text_sha256": digest(b"Alpha calibration."),
             "text_bytes": 18, "removed_metadata_lines": []},
            {"id": "doc-u", "family": "family-u", "subfamily": "family-u", "split": "held_out",
             "type": "unknown", "type_code": "PRESDOCU", "known_class": false, "document_number": "4",
             "publication_date": "2026-01-01", "agency": "U", "source_url": "https://example.test/4",
             "source_sha256": "8".repeat(64), "text_sha256": digest(b"Unknown body."),
             "text_bytes": 13, "removed_metadata_lines": []}
        ],
        "failures": [],
        "digest": ""
    });
    let mut value = value;
    value["digest"] = json!(crate::canonical_digest(&value["documents"]).unwrap());
    value
}

fn plan_value() -> Value {
    let manifest_lines = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ]
    .map(|name| format!("{MANIFEST}  {name}\n"))
    .concat();
    let size_lines = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ]
    .map(|name| format!("1  {name}\n"))
    .concat();
    json!({
        "schema_version": 1, "qualified": false, "expected_inferences": 8, "repeats": 2,
        "labels": ["alpha", "beta"], "arms": ["int8"],
        "provenance": {"corpus_receipt_sha256": "b".repeat(64), "split_audit_sha256": "c".repeat(64)},
        "runtime": {"runner_sha256": "d".repeat(64), "dependency_lock_sha256": "5".repeat(64),
            "ort_build_info": "control ORT", "os": "macos", "arch": "aarch64", "cpu": "control CPU",
            "threads": 0, "deadline_ms": 1000},
        "documents": [
            {"id": "doc-a", "family": "family-a", "split": "held_out", "source_sha256": "e".repeat(64),
             "text": "Alpha body.", "text_sha256": digest(b"Alpha body."), "gold": ["alpha"]},
            {"id": "doc-b", "family": "family-b", "split": "held_out", "source_sha256": "f".repeat(64),
             "text": "Beta body.", "text_sha256": digest(b"Beta body."), "gold": ["beta"]},
            {"id": "doc-c", "family": "family-c", "split": "calibration", "source_sha256": "9".repeat(64),
             "text": "Alpha calibration.", "text_sha256": digest(b"Alpha calibration."), "gold": ["alpha"]},
            {"id": "doc-u", "family": "family-u", "split": "held_out", "source_sha256": "8".repeat(64),
             "text": "Unknown body.", "text_sha256": digest(b"Unknown body."), "gold": []}
        ],
        "candidates": [{"id": "nli", "variants": {"int8": {
            "source_revision": "1".repeat(40), "export_recipe_sha256": "2".repeat(64),
            "license_receipt_sha256": "3".repeat(64),
            "config": {"model": {"repository": "owner/nli", "revision": "4".repeat(40),
                "model_file": "model.onnx", "external_files": [], "sha256_manifest": manifest_lines,
                "size_manifest": size_lines, "output_name": "logits",
                "entailment_index": 2, "contradiction_index": 0},
                "hypothesis_template": "This text is about {label}.", "multi_label": false,
                "confidence_threshold": 0.5, "cache_directory": null}
        }}}]
    })
}

fn prereg_value() -> Value {
    prereg_value_with(|_| {})
}

fn prereg_value_with(mut mutate: impl FnMut(&mut Value)) -> Value {
    let mut value = json!({
        "schema": 1, "issue": 2334, "posted_before_run": true,
        "posted_url": "https://github.com/xberg-io/xberg/issues/2334#issuecomment-0",
        "corpora": {"classification": {"manifest_sha256": "7".repeat(64), "digest": "6".repeat(64),
            "entries": 4, "split_counts": {"held_out": 3, "calibration": 1},
            "license": "US-PD", "license_url": "https://example.test/license"}},
        "models": [
            {"workload": "classification", "id": "nli", "repository": "owner/nli",
             "revision": "4".repeat(40), "license": "Apache-2.0", "arms": {"int8": "model.onnx"}},
            {"workload": "captioning", "id": "smolvlm", "repository": "owner/vlm",
             "revision": "5".repeat(40), "license": "Apache-2.0",
             "arms": {"fp32": "onnx/decoder_model_merged.onnx", "int8": "onnx/decoder_model_merged_int8.onnx"}}
        ],
        "classification": {
            "candidate_ids": ["nli"], "arms": ["int8"], "repeats": 2,
            "scored_splits": ["held_out"], "labels": ["alpha", "beta"],
            "unknown_class": "unknown", "hypothesis_template": "This text is about {label}.",
            "confidence_threshold": 0.5, "baseline": "always predict alpha",
            "expected": {"factors": [
                {"name": "documents", "count": 4}, {"name": "models", "count": 1},
                {"name": "arms", "count": 1}, {"name": "repeats", "count": 2}], "total": 8},
            "thresholds": {"macro_f1_noninferiority": 0.5, "macro_f1_target": 0.6,
                "per_class_recall_min": 0.5, "ece_max": 0.2, "max_abstention_on_known": 0.4,
                "unknown_rejection_min": 0.5, "ci_level": 0.95, "bootstrap_replicates": 200}
        },
        "captioning": {
            "candidate_ids": ["smolvlm"], "arms": ["fp32", "int8"], "repeats": 1,
            "expected_per_candidate": {"factors": [
                {"name": "images", "count": 1}, {"name": "models", "count": 1},
                {"name": "arms", "count": 2}, {"name": "repeats", "count": 1}], "total": 2},
            "expected_total": 2,
            "thresholds": {"number_recall_min": 0.1, "term_recall_min": 0.1,
                "max_invented_number_image_rate": 0.9, "ci_level": 0.95, "bootstrap_replicates": 200},
            "human_rating_required": true
        },
        "metric_definitions": {"macro_f1": "unweighted mean F1 over supported classes"},
        "unmeasured": [{"workload": "captioning", "clause": "visible-fact precision",
            "reason": "no automatic oracle", "requires": "blind human raters"}]
    });
    mutate(&mut value);
    value
}

fn report_value_with(mut mutate: impl FnMut(&mut Value)) -> Value {
    let manifest_lines = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ]
    .map(|name| format!("{MANIFEST}  {name}\n"))
    .concat();
    let manifest_sha = digest(manifest_lines.as_bytes());
    let rows: Vec<Value> = ["doc-a", "doc-b", "doc-u", "doc-c"]
        .into_iter()
        .flat_map(|document| {
            let manifest_sha = manifest_sha.clone();
            (1..=2).map(move |repeat| {
                json!({"document": document, "candidate": "nli", "arm": "int8", "repeat": repeat,
                    "inference": "completed", "elapsed_ms": 1.0,
                    "labels": [{"label": "alpha", "confidence": 0.8}],
                    "model": {"selected_backend": "onnx_cpu", "effective_backend": "onnx_cpu",
                        "repository": "owner/nli", "revision": "4".repeat(40),
                        "model_file": "model.onnx", "model_sha256": MANIFEST,
                        "manifest_sha256": manifest_sha, "experimental": true}})
            })
        })
        .collect();
    let plan = plan(&plan_value()).unwrap();
    let mut value = json!({
        "schema_version": 1, "qualified": false, "plan_sha256": plan.sha256().unwrap(),
        "runtime": plan.runtime, "rows": rows
    });
    mutate(&mut value);
    value
}

fn caption_corpus_value() -> Value {
    let mut value = json!({
        "schema": 1, "purpose": "control fixture", "source": {}, "selection": {}, "counts": {},
        "images": [{
            "id": "image-1", "image_file": "000000000001.jpg",
            "image_url": "https://example.test/000000000001.jpg",
            "image_sha256": "a".repeat(64), "image_bytes": 10, "width": 10, "height": 10,
            "split": "held_out", "stratum": "background",
            "reference_captions": ["A clock shows 3."],
            "reference_numbers": ["3"], "reference_terms": ["clock", "shows"]
        }],
        "digest": ""
    });
    value["digest"] = json!(crate::canonical_digest(&value["images"]).unwrap());
    value
}

fn caption_plan_value() -> Value {
    let machine = nli_qualification::RuntimeIdentity {
        runner_sha256: "d".repeat(64),
        dependency_lock_sha256: "5".repeat(64),
        ort_build_info: "control ORT".into(),
        os: "macos".into(),
        arch: "aarch64".into(),
        cpu: "control CPU".into(),
        threads: 0,
        deadline_ms: 1000,
    };
    json!({
        "schema": 1, "qualified": false, "expected_generations": 2, "repeats": 1,
        "arms": ["fp32", "int8"],
        "provenance": {"corpus_receipt_sha256": "b".repeat(64), "split_audit_sha256": "c".repeat(64)},
        "runtime": machine,
        "images": [{"id": "image-1", "image_file": "000000000001.jpg",
            "image_sha256": "a".repeat(64), "image_bytes": 10}],
        "candidates": [{"id": "smolvlm", "arms": {
            "fp32": {"precision": "fp32"}, "int8": {"precision": "quantized"}}}]
    })
}

fn caption_report_value_with(mut mutate: impl FnMut(&mut Value)) -> Value {
    let plan: CaptionPlan = serde_json::from_value(caption_plan_value()).unwrap();
    let rows: Vec<Value> = ["fp32", "int8"]
        .into_iter()
        .map(|arm| {
            json!({"image": "image-1", "candidate": "smolvlm", "arm": arm, "repeat": 1,
                "inference": "completed", "elapsed_ms": 1.0, "caption": "A clock shows 3.",
                "model": {"format": "smol_vlm", "precision": if arm == "fp32" {"fp32"} else {"quantized"},
                    "experimental": true}})
        })
        .collect();
    let mut value = json!({"schema": 1, "qualified": false, "plan_sha256": plan.sha256().unwrap(),
        "runtime": plan.runtime, "rows": rows});
    mutate(&mut value);
    value
}
