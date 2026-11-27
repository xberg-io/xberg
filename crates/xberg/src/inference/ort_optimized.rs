//! Reuse of ONNX Runtime's optimised graph across processes for CPU-only sessions (ADR 0018).
//!
//! Building a session with `GraphOptimizationLevel::All` re-runs every graph transformer on each
//! cold start. ONNX Runtime can serialise the graph it produced (`SetOptimizedModelFilePath`), and
//! loading that file with optimisations disabled yields the same graph without the rewrite work.
//! The first CPU-only session for a model saves the graph here; later processes load it.
//!
//! The level-3 layout transforms (NCHWc) bake the host CPU's vector width into the saved graph, so
//! ONNX Runtime documents an `All`-level saved model as valid only on the hardware and runtime
//! that produced it. The key therefore covers the ONNX Runtime build, the target, the CPU features
//! that select MLAS kernels, the session recipe, and the source model's verified SHA-256. Sessions
//! with a GPU or other execution provider never use the cache: compiled EP nodes cannot be saved.
//!
//! A saved graph is executed in place of the model it came from, so it is held to the model's bar:
//! - it is saved and reused only for a source file this process has just verified against the
//!   digest its model manager requires ([`crate::model_download::pinned_file`]), and is not
//!   published if that file changed while ONNX Runtime read it;
//! - a sidecar records the graph's size and SHA-256, checked before every load. ONNX Runtime
//!   builds the session from the bytes that were checked, read from one open descriptor, never by
//!   opening the path again. Hashing a graph of hundreds of megabytes costs tens of milliseconds,
//!   so once a graph has verified, the sidecar also records its file identity (device, inode,
//!   nanosecond mtime and ctime); a load whose descriptor shows that identity before and after
//!   the read skips the hash. Any write through the file system changes the ctime, so a changed
//!   graph is hashed again. The identity is recorded only once the ctime is [`STAMP_SETTLE`]
//!   old, so a write within the same coarse timestamp tick cannot go unnoticed;
//! - the cache directory must be a real directory owned by this user and not writable by group or
//!   others, and every entry a regular file with the same properties, opened without following
//!   symlinks. Anything else disables the cache for that build with a warning.
//!
//! The checks cover the cache directory and its entries, not its ancestors: whoever can rename a
//! parent can swap the whole directory between checks. The default location is under the user's
//! own cache directory; an `XBERG_CACHE_DIR` in shared space must sit below a directory other
//! users cannot rename, such as a subdirectory of the sticky `/tmp`.
//!
//! Every failure falls back to the normal online-optimised build, at most one load and one save
//! per build; a graph that fails its checks or fails to load is deleted.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ort::session::Session;
use ort::session::builder::{GraphOptimizationLevel, SessionBuilder};

use crate::model_download::PinnedFile;
use crate::ort_discovery::OrtSessionError;

/// Changed whenever the key inputs, the sidecar or the file naming change, so older entries miss.
const CACHE_SCHEMA: &str = "xberg-ort-optimized v3";
const CACHE_MODULE: &str = "ort-optimized";
/// Set to `0`, `false`, `off` or `no` to always optimise online.
const DISABLE_ENV: &str = "XBERG_ORT_OPTIMIZED_CACHE";
const MAX_META_BYTES: u64 = 4096;
/// A serialised ONNX protobuf cannot exceed 2 GiB, so a larger file is not a graph ORT wrote.
const MAX_GRAPH_BYTES: u64 = 2 << 30;
/// Staging files and unreferenced graphs younger than this may belong to a running process.
const ORPHAN_AGE: Duration = Duration::from_secs(60 * 60);
/// Saved graphs kept across all models; the least recently published are removed first.
const MAX_GRAPHS: usize = 16;
/// Keys kept per model, the newest first. A cache directory shared by hosts with different CPUs
/// or ONNX Runtime builds holds one key per host for the same model; keeping only the newest
/// would make each host delete the others' entries on every save.
const KEYS_PER_MODEL: usize = 4;
const DIR_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;
const SHA256_HEX_LEN: usize = 64;
/// How old a graph's ctime must be before its identity is recorded: at least the coarsest
/// timestamp granularity of a common file system (FAT's 2 s), so a later write in place cannot
/// share the recorded ctime.
const STAMP_SETTLE: Duration = Duration::from_secs(2);

static STAGING_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How [`commit_with_cache`] produced its session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// From a verified saved graph.
    Loaded,
    /// Optimised online, and the graph was saved.
    Saved,
    /// Optimised online without saving.
    Online,
}

/// Build a session for `model`, reusing a saved optimised graph when `cpu_only` allows it.
///
/// `configure` returns a builder with everything but the optimisation level set (threads,
/// execution providers); it may be called more than once. `recipe` names the options `configure`
/// sets and is part of the key.
pub(crate) fn commit_from_file(
    model: &Path,
    cpu_only: bool,
    recipe: &str,
    configure: impl Fn() -> Result<SessionBuilder, OrtSessionError>,
) -> Result<Session, OrtSessionError> {
    if !cpu_only || !cache_enabled() {
        return online(&configure, model);
    }
    // `ort::info()` in the key needs a loaded runtime (`ort-dynamic`); creating the environment
    // loads it.
    crate::ort_discovery::ort_environment()?;
    let Some(source) = crate::model_download::pinned_file(model) else {
        tracing::debug!(
            model = %model.display(),
            "ORT saved graph: model not verified against a pinned digest in this process; optimising online"
        );
        return online(&configure, model);
    };
    let slot = Slot::new(
        crate::cache_dir::resolve_cache_dir(CACHE_MODULE),
        model,
        &source.sha256,
        recipe,
        &Platform::current(),
    );
    commit_with_cache(model, &source, &slot, &configure).map(|(session, _)| session)
}

fn online(
    configure: &impl Fn() -> Result<SessionBuilder, OrtSessionError>,
    model: &Path,
) -> Result<Session, OrtSessionError> {
    Ok(with_level(configure()?, GraphOptimizationLevel::All)?.commit_from_file(model)?)
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

fn commit_with_cache(
    model: &Path,
    source: &PinnedFile,
    slot: &Slot,
    configure: &impl Fn() -> Result<SessionBuilder, OrtSessionError>,
) -> Result<(Session, Outcome), OrtSessionError> {
    match slot.prepare_dir() {
        Ok(true) => {}
        Ok(false) => return online(configure, model).map(|session| (session, Outcome::Online)),
        Err(reason) => {
            tracing::warn!(dir = %slot.dir.display(), %reason, "ORT saved graph cache refused; optimising online");
            return online(configure, model).map(|session| (session, Outcome::Online));
        }
    }

    if let Some(session) = load_saved(model, slot, configure)? {
        return Ok((session, Outcome::Loaded));
    }

    // Creating the staging file up front also proves the directory is writable before the
    // expensive build.
    let staged = match slot.create_staging() {
        Ok(staged) => staged,
        Err(error) => {
            tracing::debug!(dir = %slot.dir.display(), %error, "ORT saved graph: cannot stage; optimising online");
            return online(configure, model).map(|session| (session, Outcome::Online));
        }
    };
    let saving = with_level(configure()?, GraphOptimizationLevel::All)
        .and_then(|builder| {
            builder
                .with_optimized_model_path(&staged)
                .map_err(|e| ort::Error::new(e.message()))
        })
        .and_then(|mut builder| builder.commit_from_file(model));
    let session = match saving {
        Ok(session) => session,
        Err(error) => {
            let _ = fs::remove_file(&staged);
            tracing::debug!(model = %model.display(), %error, "ORT session: build with graph saving failed; building without it");
            return online(configure, model).map(|session| (session, Outcome::Online));
        }
    };

    // A model replaced while ONNX Runtime read it would leave a graph of other bytes under the
    // verified digest.
    let published = if source.still_at(model) {
        slot.publish(&staged)
    } else {
        Err("the model changed while its graph was saved".to_string())
    };
    match published {
        Ok(()) => Ok((session, Outcome::Saved)),
        Err(reason) => {
            let _ = fs::remove_file(&staged);
            tracing::warn!(model = %model.display(), %reason, "ORT saved graph not published");
            Ok((session, Outcome::Online))
        }
    }
}

/// A session built from this slot's verified saved graph, or `None` when there is none or it
/// was unusable and has been discarded.
fn load_saved(
    model: &Path,
    slot: &Slot,
    configure: &impl Fn() -> Result<SessionBuilder, OrtSessionError>,
) -> Result<Option<Session>, OrtSessionError> {
    match slot.verified_graph() {
        Ok(Some(graph)) => {
            let loaded = with_level(configure()?, GraphOptimizationLevel::Disable)
                .and_then(|mut builder| builder.commit_from_memory(&graph.bytes));
            match loaded {
                Ok(session) => {
                    tracing::debug!(
                        model = %model.display(),
                        graph = %graph.path.display(),
                        hashed = graph.hashed,
                        "ORT session: loaded saved optimized graph"
                    );
                    return Ok(Some(session));
                }
                Err(error) => {
                    tracing::warn!(graph = %graph.path.display(), %error, "ORT saved graph failed to load; rebuilding");
                    slot.discard();
                }
            }
        }
        Ok(None) => {}
        Err(reason) => {
            tracing::warn!(dir = %slot.dir.display(), %reason, "ORT saved graph rejected; rebuilding");
            slot.discard();
        }
    }
    Ok(None)
}

/// The parts of the host that change which optimised graph ONNX Runtime produces.
#[derive(Debug, Clone)]
struct Platform {
    runtime: String,
    cpu: String,
}

impl Platform {
    fn current() -> Self {
        Self {
            runtime: ort::info().to_string(),
            cpu: cpu_features(),
        }
    }
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
            "avxvnni",
            "avx512f",
            "avx512bw",
            "avx512dq",
            "avx512vl",
            "avx512vnni",
            "avx512bf16",
            "avx512fp16"
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

fn sha256_hex(parts: &[&[u8]]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hex::encode(hasher.finalize())
}

/// The `len` bytes of `file`, with their SHA-256 when `hash` is set, read from its current offset.
fn read_graph(file: &fs::File, len: u64, hash: bool) -> Result<(Vec<u8>, Option<String>), String> {
    use sha2::{Digest, Sha256};
    let capacity = usize::try_from(len).map_err(|_| format!("{len} bytes do not fit in memory"))?;
    let mut bytes = Vec::with_capacity(capacity);
    Read::take(file, len.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read failed: {e}"))?;
    if bytes.len() as u64 != len {
        return Err(format!("expected {len} bytes, read {}", bytes.len()));
    }
    let digest = hash.then(|| hex::encode(Sha256::digest(&bytes)));
    Ok((bytes, digest))
}

/// SHA-256 of `file`, which must hold exactly `len` bytes.
fn sha256_of(file: &fs::File, len: u64) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut reader = Read::take(file, len.saturating_add(1));
    let mut buf = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("read failed: {e}"))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buf[..n]);
    }
    if total != len {
        return Err(format!("expected {len} bytes, read {total}"));
    }
    Ok(hex::encode(hasher.finalize()))
}

#[allow(unsafe_code)]
fn current_uid() -> u32 {
    // SAFETY: geteuid takes no arguments, cannot fail and has no side effects.
    unsafe { libc::geteuid() }
}

/// Whether `metadata` (not following symlinks) is a directory or regular file, as `directory`
/// says, owned by this user and not writable by group or others.
fn check_owned(metadata: &fs::Metadata, directory: bool) -> Result<(), String> {
    let file_type = metadata.file_type();
    if directory && !file_type.is_dir() {
        return Err("not a directory".to_string());
    }
    if !directory && !file_type.is_file() {
        return Err("not a regular file".to_string());
    }
    if metadata.uid() != current_uid() {
        return Err(format!("owned by uid {}, not {}", metadata.uid(), current_uid()));
    }
    if metadata.mode() & 0o022 != 0 {
        return Err(format!(
            "writable by group or others (mode {:o})",
            metadata.mode() & 0o777
        ));
    }
    Ok(())
}

fn open_entry(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

fn create_entry(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

/// The identity of a graph file: any write through the file system changes the ctime, and a
/// replacement has another inode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
}

impl Stamp {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            ctime: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
        }
    }

    /// Whether the last change is at least `settle` old, so a later write gets another ctime.
    fn settled(&self, settle: Duration) -> bool {
        let (Ok(secs), Ok(nanos)) = (u64::try_from(self.ctime), u32::try_from(self.ctime_nsec)) else {
            return false;
        };
        std::time::UNIX_EPOCH
            .checked_add(Duration::new(secs, nanos))
            .is_some_and(|changed| changed.elapsed().is_ok_and(|age| age >= settle))
    }

    fn render(&self) -> String {
        format!(
            "{}:{}:{}.{}:{}.{}",
            self.dev, self.ino, self.mtime, self.mtime_nsec, self.ctime, self.ctime_nsec
        )
    }

    fn parse(text: &str) -> Option<Self> {
        let mut fields = text.split(':');
        let dev = fields.next()?.parse().ok()?;
        let ino = fields.next()?.parse().ok()?;
        let (mtime, mtime_nsec) = fields.next()?.split_once('.')?;
        let (ctime, ctime_nsec) = fields.next()?.split_once('.')?;
        fields.next().is_none().then_some(())?;
        Some(Self {
            dev,
            ino,
            mtime: mtime.parse().ok()?,
            mtime_nsec: mtime_nsec.parse().ok()?,
            ctime: ctime.parse().ok()?,
            ctime_nsec: ctime_nsec.parse().ok()?,
        })
    }
}

/// The sidecar recorded beside a published graph.
#[derive(Debug, PartialEq, Eq)]
struct Record {
    key: String,
    size: u64,
    sha256: String,
    /// The graph file's identity when it last hashed to `sha256`; `None` until it has settled.
    stamp: Option<Stamp>,
}

impl Record {
    fn render(&self) -> String {
        let stamp = self.stamp.as_ref().map_or_else(|| "-".to_string(), Stamp::render);
        format!(
            "{CACHE_SCHEMA}\nkey={}\nsize={}\nsha256={}\nstamp={stamp}\n",
            self.key, self.size, self.sha256
        )
    }

    fn parse(text: &str) -> Option<Self> {
        let mut lines = text.strip_suffix('\n')?.split('\n');
        (lines.next()? == CACHE_SCHEMA).then_some(())?;
        let key = lines.next()?.strip_prefix("key=")?.to_string();
        let size = lines.next()?.strip_prefix("size=")?.parse::<u64>().ok()?;
        let sha256 = lines.next()?.strip_prefix("sha256=")?.to_string();
        let stamp = match lines.next()?.strip_prefix("stamp=")? {
            "-" => None,
            stamp => Some(Stamp::parse(stamp)?),
        };
        let well_formed = lines.next().is_none()
            && sha256.len() == SHA256_HEX_LEN
            && sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        well_formed.then_some(Self {
            key,
            size,
            sha256,
            stamp,
        })
    }
}

/// A published graph that passed its checks: the bytes to build the session from.
#[derive(Debug, PartialEq, Eq)]
struct Graph {
    path: PathBuf,
    bytes: Vec<u8>,
    /// Whether the bytes were hashed, rather than trusted by an unchanged recorded identity.
    hashed: bool,
}

/// One model's entry in the saved-graph cache.
#[derive(Debug)]
struct Slot {
    dir: PathBuf,
    /// Shared by every key for the same source path, so superseded entries can be found.
    prefix: String,
    key: String,
    /// [`STAMP_SETTLE`], shortened by tests.
    settle: Duration,
}

impl Slot {
    fn new(dir: PathBuf, model: &Path, source_sha256: &str, recipe: &str, platform: &Platform) -> Self {
        let prefix = sha256_hex(&[model.as_os_str().as_encoded_bytes()])[..16].to_string();
        let key = sha256_hex(&[
            CACHE_SCHEMA.as_bytes(),
            platform.runtime.as_bytes(),
            std::env::consts::ARCH.as_bytes(),
            std::env::consts::OS.as_bytes(),
            platform.cpu.as_bytes(),
            b"optimization=all",
            b"providers=cpu",
            recipe.as_bytes(),
            source_sha256.as_bytes(),
        ])[..32]
            .to_string();
        Self {
            dir,
            prefix,
            key,
            settle: STAMP_SETTLE,
        }
    }

    fn meta_path(&self) -> PathBuf {
        self.dir.join(format!("{}-{}.meta", self.prefix, self.key))
    }

    /// Graphs are named by their own digest, so a racing publisher never replaces the file a
    /// sidecar points to with different bytes.
    fn graph_path(&self, sha256: &str) -> PathBuf {
        self.dir
            .join(format!("{}-{}-{}.onnx", self.prefix, self.key, &sha256[..16]))
    }

    /// Create the cache directory owner-only, or check that an existing one is safe to use.
    /// `Ok(false)` means it cannot be created (e.g. a read-only cache), `Err` that it is unsafe.
    fn prepare_dir(&self) -> Result<bool, String> {
        let created = self.dir.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| {
            match fs::DirBuilder::new().mode(DIR_MODE).create(&self.dir) {
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
                other => other,
            }
        });
        if let Err(error) = created {
            tracing::debug!(dir = %self.dir.display(), %error, "ORT saved graph cache unavailable");
            return Ok(false);
        }
        let metadata = fs::symlink_metadata(&self.dir).map_err(|e| e.to_string())?;
        check_owned(&metadata, true)?;
        if metadata.mode() & 0o077 != 0 {
            fs::set_permissions(&self.dir, fs::Permissions::from_mode(DIR_MODE))
                .map_err(|e| format!("cannot restrict to owner: {e}"))?;
        }
        Ok(true)
    }

    /// The published graph for this key, after checking its sidecar, ownership, size and digest,
    /// read once from the descriptor that was checked. `Ok(None)` means nothing is published;
    /// `Err` means the entry is unusable.
    fn verified_graph(&self) -> Result<Option<Graph>, String> {
        let meta = match open_entry(&self.meta_path()) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("sidecar: {error}")),
        };
        let metadata = meta.metadata().map_err(|e| format!("sidecar: {e}"))?;
        check_owned(&metadata, false).map_err(|e| format!("sidecar: {e}"))?;
        if metadata.len() > MAX_META_BYTES {
            return Err("sidecar: too large".to_string());
        }
        let mut text = String::new();
        Read::take(&meta, MAX_META_BYTES)
            .read_to_string(&mut text)
            .map_err(|e| format!("sidecar: {e}"))?;
        let record = Record::parse(&text).ok_or("sidecar: malformed")?;
        if record.key != self.key {
            return Err("sidecar: written for another key".to_string());
        }
        if record.size == 0 || record.size > MAX_GRAPH_BYTES {
            return Err(format!("graph: implausible size {}", record.size));
        }

        let path = self.graph_path(&record.sha256);
        let graph = open_entry(&path).map_err(|e| format!("graph: {e}"))?;
        let metadata = graph.metadata().map_err(|e| format!("graph: {e}"))?;
        check_owned(&metadata, false).map_err(|e| format!("graph: {e}"))?;
        if metadata.len() != record.size {
            return Err(format!("graph: {} bytes, sidecar says {}", metadata.len(), record.size));
        }
        let before = Stamp::of(&metadata);
        let hash = record.stamp != Some(before);
        let (bytes, digest) = read_graph(&graph, record.size, hash).map_err(|e| format!("graph: {e}"))?;
        // Unchanged identity across the read means the bytes are the file the stamp describes.
        let after = graph.metadata().map(|metadata| Stamp::of(&metadata));
        if after.as_ref().ok() != Some(&before) {
            return Err("graph: changed while it was read".to_string());
        }
        if let Some(digest) = digest {
            if digest != record.sha256 {
                return Err("graph: digest does not match its sidecar".to_string());
            }
            if before.settled(self.settle) {
                self.record_stamp(record, before);
            }
        }
        Ok(Some(Graph {
            path,
            bytes,
            hashed: hash,
        }))
    }

    /// Record `stamp` for a graph that just hashed to its sidecar's digest, so later loads that
    /// find the same identity skip the hash. Failing to only costs the next load a hash.
    fn record_stamp(&self, record: Record, stamp: Stamp) {
        let record = Record {
            stamp: Some(stamp),
            ..record
        };
        if let Err(error) = self.write_sidecar(&record, &self.staging_path().with_extension("meta.tmp")) {
            tracing::debug!(dir = %self.dir.display(), %error, "ORT saved graph: identity not recorded");
        }
    }

    /// Write `record` to `staging`, then rename it over this key's sidecar.
    fn write_sidecar(&self, record: &Record, staging: &Path) -> std::io::Result<()> {
        let written = create_entry(staging)
            .and_then(|mut file| {
                file.write_all(record.render().as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| fs::rename(staging, self.meta_path()));
        if written.is_err() {
            let _ = fs::remove_file(staging);
        }
        written
    }

    /// A staging name unique to this process and call, swept by [`Slot::evict`] once orphaned.
    fn staging_path(&self) -> PathBuf {
        self.dir.join(format!(
            ".{}-{}.{}.{}.tmp",
            self.prefix,
            self.key,
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }

    /// A new, empty owner-only file for ONNX Runtime to write the graph into.
    fn create_staging(&self) -> std::io::Result<PathBuf> {
        let path = self.staging_path();
        create_entry(&path)?;
        Ok(path)
    }

    /// Flush a complete staged graph, move it into place under its digest, then publish its
    /// sidecar the same way, and remove entries this one supersedes.
    fn publish(&self, staged: &Path) -> Result<(), String> {
        let graph = open_entry(staged).map_err(|e| format!("staged graph: {e}"))?;
        let metadata = graph.metadata().map_err(|e| format!("staged graph: {e}"))?;
        check_owned(&metadata, false).map_err(|e| format!("staged graph: {e}"))?;
        let size = metadata.len();
        if size == 0 || size > MAX_GRAPH_BYTES {
            return Err(format!("staged graph: implausible size {size}"));
        }
        graph.sync_all().map_err(|e| format!("staged graph: {e}"))?;
        let sha256 = sha256_of(&graph, size).map_err(|e| format!("staged graph: {e}"))?;
        drop(graph);

        fs::rename(staged, self.graph_path(&sha256)).map_err(|e| format!("rename graph: {e}"))?;
        // The graph was just written, so its ctime has not settled; a later load that hashes it
        // records its identity.
        let record = Record {
            key: self.key.clone(),
            size,
            sha256,
            stamp: None,
        };
        self.write_sidecar(&record, &staged.with_extension("meta.tmp"))
            .map_err(|error| format!("sidecar: {error}"))?;
        if let Ok(dir) = fs::File::open(&self.dir) {
            let _ = dir.sync_all();
        }
        tracing::debug!(dir = %self.dir.display(), key = %self.key, "ORT session: saved optimized graph");
        self.evict();
        Ok(())
    }

    /// Remove this key's sidecar and graphs.
    fn discard(&self) {
        let own = format!("{}-{}", self.prefix, self.key);
        for (name, path, _) in entries(&self.dir) {
            if name.starts_with(&own) {
                let _ = fs::remove_file(path);
            }
        }
    }

    /// Remove staging files and graphs no sidecar points to once they are old enough not to
    /// belong to a running process, this model's entries beyond the newest [`KEYS_PER_MODEL`]
    /// keys, and the least recently published entries beyond [`MAX_GRAPHS`].
    fn evict(&self) {
        let same_model = format!("{}-", self.prefix);
        let own_meta = format!("{}-{}.meta", self.prefix, self.key);
        let mut other_metas: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        let mut superseded: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        let listed = entries(&self.dir);
        for (name, path, modified) in &listed {
            let orphan_age_passed = modified.elapsed().is_ok_and(|age| age >= ORPHAN_AGE);
            if name.starts_with('.') {
                if name.ends_with(".tmp") && orphan_age_passed {
                    let _ = fs::remove_file(path);
                }
            } else if name.ends_with(".meta") {
                if *name == own_meta {
                    continue;
                }
                if name.starts_with(&same_model) {
                    superseded.push((*modified, path.clone()));
                }
                other_metas.push((*modified, path.clone()));
            } else if name.ends_with(".onnx") && orphan_age_passed && !is_referenced(name, &listed) {
                let _ = fs::remove_file(path);
            }
        }
        // This model's older keys go with their graphs, as nothing else will load them here.
        if superseded.len() >= KEYS_PER_MODEL {
            superseded.sort();
            for (_, meta) in &superseded[..=superseded.len() - KEYS_PER_MODEL] {
                let _ = fs::remove_file(meta);
                let graphs = meta
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| format!("{stem}-"));
                for (name, path, _) in &listed {
                    if graphs
                        .as_ref()
                        .is_some_and(|graphs| name.starts_with(graphs) && name.ends_with(".onnx"))
                    {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
        // Dropping a sidecar retires its entry at once; its graph goes once it is orphaned.
        other_metas.retain(|(_, meta)| meta.exists());
        if other_metas.len() >= MAX_GRAPHS {
            other_metas.sort();
            for (_, meta) in &other_metas[..=other_metas.len() - MAX_GRAPHS] {
                let _ = fs::remove_file(meta);
            }
        }
    }
}

/// Whether the graph file `name` is the one its slot's current sidecar points to.
fn is_referenced(name: &str, listed: &[(String, PathBuf, std::time::SystemTime)]) -> bool {
    // `<prefix>-<key>-<digest16>.onnx` belongs to the sidecar `<prefix>-<key>.meta`.
    let Some((slot, digest)) = name.strip_suffix(".onnx").and_then(|stem| stem.rsplit_once('-')) else {
        return false;
    };
    let meta_name = format!("{slot}.meta");
    let Some((_, meta_path, _)) = listed.iter().find(|(entry, _, _)| *entry == meta_name) else {
        return false;
    };
    let Ok(meta) = open_entry(meta_path) else {
        return false;
    };
    let mut text = String::new();
    if Read::take(&meta, MAX_META_BYTES).read_to_string(&mut text).is_err() {
        return false;
    }
    Record::parse(&text).is_some_and(|record| record.sha256.starts_with(digest))
}

/// `(file name, path, modification time)` of every entry directly in `dir` with a UTF-8 name.
fn entries(dir: &Path) -> Vec<(String, PathBuf, std::time::SystemTime)> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    listing
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((name, entry.path(), modified))
        })
        .collect()
}

#[cfg(test)]
#[path = "ort_optimized/tests.rs"]
mod tests;
