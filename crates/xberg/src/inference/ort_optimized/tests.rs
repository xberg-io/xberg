use super::*;
use tempfile::TempDir;

fn platform() -> Platform {
    Platform {
        runtime: "ort 1.0".to_string(),
        cpu: "avx2".to_string(),
    }
}

fn slot(root: &TempDir) -> Slot {
    Slot::new(
        root.path().join("cache"),
        &root.path().join("model.onnx"),
        &"a".repeat(64),
        "recipe v1",
        &platform(),
    )
}

/// Stages and publishes `bytes` as the optimised graph, as a successful ORT commit would.
fn save(slot: &Slot, bytes: &[u8]) -> PathBuf {
    slot.prepare_dir().unwrap();
    let staged = slot.create_staging().unwrap();
    fs::write(&staged, bytes).unwrap();
    slot.publish(&staged).unwrap();
    assert!(!staged.exists(), "publishing moves the staged file");
    slot.verified_graph().unwrap().expect("a published graph verifies")
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().mode() & 0o777
}

#[test]
fn key_binds_every_input_that_changes_the_graph() {
    let model = Path::new("/models/model.onnx");
    let key = |sha: &str, recipe: &str, platform: &Platform| {
        Slot::new(PathBuf::from("/cache"), model, sha, recipe, platform).key
    };
    let digest = "a".repeat(64);
    let base = key(&digest, "recipe v1", &platform());

    assert_eq!(base, key(&digest, "recipe v1", &platform()), "the key is deterministic");
    assert_ne!(
        base,
        key(&"b".repeat(64), "recipe v1", &platform()),
        "other model bytes"
    );
    assert_ne!(base, key(&digest, "recipe v2", &platform()), "other session options");
    let runtime = Platform {
        runtime: "ort 1.1".to_string(),
        ..platform()
    };
    assert_ne!(base, key(&digest, "recipe v1", &runtime), "another ONNX Runtime build");
    let cpu = Platform {
        cpu: "avx2,avx512f".to_string(),
        ..platform()
    };
    assert_ne!(base, key(&digest, "recipe v1", &cpu), "another CPU feature set");
}

#[test]
fn the_cache_dir_and_entries_are_owner_only() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    let graph = save(&slot, b"optimized graph");

    assert_eq!(mode(&slot.dir), 0o700);
    assert_eq!(mode(&graph), 0o600);
    assert_eq!(mode(&slot.meta_path()), 0o600);
}

#[test]
fn an_owner_readable_dir_is_restricted_to_the_owner() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    fs::create_dir_all(&slot.dir).unwrap();
    fs::set_permissions(&slot.dir, fs::Permissions::from_mode(0o755)).unwrap();

    slot.prepare_dir().unwrap();
    assert_eq!(mode(&slot.dir), 0o700);
}

#[test]
fn a_group_or_world_writable_dir_is_refused() {
    for unsafe_mode in [0o770, 0o707, 0o777] {
        let root = TempDir::new().unwrap();
        let slot = slot(&root);
        fs::create_dir_all(&slot.dir).unwrap();
        fs::set_permissions(&slot.dir, fs::Permissions::from_mode(unsafe_mode)).unwrap();

        let error = slot.prepare_dir().unwrap_err();
        assert!(
            error.contains("writable by group or others"),
            "{unsafe_mode:o}: {error}"
        );
    }
}

#[test]
fn a_symlinked_cache_dir_is_refused() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    let elsewhere = root.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::set_permissions(&elsewhere, fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &slot.dir).unwrap();

    assert_eq!(slot.prepare_dir().unwrap_err(), "not a directory");
}

#[test]
fn an_uncreatable_dir_is_unavailable_not_unsafe() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("blocked"), b"a file, not a directory").unwrap();
    let slot = Slot::new(
        root.path().join("blocked").join("cache"),
        &root.path().join("model.onnx"),
        &"a".repeat(64),
        "recipe v1",
        &platform(),
    );

    assert_eq!(slot.prepare_dir(), Ok(false));
}

#[test]
fn nothing_published_is_not_an_error() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    slot.prepare_dir().unwrap();
    assert_eq!(slot.verified_graph(), Ok(None));
}

#[test]
fn a_graph_whose_bytes_changed_is_rejected() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    let graph = save(&slot, b"optimized graph");

    fs::write(&graph, b"optimized GRAPH").unwrap();
    let error = slot.verified_graph().unwrap_err();
    assert!(error.contains("digest does not match"), "{error}");

    fs::write(&graph, b"optimized").unwrap();
    let error = slot.verified_graph().unwrap_err();
    assert!(error.contains("sidecar says"), "{error}");
}

#[test]
fn a_graph_without_its_sidecar_is_not_used() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    save(&slot, b"optimized graph");

    fs::remove_file(slot.meta_path()).unwrap();
    assert_eq!(slot.verified_graph(), Ok(None));
}

#[test]
fn a_malformed_or_foreign_sidecar_is_rejected() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    save(&slot, b"optimized graph");
    let meta = slot.meta_path();
    let valid = fs::read_to_string(&meta).unwrap();

    for (text, expected) in [
        ("garbage\n".to_string(), "malformed"),
        (valid.replace(&slot.key, &"0".repeat(32)), "another key"),
        (valid.replace("size=15", "size=0"), "implausible size"),
        (format!("{valid}extra=1\n"), "malformed"),
        ("x".repeat(MAX_META_BYTES as usize + 1), "too large"),
    ] {
        fs::write(&meta, &text).unwrap();
        let error = slot.verified_graph().unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}

#[test]
fn symlinked_or_unsafe_entries_are_rejected() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    let graph = save(&slot, b"optimized graph");

    let outside = root.path().join("outside.onnx");
    fs::write(&outside, b"optimized graph").unwrap();
    fs::remove_file(&graph).unwrap();
    std::os::unix::fs::symlink(&outside, &graph).unwrap();
    assert!(slot.verified_graph().is_err(), "a symlinked graph is not followed");

    fs::remove_file(&graph).unwrap();
    fs::write(&graph, b"optimized graph").unwrap();
    fs::set_permissions(&graph, fs::Permissions::from_mode(0o620)).unwrap();
    let error = slot.verified_graph().unwrap_err();
    assert!(error.contains("writable by group or others"), "{error}");

    fs::set_permissions(&graph, fs::Permissions::from_mode(0o600)).unwrap();
    let meta = slot.meta_path();
    let sidecar = root.path().join("outside.meta");
    fs::rename(&meta, &sidecar).unwrap();
    std::os::unix::fs::symlink(&sidecar, &meta).unwrap();
    assert!(slot.verified_graph().is_err(), "a symlinked sidecar is not followed");
}

#[test]
fn discard_removes_the_entry() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    let graph = save(&slot, b"optimized graph");

    slot.discard();
    assert!(!graph.exists());
    assert!(!slot.meta_path().exists());
    assert_eq!(slot.verified_graph(), Ok(None));
}

#[test]
fn publishing_a_new_key_removes_the_superseded_entry_for_the_same_model() {
    let root = TempDir::new().unwrap();
    let model = root.path().join("model.onnx");
    let dir = root.path().join("cache");
    let old = Slot::new(dir.clone(), &model, &"a".repeat(64), "recipe v1", &platform());
    let old_graph = save(&old, b"old graph");
    let other = Slot::new(
        dir.clone(),
        &root.path().join("other.onnx"),
        &"c".repeat(64),
        "recipe v1",
        &platform(),
    );
    let other_graph = save(&other, b"other graph");

    let new = Slot::new(dir, &model, &"b".repeat(64), "recipe v1", &platform());
    save(&new, b"new graph");

    assert!(!old_graph.exists(), "the superseded graph for this model is removed");
    assert!(!old.meta_path().exists());
    assert!(other_graph.exists(), "another model's graph is kept");
}

#[test]
fn stale_staging_files_are_swept_and_fresh_ones_kept() {
    let root = TempDir::new().unwrap();
    let slot = slot(&root);
    slot.prepare_dir().unwrap();
    let stale = slot.create_staging().unwrap();
    fs::File::options()
        .write(true)
        .open(&stale)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - ORPHAN_AGE - Duration::from_secs(60))
        .unwrap();
    let fresh = slot.create_staging().unwrap();

    save(&slot, b"optimized graph");

    assert!(!stale.exists(), "a crashed process's staging file is removed");
    assert!(fresh.exists(), "a running process's staging file is kept");
}

#[test]
fn the_cache_keeps_at_most_max_graphs_entries() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("cache");
    for index in 0..=MAX_GRAPHS {
        let model = root.path().join(format!("model-{index}.onnx"));
        let slot = Slot::new(dir.clone(), &model, &"a".repeat(64), "recipe v1", &platform());
        save(&slot, format!("graph {index}").as_bytes());
    }
    let metas = entries(&dir)
        .iter()
        .filter(|(name, _, _)| name.ends_with(".meta"))
        .count();
    assert_eq!(metas, MAX_GRAPHS);
}

#[test]
fn racing_publishers_leave_a_valid_entry() {
    let root = TempDir::new().unwrap();
    let slot = std::sync::Arc::new(slot(&root));
    slot.prepare_dir().unwrap();
    let payloads: Vec<Vec<u8>> = (0..8).map(|i| format!("graph from writer {i}").into_bytes()).collect();

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(payloads.len()));
    let writers: Vec<_> = payloads
        .iter()
        .cloned()
        .map(|payload| {
            let slot = std::sync::Arc::clone(&slot);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                let staged = slot.create_staging().unwrap();
                fs::write(&staged, &payload).unwrap();
                barrier.wait();
                slot.publish(&staged).unwrap();
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }

    let graph = slot.verified_graph().unwrap().expect("one writer's graph is published");
    assert!(payloads.contains(&fs::read(graph).unwrap()));
}

#[test]
fn sidecar_records_round_trip_and_reject_bad_digests() {
    let record = Record {
        key: "k".repeat(32),
        size: 42,
        sha256: "0123456789abcdef".repeat(4),
    };
    assert_eq!(Record::parse(&record.render()), Some(record));
    let upper = format!("{CACHE_SCHEMA}\nkey=k\nsize=1\nsha256={}\n", "A".repeat(64));
    assert_eq!(Record::parse(&upper), None);
    let short = format!("{CACHE_SCHEMA}\nkey=k\nsize=1\nsha256={}\n", "a".repeat(63));
    assert_eq!(Record::parse(&short), None);
}

/// Protobuf encoding helpers for a model built in memory.
mod tiny_model {
    fn varint(mut v: u64, out: &mut Vec<u8>) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                break;
            }
            out.push(b | 0x80);
        }
    }
    fn bytes(tag: u64, payload: &[u8]) -> Vec<u8> {
        let mut o = Vec::new();
        varint((tag << 3) | 2, &mut o);
        varint(payload.len() as u64, &mut o);
        o.extend_from_slice(payload);
        o
    }
    fn int(tag: u64, v: u64) -> Vec<u8> {
        let mut o = Vec::new();
        varint(tag << 3, &mut o);
        varint(v, &mut o);
        o
    }
    fn value_info(name: &str, width: u64) -> Vec<u8> {
        let shape = [bytes(1, &bytes(2, b"N")), bytes(1, &int(1, width))].concat();
        let tensor = [int(1, 1), bytes(2, &shape)].concat();
        [bytes(1, name.as_bytes()), bytes(2, &bytes(1, &tensor))].concat()
    }
    fn node(op: &str, inputs: &[&str], output: &str, name: &str) -> Vec<u8> {
        let mut n: Vec<u8> = inputs.iter().flat_map(|i| bytes(1, i.as_bytes())).collect();
        n.extend(bytes(2, output.as_bytes()));
        n.extend(bytes(3, name.as_bytes()));
        n.extend(bytes(4, op.as_bytes()));
        n
    }
    /// `Y = (X + X) * (X + X) + X` over `[N, 8]` floats, opset 13.
    pub(super) fn onnx() -> Vec<u8> {
        let graph = [
            bytes(1, &node("Add", &["X", "X"], "A", "add0")),
            bytes(1, &node("Mul", &["A", "A"], "B", "mul0")),
            bytes(1, &node("Add", &["B", "X"], "Y", "add1")),
            bytes(2, b"tiny"),
            bytes(11, &value_info("X", 8)),
            bytes(12, &value_info("Y", 8)),
        ]
        .concat();
        [int(1, 8), bytes(8, &int(2, 13)), bytes(7, &graph)].concat()
    }
}

fn cpu() -> crate::core::config::acceleration::AccelerationConfig {
    crate::core::config::acceleration::AccelerationConfig {
        provider: crate::core::config::acceleration::ExecutionProviderType::Cpu,
        device_id: 0,
    }
}

/// Writes `bytes` as a model and verifies it, as a model manager would before loading it.
fn pinned_model(root: &TempDir, name: &str, bytes: &[u8]) -> (PathBuf, PinnedFile) {
    use sha2::{Digest, Sha256};
    // `Platform::current` reads `ort::info()`, which needs the runtime loaded.
    crate::ort_discovery::ort_environment().unwrap();
    let path = root.path().join(name);
    fs::write(&path, bytes).unwrap();
    crate::model_download::verify_sha256(&path, &hex::encode(Sha256::digest(bytes)), name).unwrap();
    let pinned = crate::model_download::pinned_file(&path).expect("a verified model is pinned");
    (path, pinned)
}

fn run_tiny(session: &mut Session) -> Vec<u32> {
    let input: Vec<f32> = (0..3 * 8).map(|i| i as f32 / 7.0).collect();
    let tensor = ort::value::Tensor::from_array(([3usize, 8], input)).unwrap();
    let outputs = session.run(ort::inputs!["X" => tensor]).unwrap();
    let (_, data) = outputs["Y"].try_extract_tensor::<f32>().unwrap();
    data.iter().map(|value| value.to_bits()).collect()
}

#[test]
fn a_verified_model_is_saved_then_loaded_with_identical_outputs() {
    let root = TempDir::new().unwrap();
    let (model, pinned) = pinned_model(&root, "tiny.onnx", &tiny_model::onnx());
    let slot = Slot::new(
        root.path().join("cache"),
        &model,
        &pinned.sha256,
        "test",
        &Platform::current(),
    );
    let accel = cpu();
    let configure = || super::super::ort_backend::session_builder(Some(&accel), 1, true);

    let mut online_session = online(&configure, &model).unwrap();
    let (mut saved, first) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();
    let (mut loaded, second) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();

    assert_eq!(first, Outcome::Saved);
    assert_eq!(second, Outcome::Loaded);
    let expected = run_tiny(&mut online_session);
    assert_eq!(run_tiny(&mut saved), expected);
    assert_eq!(run_tiny(&mut loaded), expected);
}

#[test]
fn a_forged_entry_that_fails_to_load_is_replaced() {
    let root = TempDir::new().unwrap();
    let (model, pinned) = pinned_model(&root, "tiny.onnx", &tiny_model::onnx());
    let slot = Slot::new(
        root.path().join("cache"),
        &model,
        &pinned.sha256,
        "test",
        &Platform::current(),
    );
    let accel = cpu();
    let configure = || super::super::ort_backend::session_builder(Some(&accel), 1, true);
    commit_with_cache(&model, &pinned, &slot, &configure).unwrap();

    // Whoever can write the cache can also write a matching sidecar; the load still fails
    // safely on bytes that are not a graph.
    slot.discard();
    let forged = save(&slot, b"not an onnx graph");
    let (mut session, outcome) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();

    assert_eq!(outcome, Outcome::Saved, "the bad entry is discarded and rebuilt");
    assert!(!forged.exists());
    assert_eq!(
        run_tiny(&mut session),
        run_tiny(&mut online(&configure, &model).unwrap())
    );
    assert!(slot.verified_graph().unwrap().is_some());
}

#[test]
fn a_model_changed_after_verification_is_not_saved() {
    let root = TempDir::new().unwrap();
    let (model, pinned) = pinned_model(&root, "tiny.onnx", &tiny_model::onnx());
    let slot = Slot::new(
        root.path().join("cache"),
        &model,
        &pinned.sha256,
        "test",
        &Platform::current(),
    );
    let accel = cpu();
    let configure = || super::super::ort_backend::session_builder(Some(&accel), 1, true);

    let replacement = root.path().join("replacement.onnx");
    fs::write(&replacement, tiny_model::onnx()).unwrap();
    fs::rename(&replacement, &model).unwrap();
    let (_, outcome) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();

    assert_eq!(outcome, Outcome::Online);
    assert_eq!(slot.verified_graph(), Ok(None));
    let leftovers = entries(&slot.dir).len();
    assert_eq!(leftovers, 0, "no staging file is left behind");
}

#[test]
fn an_unverified_model_never_reaches_the_cache() {
    let root = TempDir::new().unwrap();
    let model = root.path().join("unverified.onnx");
    fs::write(&model, tiny_model::onnx()).unwrap();
    assert_eq!(crate::model_download::pinned_file(&model), None);
}

#[cfg(feature = "layout-detection")]
type Inputs = Vec<(std::borrow::Cow<'static, str>, ort::session::SessionInputValue<'static>)>;

/// Runs `session` on inputs from `make_inputs` and returns every output as raw bits.
#[cfg(feature = "layout-detection")]
fn output_bits(session: &mut Session, make_inputs: &dyn Fn(&Session) -> Inputs) -> Vec<(String, Vec<u64>)> {
    use ort::value::TensorElementType;

    let inputs = make_inputs(session);
    let outputs = session.run(inputs).unwrap();
    outputs
        .into_iter()
        .map(|(name, value)| {
            let bits: Vec<u64> = match value.dtype().tensor_type() {
                Some(TensorElementType::Float32) => value
                    .try_extract_array::<f32>()
                    .unwrap()
                    .iter()
                    .map(|v| u64::from(v.to_bits()))
                    .collect(),
                Some(TensorElementType::Int64) => value
                    .try_extract_array::<i64>()
                    .unwrap()
                    .iter()
                    .map(|v| *v as u64)
                    .collect(),
                other => panic!("unexpected output type {other:?} for {name}"),
            };
            (name.to_string(), bits)
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
fn image_tensor(height: usize, width: usize) -> ort::session::SessionInputValue<'static> {
    let pixels: Vec<f32> = (0..3 * height * width)
        .map(|i: usize| ((i * 7919) % 1000) as f32 / 1000.0)
        .collect();
    let images = ndarray::Array4::from_shape_vec((1, 3, height, width), pixels).unwrap();
    ort::value::Tensor::from_array(images).unwrap().into()
}

/// Whether model-backed coverage is mandatory: the variable CI sets for the model parity
/// suites (`inference::tract_backend::parity_required`, which only exists with `tract`).
#[cfg(feature = "layout-detection")]
fn parity_required() -> bool {
    std::env::var_os("XBERG_REQUIRE_TRACT_PARITY").is_some()
}

/// A pinned layout model from the local cache, downloaded when coverage is required.
#[cfg(feature = "layout-detection")]
fn layout_model(model_type: &str) -> Option<PathBuf> {
    let manager = crate::layout::LayoutModelManager::new(None);
    if let Some(path) = manager.verified_model_path(model_type) {
        return Some(path);
    }
    if !parity_required() {
        return None;
    }
    let downloaded = match model_type {
        "rtdetr" => manager.ensure_rtdetr_model(),
        "tatr" => manager.ensure_tatr_model(),
        other => panic!("no downloader for {other}"),
    };
    Some(downloaded.unwrap_or_else(|e| panic!("XBERG_REQUIRE_TRACT_PARITY is set but {model_type} failed: {e}")))
}

/// Saves and reloads `model_type` through the production `configure` closure of its route and
/// requires every output to match the online-optimised session bit for bit.
#[cfg(feature = "layout-detection")]
fn assert_saved_graph_is_bit_identical(
    model_type: &str,
    recipe: &str,
    configure: &dyn Fn() -> Result<SessionBuilder, ort::Error>,
    make_inputs: &dyn Fn(&Session) -> Inputs,
) -> bool {
    let Some(model) = layout_model(model_type) else {
        eprintln!("skip: no verified {model_type} model in the local cache (run `xberg cache warm`)");
        return false;
    };
    crate::ort_discovery::ort_environment().unwrap();
    // The manager pins the model when it hashes it; a build that skips the hash is pinned here.
    let pinned = crate::model_download::pinned_file(&model).unwrap_or_else(|| {
        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(fs::read(&model).unwrap()));
        crate::model_download::verify_sha256(&model, &digest, model_type).unwrap();
        crate::model_download::pinned_file(&model).expect("a verified model is pinned")
    });
    let root = TempDir::new().unwrap();
    let slot = Slot::new(
        root.path().join("cache"),
        &model,
        &pinned.sha256,
        recipe,
        &Platform::current(),
    );

    let mut online_session = online(&configure, &model).unwrap();
    let (mut saved, first) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();
    let (mut loaded, second) = commit_with_cache(&model, &pinned, &slot, &configure).unwrap();
    assert_eq!(first, Outcome::Saved, "{model_type}: the first build saves the graph");
    assert_eq!(second, Outcome::Loaded, "{model_type}: the second build loads it");

    let expected = output_bits(&mut online_session, make_inputs);
    assert!(!expected.is_empty(), "{model_type}: the model produced no outputs");
    assert_eq!(
        output_bits(&mut saved, make_inputs),
        expected,
        "{model_type}: saving changed outputs"
    );
    assert_eq!(
        output_bits(&mut loaded, make_inputs),
        expected,
        "{model_type}: the saved graph changed outputs"
    );
    true
}

/// Every model routed through the cache must produce bit-identical outputs from its saved
/// graph, or reusing the graph would change detections. RT-DETR takes the inference-seam
/// route (`OrtBackend`), TATR the bare-session layout route.
#[cfg(feature = "layout-detection")]
#[test]
fn saved_layout_graphs_reproduce_online_outputs_bit_for_bit() {
    let accel = cpu();
    let rtdetr = assert_saved_graph_is_bit_identical(
        "rtdetr",
        super::super::ort_backend::SESSION_RECIPE,
        &|| super::super::ort_backend::session_builder(Some(&accel), 1, true),
        &|session| {
            let sizes = ndarray::Array2::from_shape_vec((1, 2), vec![612_i64, 792]).unwrap();
            vec![
                (session.inputs()[0].name().to_string().into(), image_tensor(640, 640)),
                (
                    session.inputs()[1].name().to_string().into(),
                    ort::value::Tensor::from_array(sizes).unwrap().into(),
                ),
            ]
        },
    );
    let tatr = assert_saved_graph_is_bit_identical(
        "tatr",
        crate::layout::session::SESSION_RECIPE,
        &|| crate::layout::session::session_builder(Some(&accel), 1),
        &|session| vec![(session.inputs()[0].name().to_string().into(), image_tensor(800, 1000))],
    );
    assert!(
        (rtdetr && tatr) || !parity_required(),
        "XBERG_REQUIRE_TRACT_PARITY is set but a saved layout graph was not compared"
    );
}
