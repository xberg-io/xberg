use nli_qualification::{FrozenPlan, RunReport, digest};
use serde_json::{Value, json};

pub fn plan_value() -> Value {
    let manifest = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ]
    .map(|name| format!("{}  {name}\n", "a".repeat(64)))
    .concat();
    json!({
        "schema_version": 1, "qualified": false, "expected_inferences": 4, "repeats": 2,
        "labels": ["science", "finance"], "arms": ["fp32"],
        "provenance": {"corpus_receipt_sha256": "b".repeat(64), "split_audit_sha256": "c".repeat(64)},
        "runtime": {"runner_sha256": "d".repeat(64), "dependency_lock_sha256": "5".repeat(64),
            "ort_build_info": "test ORT",
            "os": "macos", "arch": "aarch64", "cpu": "test CPU", "threads": 0, "deadline_ms": 1000},
        "documents": [
            {"id": "doc-a", "family": "source-a", "split": "calibration", "source_sha256": "e".repeat(64),
             "text": "Scientific work.", "text_sha256": digest(b"Scientific work."), "gold": ["science"]},
            {"id": "doc-b", "family": "source-b", "split": "held_out", "source_sha256": "f".repeat(64),
             "text": "A financial report.", "text_sha256": digest(b"A financial report."), "gold": []}
        ],
        "candidates": [{"id": "nli", "variants": {"fp32": {
            "source_revision": "1".repeat(40), "export_recipe_sha256": "2".repeat(64),
            "license_receipt_sha256": "3".repeat(64),
            "config": {"model": {"repository": "owner/nli", "revision": "4".repeat(40),
                "model_file": "model.onnx", "external_files": [], "sha256_manifest": manifest,
                "output_name": "logits", "entailment_index": 2, "contradiction_index": 0},
                "hypothesis_template": "This text is about {label}.", "multi_label": false,
                "confidence_threshold": 0.75, "cache_directory": null}
        }}}]
    })
}

pub fn plan() -> FrozenPlan {
    serde_json::from_value(plan_value()).unwrap()
}

pub fn report(plan: &FrozenPlan) -> RunReport {
    let manifest = &plan.candidates[0].variants["fp32"].config.model.sha256_manifest;
    let rows = ["doc-a", "doc-b"]
        .into_iter()
        .flat_map(|document| {
            (1..=2).map(move |repeat| {
                json!({"document": document, "candidate": "nli", "arm": "fp32", "repeat": repeat,
            "inference": "completed", "elapsed_ms": 1.0, "labels": [],
            "model": {"selected_backend": "onnx_cpu", "effective_backend": "onnx_cpu",
                "repository": "owner/nli", "revision": "4".repeat(40), "model_file": "model.onnx",
                "model_sha256": "a".repeat(64),
                "manifest_sha256": digest(manifest.as_bytes()),
                "experimental": true}})
            })
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({"schema_version": 1, "qualified": false,
        "plan_sha256": plan.sha256().unwrap(), "runtime": plan.runtime, "rows": rows}))
    .unwrap()
}
