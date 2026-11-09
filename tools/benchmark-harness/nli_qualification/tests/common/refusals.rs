use super::common::{plan, report};
use serde_json::{Value, json};

pub struct Case {
    pub name: String,
    pub input: Value,
    pub error: &'static str,
}

fn plan_case(path: &str, value: Value, error: &'static str) -> Case {
    let mut input = serde_json::to_value(plan()).unwrap();
    *input.pointer_mut(path).unwrap() = value;
    Case {
        name: path.into(),
        input,
        error,
    }
}

fn config_case(path: &str, value: Value, error: &'static str) -> Case {
    plan_case(&format!("/candidates/0/variants/fp32/config{path}"), value, error)
}

pub fn plan_cases() -> Vec<Case> {
    let mut cases = plan_fields();
    cases.extend(runtime_fields());
    cases.extend(pin_fields());
    cases.extend(configuration_fields());
    cases.extend(artifact_paths());
    cases.extend(descriptor_bounds());
    cases.extend(accounting_cases());
    cases.extend(inventory_cases());
    cases.push(plan_case(
        "/documents/0/source_sha256",
        json!("g".repeat(64)),
        "document text digest, source digest or input size is invalid",
    ));
    let mut labels = vec!["science".to_string(), "finance".to_string()];
    labels.extend((0..63).map(|i| format!("extra{i}")));
    cases.push(plan_case("/labels", json!(labels), "labels must contain 1..64 entries"));
    cases.push(config_case(
        "/model/sha256_manifest",
        json!(
            plan().candidates[0].variants["fp32"]
                .config
                .model
                .sha256_manifest
                .replacen(&"a".repeat(64), &"g".repeat(64), 1)
        ),
        "invalid artifact manifest pin",
    ));
    cases
}

fn plan_fields() -> Vec<Case> {
    let mut cases = vec![
        plan_case("/schema_version", json!(2), "schema must be version 1 and unqualified"),
        plan_case("/qualified", json!(true), "schema must be version 1 and unqualified"),
        plan_case("/arms/0", json!("fp16"), "unsupported arm"),
        plan_case("/labels", json!([]), "labels must contain 1..64 entries"),
        plan_case("/labels", json!(["science", "SCIENCE"]), "empty or duplicate labels"),
        plan_case("/labels/0", json!(" science"), "empty or duplicate labels"),
        plan_case("/labels/0", json!(""), "empty or duplicate labels"),
        plan_case("/documents/0/gold", json!(["missing"]), "unknown gold label"),
        plan_case(
            "/documents/0/gold",
            json!(["science", "science"]),
            "empty or duplicate gold labels",
        ),
        plan_case("/documents/1/id", json!("doc-a"), "empty or duplicate document IDs"),
        plan_case("/candidates/0/variants", json!({}), "candidate arms do not match plan"),
    ];
    for path in ["/documents/0/id", "/documents/0/family", "/candidates/0/id"] {
        for value in ["x".repeat(129), "bad/id".into()] {
            let error = if path.starts_with("/documents") {
                "document and family identifiers must be opaque"
            } else {
                "invalid candidate ID"
            };
            cases.push(plan_case(path, json!(value), error));
        }
    }
    let mut duplicate_arm = plan_case("/arms", json!(["fp32", "fp32"]), "empty or duplicate arms");
    duplicate_arm.input["expected_inferences"] = json!(8);
    cases.push(duplicate_arm);
    let mut duplicate_candidate = plan_case("/candidates", json!([]), "empty or duplicate candidate IDs");
    let candidate = serde_json::to_value(&plan().candidates[0]).unwrap();
    duplicate_candidate.input["candidates"] = json!([candidate, candidate]);
    duplicate_candidate.input["expected_inferences"] = json!(8);
    cases.push(duplicate_candidate);
    cases
}

fn runtime_fields() -> Vec<Case> {
    let error = "runtime requires pinned identity, SDK-default threads (0) and a positive deadline";
    let mut cases = vec![
        plan_case("/runtime/threads", json!(1), error),
        plan_case("/runtime/deadline_ms", json!(0), error),
    ];
    for key in ["ort_build_info", "os", "arch", "cpu"] {
        cases.push(plan_case(&format!("/runtime/{key}"), json!("  "), error));
    }
    for key in ["runner_sha256", "dependency_lock_sha256"] {
        for value in ["g".repeat(64), "a".repeat(63)] {
            cases.push(plan_case(&format!("/runtime/{key}"), json!(value), error));
        }
    }
    cases
}

fn pin_fields() -> Vec<Case> {
    let mut cases = vec![];
    for key in ["corpus_receipt_sha256", "split_audit_sha256"] {
        cases.push(plan_case(
            &format!("/provenance/{key}"),
            json!("g".repeat(64)),
            "provenance receipts require SHA-256",
        ));
    }
    for (key, length) in [
        ("source_revision", 40),
        ("export_recipe_sha256", 64),
        ("license_receipt_sha256", 64),
    ] {
        for value in ["g".repeat(length), "a".repeat(length - 1)] {
            cases.push(plan_case(
                &format!("/candidates/0/variants/fp32/{key}"),
                json!(value),
                "candidate provenance is not pinned",
            ));
        }
    }
    for revision in ["g".repeat(40), "a".repeat(39)] {
        cases.push(config_case(
            "/model/revision",
            json!(revision),
            "model revision or repository is invalid",
        ));
    }
    for repository in ["owner", "../nli", "owner/nli/extra", "owner/name:port"] {
        cases.push(config_case(
            "/model/repository",
            json!(repository),
            "model revision or repository is invalid",
        ));
    }
    cases
}

fn configuration_fields() -> Vec<Case> {
    let mut cases = vec![];
    let limits = serde_json::to_value(plan().candidates[0].variants["fp32"].config.limits.clone()).unwrap();
    for key in ["max_input_bytes", "max_label_bytes", "max_labels", "max_tokens"] {
        for value in [0, limits[key].as_u64().unwrap() + 1] {
            cases.push(config_case(
                &format!("/limits/{key}"),
                json!(value),
                "NLI limits exceed SDK bounds",
            ));
        }
    }
    cases.extend(template_cache_fields());
    for value in [-0.1, 1.1] {
        cases.push(config_case(
            "/confidence_threshold",
            json!(value),
            "invalid NLI template or threshold",
        ));
    }
    for (key, value) in [
        ("entailment_index", 3),
        ("contradiction_index", 3),
        ("entailment_index", 0),
    ] {
        cases.push(config_case(
            &format!("/model/{key}"),
            json!(value),
            "invalid three-way NLI indices",
        ));
    }
    for output in [" ".into(), "x".repeat(257)] {
        cases.push(config_case(
            "/model/output_name",
            json!(output),
            "NLI model descriptor exceeds SDK bounds",
        ));
    }
    cases
}

fn artifact_paths() -> Vec<Case> {
    let mut cases = vec![];
    for path in [
        "CON.onnx",
        "prn",
        "AUX.bin",
        "nul",
        "COM1.onnx",
        "lpt9.onnx",
        "COM¹.onnx",
        "LPT².onnx",
        "COM³",
        "weights./model.onnx",
        "../model.onnx",
        "/model.onnx",
        "a\\b.onnx",
    ] {
        let mut case = config_case("/model/model_file", json!(path), "invalid or duplicate artifact path");
        let manifest = case
            .input
            .pointer_mut("/candidates/0/variants/fp32/config/model/sha256_manifest")
            .unwrap();
        *manifest = json!(manifest.as_str().unwrap().replace("model.onnx", path));
        case.name = format!("artifact path {path}");
        cases.push(case);
    }
    let mut case = config_case(
        "/model/external_files",
        json!(["other/weights.bin"]),
        "invalid graph-relative external tensor path",
    );
    let model = &mut case.input["candidates"][0]["variants"]["fp32"]["config"]["model"];
    model["model_file"] = json!("graph/model.onnx");
    model["sha256_manifest"] = json!(format!(
        "{}{}  other/weights.bin\n",
        model["sha256_manifest"]
            .as_str()
            .unwrap()
            .replace("model.onnx", "graph/model.onnx"),
        "a".repeat(64)
    ));
    cases.push(case);
    cases
}

pub fn report_cases() -> Vec<Case> {
    let mut cases = model_identity_fields();
    cases.extend(report_row_fields());
    cases.extend(report_runtime_fields());
    cases.extend(report_accounting_fields());
    cases
}

fn model_identity_fields() -> Vec<Case> {
    let mut cases = vec![];
    for (key, value) in [
        ("repository", json!("other/nli")),
        ("revision", json!("6".repeat(40))),
        ("model_file", json!("other.onnx")),
        ("model_sha256", json!("7".repeat(64))),
        ("manifest_sha256", json!("8".repeat(64))),
        ("experimental", json!(false)),
        ("selected_backend", json!("provider")),
        ("effective_backend", json!("provider")),
    ] {
        let mut input = serde_json::to_value(report(&plan())).unwrap();
        input["rows"][0]["model"][key] = value;
        cases.push(Case {
            name: format!("model {key}"),
            input,
            error: if key.ends_with("backend") {
                "unknown variant"
            } else {
                "model identity mismatch"
            },
        });
    }
    cases
}

fn report_row_fields() -> Vec<Case> {
    let mut cases = vec![];
    for (path, value, error) in [
        ("/qualified", json!(true), "report must remain unqualified"),
        ("/rows/0/inference", json!("skipped"), "inference must complete"),
        ("/rows/0/elapsed_ms", json!(0), "inference must complete"),
        ("/rows/0/document", json!("other"), "unexpected document"),
        ("/rows/0/candidate", json!("other"), "unexpected candidate"),
        ("/rows/0/arm", json!("int8"), "unexpected arm"),
        ("/rows/0/repeat", json!(0), "unexpected document or repeat"),
        ("/rows/0/repeat", json!(3), "unexpected document or repeat"),
        (
            "/rows/0/labels",
            json!([{"label": "unknown", "confidence": 0.9}]),
            "invalid prediction",
        ),
        (
            "/rows/0/labels",
            json!([{"label": "science", "confidence": 0.7}]),
            "invalid prediction",
        ),
        (
            "/rows/0/labels",
            json!([{"label": "science", "confidence": null}]),
            "single-label maximum probability",
        ),
    ] {
        let mut input = serde_json::to_value(report(&plan())).unwrap();
        *input.pointer_mut(path).unwrap() = value;
        cases.push(Case {
            name: path.into(),
            input,
            error,
        });
    }
    cases
}

fn report_runtime_fields() -> Vec<Case> {
    let mut cases = vec![];
    for (key, value) in [
        ("runner_sha256", json!("1".repeat(64))),
        ("dependency_lock_sha256", json!("2".repeat(64))),
        ("ort_build_info", json!("other")),
        ("os", json!("linux")),
        ("arch", json!("x86_64")),
        ("cpu", json!("other CPU")),
        ("threads", json!(1)),
        ("deadline_ms", json!(2)),
    ] {
        let mut input = serde_json::to_value(report(&plan())).unwrap();
        input["runtime"][key] = value;
        cases.push(Case {
            name: format!("report runtime {key}"),
            input,
            error: "runtime identity mismatch",
        });
    }
    cases
}

fn report_accounting_fields() -> Vec<Case> {
    let mut cases = vec![];
    for kind in ["missing", "extra", "duplicate", "digest"] {
        let mut input = serde_json::to_value(report(&plan())).unwrap();
        let rows = input["rows"].as_array_mut().unwrap();
        let error = match kind {
            "missing" => {
                rows.pop();
                "inference row count mismatch"
            }
            "extra" => {
                rows.push(rows[0].clone());
                "inference row count mismatch"
            }
            "duplicate" => {
                rows[1] = rows[0].clone();
                "duplicate inference row"
            }
            _ => {
                input["plan_sha256"] = json!("0".repeat(64));
                "frozen plan digest mismatch"
            }
        };
        cases.push(Case {
            name: kind.into(),
            input,
            error,
        });
    }
    cases
}

fn descriptor_bounds() -> Vec<Case> {
    let mut cases = vec![];
    let mut large_repository = config_case(
        "/model/repository",
        json!(format!("owner/{}", "x".repeat(251))),
        "NLI model descriptor exceeds SDK bounds",
    );
    large_repository.name = "repository byte ceiling".into();
    cases.push(large_repository);
    cases.push(long_graph_descriptor());
    let mut manifest = plan().candidates[0].variants["fp32"]
        .config
        .model
        .sha256_manifest
        .clone();
    manifest.push_str(&"\n".repeat(1024 * 1024));
    cases.push(config_case(
        "/model/sha256_manifest",
        json!(manifest),
        "NLI model descriptor exceeds SDK bounds",
    ));
    cases.push(excess_external_descriptors());
    let mut external = config_case(
        "/model/external_files",
        json!([format!("{}.bin", "x".repeat(1024))]),
        "invalid graph-relative external tensor path",
    );
    let model = &mut external.input["candidates"][0]["variants"]["fp32"]["config"]["model"];
    model["sha256_manifest"] = json!(format!(
        "{}{}  {}\n",
        model["sha256_manifest"].as_str().unwrap(),
        "a".repeat(64),
        model["external_files"][0].as_str().unwrap()
    ));
    cases.push(external);
    cases
}

fn long_graph_descriptor() -> Case {
    let long_path = format!("{}.onnx", "x".repeat(1024));
    let mut graph = config_case(
        "/model/model_file",
        json!(long_path),
        "NLI model descriptor exceeds SDK bounds",
    );
    let model = &mut graph.input["candidates"][0]["variants"]["fp32"]["config"]["model"];
    model["sha256_manifest"] = json!(
        model["sha256_manifest"]
            .as_str()
            .unwrap()
            .replace("model.onnx", model["model_file"].as_str().unwrap())
    );
    graph
}

fn excess_external_descriptors() -> Case {
    let mut tensors = config_case(
        "/model/external_files",
        json!([]),
        "NLI model descriptor exceeds SDK bounds",
    );
    let model = &mut tensors.input["candidates"][0]["variants"]["fp32"]["config"]["model"];
    let paths: Vec<_> = (0..65).map(|index| format!("tensor{index}.bin")).collect();
    model["external_files"] = json!(paths);
    model["sha256_manifest"] = json!(format!(
        "{}{}",
        model["sha256_manifest"].as_str().unwrap(),
        paths
            .iter()
            .map(|path| format!("{}  {path}\n", "a".repeat(64)))
            .collect::<String>()
    ));
    tensors
}

fn template_cache_fields() -> Vec<Case> {
    let mut cases = vec![];
    for template in ["no placeholder", "{label} {label}"] {
        cases.push(config_case(
            "/hypothesis_template",
            json!(template),
            "invalid NLI template or threshold",
        ));
    }
    cases.push(config_case(
        "/hypothesis_template",
        json!(format!("{}{{label}}", "x".repeat(4090))),
        "NLI template or cache path exceeds SDK bounds",
    ));
    for cache in ["  ".into(), "x".repeat(4097)] {
        cases.push(config_case(
            "/cache_directory",
            json!(cache),
            "NLI template or cache path exceeds SDK bounds",
        ));
    }
    cases
}

fn accounting_cases() -> Vec<Case> {
    let mut cases = vec![];
    for (path, value, error) in [
        ("/documents", json!([]), "documents must not be empty"),
        ("/candidates", json!([]), "candidates must not be empty"),
        ("/arms", json!([]), "arms must not be empty"),
        ("/repeats", json!(0), "repeats must be positive"),
        ("/expected_inferences", json!(3), "expected_inferences must equal"),
        (
            "/documents/0/family",
            json!(""),
            "document and family identifiers must be opaque",
        ),
        ("/documents/0/text", json!("changed"), "document text digest"),
    ] {
        cases.push(plan_case(path, value, error));
    }
    let mut cap = plan_case("/repeats", json!(50001), "expected_inferences must equal");
    cap.input["expected_inferences"] = json!(100002);
    cases.push(cap);
    for key in ["max_input_bytes", "max_label_bytes", "max_labels"] {
        cases.push(config_case(
            &format!("/limits/{key}"),
            json!(1),
            "NLI request exceeds variant input/label limits",
        ));
    }
    cases
}

fn inventory_cases() -> Vec<Case> {
    let mut cases = vec![];
    let original = plan().candidates[0].variants["fp32"]
        .config
        .model
        .sha256_manifest
        .clone();
    for (name, manifest, error) in [
        (
            "missing graph",
            manifest_without(&original, "model.onnx"),
            "artifact manifest lacks graph",
        ),
        (
            "missing companion",
            manifest_without(&original, "config.json"),
            "missing or ambiguous companion artifact pin",
        ),
        (
            "extra pin",
            format!("{original}{}  extra.bin\n", "a".repeat(64)),
            "unexpected artifact pin",
        ),
        (
            "duplicate path",
            format!("{original}{}  model.onnx\n", "a".repeat(64)),
            "invalid or duplicate artifact path",
        ),
        (
            "case duplicate path",
            format!("{original}{}  MODEL.ONNX\n", "a".repeat(64)),
            "invalid or duplicate artifact path",
        ),
        (
            "bad manifest columns",
            format!("{}  model.onnx trailing\n", "a".repeat(64)),
            "invalid artifact manifest pin",
        ),
    ] {
        let mut case = config_case("/model/sha256_manifest", json!(manifest), error);
        case.name = name.into();
        cases.push(case);
    }
    cases.push(config_case(
        "/model/external_files",
        json!(["weights.bin"]),
        "missing external artifact pin",
    ));
    cases
}

fn manifest_without(manifest: &str, file: &str) -> String {
    manifest
        .lines()
        .filter(|line| !line.ends_with(&format!("  {file}")))
        .map(|line| format!("{line}\n"))
        .collect()
}
