//! Reuse of ONNX Runtime's optimised graph across processes for CPU-only sessions.
//!
//! Building a session with `GraphOptimizationLevel::All` re-runs every graph transformer on each
//! cold start. ONNX Runtime can serialise the graph it produced (`SetOptimizedModelFilePath`), and
//! loading that file with optimisations disabled yields the same graph without the rewrite work.
//! The first CPU-only session for a model saves the graph here; later processes load it.
//!
//! The level-3 layout transforms (NCHWc) bake the host CPU's vector width into the saved graph, so
//! ONNX Runtime documents an `All`-level saved model as valid only on the hardware and runtime
//! that produced it. The cache key therefore covers the ONNX Runtime build, the target, the CPU
//! features that select MLAS kernels, and the source model's identity. Sessions with a GPU or
//! other execution provider never use the cache: their partitioning is not part of the key and
//! compiled EP nodes cannot be serialised.
//!
//! Every failure falls back to the normal online-optimised build: a saved graph that fails to load
//! is deleted and rebuilt. A saved graph is trusted only while a sidecar written after it was
//! published still matches its size, device, inode and change times.

use std::fs;
use std::path::{Path, PathBuf};

use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};

const CACHE_FORMAT: &str = "xberg-ort-optimized v1";
const MAX_META_BYTES: u64 = 4096;
/// Set to `0`, `false` or `off` to always optimise online, e.g. for an A/B comparison.
const DISABLE_ENV: &str = "XBERG_ORT_OPTIMIZED_CACHE";

static STAGING_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Build a session for `model`, reusing a saved optimised graph when `cpu_only` allows it.
///
/// `configure` returns a builder with everything but the optimisation level set (threads,
/// execution providers); it may be called more than once.
pub(crate) fn commit_from_file(
    model: &Path,
    cpu_only: bool,
    configure: impl Fn() -> Result<SessionBuilder, ort::Error>,
) -> Result<Session, ort::Error> {
    let cache = if cpu_only && cache_enabled() {
        OptimizedModelCache::for_model(crate::cache_dir::resolve_cache_dir("ort-optimized"), model)
    } else {
        None
    };
    let Some(cache) = cache else {
        return with_level(configure()?, GraphOptimizationLevel::All)?.commit_from_file(model);
    };

    if let Some(saved) = cache.saved_model() {
        match with_level(configure()?, GraphOptimizationLevel::Disable).and_then(|mut b| b.commit_from_file(&saved)) {
            Ok(session) => {
                tracing::debug!(model = %model.display(), saved = %saved.display(), "ORT session: loaded saved optimized graph");
                return Ok(session);
            }
            Err(error) => {
                tracing::warn!(saved = %saved.display(), %error, "ORT session: saved optimized graph failed to load; rebuilding");
                cache.discard();
            }
        }
    }

    // A model changed within the last tick could change again under the same fingerprint, so a
    // graph saved now might later be keyed to different weights ("racy git"). Skip saving it.
    let staged = if changed_recently(model) {
        None
    } else {
        cache.staging_path()
    };
    let Some(staged) = staged else {
        return with_level(configure()?, GraphOptimizationLevel::All)?.commit_from_file(model);
    };
    let saving = with_level(configure()?, GraphOptimizationLevel::All)
        .and_then(|builder| {
            builder
                .with_optimized_model_path(&staged)
                .map_err(|e| ort::Error::new(e.message()))
        })
        .and_then(|mut builder| builder.commit_from_file(model));
    match saving {
        Ok(session) => {
            cache.publish(&staged);
            Ok(session)
        }
        Err(error) => {
            let _ = fs::remove_file(&staged);
            tracing::debug!(model = %model.display(), %error, "ORT session: build with graph saving failed; building without it");
            with_level(configure()?, GraphOptimizationLevel::All)?.commit_from_file(model)
        }
    }
}

fn with_level(builder: SessionBuilder, level: GraphOptimizationLevel) -> Result<SessionBuilder, ort::Error> {
    builder
        .with_optimization_level(level)
        .map_err(|e| ort::Error::new(e.message()))
}

fn cache_enabled() -> bool {
    !std::env::var(DISABLE_ENV)
        .is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "false" | "off" | "no"))
}

const RACY_WINDOW: std::time::Duration = std::time::Duration::from_secs(2);

fn changed_recently(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return true;
    };
    #[cfg(unix)]
    let ctime = {
        use std::os::unix::fs::MetadataExt;
        u64::try_from(metadata.ctime()).ok().and_then(|secs| {
            let nanos = u32::try_from(metadata.ctime_nsec()).ok()?;
            std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::new(secs, nanos))
        })
    };
    #[cfg(not(unix))]
    let ctime: Option<std::time::SystemTime> = None;
    let Some(last_change) = metadata.modified().ok().into_iter().chain(ctime).max() else {
        return true;
    };
    !std::time::SystemTime::now()
        .duration_since(last_change)
        .is_ok_and(|age| age >= RACY_WINDOW)
}

/// The CPU features that change which MLAS kernels and NCHWc block size ONNX Runtime selects.
fn cpu_features() -> String {
    #[cfg_attr(
        not(any(target_arch = "x86_64", target_arch = "aarch64")),
        allow(unused_mut, reason = "no feature probes on this architecture")
    )]
    let mut features: Vec<&str> = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        macro_rules! detect {
            ($($feature:tt),*) => {$(
                if std::arch::is_x86_feature_detected!($feature) {
                    features.push($feature);
                }
            )*};
        }
        detect!(
            "sse4.1",
            "avx",
            "avx2",
            "fma",
            "f16c",
            "avx512f",
            "avx512bw",
            "avx512dq",
            "avx512vl",
            "avx512vnni"
        );
    }
    #[cfg(target_arch = "aarch64")]
    {
        macro_rules! detect {
            ($($feature:tt),*) => {$(
                if std::arch::is_aarch64_feature_detected!($feature) {
                    features.push($feature);
                }
            )*};
        }
        detect!("neon", "dotprod", "fp16", "i8mm", "bf16", "sve", "sve2");
    }
    features.join(",")
}

/// Identity and change times of a file; any write, rename over or replacement changes it.
#[cfg(unix)]
fn file_fingerprint(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then(|| {
        format!(
            "size={}\ndev={}\nino={}\nmtime={}.{:09}\nctime={}.{:09}\n",
            metadata.size(),
            metadata.dev(),
            metadata.ino(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        )
    })
}

#[cfg(not(unix))]
fn file_fingerprint(_path: &Path) -> Option<String> {
    None
}

fn sha256_hex(parts: &[&[u8]]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hex::encode(hasher.finalize())
}

/// One model's slot in the saved-graph cache.
#[derive(Debug)]
struct OptimizedModelCache {
    dir: PathBuf,
    /// Shared by every key for the same source path, so a stale entry can be found and removed.
    prefix: String,
    key: String,
}

impl OptimizedModelCache {
    fn for_model(dir: PathBuf, model: &Path) -> Option<Self> {
        Self::with_environment(dir, model, ort::info(), &cpu_features())
    }

    fn with_environment(dir: PathBuf, model: &Path, runtime: &str, cpu: &str) -> Option<Self> {
        let model_fingerprint = file_fingerprint(model)?;
        let model_path = model.as_os_str().as_encoded_bytes();
        let prefix = sha256_hex(&[model_path])[..16].to_string();
        let key = sha256_hex(&[
            CACHE_FORMAT.as_bytes(),
            runtime.as_bytes(),
            std::env::consts::ARCH.as_bytes(),
            std::env::consts::OS.as_bytes(),
            cpu.as_bytes(),
            b"optimization=all;provider=cpu",
            model_path,
            model_fingerprint.as_bytes(),
        ])[..32]
            .to_string();
        Some(Self { dir, prefix, key })
    }

    fn saved_path(&self) -> PathBuf {
        self.dir.join(format!("{}-{}.onnx", self.prefix, self.key))
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join(format!("{}-{}.meta", self.prefix, self.key))
    }

    fn meta_record(&self, saved_fingerprint: &str) -> String {
        format!("{CACHE_FORMAT}\nkey={}\n{saved_fingerprint}", self.key)
    }

    /// The saved graph, if one was published for this key and has not changed since.
    fn saved_model(&self) -> Option<PathBuf> {
        let saved = self.saved_path();
        let meta = read_small(&self.meta_path())?;
        let fingerprint = file_fingerprint(&saved)?;
        (meta == self.meta_record(&fingerprint)).then_some(saved)
    }

    /// A unique path for ONNX Runtime to write the graph to, or `None` if the cache directory
    /// cannot be created.
    fn staging_path(&self) -> Option<PathBuf> {
        if let Err(error) = fs::create_dir_all(&self.dir) {
            tracing::debug!(dir = %self.dir.display(), %error, "ORT optimized graph cache unavailable");
            return None;
        }
        Some(self.dir.join(format!(
            ".{}-{}.{}.{}.tmp",
            self.prefix,
            self.key,
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )))
    }

    /// Move a complete staged graph into place, record its fingerprint, and drop entries for
    /// the same source model under other keys.
    fn publish(&self, staged: &Path) {
        let saved = self.saved_path();
        let result = fs::rename(staged, &saved).and_then(|()| {
            let fingerprint = file_fingerprint(&saved)
                .ok_or_else(|| std::io::Error::other("saved optimized graph has no fingerprint"))?;
            let meta_tmp = staged.with_extension("meta.tmp");
            fs::write(&meta_tmp, self.meta_record(&fingerprint))
                .and_then(|()| fs::rename(&meta_tmp, self.meta_path()))
                .inspect_err(|_| {
                    let _ = fs::remove_file(&meta_tmp);
                })
        });
        match result {
            Ok(()) => {
                tracing::debug!(saved = %saved.display(), "ORT session: saved optimized graph");
                self.remove_stale_entries();
            }
            Err(error) => {
                let _ = fs::remove_file(staged);
                tracing::debug!(saved = %saved.display(), %error, "ORT session: could not save optimized graph");
            }
        }
    }

    fn remove_stale_entries(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let own = format!("{}-{}.", self.prefix, self.key);
        let same_model = format!("{}-", self.prefix);
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with(&same_model) && !name.starts_with(&own) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    fn discard(&self) {
        let _ = fs::remove_file(self.meta_path());
        let _ = fs::remove_file(self.saved_path());
    }
}

fn read_small(path: &Path) -> Option<String> {
    use std::io::Read;

    let file = fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_META_BYTES {
        return None;
    }
    let mut contents = String::new();
    file.take(MAX_META_BYTES).read_to_string(&mut contents).ok()?;
    Some(contents)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn model(root: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.path().join(name);
        write_at(&path, bytes, 1_000_000);
        path
    }

    /// Writes `bytes` with a fixed mtime so two writes never share a timestamp tick.
    fn write_at(path: &Path, bytes: &[u8], mtime_secs: u64) {
        fs::write(path, bytes).unwrap();
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(mtime_secs))
            .unwrap();
    }

    fn cache(root: &TempDir, model: &Path) -> OptimizedModelCache {
        OptimizedModelCache::with_environment(root.path().join("cache"), model, "ort 1.0", "avx2").unwrap()
    }

    /// Stages and publishes `bytes` as the optimised graph, as a successful ORT commit would.
    fn save(cache: &OptimizedModelCache, bytes: &[u8]) -> PathBuf {
        let staged = cache.staging_path().unwrap();
        fs::write(&staged, bytes).unwrap();
        cache.publish(&staged);
        assert!(!staged.exists(), "publishing moves the staged file");
        cache.saved_path()
    }

    #[test]
    fn key_covers_runtime_cpu_and_model_identity() {
        let root = TempDir::new().unwrap();
        let a = model(&root, "a.onnx", b"model a");
        let b = model(&root, "b.onnx", b"model b");
        let dir = root.path().join("cache");
        let key = |model: &Path, runtime: &str, cpu: &str| {
            OptimizedModelCache::with_environment(dir.clone(), model, runtime, cpu)
                .unwrap()
                .key
        };

        let base = key(&a, "ort 1.0", "avx2");
        assert_eq!(base, key(&a, "ort 1.0", "avx2"), "the key is deterministic");
        assert_ne!(base, key(&b, "ort 1.0", "avx2"), "a different model");
        assert_ne!(base, key(&a, "ort 1.1", "avx2"), "a different ONNX Runtime build");
        assert_ne!(base, key(&a, "ort 1.0", "avx2,avx512f"), "a different CPU feature set");

        write_at(&a, b"model A", 2_000_000);
        assert_ne!(base, key(&a, "ort 1.0", "avx2"), "the same path with new contents");
    }

    #[test]
    fn a_just_written_model_counts_as_recently_changed() {
        let root = TempDir::new().unwrap();
        let path = root.path().join("fresh.onnx");
        fs::write(&path, b"fresh").unwrap();
        assert!(changed_recently(&path), "ctime is now");
        assert!(changed_recently(&root.path().join("missing.onnx")));
    }

    #[test]
    fn missing_model_has_no_cache_slot() {
        let root = TempDir::new().unwrap();
        let missing = root.path().join("missing.onnx");
        assert!(OptimizedModelCache::with_environment(root.path().to_path_buf(), &missing, "ort", "").is_none());
    }

    #[test]
    fn published_graph_is_reused() {
        let root = TempDir::new().unwrap();
        let source = model(&root, "model.onnx", b"source");
        let cache = cache(&root, &source);
        assert_eq!(cache.saved_model(), None);

        let saved = save(&cache, b"optimized graph");
        assert_eq!(cache.saved_model(), Some(saved.clone()));
        assert_eq!(fs::read(saved).unwrap(), b"optimized graph");
    }

    #[test]
    fn truncated_or_rewritten_graph_is_not_trusted() {
        let root = TempDir::new().unwrap();
        let source = model(&root, "model.onnx", b"source");
        let cache = cache(&root, &source);
        let saved = save(&cache, b"optimized graph");

        fs::write(&saved, b"optimized").unwrap();
        assert_eq!(cache.saved_model(), None, "a truncated graph must be rebuilt");

        let saved = save(&cache, b"optimized graph");
        assert_eq!(cache.saved_model(), Some(saved));
    }

    #[test]
    fn graph_without_its_sidecar_is_not_trusted() {
        let root = TempDir::new().unwrap();
        let source = model(&root, "model.onnx", b"source");
        let cache = cache(&root, &source);
        save(&cache, b"optimized graph");

        fs::remove_file(cache.meta_path()).unwrap();
        assert_eq!(cache.saved_model(), None);
    }

    #[test]
    fn discard_removes_the_entry() {
        let root = TempDir::new().unwrap();
        let source = model(&root, "model.onnx", b"source");
        let cache = cache(&root, &source);
        let saved = save(&cache, b"optimized graph");

        cache.discard();
        assert!(!saved.exists());
        assert_eq!(cache.saved_model(), None);
    }

    #[test]
    fn publishing_a_new_key_removes_the_stale_entry_for_the_same_model() {
        let root = TempDir::new().unwrap();
        let source = model(&root, "model.onnx", b"source");
        let other = model(&root, "other.onnx", b"other");
        let old = cache(&root, &source);
        let old_saved = save(&old, b"old graph");
        let other_saved = save(&cache(&root, &other), b"other graph");

        let new = OptimizedModelCache::with_environment(root.path().join("cache"), &source, "ort 2.0", "avx2").unwrap();
        save(&new, b"new graph");

        assert!(!old_saved.exists(), "the superseded graph for this model is removed");
        assert!(!old.meta_path().exists());
        assert!(other_saved.exists(), "another model's graph is kept");
        assert!(new.saved_model().is_some());
    }

    /// Runs RT-DETR on a fixed synthetic input and returns every output as raw bits.
    #[cfg(feature = "layout-detection")]
    fn run_rtdetr(session: &mut Session) -> Vec<(String, Vec<u64>)> {
        use ort::session::SessionInputValue;
        use ort::value::{Tensor, TensorElementType};
        use std::borrow::Cow;

        let pixels: Vec<f32> = (0..3 * 640 * 640)
            .map(|i: usize| ((i * 7919) % 1000) as f32 / 1000.0)
            .collect();
        let images = ndarray::Array4::from_shape_vec((1, 3, 640, 640), pixels).unwrap();
        let sizes = ndarray::Array2::from_shape_vec((1, 2), vec![612_i64, 792]).unwrap();
        let names: Vec<String> = session.inputs().iter().map(|input| input.name().to_string()).collect();
        let inputs: Vec<(Cow<'static, str>, SessionInputValue<'static>)> = vec![
            (Cow::Owned(names[0].clone()), Tensor::from_array(images).unwrap().into()),
            (Cow::Owned(names[1].clone()), Tensor::from_array(sizes).unwrap().into()),
        ];
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
                    other => panic!("unexpected RT-DETR output type {other:?}"),
                };
                (name.to_string(), bits)
            })
            .collect()
    }

    /// The saved graph must reproduce the online-optimised session exactly, or reusing it would
    /// change detections.
    #[cfg(feature = "layout-detection")]
    #[test]
    #[ignore = "requires the cached RT-DETR model (ORT inference)"]
    fn saved_rtdetr_graph_reproduces_online_outputs_bit_for_bit() {
        let Some(model) = crate::layout::LayoutModelManager::new(None).verified_model_path("rtdetr") else {
            eprintln!(
                "SKIPPED saved_rtdetr_graph_reproduces_online_outputs_bit_for_bit: no verified RT-DETR model in the \
                 local cache (run `xberg cache warm`)"
            );
            return;
        };
        let root = TempDir::new().unwrap();
        let cache = OptimizedModelCache::for_model(root.path().join("cache"), &model).expect("model fingerprint");
        let configure = || -> Result<SessionBuilder, ort::Error> {
            Session::builder(crate::ort_discovery::ort_environment()?)?
                .with_intra_threads(1)
                .map_err(|e| ort::Error::new(e.message()))?
                .with_inter_threads(1)
                .map_err(|e| ort::Error::new(e.message()))
        };

        let mut online = with_level(configure().unwrap(), GraphOptimizationLevel::All)
            .unwrap()
            .commit_from_file(&model)
            .unwrap();
        let staged = cache.staging_path().unwrap();
        let mut saving = with_level(configure().unwrap(), GraphOptimizationLevel::All)
            .and_then(|builder| {
                builder
                    .with_optimized_model_path(&staged)
                    .map_err(|e| ort::Error::new(e.message()))
            })
            .unwrap()
            .commit_from_file(&model)
            .unwrap();
        cache.publish(&staged);
        let saved = cache.saved_model().expect("the saved graph is published");
        let mut reloaded = with_level(configure().unwrap(), GraphOptimizationLevel::Disable)
            .unwrap()
            .commit_from_file(&saved)
            .unwrap();

        let expected = run_rtdetr(&mut online);
        assert!(!expected.is_empty());
        assert_eq!(
            run_rtdetr(&mut saving),
            expected,
            "saving must not change the online session"
        );
        assert_eq!(
            run_rtdetr(&mut reloaded),
            expected,
            "the reloaded graph must match bit for bit"
        );
    }
}
