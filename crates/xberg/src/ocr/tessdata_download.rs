//! Tessdata language pack download utilities for runtime language pack materialization.
//!
//! Provides functions to download `.traineddata` files from the official Tesseract
//! tessdata_fast repository on GitHub, with fallback URLs for redundancy.

#![cfg(not(target_arch = "wasm32"))]

use crate::ocr::error::OcrError;
use sha2::{Digest as _, Sha256};
use std::io::{ErrorKind, Write as _};
use std::path::Path;
use std::time::Duration;

mod pins;

pub(super) use pins::TESSDATA_FAST_PACKS;
use pins::{PinnedPack, TESSDATA_FAST_REVISION};

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TRAINEDDATA_BYTES: u64 = 64 * 1024 * 1024;

fn pinned_pack(lang: &str) -> Option<&'static PinnedPack> {
    let index = TESSDATA_FAST_PACKS.binary_search_by(|entry| entry.0.cmp(lang)).ok()?;
    Some(&TESSDATA_FAST_PACKS[index])
}

fn source_language(lang: &str) -> &str {
    // ~keep Upstream frk.traineddata is a Git symlink. Fetch its actual model bytes.
    if lang == "frk" { "deu_latf" } else { lang }
}

pub(super) fn pinned_source_url(lang: &str) -> String {
    format!(
        "https://github.com/tesseract-ocr/tessdata_fast/raw/{TESSDATA_FAST_REVISION}/{}.traineddata",
        source_language(lang)
    )
}

pub(crate) fn download_language_pack(lang: &str, output_dir: &Path) -> Result<(), OcrError> {
    download_missing_language_pack(lang, output_dir).map(|_| ())
}

/// Returns whether this call published a newly downloaded pack.
pub(super) fn download_missing_language_pack(lang: &str, output_dir: &Path) -> Result<bool, OcrError> {
    let &(_, sha256, size) = pinned_pack(lang)
        .ok_or_else(|| OcrError::InvalidLanguageCode(format!("No pinned tessdata_fast language pack for '{lang}'")))?;
    let output_path = output_dir.join(format!("{lang}.traineddata"));
    if output_path.is_file() {
        return Ok(false);
    }

    let urls = [
        pinned_source_url(lang),
        format!(
            "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{TESSDATA_FAST_REVISION}/{}.traineddata",
            source_language(lang)
        ),
    ];
    let agent = download_agent(DOWNLOAD_TIMEOUT);
    let mut failures = Vec::new();
    for url in &urls {
        match download_file(&agent, url, &output_path, sha256, size) {
            Ok(created) => return Ok(created),
            Err(error) => {
                tracing::warn!(lang, url, %error, "Tesseract language pack download failed");
                failures.push(format!("{url}: {error}"));
            }
        }
    }
    Err(OcrError::TesseractInitializationFailed(format!(
        "Failed to download pinned tessdata_fast language pack '{lang}': {}",
        failures.join("; ")
    )))
}

fn download_agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

/// Verify the response before atomically publishing it without replacing an existing pack.
fn download_file(
    agent: &ureq::Agent,
    url: &str,
    output_path: &Path,
    expected_sha256: &str,
    max_bytes: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let response = agent.get(url).call()?;
    if response.status() != 200 {
        return Err(format!("HTTP {}", response.status()).into());
    }
    let bytes = response
        .into_body()
        .with_config()
        .limit(max_bytes.min(MAX_TRAINEDDATA_BYTES))
        .read_to_vec()?;
    let actual_sha256 = hex::encode(Sha256::digest(&bytes));
    if actual_sha256 != expected_sha256 {
        return Err(format!("SHA-256 mismatch: expected {expected_sha256}, got {actual_sha256}").into());
    }

    let directory = output_path.parent().ok_or("Missing language pack directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(&bytes)?;
    match temporary.persist_noclobber(output_path) {
        Ok(_) => Ok(true),
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists && output_path.is_file() => Ok(false),
        Err(error) => Err(error.into()),
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
        use super::{TESSDATA_FAST_PACKS, pinned_pack};
        use crate::core::config_validation::TESSERACT_LANGUAGE_CODES;

        assert_eq!(TESSDATA_FAST_PACKS.len(), TESSERACT_LANGUAGE_CODES.len());
        assert!(TESSDATA_FAST_PACKS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for lang in TESSERACT_LANGUAGE_CODES {
            let (_, digest, size) = pinned_pack(lang).unwrap_or_else(|| panic!("missing pinned pack for {lang}"));
            assert_eq!(digest.len(), 64, "{lang}");
            assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()), "{lang}");
            assert!(*size > 0 && *size <= super::MAX_TRAINEDDATA_BYTES, "{lang}");
        }
        assert_eq!(
            pinned_pack("eng"),
            Some(&(
                "eng",
                "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2",
                4_113_088
            ))
        );
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
        assert!(error.to_string().contains("SHA-256 mismatch"), "{error}");
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
            matches!(error.downcast_ref::<ureq::Error>(), Some(ureq::Error::Timeout(_))),
            "{error}"
        );
        assert_eq!(directory.path().read_dir().unwrap().count(), 0);
    }

    fn local_agent(timeout: Duration) -> ureq::Agent {
        ureq::Agent::config_builder()
            .proxy(None)
            .timeout_global(Some(timeout))
            .build()
            .into()
    }

    fn payload_digest() -> String {
        hex::encode(Sha256::digest(PAYLOAD))
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
                &download_agent(TIMEOUT),
                DOWNLOAD_URL,
                &output_path,
                &payload_digest(),
                PAYLOAD.len() as u64,
            )
            .expect("download through authenticated proxy");
            assert!(created);
            assert_eq!(std::fs::read(&output_path).unwrap(), PAYLOAD);
            assert_eq!(output_path.parent().unwrap().read_dir().unwrap().count(), 1);
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
        // ~keep A child isolates proxy environment changes from concurrently running tests.
        let mut command = Command::new(std::env::current_exe().unwrap());
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
