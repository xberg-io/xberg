//! Tessdata language pack download utilities for runtime language pack materialization.
//!
//! Provides functions to download `.traineddata` files from the official Tesseract
//! tessdata_fast and tessdata_best repositories on GitHub, pinned to one revision each and
//! SHA-256 verified, with fallback URLs for redundancy.

#![cfg(not(target_arch = "wasm32"))]

use crate::core::config::TessdataModel;
use crate::ocr::error::OcrError;
use crate::ocr::processor::validation::FileStamp;
use sha2::{Digest as _, Sha256};
use std::collections::HashMap;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

mod pins;

use pins::{PinnedPack, TESSDATA_BEST_PACKS, TESSDATA_BEST_REVISION, TESSDATA_FAST_PACKS, TESSDATA_FAST_REVISION};

const REQUEST_PHASE_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const RETRY_BACKOFF: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(2)];
const MAX_TRAINEDDATA_BYTES: u64 = 64 * 1024 * 1024;
/// How long a failed download is remembered. An offline host otherwise spends the whole retry
/// budget (about 93 s) per language on every image.
const FAILURE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
enum DownloadError {
    #[error("HTTP transfer failed: {0}")]
    Request(#[from] ureq::Error),
    #[error("SHA-256 mismatch: expected {expected}, got {actual}")]
    Integrity { expected: String, actual: String },
    #[error("language pack could not be published: {0}")]
    Publish(#[from] std::io::Error),
}

impl DownloadError {
    fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Request(
                ureq::Error::StatusCode(408 | 429 | 500..=599)
                    | ureq::Error::ConnectionFailed
                    | ureq::Error::HostNotFound
                    | ureq::Error::Io(_)
                    | ureq::Error::Timeout(_)
            )
        )
    }

    fn can_try_mirror(&self) -> bool {
        matches!(self, Self::Request(error) if !matches!(error, ureq::Error::BodyExceedsLimit(_)))
    }
}

/// One pinned pack: language, SHA-256 and byte size.
pub(crate) type Pin<'a> = (&'a str, &'a str, u64);

/// The pinned pack a `.traineddata` file was verified to be, by size and SHA-256.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifiedSet {
    /// The pinned `tessdata_fast` pack of its language.
    Fast,
    /// The pinned `tessdata_best` pack of its language.
    Best,
    /// The pack both sets pin with the same bytes (`osd`, `equ`).
    Shared,
    /// No pinned pack of its language: user-trained, another revision, damaged, unreadable or
    /// missing.
    Unknown,
}

impl VerifiedSet {
    /// Whether the file is the pinned `model` pack of its language.
    pub(crate) fn is(self, model: TessdataModel) -> bool {
        match self {
            Self::Shared => true,
            Self::Fast => model == TessdataModel::Fast,
            Self::Best => model == TessdataModel::Best,
            Self::Unknown => false,
        }
    }
}

/// Pin tables per model set, each sorted by language code: [`PINNED`] in production, small
/// tables in tests.
pub(crate) struct PinTables<'a> {
    pub(crate) fast: &'a [Pin<'a>],
    pub(crate) best: &'a [Pin<'a>],
}

/// The pins committed in `pins.rs`.
pub(crate) const PINNED: PinTables<'static> = PinTables {
    fast: TESSDATA_FAST_PACKS,
    best: TESSDATA_BEST_PACKS,
};

impl<'a> PinTables<'a> {
    /// The pinned packs of `model`.
    pub(crate) fn packs(&self, model: TessdataModel) -> &'a [Pin<'a>] {
        match model {
            TessdataModel::Fast => self.fast,
            TessdataModel::Best => self.best,
        }
    }

    /// The pin of `lang` in `model`.
    pub(crate) fn pin(&self, lang: &str, model: TessdataModel) -> Option<&'a Pin<'a>> {
        let packs = self.packs(model);
        let index = packs.binary_search_by(|entry| entry.0.cmp(lang)).ok()?;
        Some(&packs[index])
    }

    /// The languages whose `fast` and `best` packs are byte-identical (`equ`, `osd`).
    pub(crate) fn shared_languages(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.best
            .iter()
            .filter(|best| self.pin(best.0, TessdataModel::Fast) == Some(*best))
            .map(|pack| pack.0)
    }

    /// Which pinned pack of `lang` the file at `path` is.
    ///
    /// The size is compared first, so a file that is no pinned size (every user-trained model)
    /// is never read; a file of a pinned size is hashed once per (path, [`FileStamp`]) in this
    /// process.
    pub(crate) fn classify(&self, lang: &str, path: &Path) -> VerifiedSet {
        let Ok(metadata) = std::fs::metadata(path) else {
            return VerifiedSet::Unknown;
        };
        let sized = |model| {
            self.pin(lang, model)
                .filter(|pin| metadata.is_file() && pin.2 == metadata.len())
        };
        let (fast, best) = (sized(TessdataModel::Fast), sized(TessdataModel::Best));
        if fast.is_none() && best.is_none() {
            return VerifiedSet::Unknown;
        }
        let Some(digest) = memoized_sha256(path, &metadata) else {
            return VerifiedSet::Unknown;
        };
        match (
            fast.is_some_and(|pin| pin.1 == digest),
            best.is_some_and(|pin| pin.1 == digest),
        ) {
            (true, true) => VerifiedSet::Shared,
            (true, false) => VerifiedSet::Fast,
            (false, true) => VerifiedSet::Best,
            (false, false) => VerifiedSet::Unknown,
        }
    }

    /// The set every one of `languages` in `directory` was verified to be, ignoring the packs
    /// both sets share. `None` when a pack is not a pinned one, when the sets are mixed, or when
    /// only shared packs were named: the directory then holds models xberg cannot vouch for.
    pub(crate) fn verified_model<S: AsRef<str>>(&self, directory: &Path, languages: &[S]) -> Option<TessdataModel> {
        let mut verified = None;
        for lang in languages {
            let lang = lang.as_ref();
            let model = match self.classify(lang, &directory.join(format!("{lang}.traineddata"))) {
                VerifiedSet::Shared => continue,
                VerifiedSet::Fast => TessdataModel::Fast,
                VerifiedSet::Best => TessdataModel::Best,
                VerifiedSet::Unknown => return None,
            };
            if verified.is_some_and(|seen| seen != model) {
                return None;
            }
            verified = Some(model);
        }
        verified
    }

    /// Move a file in `output_dir` that is not the pinned `model` pack of `lang` out of the way
    /// (to `<lang>.traineddata.unverified`, or a unique name beside it when an earlier file holds
    /// that one), so the pinned pack can be published there without replacing anything. A
    /// language with no pin is left alone.
    ///
    /// For the directories xberg vouches for: a wrong file there would otherwise keep being
    /// served as the pinned pack. A directory that cannot be written (permissions, a read-only
    /// file system) keeps the file, with a warning, and reports [`SetAside::Blocked`].
    pub(crate) fn set_aside_unverified(
        &self,
        lang: &str,
        model: TessdataModel,
        output_dir: &Path,
    ) -> std::io::Result<SetAside> {
        // ~keep One mover at a time, deciding on what is there now: a thread that saw the wrong
        // file earlier must not move the pack another thread has published since.
        static MOVING: Mutex<()> = Mutex::new(());
        let _alone = MOVING.lock().unwrap_or_else(PoisonError::into_inner);

        let path = output_dir.join(format!("{lang}.traineddata"));
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(SetAside::Untouched),
            Err(error) => return Err(error),
        }
        if self.pin(lang, model).is_none() || self.classify(lang, &path).is(model) {
            return Ok(SetAside::Untouched);
        }
        let aside = unused_unverified_path(output_dir, lang);
        match std::fs::rename(&path, &aside) {
            Ok(()) => {}
            // ~keep Another process moved it first.
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(SetAside::Untouched),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem
                ) =>
            {
                warn_blocked_once(&path, model, &error);
                return Ok(SetAside::Blocked);
            }
            Err(error) => return Err(error),
        }
        // ~keep Where the platform has no inode, the pack published next can share the size and
        // modification time of the file just moved, which is all the digest memo tells apart there.
        DIGESTS.lock().unwrap_or_else(PoisonError::into_inner).remove(&path);
        tracing::warn!(
            path = %path.display(),
            moved_to = %aside.display(),
            model = %model,
            "A file in xberg's tessdata directory is not the pinned language pack; moved it aside to \
             fetch the pinned one"
        );
        Ok(SetAside::Moved)
    }

    /// Copy the pinned `model` pack for `lang` into `output_dir` from the first of `sources`
    /// that holds exactly those bytes. Never downloads. Returns whether `output_dir` now holds
    /// the pack.
    pub(crate) fn adopt(&self, lang: &str, model: TessdataModel, output_dir: &Path, sources: &[PathBuf]) -> bool {
        let Some(&(_, sha256, size)) = self.pin(lang, model) else {
            return false;
        };
        let file_name = format!("{lang}.traineddata");
        let output_path = output_dir.join(&file_name);
        sources
            .iter()
            .filter(|source| source.as_path() != output_dir)
            .any(|source| adopt_verified(&source.join(&file_name), &output_path, sha256, size))
    }
}

/// What [`PinTables::set_aside_unverified`] did with a language's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetAside {
    /// Nothing to move: no file, no pin, or the pinned pack.
    Untouched,
    /// The file was moved aside; the pack can be published in its place.
    Moved,
    /// The file is not the pinned pack, but the directory cannot be written, so it stays and is
    /// used as given.
    Blocked,
}

/// `<lang>.traineddata.unverified` in `output_dir`, or, when an earlier set-aside file holds that
/// name, one made unique by the time and process.
fn unused_unverified_path(output_dir: &Path, lang: &str) -> PathBuf {
    let plain = output_dir.join(format!("{lang}.traineddata.unverified"));
    if matches!(std::fs::symlink_metadata(&plain), Err(error) if error.kind() == ErrorKind::NotFound) {
        return plain;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    output_dir.join(format!("{lang}.traineddata.unverified.{nanos}.{}", std::process::id()))
}

/// Warn, once per file in this process, that a wrong file in a verified directory stays because
/// the directory cannot be written.
fn warn_blocked_once(path: &Path, model: TessdataModel, error: &std::io::Error) {
    static WARNED: LazyLock<Mutex<std::collections::HashSet<PathBuf>>> = LazyLock::new(Default::default);
    if WARNED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.to_path_buf())
    {
        tracing::warn!(
            path = %path.display(),
            model = %model,
            %error,
            "A file in xberg's tessdata directory is not the pinned language pack and cannot be moved \
             aside; it is used as given. Replace it with the pinned pack or make the directory writable"
        );
    }
}

/// Stamp and SHA-256 of the files [`memoized_sha256`] has read.
type SeenDigest = (FileStamp, String);
static DIGESTS: LazyLock<Mutex<HashMap<PathBuf, SeenDigest>>> = LazyLock::new(Default::default);

/// The SHA-256 of the regular file at `path`, read once per (path, [`FileStamp`]) in this
/// process. `metadata` is what the caller just read for `path`.
fn memoized_sha256(path: &Path, metadata: &std::fs::Metadata) -> Option<String> {
    let state = FileStamp::of(metadata)?;
    if let Some((seen, digest)) = DIGESTS.lock().unwrap_or_else(PoisonError::into_inner).get(path)
        && *seen == state
    {
        return Some(digest.clone());
    }
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut reader = file.take(MAX_TRAINEDDATA_BYTES.saturating_add(1));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut read = 0_u64;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                hasher.update(&buffer[..count]);
                read += count as u64;
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    // ~keep A file that changed while it was read has no digest worth remembering under the
    // size and time read before it.
    if read != metadata.len() {
        return None;
    }
    let digest = hex::encode(hasher.finalize());
    DIGESTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.to_path_buf(), (state, digest.clone()));
    Some(digest)
}

/// The pinned language packs of `model`, sorted by language code.
pub(crate) fn pinned_packs(model: TessdataModel) -> &'static [PinnedPack] {
    PINNED.packs(model)
}

fn pinned_revision(model: TessdataModel) -> &'static str {
    match model {
        TessdataModel::Fast => TESSDATA_FAST_REVISION,
        TessdataModel::Best => TESSDATA_BEST_REVISION,
    }
}

fn pinned_pack(lang: &str, model: TessdataModel) -> Option<&'static PinnedPack> {
    PINNED.pin(lang, model)
}

fn source_language(lang: &str) -> &str {
    // ~keep Upstream frk.traineddata is a Git symlink in tessdata_fast and absent from
    // tessdata_best; both publish the same Fraktur model as deu_latf.
    if lang == "frk" { "deu_latf" } else { lang }
}

pub(super) fn pinned_source_url(lang: &str, model: TessdataModel) -> String {
    format!(
        "https://github.com/tesseract-ocr/{}/raw/{}/{}.traineddata",
        model.repository(),
        pinned_revision(model),
        source_language(lang)
    )
}

/// Make the pinned `model` pack for `lang` present in `output_dir`: keep an existing file, else
/// copy a verified one from the first of `sources` holding it, else download it.
///
/// For [`TessdataModel::Best`] an existing file is kept only when it is the pinned pack; the
/// caller moves a wrong one aside first ([`PinTables::set_aside_unverified`]), and one that is
/// still there (another process wrote it meanwhile) is an error rather than reported as present.
///
/// Returns whether this call published a new file.
pub(crate) fn ensure_language_pack(
    lang: &str,
    model: TessdataModel,
    output_dir: &Path,
    sources: &[PathBuf],
) -> Result<bool, OcrError> {
    let output_path = output_dir.join(format!("{lang}.traineddata"));
    let published = materialize_once(&output_path, || {
        if PINNED.adopt(lang, model, output_dir, sources) {
            return Ok(true);
        }
        download_missing_language_pack(lang, model, output_dir)
    })?;
    if !published
        && model == TessdataModel::Best
        && PINNED.pin(lang, model).is_some()
        && !PINNED.classify(lang, &output_path).is(model)
    {
        return Err(OcrError::TesseractInitializationFailed(format!(
            "'{}' is not the pinned {} language pack. Remove it",
            output_path.display(),
            model.repository()
        )));
    }
    Ok(published)
}

/// Run `materialize` unless `output_path` is already a file, one caller at a time per path in
/// this process, so the pages of one document do not each download the same pack. A caller that
/// waited finds the file and returns `Ok(false)`.
fn materialize_once(
    output_path: &Path,
    materialize: impl FnOnce() -> Result<bool, OcrError>,
) -> Result<bool, OcrError> {
    static IN_FLIGHT: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = LazyLock::new(Default::default);

    if output_path.is_file() {
        return Ok(false);
    }
    let flight = Arc::clone(
        IN_FLIGHT
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(output_path.to_path_buf())
            .or_default(),
    );
    let _alone = flight.lock().unwrap_or_else(PoisonError::into_inner);
    if output_path.is_file() {
        return Ok(false);
    }
    materialize()
}

/// Download failures of this process per (set, language), so a pack that just failed to download
/// is not requested again until [`FAILURE_TTL`] has passed.
struct FailureMemo {
    ttl: Duration,
    failed: Mutex<HashMap<(TessdataModel, String), (Instant, String)>>,
}

impl FailureMemo {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            failed: Mutex::default(),
        }
    }

    /// The message of a failure recorded less than the time to live before `now`.
    fn recent(&self, lang: &str, model: TessdataModel, now: Instant) -> Option<String> {
        let mut failed = self.failed.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (model, lang.to_owned());
        match failed.get(&key) {
            Some((at, message)) if now.saturating_duration_since(*at) < self.ttl => Some(message.clone()),
            Some(_) => {
                failed.remove(&key);
                None
            }
            None => None,
        }
    }

    fn record(&self, lang: &str, model: TessdataModel, now: Instant, message: String) {
        self.failed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((model, lang.to_owned()), (now, message));
    }
}

static DOWNLOAD_FAILURES: LazyLock<FailureMemo> = LazyLock::new(|| FailureMemo::new(FAILURE_TTL));

/// Run `download` unless the same pack failed to download within the memo's time to live.
///
/// A transfer or integrity failure is remembered: it is a property of the (set, language) pack,
/// not of the directory. A failure to write the file is not, since another directory may work.
fn download_unless_failed_recently(
    memo: &FailureMemo,
    lang: &str,
    model: TessdataModel,
    now: impl Fn() -> Instant,
    download: impl FnOnce() -> Result<bool, DownloadError>,
) -> Result<bool, OcrError> {
    if let Some(message) = memo.recent(lang, model, now()) {
        return Err(download_failure(
            lang,
            model,
            &format!(
                "{message} (not retried: a failed download is retried after {} s)",
                memo.ttl.as_secs()
            ),
        ));
    }
    download().map_err(|error| {
        let message = error.to_string();
        if !matches!(error, DownloadError::Publish(_)) {
            memo.record(lang, model, now(), message.clone());
        }
        download_failure(lang, model, &message)
    })
}

fn download_failure(lang: &str, model: TessdataModel, cause: &str) -> OcrError {
    OcrError::TesseractInitializationFailed(format!(
        "Failed to download pinned {} language pack '{lang}': {cause}. To provide it without a download, run \
         `xberg cache warm --tessdata-model {model} --tessdata-languages {lang}` where the network is \
         reachable, or point TESSDATA_PREFIX or ocr.tessdata_path at a directory that holds \
         {lang}.traineddata",
        model.repository()
    ))
}

/// Publish a copy of `source` at `output_path` when the bytes read from it are exactly `size`
/// bytes with digest `expected_sha256`.
///
/// The bytes that are hashed are the bytes written, to a temporary file in the target directory
/// that is then published without replacing anything, so a source that is a symlink or changes
/// meanwhile can only ever produce a verified copy.
fn adopt_verified(source: &Path, output_path: &Path, expected_sha256: &str, size: u64) -> bool {
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        // ~keep Check before opening: opening a FIFO or device blocks or has side effects.
        let metadata = std::fs::metadata(source)?;
        if !metadata.is_file() || metadata.len() != size {
            return Ok(None);
        }
        let file = std::fs::File::open(source)?;
        if !file.metadata()?.is_file() {
            return Ok(None);
        }
        let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or_default());
        file.take(size.saturating_add(1)).read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    };
    let bytes = match read() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return false,
        Err(error) if error.kind() == ErrorKind::NotFound => return false,
        Err(error) => {
            tracing::debug!(source = %source.display(), %error, "Cannot read a tessdata file to adopt");
            return false;
        }
    };
    if bytes.len() as u64 != size || hex::encode(Sha256::digest(&bytes)) != expected_sha256 {
        return false;
    }
    match publish(&bytes, output_path) {
        Ok(_) => {
            tracing::debug!(source = %source.display(), target = %output_path.display(), "Adopted verified tessdata");
            true
        }
        Err(error) => {
            tracing::debug!(target = %output_path.display(), %error, "Cannot publish adopted tessdata");
            false
        }
    }
}

/// Returns whether this call published a newly downloaded pack.
fn download_missing_language_pack(lang: &str, model: TessdataModel, output_dir: &Path) -> Result<bool, OcrError> {
    let repository = model.repository();
    let &(_, sha256, size) = pinned_pack(lang, model)
        .ok_or_else(|| OcrError::InvalidLanguageCode(format!("No pinned {repository} language pack for '{lang}'")))?;
    let output_path = output_dir.join(format!("{lang}.traineddata"));
    if output_path.is_file() {
        return Ok(false);
    }

    let urls = [
        pinned_source_url(lang, model),
        format!(
            "https://raw.githubusercontent.com/tesseract-ocr/{repository}/{}/{}.traineddata",
            pinned_revision(model),
            source_language(lang)
        ),
    ];
    download_unless_failed_recently(&DOWNLOAD_FAILURES, lang, model, Instant::now, || {
        download_with_retries(
            &download_agent(),
            [&urls[0], &urls[1]],
            &output_path,
            sha256,
            size,
            std::thread::sleep,
        )
    })
}

fn download_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_resolve(Some(REQUEST_PHASE_TIMEOUT))
        .timeout_connect(Some(REQUEST_PHASE_TIMEOUT))
        .timeout_recv_response(Some(REQUEST_PHASE_TIMEOUT))
        .timeout_per_read(Some(REQUEST_PHASE_TIMEOUT))
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .into()
}

fn download_with_retries(
    agent: &ureq::Agent,
    [primary, mirror]: [&str; 2],
    output_path: &Path,
    sha256: &str,
    size: u64,
    mut sleep: impl FnMut(Duration),
) -> Result<bool, DownloadError> {
    let mut url = primary;
    for (attempt, delay) in RETRY_BACKOFF.into_iter().enumerate() {
        match download_file(agent, url, output_path, sha256, size) {
            Ok(created) => return Ok(created),
            Err(error) if error.is_transient() => {
                tracing::warn!(url, attempt = attempt + 1, %error, "Retrying tessdata download");
                sleep(delay);
            }
            Err(error) if attempt == 0 && error.can_try_mirror() => {
                tracing::warn!(url, %error, "Trying the alternate tessdata endpoint");
            }
            Err(error) => return Err(error),
        }
        url = mirror;
    }
    download_file(agent, mirror, output_path, sha256, size)
}

/// Verify the response before atomically publishing it without replacing an existing pack.
fn download_file(
    agent: &ureq::Agent,
    url: &str,
    output_path: &Path,
    expected_sha256: &str,
    max_bytes: u64,
) -> Result<bool, DownloadError> {
    let response = agent.get(url).call()?;
    if response.status() != 200 {
        return Err(ureq::Error::StatusCode(response.status().as_u16()).into());
    }
    let bytes = response
        .into_body()
        .with_config()
        .limit(max_bytes.min(MAX_TRAINEDDATA_BYTES))
        .read_to_vec()?;
    let actual_sha256 = hex::encode(Sha256::digest(&bytes));
    if actual_sha256 != expected_sha256 {
        return Err(DownloadError::Integrity {
            expected: expected_sha256.to_owned(),
            actual: actual_sha256,
        });
    }

    Ok(publish(&bytes, output_path)?)
}

/// Atomically publish `bytes` at `output_path` without replacing an existing pack. Returns whether
/// this call created the file.
pub(crate) fn publish(bytes: &[u8], output_path: &Path) -> std::io::Result<bool> {
    let directory = output_path
        .parent()
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "missing language pack directory"))?;
    #[cfg(unix)]
    let mut temporary = {
        use std::os::unix::fs::PermissionsExt as _;

        // ~keep Match normal file creation so a shared tessdata cache respects the caller's umask.
        tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o666))
            .tempfile_in(directory)?
    };
    #[cfg(not(unix))]
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(bytes)?;
    match temporary.persist_noclobber(output_path) {
        Ok(_) => Ok(true),
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists && output_path.is_file() => Ok(false),
        Err(error) => Err(error.error),
    }
}

#[cfg(test)]
mod tests {
    use super::{download_agent, download_file};
    use sha2::{Digest as _, Sha256};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::process::{Child, Command, ExitStatus};
    use std::time::{Duration, Instant};

    const CHILD_OUTPUT: &str = "XBERG_TESSDATA_PROXY_TEST_OUTPUT";
    const TEST_NAME: &str =
        "ocr::tessdata_download::tests::should_decode_percent_encoded_proxy_credentials_when_downloading";
    const DOWNLOAD_URL: &str = "http://tessdata.invalid/eng.traineddata";
    const PAYLOAD: &[u8] = b"synthetic traineddata\0\xff";
    const TIMEOUT: Duration = Duration::from_secs(15);
    const POLL_INTERVAL: Duration = Duration::from_millis(10);

    #[test]
    fn pinned_packs_cover_all_supported_languages() {
        use super::{TessdataModel, pinned_pack, pinned_packs};
        use crate::core::config_validation::TESSERACT_LANGUAGE_CODES;

        let extra = ["chi_sim_vert", "chi_tra_vert", "deu_latf", "jpn_vert", "kor_vert"];
        for model in [TessdataModel::Fast, TessdataModel::Best] {
            let packs = pinned_packs(model);
            assert!(packs.windows(2).all(|pair| pair[0].0 < pair[1].0), "{model}");
            for lang in TESSERACT_LANGUAGE_CODES.iter().copied().chain(extra) {
                let (_, digest, size) =
                    pinned_pack(lang, model).unwrap_or_else(|| panic!("missing pinned {model} pack for {lang}"));
                assert_eq!(digest.len(), 64, "{model} {lang}");
                assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()), "{model} {lang}");
                assert!(*size > 0 && *size <= super::MAX_TRAINEDDATA_BYTES, "{model} {lang}");
            }
        }
        let languages = |model| pinned_packs(model).iter().map(|pack| pack.0).collect::<Vec<_>>();
        assert_eq!(languages(TessdataModel::Fast), languages(TessdataModel::Best));
        assert_eq!(
            pinned_pack("eng", TessdataModel::Fast),
            Some(&(
                "eng",
                "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2",
                4_113_088
            ))
        );
        assert_eq!(
            pinned_pack("eng", TessdataModel::Best),
            Some(&(
                "eng",
                "8280aed0782fe27257a68ea10fe7ef324ca0f8d85bd2fd145d1c2b560bcb66ba",
                15_400_601
            ))
        );
        assert_eq!(
            pinned_pack("frk", TessdataModel::Best).map(|pack| (pack.1, pack.2)),
            pinned_pack("deu_latf", TessdataModel::Best).map(|pack| (pack.1, pack.2))
        );
    }

    #[test]
    fn each_model_downloads_from_its_own_pinned_repository() {
        use super::{TessdataModel, pinned_source_url};

        assert_eq!(
            pinned_source_url("eng", TessdataModel::Fast),
            "https://github.com/tesseract-ocr/tessdata_fast/raw/87416418657359cb625c412a48b6e1d6d41c29bd/eng.traineddata"
        );
        assert_eq!(
            pinned_source_url("frk", TessdataModel::Best),
            "https://github.com/tesseract-ocr/tessdata_best/raw/e12c65a915945e4c28e237a9b52bc4a8f39a0cec/deu_latf.traineddata"
        );
    }

    #[test]
    fn only_osd_and_equ_are_shared_between_the_sets() {
        assert_eq!(super::PINNED.shared_languages().collect::<Vec<_>>(), ["equ", "osd"]);
    }

    #[test]
    fn a_pinned_pack_is_adopted_through_its_pin() {
        use super::{PinTables, TessdataModel};

        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("eng.traineddata"), PAYLOAD).unwrap();
        let digest = payload_digest();
        let fast = [("eng", digest.as_str(), PAYLOAD.len() as u64)];
        let wrong_size = [("eng", digest.as_str(), PAYLOAD.len() as u64 + 1)];
        let pins = PinTables {
            fast: &fast,
            best: &wrong_size,
        };
        let sources = [target.path().to_path_buf(), source.path().to_path_buf()];

        assert!(!pins.adopt("eng", TessdataModel::Best, target.path(), &sources));
        assert!(!pins.adopt("deu", TessdataModel::Fast, target.path(), &sources));
        assert_eq!(target.path().read_dir().unwrap().count(), 0);
        assert!(pins.adopt("eng", TessdataModel::Fast, target.path(), &sources));
        assert_eq!(std::fs::read(target.path().join("eng.traineddata")).unwrap(), PAYLOAD);
        assert_eq!(pins.shared_languages().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_fifo_source_is_skipped_without_opening_it() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let fifo = source.path().join("eng.traineddata");
        let status = std::process::Command::new("mkfifo").arg(&fifo).status().unwrap();
        assert!(status.success());

        let output = target.path().join("eng.traineddata");
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done.send(super::adopt_verified(&fifo, &output, &payload_digest(), 0));
        });
        let adopted = finished
            .recv_timeout(TIMEOUT)
            .expect("adopting a FIFO must not block on opening it");
        assert!(!adopted);
        assert_eq!(target.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn a_verified_file_is_adopted_as_an_independent_copy() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let original = source.path().join("eng.traineddata");
        std::fs::write(&original, PAYLOAD).unwrap();
        let output = target.path().join("eng.traineddata");

        assert!(super::adopt_verified(
            &original,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64
        ));

        assert_eq!(std::fs::read(&output).unwrap(), PAYLOAD);
        std::fs::write(&original, b"rewritten later").unwrap();
        assert_eq!(
            std::fs::read(&output).unwrap(),
            PAYLOAD,
            "the adopted file must be a copy"
        );
        assert_eq!(
            target.path().read_dir().unwrap().count(),
            1,
            "no temporary file may remain"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_source_is_copied_not_linked() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let real = source.path().join("real.traineddata");
        std::fs::write(&real, PAYLOAD).unwrap();
        let link = source.path().join("eng.traineddata");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let output = target.path().join("eng.traineddata");

        assert!(super::adopt_verified(
            &link,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64
        ));

        assert!(!output.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(&output).unwrap(), PAYLOAD);
    }

    #[test]
    fn an_unverified_file_is_never_adopted() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let original = source.path().join("eng.traineddata");
        let output = target.path().join("eng.traineddata");
        let wrong_bytes = vec![b'x'; PAYLOAD.len()];
        std::fs::write(&original, &wrong_bytes).unwrap();

        assert!(!super::adopt_verified(
            &original,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64
        ));
        assert!(!super::adopt_verified(
            &source.path().join("missing.traineddata"),
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64
        ));
        assert_eq!(target.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn adoption_never_replaces_an_existing_target() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let original = source.path().join("eng.traineddata");
        std::fs::write(&original, PAYLOAD).unwrap();
        let output = target.path().join("eng.traineddata");
        std::fs::write(&output, b"locally supplied pack").unwrap();

        assert!(super::adopt_verified(
            &original,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64
        ));
        assert_eq!(std::fs::read(&output).unwrap(), b"locally supplied pack");
    }

    #[test]
    fn a_pinned_pack_is_adopted_from_the_first_source_that_matches_its_own_set() {
        use super::{PINNED, TessdataModel};

        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("eng.traineddata"), b"not a pinned model").unwrap();
        let sources = [source.path().to_path_buf()];

        assert!(!PINNED.adopt("eng", TessdataModel::Fast, target.path(), &sources));
        assert!(!PINNED.adopt("eng", TessdataModel::Best, target.path(), &sources));
        assert_eq!(target.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn a_cached_fast_model_is_kept_without_reverification() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("jpn_vert.traineddata");
        std::fs::write(&output, b"locally supplied vertical model").unwrap();
        assert!(!super::ensure_language_pack("jpn_vert", super::TessdataModel::Fast, directory.path(), &[]).unwrap());
        assert_eq!(std::fs::read(output).unwrap(), b"locally supplied vertical model");
        assert_eq!(directory.path().read_dir().unwrap().count(), 1);
    }

    #[test]
    fn an_existing_best_file_that_is_not_the_pinned_pack_is_not_reported_present() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("jpn_vert.traineddata");
        std::fs::write(&output, b"locally supplied vertical model").unwrap();
        let error = super::ensure_language_pack("jpn_vert", super::TessdataModel::Best, directory.path(), &[])
            .expect_err("an unpinned file must not pass for the best pack");
        assert!(error.to_string().contains("tessdata_best"), "{error}");
        assert_eq!(std::fs::read(output).unwrap(), b"locally supplied vertical model");
        assert_eq!(directory.path().read_dir().unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_replacement_that_keeps_size_and_modification_time_is_classified_again() {
        use super::{PinTables, VerifiedSet};

        let directory = tempfile::tempdir().unwrap();
        let fast_digest = hex::encode(Sha256::digest(b"fast english"));
        let fast = [("eng", fast_digest.as_str(), 12)];
        let pins = PinTables { fast: &fast, best: &[] };
        let pack = directory.path().join("eng.traineddata");
        std::fs::write(&pack, b"fast english").unwrap();
        assert_eq!(pins.classify("eng", &pack), VerifiedSet::Fast);

        let modified = std::fs::metadata(&pack).unwrap().modified().unwrap();
        let staged = directory.path().join("staged");
        std::fs::write(&staged, b"user english").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&staged)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        std::fs::rename(&staged, &pack).unwrap();
        let replaced = std::fs::metadata(&pack).unwrap();
        assert_eq!((replaced.len(), replaced.modified().unwrap()), (12, modified));

        assert_eq!(
            pins.classify("eng", &pack),
            VerifiedSet::Unknown,
            "the memo must not vouch for the bytes it read before"
        );
    }

    #[test]
    fn a_pack_is_classified_by_its_pinned_size_and_digest() {
        use super::{PinTables, TessdataModel, VerifiedSet};

        let directory = tempfile::tempdir().unwrap();
        let digest = |bytes: &[u8]| hex::encode(Sha256::digest(bytes));
        let (fast_eng, best_eng, shared) = (digest(b"fast english"), digest(b"best english!"), digest(b"osd"));
        let fast = [("eng", fast_eng.as_str(), 12), ("osd", shared.as_str(), 3)];
        let best = [("eng", best_eng.as_str(), 13), ("osd", shared.as_str(), 3)];
        let pins = PinTables {
            fast: &fast,
            best: &best,
        };
        let path = |lang: &str| directory.path().join(format!("{lang}.traineddata"));
        let classify = |lang: &str, bytes: &[u8]| {
            std::fs::write(path(lang), bytes).unwrap();
            pins.classify(lang, &path(lang))
        };

        assert_eq!(pins.classify("eng", &path("eng")), VerifiedSet::Unknown, "missing");
        assert_eq!(classify("eng", b"fast english"), VerifiedSet::Fast);
        assert_eq!(pins.classify("eng", &path("eng")), VerifiedSet::Fast, "memoized");
        assert_eq!(classify("eng", b"best english!"), VerifiedSet::Best);
        // ~keep Consecutive writes differ in length: two writes within one clock tick of the
        // file system can share a modification time, which is all the memo could tell apart.
        assert_eq!(
            classify("eng", b"user trained"),
            VerifiedSet::Unknown,
            "pinned size, other bytes"
        );
        assert_eq!(classify("eng", b"BEST ENGLISH!"), VerifiedSet::Unknown);
        assert_eq!(classify("eng", b""), VerifiedSet::Unknown);
        assert_eq!(classify("osd", b"osd"), VerifiedSet::Shared);
        assert_eq!(classify("custom", b"fast english"), VerifiedSet::Unknown, "no pin");
        std::fs::create_dir(path("deu")).unwrap();
        assert_eq!(pins.classify("deu", &path("deu")), VerifiedSet::Unknown, "not a file");

        assert!(VerifiedSet::Shared.is(TessdataModel::Best) && VerifiedSet::Shared.is(TessdataModel::Fast));
        assert!(VerifiedSet::Best.is(TessdataModel::Best) && !VerifiedSet::Best.is(TessdataModel::Fast));
        assert!(!VerifiedSet::Fast.is(TessdataModel::Best) && !VerifiedSet::Unknown.is(TessdataModel::Fast));
    }

    #[test]
    fn only_a_wrong_file_for_a_pinned_language_is_set_aside() {
        use super::{PinTables, SetAside, TessdataModel};

        let directory = tempfile::tempdir().unwrap();
        let digest = |bytes: &[u8]| hex::encode(Sha256::digest(bytes));
        let (fast_pack, best_pack) = (digest(b"fast"), digest(b"best!"));
        let fast = [("eng", fast_pack.as_str(), 4)];
        let best = [("eng", best_pack.as_str(), 5)];
        let pins = PinTables {
            fast: &fast,
            best: &best,
        };
        let pack = |lang: &str| directory.path().join(format!("{lang}.traineddata"));
        let set_aside = |lang: &str| {
            pins.set_aside_unverified(lang, TessdataModel::Best, directory.path())
                .unwrap()
        };
        let set_aside_files = || {
            let mut contents: Vec<Vec<u8>> = directory
                .path()
                .read_dir()
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.to_string_lossy().contains(".traineddata.unverified"))
                .map(|path| std::fs::read(path).unwrap())
                .collect();
            contents.sort();
            contents
        };

        assert_eq!(set_aside("eng"), SetAside::Untouched, "nothing there");
        std::fs::write(pack("custom"), b"user trained").unwrap();
        assert_eq!(set_aside("custom"), SetAside::Untouched, "no pin to hold it to");
        std::fs::write(pack("eng"), b"best!").unwrap();
        assert_eq!(set_aside("eng"), SetAside::Untouched, "the pinned pack stays");
        assert_eq!(std::fs::read(pack("eng")).unwrap(), b"best!");

        std::fs::remove_file(pack("eng")).unwrap();
        std::fs::write(pack("eng"), b"fast").unwrap();
        assert_eq!(set_aside("eng"), SetAside::Moved);
        assert!(!pack("eng").exists());
        assert_eq!(
            std::fs::read(directory.path().join("eng.traineddata.unverified")).unwrap(),
            b"fast"
        );
        std::fs::write(pack("eng"), b"").unwrap();
        assert_eq!(
            set_aside("eng"),
            SetAside::Moved,
            "an earlier set-aside file does not block the next"
        );
        assert!(!pack("eng").exists());
        assert_eq!(
            set_aside_files(),
            [b"".to_vec(), b"fast".to_vec()],
            "the earlier set-aside file is kept, not overwritten"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_wrong_file_in_a_directory_that_cannot_be_written_stays() {
        use super::{PinTables, SetAside, TessdataModel};
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().unwrap();
        let best_pack = hex::encode(Sha256::digest(b"best!"));
        let best = [("eng", best_pack.as_str(), 5)];
        let pins = PinTables { fast: &[], best: &best };
        let pack = directory.path().join("eng.traineddata");
        std::fs::write(&pack, b"fast").unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let writable = std::fs::write(directory.path().join("probe"), b"").is_ok();

        let outcome = pins.set_aside_unverified("eng", TessdataModel::Best, directory.path());
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        if writable {
            eprintln!("SKIP: permissions do not restrict this user");
            return;
        }
        assert_eq!(outcome.unwrap(), SetAside::Blocked);
        assert_eq!(std::fs::read(&pack).unwrap(), b"fast");
    }

    #[test]
    fn a_directory_is_verified_only_when_every_named_pack_is_of_one_set() {
        use super::{PinTables, TessdataModel};

        let directory = tempfile::tempdir().unwrap();
        let digest = |bytes: &[u8]| hex::encode(Sha256::digest(bytes));
        let (fast_pack, best_pack, shared) = (digest(b"fast"), digest(b"best!"), digest(b"osd"));
        let fast = [
            ("deu", fast_pack.as_str(), 4),
            ("eng", fast_pack.as_str(), 4),
            ("osd", shared.as_str(), 3),
        ];
        let best = [
            ("deu", best_pack.as_str(), 5),
            ("eng", best_pack.as_str(), 5),
            ("osd", shared.as_str(), 3),
        ];
        let pins = PinTables {
            fast: &fast,
            best: &best,
        };
        let write = |lang: &str, bytes: &[u8]| {
            std::fs::write(directory.path().join(format!("{lang}.traineddata")), bytes).unwrap();
        };
        let verified = |languages: &[&str]| pins.verified_model(directory.path(), languages);

        write("eng", b"best!");
        write("osd", b"osd");
        assert_eq!(verified(&["eng"]), Some(TessdataModel::Best));
        assert_eq!(verified(&["eng", "osd"]), Some(TessdataModel::Best));
        assert_eq!(verified(&["osd"]), None, "shared packs name no set");
        assert_eq!(verified(&["eng", "deu"]), None, "a missing pack");
        write("deu", b"fast");
        assert_eq!(verified(&["eng", "deu"]), None, "mixed sets");
        assert_eq!(verified(&["deu"]), Some(TessdataModel::Fast));
        write("deu", b"user trained");
        assert_eq!(verified(&["deu"]), None);
    }

    #[test]
    fn a_failed_download_is_not_repeated_until_its_memo_expires() {
        use super::{DownloadError, FailureMemo, TessdataModel, download_unless_failed_recently};

        let memo = FailureMemo::new(Duration::from_secs(60));
        let start = Instant::now();
        let clock = std::cell::Cell::new(start);
        let calls = std::cell::Cell::new(0_u32);
        let fetch = |lang: &str, model, outcome: fn() -> Result<bool, DownloadError>| {
            download_unless_failed_recently(
                &memo,
                lang,
                model,
                || clock.get(),
                || {
                    calls.set(calls.get() + 1);
                    outcome()
                },
            )
        };
        fn unreachable() -> Result<bool, DownloadError> {
            Err(DownloadError::Request(ureq::Error::ConnectionFailed))
        }

        for _ in 0..5 {
            let message = fetch("eng", TessdataModel::Best, unreachable).unwrap_err().to_string();
            for remedy in [
                "xberg cache warm --tessdata-model best --tessdata-languages eng",
                "TESSDATA_PREFIX",
                "ocr.tessdata_path",
            ] {
                assert!(message.contains(remedy), "{message}");
            }
        }
        assert_eq!(calls.get(), 1, "one failure answers the following requests");

        assert!(fetch("eng", TessdataModel::Fast, unreachable).is_err());
        assert!(fetch("deu", TessdataModel::Best, unreachable).is_err());
        assert_eq!(calls.get(), 3, "the memo is per set and language");

        clock.set(start + Duration::from_secs(59));
        assert!(fetch("eng", TessdataModel::Best, unreachable).is_err());
        assert_eq!(calls.get(), 3);
        clock.set(start + Duration::from_secs(60));
        assert!(fetch("eng", TessdataModel::Best, || Ok(true)).unwrap());
        assert_eq!(calls.get(), 4, "the download is tried again once the memo expires");
    }

    #[test]
    fn a_failure_to_write_the_pack_is_not_remembered() {
        use super::{DownloadError, FailureMemo, TessdataModel, download_unless_failed_recently};

        let memo = FailureMemo::new(Duration::from_secs(60));
        let now = Instant::now();
        let read_only = || {
            Err(DownloadError::Publish(std::io::Error::from(
                std::io::ErrorKind::PermissionDenied,
            )))
        };

        assert!(download_unless_failed_recently(&memo, "eng", TessdataModel::Best, || now, read_only).is_err());
        assert!(
            download_unless_failed_recently(&memo, "eng", TessdataModel::Best, || now, || Ok(true)).unwrap(),
            "another directory may be writable"
        );
    }

    #[test]
    fn concurrent_requests_for_one_pack_materialize_it_once() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("eng.traineddata");
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let start = std::sync::Barrier::new(8);

        let published: Vec<bool> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        super::materialize_once(&output, || {
                            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(50));
                            std::fs::write(&output, PAYLOAD).unwrap();
                            Ok(true)
                        })
                        .unwrap()
                    })
                })
                .collect();
            workers.into_iter().map(|worker| worker.join().unwrap()).collect()
        });

        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(published.iter().filter(|created| **created).count(), 1);
    }

    #[test]
    fn transport_timeouts_allow_a_slow_bounded_transfer() {
        let agent = download_agent();
        let timeouts = agent.config().timeouts();
        assert_eq!(timeouts.resolve, Some(Duration::from_secs(30)));
        assert_eq!(timeouts.connect, Some(Duration::from_secs(30)));
        assert_eq!(timeouts.recv_response, Some(Duration::from_secs(30)));
        assert_eq!(agent.config().timeout_per_read(), Some(Duration::from_secs(30)));
        assert_eq!(timeouts.global, Some(Duration::from_secs(600)));
    }

    #[test]
    fn transient_statuses_back_off_and_stop_after_three_attempts() {
        for final_status in [200, 503] {
            let directory = tempfile::tempdir().unwrap();
            let output = directory.path().join("eng.traineddata");
            let (primary, first) = serve_test_statuses(&[503]);
            let (mirror, retries) = serve_test_statuses(&[503, final_status]);
            let mut backoffs = Vec::new();
            let result = super::download_with_retries(
                &local_agent(TIMEOUT),
                [&primary, &mirror],
                &output,
                &payload_digest(),
                PAYLOAD.len() as u64,
                |delay| backoffs.push(delay),
            );
            first.join().unwrap();
            retries.join().unwrap();
            assert_eq!(backoffs, [Duration::from_secs(1), Duration::from_secs(2)]);
            if final_status == 200 {
                assert!(result.unwrap());
                assert_eq!(std::fs::read(output).unwrap(), PAYLOAD);
            } else {
                assert!(matches!(
                    result,
                    Err(super::DownloadError::Request(ureq::Error::StatusCode(503)))
                ));
                assert_eq!(directory.path().read_dir().unwrap().count(), 0);
            }
        }
    }

    #[test]
    fn a_checksum_mismatch_never_reaches_the_mirror_or_backoff() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("eng.traineddata");
        let (primary, server) = serve_test_payload(PAYLOAD);
        let result = super::download_with_retries(
            &local_agent(TIMEOUT),
            [&primary, &primary],
            &output,
            &"0".repeat(64),
            PAYLOAD.len() as u64,
            |_| panic!("checksum mismatches cannot be retried"),
        );
        server.join().unwrap();
        assert!(matches!(result, Err(super::DownloadError::Integrity { .. })));
        assert_eq!(directory.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn refuses_bad_checksum_before_publishing_download() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("eng.traineddata");
        let (url, server) = serve_test_payload(PAYLOAD);
        let result = download_file(
            &local_agent(TIMEOUT),
            &url,
            &output,
            &"0".repeat(64),
            PAYLOAD.len() as u64,
        );
        server.join().unwrap();
        let error = result.expect_err("digest mismatch must fail");
        assert!(matches!(error, super::DownloadError::Integrity { .. }), "{error}");
        assert_eq!(directory.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn does_not_replace_existing_pack_when_publishing_download() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("eng.traineddata");
        std::fs::write(&output, b"locally supplied pack").unwrap();
        let (url, server) = serve_test_payload(PAYLOAD);
        let created = download_file(
            &local_agent(TIMEOUT),
            &url,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64,
        )
        .unwrap();
        server.join().unwrap();
        assert!(!created);
        assert_eq!(std::fs::read(output).unwrap(), b"locally supplied pack");
        assert_eq!(directory.path().read_dir().unwrap().count(), 1);
    }

    #[test]
    fn stalled_response_body_times_out_without_publishing() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("eng.traineddata");
        let (release, wait) = std::sync::mpsc::channel::<()>();
        let (url, server) = serve_test_response(PAYLOAD.len(), move |_| {
            let _ = wait.recv_timeout(TIMEOUT);
        });
        let result = download_file(
            &local_agent(Duration::from_millis(500)),
            &url,
            &output,
            &payload_digest(),
            PAYLOAD.len() as u64,
        );
        drop(release);
        server.join().unwrap();
        let error = result.expect_err("stalled body must time out");
        assert!(
            matches!(
                error,
                super::DownloadError::Request(ureq::Error::Timeout(ureq::Timeout::PerRead))
            ),
            "{error}"
        );
        assert_eq!(directory.path().read_dir().unwrap().count(), 0);
    }

    fn local_agent(timeout: Duration) -> ureq::Agent {
        ureq::Agent::config_builder()
            .proxy(None)
            .timeout_per_read(Some(timeout))
            .timeout_global(Some(TIMEOUT))
            .build()
            .into()
    }

    fn payload_digest() -> String {
        hex::encode(Sha256::digest(PAYLOAD))
    }

    fn serve_test_statuses(statuses: &[u16]) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/eng.traineddata", listener.local_addr().unwrap());
        let statuses = statuses.to_vec();
        let server = std::thread::spawn(move || {
            for status in statuses {
                let mut stream = accept_with_deadline(&listener);
                let _request = read_request(&stream);
                let payload = if status == 200 { PAYLOAD } else { b"" };
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                )
                .unwrap();
                stream.write_all(payload).unwrap();
            }
        });
        (url, server)
    }

    fn serve_test_payload(payload: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
        serve_test_response(payload.len(), |stream| stream.write_all(payload).unwrap())
    }

    fn serve_test_response(
        payload_size: usize,
        send_body: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/eng.traineddata", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut stream = accept_with_deadline(&listener);
            let _request = read_request(&stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {payload_size}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            send_body(&mut stream);
        });
        (url, server)
    }

    #[test]
    fn should_decode_percent_encoded_proxy_credentials_when_downloading() {
        if let Some(output_path) = std::env::var_os(CHILD_OUTPUT) {
            let output_path = std::path::PathBuf::from(output_path);
            let created = download_file(
                &download_agent(),
                DOWNLOAD_URL,
                &output_path,
                &payload_digest(),
                PAYLOAD.len() as u64,
            )
            .expect("download through authenticated proxy");
            assert!(created);
            assert_eq!(std::fs::read(&output_path).unwrap(), PAYLOAD);
            assert_eq!(output_path.parent().unwrap().read_dir().unwrap().count(), 1);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;

                assert_eq!(output_path.metadata().unwrap().permissions().mode() & 0o777, 0o640);
            }
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let directory = tempfile::tempdir().unwrap();
        let output_path = directory.path().join("eng.traineddata");
        let proxy = format!("http://user%25name:p%25ass%3Aword@{}", listener.local_addr().unwrap());
        let mut child = proxy_child(&proxy, &output_path);
        let request = std::panic::catch_unwind(|| serve_proxy(&listener));
        if request.is_err() {
            let _ = child.kill();
        }
        let status = wait_for_child(&mut child);
        let (request, tunneled_request) = request.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        assert!(status.success(), "isolated downloader failed: {status}");
        assert_eq!(std::fs::read(output_path).unwrap(), PAYLOAD);
        assert_eq!(request[0], "CONNECT tessdata.invalid:80 HTTP/1.1");
        assert_eq!(tunneled_request[0], "GET /eng.traineddata HTTP/1.1");
        let authorization = request.iter().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("proxy-authorization").then_some(value.trim())
        });
        assert_eq!(authorization, Some("Basic dXNlciVuYW1lOnAlYXNzOndvcmQ="));
    }

    fn proxy_child(proxy: &str, output_path: &std::path::Path) -> Child {
        // ~keep A child isolates proxy environment and umask changes from other tests.
        let test_binary = std::env::current_exe().unwrap();
        #[cfg(unix)]
        let mut command = {
            let mut shell = Command::new("sh");
            shell.args(["-c", r#"umask 027 && exec "$@""#, "tessdata-proxy-test"]);
            shell.arg(test_binary);
            shell
        };
        #[cfg(not(unix))]
        let mut command = Command::new(test_binary);
        command.args(["--exact", TEST_NAME, "--nocapture"]);
        for name in ["HTTP_PROXY", "http_proxy"] {
            command.env(name, proxy);
        }
        for name in ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
            command.env_remove(name);
        }
        command.env("NO_PROXY", "").env("no_proxy", "");
        command.env(CHILD_OUTPUT, output_path).spawn().unwrap()
    }

    fn accept_with_deadline(listener: &TcpListener) -> std::net::TcpStream {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + TIMEOUT;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "downloader did not connect");
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(error) => panic!("accept proxy connection: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream.set_read_timeout(Some(TIMEOUT)).unwrap();
        stream.set_write_timeout(Some(TIMEOUT)).unwrap();
        stream
    }

    fn serve_proxy(listener: &TcpListener) -> (Vec<String>, Vec<String>) {
        let mut stream = accept_with_deadline(listener);
        let request = read_request(&stream);
        stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .unwrap();
        let tunneled_request = read_request(&stream);
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            PAYLOAD.len()
        )
        .into_bytes();
        response.extend_from_slice(PAYLOAD);
        stream.write_all(&response).unwrap();
        (request, tunneled_request)
    }

    fn read_request(stream: &std::net::TcpStream) -> Vec<String> {
        let mut reader = BufReader::new(stream);
        let mut request = Vec::new();
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0, "incomplete proxy request");
            if line == "\r\n" {
                return request;
            }
            request.push(line.trim_end().to_owned());
        }
    }

    fn wait_for_child(child: &mut Child) -> ExitStatus {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated downloader exceeded timeout");
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}
