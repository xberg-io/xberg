use super::*;

#[test]
fn old_framework_provenance_deserializes_without_thread_budget() {
    let provenance: FrameworkProvenance = serde_json::from_value(serde_json::json!({
        "name": "xberg-markdown-baseline-batch",
        "version": "1.0.0",
        "executable": null,
        "models": [],
        "batch_capability": null,
        "requested_workers": 4,
        "effective_workers": null,
        "worker_semantics": "legacy",
        "effective_warmup_iterations": 0,
        "eligible_documents": 4,
        "batch_partitions": 1
    }))
    .unwrap();

    assert_eq!(provenance.configured_thread_budget, None);
    assert_eq!(provenance.tessdata_model, None);
}

#[test]
fn framework_provenance_serializes_best_tessdata_model_and_omits_the_default() {
    let mut provenance: FrameworkProvenance = serde_json::from_value(serde_json::json!({
        "name": "xberg-markdown-baseline-batch",
        "version": "1.0.0",
        "executable": null,
        "models": [],
        "batch_capability": null,
        "requested_workers": 4,
        "effective_workers": null,
        "worker_semantics": "legacy",
        "effective_warmup_iterations": 0,
        "eligible_documents": 4,
        "batch_partitions": 1
    }))
    .unwrap();

    assert!(
        serde_json::to_value(&provenance)
            .unwrap()
            .get("tessdata_model")
            .is_none()
    );

    provenance.tessdata_model = Some(xberg::TessdataModel::Best);
    let value = serde_json::to_value(&provenance).unwrap();
    assert_eq!(value["tessdata_model"], "best");
    let restored: FrameworkProvenance = serde_json::from_value(value).unwrap();
    assert_eq!(restored.tessdata_model, Some(xberg::TessdataModel::Best));
}

#[test]
fn xberg_provenance_records_only_a_non_default_tessdata_model() {
    use crate::adapters::subprocess::SubprocessAdapter;

    let best = SubprocessAdapter::new("xberg-test", "echo", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Best);
    let fast = SubprocessAdapter::new("xberg-test", "echo", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Fast);
    let other = SubprocessAdapter::new("docling", "python", vec![], vec![], vec!["pdf".to_string()])
        .with_tessdata_model(xberg::TessdataModel::Best);

    assert_eq!(best.configured_tessdata_model(), Some(xberg::TessdataModel::Best));
    assert_eq!(fast.configured_tessdata_model(), None);
    assert_eq!(other.configured_tessdata_model(), None);
}

#[test]
fn framework_provenance_serializes_injected_xberg_env_and_omits_it_when_none() {
    let mut provenance: FrameworkProvenance = serde_json::from_value(serde_json::json!({
        "name": "xberg-markdown-baseline",
        "version": "1.0.0",
        "executable": null,
        "models": [],
        "batch_capability": null,
        "requested_workers": 1,
        "effective_workers": 1,
        "worker_semantics": "legacy",
        "effective_warmup_iterations": 1,
        "eligible_documents": 4,
        "batch_partitions": null
    }))
    .unwrap();

    assert_eq!(provenance.xberg_env, None);
    assert!(serde_json::to_value(&provenance).unwrap().get("xberg_env").is_none());

    provenance.xberg_env = Some(BTreeMap::from([("MallocMaxMagazines".to_string(), "1".to_string())]));
    let value = serde_json::to_value(&provenance).unwrap();
    assert_eq!(value["xberg_env"], serde_json::json!({ "MallocMaxMagazines": "1" }));
    let restored: FrameworkProvenance = serde_json::from_value(value).unwrap();
    assert_eq!(restored.xberg_env, provenance.xberg_env);
}

/// Only a process the harness starts for xberg can carry the launcher environment: a competitor
/// records none, and the in-process native adapter cannot get it because libmalloc reads it at
/// process start. ~keep
#[test]
fn provenance_records_injected_xberg_env_only_for_xberg_processes() {
    use crate::adapters::NativeAdapter;
    use crate::adapters::subprocess::SubprocessAdapter;

    let xberg = SubprocessAdapter::new("xberg-test", "echo", vec![], vec![], vec!["pdf".to_string()])
        .with_launcher_env(vec![("MallocMaxMagazines".to_string(), "1".to_string())]);
    let competitor = SubprocessAdapter::new("docling", "python3", vec![], vec![], vec!["pdf".to_string()]);
    let native = NativeAdapter::with_config(xberg::ExtractionConfig::default());

    assert_eq!(
        injected_xberg_env(&xberg),
        Some(BTreeMap::from([("MallocMaxMagazines".to_string(), "1".to_string())]))
    );
    assert_eq!(injected_xberg_env(&competitor), None);
    assert_eq!(injected_xberg_env(&native), None);
}

fn run_provenance_without_tessdata() -> RunProvenance {
    RunProvenance {
        schema_version: PROVENANCE_SCHEMA_VERSION,
        harness_version: "test".to_string(),
        repository: RepositoryProvenance {
            commit: None,
            dirty: None,
        },
        corpus: CorpusProvenance {
            cohort: None,
            cohort_manifest_blake3: None,
            ordered_fixtures: Vec::new(),
        },
        frameworks: Vec::new(),
        timing: TimingProvenance {
            mode: BenchmarkMode::SingleFile,
            warmup_iterations: 0,
            benchmark_iterations: 1,
            timeout_ms: 1000,
            output_format: OutputFormat::Markdown,
        },
        fixed_batch_size: None,
        coverage: None,
        tessdata: None,
    }
}

#[test]
fn run_provenance_without_tessdata_omits_the_field_and_still_loads() {
    let value = serde_json::to_value(run_provenance_without_tessdata()).unwrap();
    assert!(value.get("tessdata").is_none(), "{value}");

    let restored: RunProvenance = serde_json::from_value(value).unwrap();
    assert_eq!(restored.tessdata, None);
    assert_eq!(
        restored.schema_version, 3,
        "an optional field does not change the schema version"
    );
}

#[test]
fn run_provenance_round_trips_resolved_tessdata_packs() {
    let tessdata = TessdataProvenance {
        requested_model: xberg::TessdataModel::Best,
        requests: vec![
            TessdataRequestProvenance {
                languages: "deu+eng".to_string(),
                directory: Some("<cache>/tessdata/best".to_string()),
                packs: vec![
                    TessdataPackProvenance {
                        language: "deu".to_string(),
                        bytes: 8,
                        sha256: "a".repeat(64),
                    },
                    TessdataPackProvenance {
                        language: "eng".to_string(),
                        bytes: 9,
                        sha256: "b".repeat(64),
                    },
                ],
            },
            TessdataRequestProvenance {
                languages: "fra".to_string(),
                directory: None,
                packs: Vec::new(),
            },
        ],
    };
    let mut provenance = run_provenance_without_tessdata();
    provenance.tessdata = Some(tessdata.clone());

    let value = serde_json::to_value(&provenance).unwrap();
    assert_eq!(
        value["tessdata"],
        serde_json::json!({
            "requested_model": "best",
            "requests": [
                {
                    "languages": "deu+eng",
                    "directory": "<cache>/tessdata/best",
                    "packs": [
                        { "language": "deu", "bytes": 8, "sha256": "a".repeat(64) },
                        { "language": "eng", "bytes": 9, "sha256": "b".repeat(64) }
                    ]
                },
                { "languages": "fra", "directory": null, "packs": [] }
            ]
        })
    );
    let restored: RunProvenance = serde_json::from_value(value).unwrap();
    assert_eq!(restored.tessdata, Some(tessdata));
}
