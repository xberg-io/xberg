use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{LayoutModel, PpDocLayoutV3Model};
use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use crate::inference::InferenceTensor;
use crate::inference::recording::{Captures, record_session, with_profile};

const MODEL_SHA256: &str = "93d1197e55f1c9cb6720275a89684e7ea61cd5830008a837d8c51b19d47926c1";
const PAGE_COUNT: usize = 2;
const TENSORS_PER_FORWARD: usize = 3;

fn required_path(name: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")));
    assert!(path.is_absolute(), "{name} must be absolute");
    path
}

fn sha256(path: &Path) -> String {
    bytes_sha256(&fs::read(path).expect("hashed artifact"))
}

fn bytes_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn take_forward(captures: &Captures) -> crate::inference::recording::ForwardCapture {
    let mut captures = captures.lock().expect("captured real forward");
    assert_eq!(captures.len(), 1, "exactly one real forward per page");
    captures.pop().expect("one real forward")
}

fn tensor_bytes(tensor: &InferenceTensor) -> (&'static str, Vec<usize>, Vec<u8>) {
    match tensor {
        InferenceTensor::F32(array) => {
            assert!(array.iter().all(|value| value.is_finite()), "finite f32 tensor");
            (
                "<f4",
                array.shape().to_vec(),
                array.iter().flat_map(|value| value.to_le_bytes()).collect(),
            )
        }
        InferenceTensor::I32(array) => (
            "<i4",
            array.shape().to_vec(),
            array.iter().flat_map(|value| value.to_le_bytes()).collect(),
        ),
        other => panic!("unexpected V3 tensor type: {other:?}"),
    }
}

fn expected_tensor(name: &str) -> (&'static str, Vec<usize>) {
    match name {
        "image" => ("<f4", vec![1, 3, 800, 800]),
        "im_shape" | "scale_factor" => ("<f4", vec![1, 2]),
        "fetch_name_0" => ("<f4", vec![]),
        "fetch_name_1" => ("<i4", vec![1]),
        "fetch_name_2" => ("<i4", vec![]),
        other => panic!("unexpected V3 tensor name: {other}"),
    }
}

fn save_tensors(output: &Path, tensors: &[(String, InferenceTensor)], expected: &[&str]) -> Vec<Value> {
    let mut names = tensors.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(names, expected, "all named tensors must be captured exactly once");
    tensors
        .iter()
        .map(|(name, tensor)| {
            let (dtype, shape, bytes) = tensor_bytes(tensor);
            let (expected_dtype, expected_shape) = expected_tensor(name);
            assert_eq!(dtype, expected_dtype);
            if !expected_shape.is_empty() {
                assert_eq!(shape, expected_shape);
            }
            let path = output.join(format!("{name}.bin"));
            fs::write(&path, &bytes).expect("raw tensor artifact");
            json!({"name": name, "dtype": dtype, "shape": shape, "file": format!("{name}.bin"),
            "bytes": bytes.len(), "sha256": bytes_sha256(&bytes)})
        })
        .collect()
}

fn capture_page(model: &mut PpDocLayoutV3Model, captures: &Captures, out: &Path, input: &Value) -> Value {
    let name = input["name"].as_str().expect("input name");
    let image_path = PathBuf::from(input["path"].as_str().expect("absolute PNG path"));
    assert_eq!(sha256(&image_path), input["sha256"].as_str().expect("image hash"));
    let image = image::load_from_memory(&fs::read(&image_path).expect("PNG bytes"))
        .expect("valid PNG")
        .to_rgb8();
    assert_eq!(json!([image.height(), image.width(), 3]), input["shape_hwc"]);
    let detections = model
        .detect_with_threshold(&image, 0.5)
        .expect("real production V3 inference");
    let forward = take_forward(captures);
    validate_outputs(&forward.outputs);
    let page = out.join(name);
    fs::create_dir(&page).expect("new page artifact directory");
    let inputs = page.join("inputs");
    let outputs = page.join("outputs");
    fs::create_dir(&inputs).expect("input tensor directory");
    fs::create_dir(&outputs).expect("output tensor directory");
    json!({"input": name, "input_sha256": sha256(&image_path),
        "inputs": save_tensors(&inputs, &forward.inputs, &["im_shape", "image", "scale_factor"]),
        "outputs": save_tensors(&outputs, &forward.outputs, &["fetch_name_0", "fetch_name_1", "fetch_name_2"]),
        "processed_detections": detections})
}

#[test]
#[ignore = "requires pinned local V3 model, two-page native parity artifacts and ORT; runs two real forwards"]
fn should_capture_two_production_v3_forwards() {
    let root = required_path("XBERG_V3_PARITY_ROOT");
    let model_path = required_path("XBERG_V3_PARITY_MODEL");
    let output = required_path("XBERG_V3_CAPTURE_OUT");
    assert!(!output.exists(), "capture output must not already exist");
    assert_artifact_hash(&model_path, MODEL_SHA256, "pinned ONNX identity");
    assert_eq!(
        sha256(&root.join("comparison-contract-v1.json")),
        "3875aff43e1a64952067a36962a5727ff38a64de9bf82e2bc109c09b442a3ef9",
        "frozen contract identity"
    );
    assert_eq!(
        sha256(&root.join("inputs.json")),
        "9f4f5403eab29a68722e5828cbd2b14fdba70304b630adae26ba56da3c5119c1",
        "frozen input manifest identity"
    );
    let inputs: Vec<Value> = serde_json::from_slice(&fs::read(root.join("inputs.json")).expect("input manifest"))
        .expect("valid input manifest");
    validate_inputs(&inputs);
    fs::create_dir(&output).expect("new capture output");
    let producer = preserve_producer(&output);
    let acceleration = AccelerationConfig {
        provider: ExecutionProviderType::Cpu,
        ..Default::default()
    };
    let model = with_profile(&output.join("ort-profile"), || {
        PpDocLayoutV3Model::from_file_with_thread_budget(
            model_path.to_str().expect("UTF-8 model path"),
            Some(&acceleration),
            1,
        )
    })
    .expect("real default ORT session");
    let runtime = runtime_witness();
    let (session, captures) = record_session(model.session);
    let mut model = PpDocLayoutV3Model {
        session,
        input_names: model.input_names,
    };
    let rows = inputs
        .iter()
        .map(|input| capture_page(&mut model, &captures, &output, input))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), PAGE_COUNT);
    assert!(captures.lock().expect("capture mutex").is_empty());
    drop(model);
    let profile = profile_witness(&output);
    let manifest = json!({"schema_version": 1, "qualification_complete": false,
        "backend": "production default ORT", "cpu_threads": 1, "runtime": runtime, "profile": profile,
        "model_sha256": MODEL_SHA256, "source_inputs_sha256": sha256(&root.join("inputs.json")),
        "comparison_contract_sha256": sha256(&root.join("comparison-contract-v1.json")),
        "executable_sha256": producer["executable"]["sha256"],
        "executable_file": producer["executable"]["file"],
        "producer_manifest_file": "producer/manifest.json",
        "sources": producer["sources"], "real_forward_passes": rows.len(),
        "input_tensors": rows.len() * TENSORS_PER_FORWARD,
        "output_tensors": rows.len() * TENSORS_PER_FORWARD, "rows": rows});
    fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("capture JSON"),
    )
    .expect("capture manifest");
}

fn validate_inputs(inputs: &[Value]) {
    assert_eq!(
        inputs.len(),
        PAGE_COUNT,
        "exactly two input pages before model construction"
    );
    assert_eq!(
        inputs
            .iter()
            .map(|row| row["name"].as_str().expect("input name"))
            .collect::<Vec<_>>(),
        ["document-1", "document-2"]
    );
    for input in inputs {
        let path = PathBuf::from(input["path"].as_str().expect("PNG path"));
        assert!(path.is_absolute(), "absolute PNG path");
        assert_artifact_hash(
            &path,
            input["sha256"].as_str().expect("PNG hash"),
            "pinned PNG identity",
        );
        let bytes = fs::read(&path).expect("PNG bytes");
        assert_eq!(
            image::guess_format(&bytes).expect("image format"),
            image::ImageFormat::Png
        );
        let image = image::load_from_memory(&bytes).expect("valid PNG");
        assert_eq!(image.color(), image::ColorType::Rgb8);
        assert_eq!(json!([image.height(), image.width(), 3]), input["shape_hwc"]);
    }
}

fn validate_outputs(outputs: &[(String, InferenceTensor)]) {
    let get = |name| {
        &outputs
            .iter()
            .find(|(key, _)| key == name)
            .expect("required raw output")
            .1
    };
    let InferenceTensor::F32(boxes) = get("fetch_name_0") else {
        panic!("f32 boxes")
    };
    let InferenceTensor::I32(counts) = get("fetch_name_1") else {
        panic!("i32 counts")
    };
    let InferenceTensor::I32(masks) = get("fetch_name_2") else {
        panic!("i32 masks")
    };
    assert_eq!(counts.shape(), &[1]);
    let count = usize::try_from(counts[[0]]).expect("nonnegative output count");
    assert!(count > 0, "known development page must have detections");
    assert_eq!(boxes.shape(), &[count, 7], "raw row count must match bbox_num");
    assert_eq!(masks.shape(), &[count, 200, 200], "raw mask rows must match bbox_num");
    assert!(boxes.iter().all(|value| value.is_finite()));
}

fn runtime_witness() -> Value {
    let paths = mapped_runtime_paths();
    assert_eq!(paths.len(), 1, "exactly one mapped ONNX Runtime library");
    let path = &paths[0];
    json!({"mapped_library_path": path, "library_sha256": sha256(path), "build_info": ort::info()})
}

fn mapped_runtime_paths() -> Vec<PathBuf> {
    let text = if cfg!(target_os = "linux") {
        fs::read_to_string("/proc/self/maps").expect("own process memory map")
    } else {
        let result = std::process::Command::new("/usr/sbin/lsof")
            .args(["-a", "-p", &std::process::id().to_string(), "-Fn"])
            .output()
            .expect("own process library inventory");
        assert!(result.status.success(), "library inventory must succeed");
        String::from_utf8(result.stdout).expect("library paths")
    };
    let mut paths = text
        .lines()
        .filter_map(|line| {
            let path = if cfg!(target_os = "linux") {
                line.split_whitespace().last()?
            } else {
                line.strip_prefix('n')?
            };
            let name = Path::new(path).file_name()?.to_str()?;
            (name.starts_with("libonnxruntime.") || name.starts_with("libonnxruntime.so"))
                .then(|| fs::canonicalize(path).expect("mapped runtime exists"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn profile_witness(output: &Path) -> Value {
    let paths = fs::read_dir(output)
        .expect("profile directory")
        .map(|entry| entry.expect("profile entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("ort-profile") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1, "one finalized ORT profile");
    let events: Vec<Value> =
        serde_json::from_slice(&fs::read(&paths[0]).expect("profile bytes")).expect("ORT profile JSON");
    let providers = events
        .iter()
        .filter_map(|event| event["args"]["provider"].as_str())
        .collect::<Vec<_>>();
    assert!(!providers.is_empty(), "actual kernel provider events required");
    assert!(
        providers.iter().all(|provider| *provider == "CPUExecutionProvider"),
        "all executed kernels must use CPU"
    );
    let forwards = events.iter().filter(|event| event["name"] == "model_run").count();
    assert_eq!(forwards, PAGE_COUNT, "profile must witness exactly two model runs");
    json!({"file": paths[0].file_name().and_then(|name| name.to_str()).expect("UTF-8 profile file"), "sha256": sha256(&paths[0]),
        "real_model_runs": forwards, "cpu_kernel_events": providers.len(), "effective_provider": "CPUExecutionProvider"})
}

fn source_witness(output: &Path) -> Vec<Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        "Cargo.toml",
        "../../Cargo.toml",
        "../../Cargo.lock",
        "../../rust-toolchain.toml",
        "src/inference/mod.rs",
        "src/inference/backend.rs",
        "src/inference/tensor.rs",
        "src/inference/recording.rs",
        "src/inference/ort_backend.rs",
        "src/ort_discovery.rs",
        "src/core/config/acceleration.rs",
        "src/layout/models/mod.rs",
        "src/layout/types.rs",
        "src/layout/error.rs",
        "src/layout/models/pp_doclayout_v3.rs",
        "src/layout/models/pp_doclayout_v3/production_parity.rs",
        "src/layout/models/pp_doclayout_v3/preprocessing.rs",
    ]
    .iter()
    .enumerate()
    .map(|(index, path)| {
        let name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .expect("source filename");
        let file = format!("producer/sources/{index:02}-{name}");
        let mut artifact = preserve_artifact(&root.join(path), output, &file);
        artifact["path"] = json!(path);
        artifact
    })
    .collect()
}

fn preserve_producer(output: &Path) -> Value {
    fs::create_dir(output.join("producer")).expect("new producer artifact directory");
    fs::create_dir(output.join("producer/sources")).expect("new source artifact directory");
    let executable = preserve_artifact(
        &std::env::current_exe().expect("test executable"),
        output,
        "producer/executable",
    );
    let sources = source_witness(output);
    assert_eq!(sources.len(), 17, "all producer source artifacts preserved");
    let producer = json!({"executable": executable, "sources": sources});
    fs::write(
        output.join("producer/manifest.json"),
        serde_json::to_vec_pretty(&producer).expect("producer JSON"),
    )
    .expect("producer manifest before model construction");
    producer
}

fn preserve_artifact(source: &Path, output: &Path, relative: &str) -> Value {
    let relative_path = Path::new(relative);
    assert!(
        relative_path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_))),
        "producer destination must contain only normal relative components"
    );
    assert!(!relative.is_empty(), "producer destination must not be empty");
    let original_hash = sha256(source);
    let destination = output.join(relative_path);
    let mut original = fs::File::open(source).expect("original producer artifact");
    let mut copied = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .expect("new producer artifact destination");
    let bytes = std::io::copy(&mut original, &mut copied).expect("complete producer artifact copy");
    copied.sync_all().expect("persisted producer artifact copy");
    assert_artifact_hash(&destination, &original_hash, "copied producer artifact identity");
    assert_artifact_hash(source, &original_hash, "producer artifact unchanged during copy");
    let mut permissions = original.metadata().expect("producer permissions").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&destination, permissions).expect("read-only preserved producer artifact");
    json!({"path": source, "file": relative, "sha256": original_hash, "bytes": bytes})
}

#[test]
fn should_serialize_tensor_artifact_names_as_portable_strings() {
    let output = tempfile::tempdir().expect("tensor artifact directory");
    let tensor = InferenceTensor::F32(ndarray::arr2(&[[800.0f32, 800.0]]).into_dyn());
    let rows = save_tensors(output.path(), &[("im_shape".into(), tensor)], &["im_shape"]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["file"], "im_shape.bin");
    assert_eq!(
        fs::read(output.path().join("im_shape.bin")).expect("tensor bytes"),
        [800.0f32.to_le_bytes(), 800.0f32.to_le_bytes()].concat()
    );
}

#[test]
#[should_panic(expected = "exactly two input pages before model construction")]
fn should_reject_zero_input_pages_before_model_construction() {
    validate_inputs(&[]);
}

#[test]
#[should_panic(expected = "exactly one real forward per page")]
fn should_reject_zero_recorded_forwards() {
    let captures = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let _ = take_forward(&captures);
}

fn assert_artifact_hash(path: &Path, expected: &str, reason: &str) {
    assert_eq!(sha256(path), expected, "{reason}");
}

#[test]
#[should_panic(expected = "pinned ONNX identity")]
fn should_reject_wrong_model_hash_before_model_construction() {
    let artifact = tempfile::NamedTempFile::new().expect("model artifact");
    fs::write(artifact.path(), b"wrong model").expect("changed model bytes");
    assert_artifact_hash(artifact.path(), MODEL_SHA256, "pinned ONNX identity");
}

#[test]
#[should_panic(expected = "pinned PNG identity")]
fn should_reject_changed_png_hash_before_model_construction() {
    let artifact = tempfile::NamedTempFile::new().expect("PNG artifact");
    fs::write(artifact.path(), b"changed PNG").expect("changed PNG bytes");
    assert_artifact_hash(
        artifact.path(),
        "c6ebaf5beb3f43640f8834ec05f20e952af6f7042761bf819b183adf4d1d6add",
        "pinned PNG identity",
    );
}

#[test]
fn should_preserve_producer_bytes_after_original_changes() {
    let root = tempfile::tempdir().expect("producer copy directory");
    let source = root.path().join("original.rs");
    fs::write(&source, b"original source").expect("original bytes");
    let artifact = preserve_artifact(&source, root.path(), "copy.rs");
    fs::write(&source, b"later source").expect("changed original bytes");
    assert_eq!(artifact["file"], "copy.rs");
    assert_eq!(artifact["sha256"], bytes_sha256(b"original source"));
    assert_eq!(
        fs::read(root.path().join("copy.rs")).expect("copied bytes"),
        b"original source"
    );
    assert!(
        fs::metadata(root.path().join("copy.rs"))
            .expect("copy metadata")
            .permissions()
            .readonly()
    );
}

#[test]
#[should_panic(expected = "copied producer artifact identity")]
fn should_reject_changed_preserved_producer_bytes() {
    let artifact = tempfile::NamedTempFile::new().expect("copied producer artifact");
    fs::write(artifact.path(), b"changed producer bytes").expect("changed copy");
    assert_artifact_hash(
        artifact.path(),
        &bytes_sha256(b"original source"),
        "copied producer artifact identity",
    );
}

#[test]
#[should_panic(expected = "producer destination must contain only normal relative components")]
fn should_reject_traversal_in_producer_destination() {
    let root = tempfile::tempdir().expect("producer copy directory");
    let _ = preserve_artifact(Path::new("unused"), root.path(), "../escaped");
}
