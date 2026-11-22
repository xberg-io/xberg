use nli_qualification::{FrozenPlan, RunReport};
use onnx_qualification::caption::{CaptionPlan, CaptionReport};
use onnx_qualification::corpus::{CaptionManifest, CorpusManifest};
use onnx_qualification::digest;
use onnx_qualification::prereg::Preregistration;
use serde_json::{Value, json};

pub const MANIFEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub fn corpus() -> Value {
    let mut value = json!({
        "schema": 1, "purpose": "test fixture", "source": {}, "selection": {}, "counts": {},
        "documents": [
            {"id": "d1", "family": "f1", "subfamily": "f1", "split": "held_out", "type": "alpha",
             "type_code": "RULE", "known_class": true, "document_number": "1",
             "publication_date": "2026-01-01", "agency": "A", "source_url": "https://example.test/1",
             "source_sha256": "1".repeat(64), "text_sha256": digest(b"Alpha one."),
             "text_bytes": 10, "removed_metadata_lines": []},
            {"id": "d2", "family": "f1", "subfamily": "f1", "split": "held_out", "type": "beta",
             "type_code": "NOTICE", "known_class": true, "document_number": "2",
             "publication_date": "2026-01-01", "agency": "A", "source_url": "https://example.test/2",
             "source_sha256": "2".repeat(64), "text_sha256": digest(b"Beta two."),
             "text_bytes": 9, "removed_metadata_lines": []},
            {"id": "d3", "family": "f2", "subfamily": "f2", "split": "held_out", "type": "alpha",
             "type_code": "RULE", "known_class": true, "document_number": "3",
             "publication_date": "2026-01-01", "agency": "B", "source_url": "https://example.test/3",
             "source_sha256": "3".repeat(64), "text_sha256": digest(b"Alpha three."),
             "text_bytes": 12, "removed_metadata_lines": []},
            {"id": "d4", "family": "f2", "subfamily": "f2", "split": "held_out", "type": "unknown",
             "type_code": "PRESDOCU", "known_class": false, "document_number": "4",
             "publication_date": "2026-01-01", "agency": "B", "source_url": "https://example.test/4",
             "source_sha256": "4".repeat(64), "text_sha256": digest(b"Unknown four."),
             "text_bytes": 13, "removed_metadata_lines": []},
            {"id": "d5", "family": "f3", "subfamily": "f3", "split": "calibration", "type": "beta",
             "type_code": "NOTICE", "known_class": true, "document_number": "5",
             "publication_date": "2026-01-01", "agency": "C", "source_url": "https://example.test/5",
             "source_sha256": "5".repeat(64), "text_sha256": digest(b"Beta five."),
             "text_bytes": 10, "removed_metadata_lines": []},
            {"id": "d6", "family": "f4", "subfamily": "f4", "split": "training", "type": "alpha",
             "type_code": "RULE", "known_class": true, "document_number": "6",
             "publication_date": "2026-01-01", "agency": "D", "source_url": "https://example.test/6",
             "source_sha256": "6".repeat(64), "text_sha256": digest(b"Alpha six."),
             "text_bytes": 10, "removed_metadata_lines": []}
        ],
        "failures": [], "digest": ""
    });
    value["digest"] = json!(onnx_qualification::canonical_digest(&value["documents"]).unwrap());
    value
}

pub fn corpus_manifest() -> CorpusManifest {
    serde_json::from_value(corpus()).unwrap()
}

pub fn plan() -> FrozenPlan {
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
    serde_json::from_value(json!({
        "schema_version": 1, "qualified": false, "expected_inferences": 10, "repeats": 2,
        "labels": ["alpha", "beta"], "arms": ["int8"],
        "provenance": {"corpus_receipt_sha256": "b".repeat(64), "split_audit_sha256": "c".repeat(64)},
        "runtime": {"runner_sha256": "d".repeat(64), "dependency_lock_sha256": "5".repeat(64),
            "ort_build_info": "test ORT", "os": "macos", "arch": "aarch64", "cpu": "test CPU",
            "threads": 0, "deadline_ms": 1000},
        "documents": [
            {"id": "d1", "family": "f1", "split": "held_out", "source_sha256": "1".repeat(64),
             "text": "Alpha one.", "text_sha256": digest(b"Alpha one."), "gold": ["alpha"]},
            {"id": "d2", "family": "f1", "split": "held_out", "source_sha256": "2".repeat(64),
             "text": "Beta two.", "text_sha256": digest(b"Beta two."), "gold": ["beta"]},
            {"id": "d3", "family": "f2", "split": "held_out", "source_sha256": "3".repeat(64),
             "text": "Alpha three.", "text_sha256": digest(b"Alpha three."), "gold": ["alpha"]},
            {"id": "d4", "family": "f2", "split": "held_out", "source_sha256": "4".repeat(64),
             "text": "Unknown four.", "text_sha256": digest(b"Unknown four."), "gold": []},
            {"id": "d5", "family": "f3", "split": "calibration", "source_sha256": "5".repeat(64),
             "text": "Beta five.", "text_sha256": digest(b"Beta five."), "gold": ["beta"]}
        ],
        "candidates": [{"id": "nli", "variants": {"int8": {
            "source_revision": "1".repeat(40), "export_recipe_sha256": "2".repeat(64),
            "license_receipt_sha256": "3".repeat(64),
            "config": {"model": {"repository": "owner/nli", "revision": "4".repeat(40),
                "model_file": "model.onnx", "external_files": [], "sha256_manifest": manifest_lines,
                "size_manifest": size_lines, "output_name": "logits",
                "entailment_index": 2, "contradiction_index": 0},
                "hypothesis_template": "This text is about {label}.", "multi_label": false,
                "confidence_threshold": 0.6, "cache_directory": null}
        }}}]
    }))
    .unwrap()
}

pub fn preregistration() -> Preregistration {
    serde_json::from_value(prereg_value()).unwrap()
}

pub fn prereg_value() -> Value {
    json!({
        "schema": 1, "issue": 2334, "posted_before_run": true,
        "posted_url": "https://github.com/xberg-io/xberg/issues/2334#issuecomment-1",
        "corpora": {
            "classification": {"manifest_sha256": "7".repeat(64),
                "digest": corpus_manifest().digest, "entries": 6,
                "split_counts": {"training": 1, "calibration": 1, "held_out": 4},
                "license": "US-PD", "license_url": "https://example.test/license"},
            "captioning": {"manifest_sha256": "8".repeat(64),
                "digest": caption_manifest().digest, "entries": 2,
                "split_counts": {"held_out": 2},
                "license": "CC-BY-4.0", "license_url": "https://example.test/caption-license"}
        },
        "models": [
            {"workload": "classification", "id": "nli", "repository": "owner/nli",
             "revision": "4".repeat(40), "license": "Apache-2.0", "arms": {"int8": "model.onnx"}},
            {"workload": "captioning", "id": "smolvlm", "repository": "owner/vlm",
             "revision": "5".repeat(40), "license": "Apache-2.0",
             "arms": {"fp32": "onnx/decoder_model_merged.onnx", "int8": "onnx/decoder_model_merged_int8.onnx"}}
        ],
        "classification": {
            "candidate_ids": ["nli"], "arms": ["int8"], "repeats": 2,
            "scored_splits": ["calibration", "held_out"],
            "expected": {"factors": [
                {"name": "documents", "count": 5}, {"name": "models", "count": 1},
                {"name": "arms", "count": 1}, {"name": "repeats", "count": 2}], "total": 10},
            "labels": ["alpha", "beta"], "unknown_class": "unknown",
            "hypothesis_template": "This text is about {label}.", "confidence_threshold": 0.6,
            "baseline": "always predict the most frequent gold class",
            "thresholds": {"macro_f1_noninferiority": 0.5, "macro_f1_target": 0.65,
                "per_class_recall_min": 0.45, "ece_max": 0.2, "max_abstention_on_known": 0.4,
                "unknown_rejection_min": 0.7, "ci_level": 0.95, "bootstrap_replicates": 300}
        },
        "captioning": {
            "candidate_ids": ["smolvlm"], "arms": ["fp32", "int8"], "repeats": 1,
            "expected_per_candidate": {"factors": [
                {"name": "images", "count": 2}, {"name": "models", "count": 1},
                {"name": "arms", "count": 2}, {"name": "repeats", "count": 1}], "total": 4},
            "expected_total": 4,
            "thresholds": {"number_recall_min": 0.2, "term_recall_min": 0.35,
                "max_invented_number_image_rate": 0.6, "ci_level": 0.95, "bootstrap_replicates": 300},
            "human_rating_required": true
        },
        "metric_definitions": {"macro_f1": "unweighted mean per-class F1"},
        "unmeasured": [{"workload": "captioning", "clause": "visible-fact precision",
            "reason": "no automatic oracle", "requires": "blind human raters"}]
    })
}

/// Repeat 1 leaves d3 abstained and d4 forced; repeat 2 answers everything correctly.
pub fn report() -> RunReport {
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
    let row = |document: &str, repeat: u32, labels: Value| -> Value {
        json!({"document": document, "candidate": "nli", "arm": "int8", "repeat": repeat,
            "inference": "completed", "elapsed_ms": 1.0, "labels": labels,
            "model": {"selected_backend": "onnx_cpu", "effective_backend": "onnx_cpu",
                "repository": "owner/nli", "revision": "4".repeat(40),
                "model_file": "model.onnx", "model_sha256": MANIFEST,
                "manifest_sha256": manifest_sha, "experimental": true}})
    };
    let label = |name: &str, confidence: f64| json!([{"label": name, "confidence": confidence}]);
    let rows = vec![
        row("d1", 1, label("alpha", 0.9)),
        row("d2", 1, label("alpha", 0.7)),
        row("d3", 1, json!([])),
        row("d4", 1, label("beta", 0.6)),
        row("d5", 1, label("beta", 0.8)),
        row("d1", 2, label("alpha", 0.9)),
        row("d2", 2, label("beta", 0.8)),
        row("d3", 2, label("alpha", 0.7)),
        row("d4", 2, json!([])),
        row("d5", 2, label("beta", 0.8)),
    ];
    serde_json::from_value(json!({
        "schema_version": 1, "qualified": false,
        "plan_sha256": plan().sha256().unwrap(),
        "runtime": plan().runtime,
        "rows": rows
    }))
    .unwrap()
}

pub fn report_value() -> Value {
    serde_json::to_value(report()).unwrap()
}

pub fn caption_manifest() -> CaptionManifest {
    let mut value = json!({
        "schema": 1, "purpose": "test fixture", "source": {}, "selection": {}, "counts": {},
        "images": [
            {"id": "a", "image_file": "a.jpg", "image_url": "https://example.test/a.jpg",
             "image_sha256": "a".repeat(64), "image_bytes": 10, "width": 10, "height": 10,
             "split": "held_out", "stratum": "numbers",
             "reference_captions": ["A clock shows 3.", "A clock shows 45."],
             "reference_numbers": ["3", "45"], "reference_terms": ["clock", "shows"]},
            {"id": "b", "image_file": "b.jpg", "image_url": "https://example.test/b.jpg",
             "image_sha256": "b".repeat(64), "image_bytes": 10, "width": 10, "height": 10,
             "split": "held_out", "stratum": "background",
             "reference_captions": ["A dog runs."],
             "reference_numbers": [], "reference_terms": ["dog", "runs"]}
        ],
        "digest": ""
    });
    value["digest"] = json!(onnx_qualification::canonical_digest(&value["images"]).unwrap());
    serde_json::from_value(value).unwrap()
}

pub fn caption_plan() -> CaptionPlan {
    let machine = nli_qualification::RuntimeIdentity {
        runner_sha256: "d".repeat(64),
        dependency_lock_sha256: "5".repeat(64),
        ort_build_info: "test ORT".into(),
        os: "macos".into(),
        arch: "aarch64".into(),
        cpu: "test CPU".into(),
        threads: 0,
        deadline_ms: 1000,
    };
    serde_json::from_value(json!({
        "schema": 1, "qualified": false, "expected_generations": 4, "repeats": 1,
        "arms": ["fp32", "int8"],
        "provenance": {"corpus_receipt_sha256": "b".repeat(64), "split_audit_sha256": "c".repeat(64)},
        "runtime": machine,
        "images": [
            {"id": "a", "image_file": "a.jpg", "image_sha256": "a".repeat(64), "image_bytes": 10},
            {"id": "b", "image_file": "b.jpg", "image_sha256": "b".repeat(64), "image_bytes": 10}
        ],
        "candidates": [{"id": "smolvlm", "arms": {
            "fp32": {"precision": "fp32"}, "int8": {"precision": "quantized"}}}]
    }))
    .unwrap()
}

pub fn caption_report() -> CaptionReport {
    let plan = caption_plan();
    let rows = ["fp32", "int8"]
        .into_iter()
        .flat_map(|arm| {
            [
                json!({"image": "a", "candidate": "smolvlm", "arm": arm, "repeat": 1,
                    "inference": "completed", "elapsed_ms": 1.0, "caption": "A clock shows 3.",
                    "model": {"format": "smol_vlm",
                        "precision": if arm == "fp32" {"fp32"} else {"quantized"},
                        "experimental": true}}),
                json!({"image": "b", "candidate": "smolvlm", "arm": arm, "repeat": 1,
                    "inference": "completed", "elapsed_ms": 1.0,
                    "caption": "A dog runs at 5 pm.",
                    "model": {"format": "smol_vlm",
                        "precision": if arm == "fp32" {"fp32"} else {"quantized"},
                        "experimental": true}}),
            ]
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({
        "schema": 1, "qualified": false,
        "plan_sha256": plan.sha256().unwrap(),
        "runtime": plan.runtime,
        "rows": rows
    }))
    .unwrap()
}
