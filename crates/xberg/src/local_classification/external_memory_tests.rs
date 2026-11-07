const CHILD_PROBE: &str = "XBERG_NLI_EXTERNAL_MEMORY_PROBE";
const WARM_FILE: &str = "undeclared-external.data";

fn memory_session(graph: &[u8], external: Option<(&str, Vec<u8>)>) -> ort::Result<ort::session::Session> {
    let mut builder = ort::session::Session::builder(crate::ort_discovery::ort_environment()?)?
        .with_no_environment_execution_providers()
        .map_err(|error| ort::Error::new(error.message()))?
        .with_intra_threads(1)
        .map_err(|error| ort::Error::new(error.message()))?
        .with_inter_threads(1)
        .map_err(|error| ort::Error::new(error.message()))?;
    if let Some((name, bytes)) = external {
        builder = builder
            .with_external_initializer_file_in_memory(name, bytes.into())
            .map_err(|error| ort::Error::new(error.message()))?;
    }
    builder.commit_from_memory(graph)
}

fn assert_external_rejected(location: &str) {
    assert!(
        std::path::Path::new(location).is_file(),
        "probe must have readable external bytes"
    );
    let graph = super::toy_model::external_token_sum_nli_model_bytes(location);
    let error = memory_session(&graph, None).expect_err("unbound external data must not load from the filesystem");
    let message = error.to_string().to_ascii_lowercase();
    assert!(
        message.contains("external") || message.contains("model_path") || message.contains("absolute"),
        "failure must identify external-data resolution, not a runtime/schema failure: {error}",
    );
}

fn relative_probe(test_name: &str, location: &str) {
    if std::env::var(CHILD_PROBE).ok().as_deref() == Some(test_name) {
        assert_external_rejected(location);
        return;
    }
    let directory = tempfile::Builder::new()
        .prefix("xberg-nli-memory-probe-")
        .tempdir()
        .unwrap();
    let working = directory.path().join("working");
    std::fs::create_dir(&working).unwrap();
    std::fs::write(directory.path().join(WARM_FILE), 0.1f32.to_le_bytes()).unwrap();
    std::fs::write(working.join(WARM_FILE), 0.1f32.to_le_bytes()).unwrap();
    let full_name = format!("local_classification::external_memory_tests::{test_name}");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &full_name, "--nocapture"])
        .env(CHILD_PROBE, test_name)
        .current_dir(&working)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "isolated external-data probe failed: {stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("1 passed; 0 failed; 0 ignored"),
        "child must collect exactly its probe: {stdout}"
    );
}

#[test]
fn memory_binding_executes_the_external_tensor_without_a_model_file() {
    let graph = super::toy_model::external_token_sum_nli_model_bytes("weights.data");
    let mut session = memory_session(&graph, Some(("weights.data", 0.1f32.to_le_bytes().to_vec()))).unwrap();
    let ids = ort::value::Tensor::from_array(([1usize, 6], vec![2i64, 4, 3, 6, 7, 3])).unwrap();
    let mask = ort::value::Tensor::from_array(([1usize, 6], vec![1i64; 6])).unwrap();
    let types = ort::value::Tensor::from_array(([1usize, 6], vec![0i64, 0, 0, 1, 1, 1])).unwrap();
    let output = session
        .run(ort::inputs!["input_ids" => ids, "attention_mask" => mask, "token_type_ids" => types])
        .unwrap();
    let (shape, values) = output["logits"].try_extract_tensor::<f32>().unwrap();
    assert_eq!(&shape[..], &[1, 3]);
    assert_eq!(values, &[0.0, 0.0, 2.5]);
}

#[test]
fn memory_loading_rejects_an_undeclared_absolute_external_path() {
    let directory = tempfile::tempdir().unwrap();
    let weights = directory.path().join(WARM_FILE);
    std::fs::write(&weights, 0.1f32.to_le_bytes()).unwrap();
    assert!(weights.is_absolute());
    assert_external_rejected(weights.to_str().unwrap());
}

#[test]
fn memory_loading_rejects_an_undeclared_parent_traversal_to_warm_weights() {
    relative_probe(
        "memory_loading_rejects_an_undeclared_parent_traversal_to_warm_weights",
        "../undeclared-external.data",
    );
}

#[test]
fn memory_loading_rejects_undeclared_weights_in_the_working_directory() {
    relative_probe(
        "memory_loading_rejects_undeclared_weights_in_the_working_directory",
        WARM_FILE,
    );
}
