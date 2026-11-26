//! Tessdata language pack download utilities for runtime language pack materialization.
//!
//! Provides functions to download `.traineddata` files from the official Tesseract
//! tessdata_fast and tessdata_best repositories on GitHub, pinned to one revision each and
//! SHA-256 verified, with fallback URLs for redundancy.

#![cfg(not(target_arch = "wasm32"))]

use crate::core::config::TessdataModel;
use crate::ocr::error::OcrError;
use sha2::{Digest as _, Sha256};
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

mod pins;

use pins::{PinnedPack, TESSDATA_BEST_PACKS, TESSDATA_BEST_REVISION, TESSDATA_FAST_PACKS, TESSDATA_FAST_REVISION};

const REQUEST_PHASE_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const RETRY_BACKOFF: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(2)];
const MAX_TRAINEDDATA_BYTES: u64 = 64 * 1024 * 1024;

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

/// The pinned language packs of `model`, sorted by language code.
pub(crate) fn pinned_packs(model: TessdataModel) -> &'static [PinnedPack] {
    match model {
        TessdataModel::Fast => TESSDATA_FAST_PACKS,
        TessdataModel::Best => TESSDATA_BEST_PACKS,
    }
}

fn pinned_revision(model: TessdataModel) -> &'static str {
    match model {
        TessdataModel::Fast => TESSDATA_FAST_REVISION,
        TessdataModel::Best => TESSDATA_BEST_REVISION,
    }
}

fn pinned_pack(lang: &str, model: TessdataModel) -> Option<&'static PinnedPack> {
    let packs = pinned_packs(model);
    let index = packs.binary_search_by(|entry| entry.0.cmp(lang)).ok()?;
    Some(&packs[index])
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

/// The languages whose `fast` and `best` packs are byte-identical (`equ`, `osd`).
pub(crate) fn shared_languages() -> impl Iterator<Item = &'static str> {
    TESSDATA_BEST_PACKS
        .iter()
        .filter(|best| pinned_pack(best.0, TessdataModel::Fast).is_some_and(|fast| fast == *best))
        .map(|pack| pack.0)
}

/// Make the pinned `model` pack for `lang` present in `output_dir`: keep an existing file, else
/// copy a verified one from the first of `sources` holding it, else download it.
///
/// Returns whether this call published a new file.
pub(crate) fn ensure_language_pack(
    lang: &str,
    model: TessdataModel,
    output_dir: &Path,
    sources: &[PathBuf],
) -> Result<bool, OcrError> {
    if output_dir.join(format!("{lang}.traineddata")).is_file() {
        return Ok(false);
    }
    if adopt_pinned_pack(lang, model, output_dir, sources) {
        return Ok(true);
    }
    download_missing_language_pack(lang, model, output_dir)
}

/// Copy the pinned `model` pack for `lang` into `output_dir` from the first of `sources` that
/// holds exactly those bytes. Never downloads. Returns whether `output_dir` now holds the pack.
pub(crate) fn adopt_pinned_pack(lang: &str, model: TessdataModel, output_dir: &Path, sources: &[PathBuf]) -> bool {
    let Some(&(_, sha256, size)) = pinned_pack(lang, model) else {
        return false;
    };
    let file_name = format!("{lang}.traineddata");
    let output_path = output_dir.join(&file_name);
    sources
        .iter()
        .filter(|source| source.as_path() != output_dir)
        .any(|source| adopt_verified(&source.join(&file_name), &output_path, sha256, size))
}

/// Publish a copy of `source` at `output_path` when the bytes read from it are exactly `size`
/// bytes with digest `expected_sha256`.
///
/// The bytes that are hashed are the bytes written, to a temporary file in the target directory
/// that is then published without replacing anything, so a source that is a symlink or changes
/// meanwhile can only ever produce a verified copy.
fn adopt_verified(source: &Path, output_path: &Path, expected_sha256: &str, size: u64) -> bool {
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        let file = std::fs::File::open(source)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() != size {
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
    download_with_retries(
        &download_agent(),
        [&urls[0], &urls[1]],
        &output_path,
        sha256,
        size,
        std::thread::sleep,
    )
    .map_err(|error| {
        OcrError::TesseractInitializationFailed(format!(
            "Failed to download pinned {repository} language pack '{lang}': {error}"
        ))
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
fn publish(bytes: &[u8], output_path: &Path) -> std::io::Result<bool> {
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
        assert_eq!(super::shared_languages().collect::<Vec<_>>(), ["equ", "osd"]);
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
        use super::{TessdataModel, adopt_pinned_pack};

        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("eng.traineddata"), b"not a pinned model").unwrap();
        let sources = [source.path().to_path_buf()];

        assert!(!adopt_pinned_pack("eng", TessdataModel::Fast, target.path(), &sources));
        assert!(!adopt_pinned_pack("eng", TessdataModel::Best, target.path(), &sources));
        assert_eq!(target.path().read_dir().unwrap().count(), 0);
    }

    #[test]
    fn a_cached_vertical_model_is_kept_without_reverification() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("jpn_vert.traineddata");
        std::fs::write(&output, b"locally supplied vertical model").unwrap();
        assert!(!super::ensure_language_pack("jpn_vert", super::TessdataModel::Best, directory.path(), &[]).unwrap());
        assert_eq!(std::fs::read(output).unwrap(), b"locally supplied vertical model");
        assert_eq!(directory.path().read_dir().unwrap().count(), 1);
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
