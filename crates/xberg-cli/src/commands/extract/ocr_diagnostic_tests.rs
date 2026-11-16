use super::{ExtractInputSource, RUNTIME_WORKER_STACK_SIZE_BYTES, extract_input_sync};
use std::error::Error as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, XbergError};

const BACKEND_NAME: &str = "cli-diagnostic-failing-ocr";
const PATH_SENTINEL: &str = "/synthetic/OCR_PATH_SENTINEL/model.onnx";
const SECRET_SENTINEL: &str = "OCR_SECRET_SENTINEL";

#[derive(Default)]
struct FailingOcrBackend {
    calls: AtomicUsize,
}

impl Plugin for FailingOcrBackend {
    fn name(&self) -> &str {
        BACKEND_NAME
    }
}

#[async_trait::async_trait]
impl OcrBackend for FailingOcrBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, image: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        assert!(!image.is_empty(), "the backend needs a rendered page");
        self.calls.fetch_add(1, Ordering::Relaxed);
        Err(XbergError::ocr_with_source(
            PATH_SENTINEL,
            std::io::Error::other(SECRET_SENTINEL),
        ))
    }
}

fn extract_backend_failure() -> (XbergError, anyhow::Error) {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../xberg/tests/fixtures/ocr/scanned_hello.pdf"
    );
    assert!(
        std::path::Path::new(fixture).is_file(),
        "tracked scanned PDF: {fixture}"
    );
    let backend = Arc::new(FailingOcrBackend::default());
    register_ocr_backend(backend.clone()).expect("register the diagnostic test backend");
    let config = ExtractionConfig {
        use_cache: false,
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: BACKEND_NAME.into(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_stack_size(RUNTIME_WORKER_STACK_SIZE_BYTES)
        .enable_all()
        .build()
        .expect("raw extraction runtime");
    let raw = runtime.block_on(xberg::extract(ExtractInput::from_uri(fixture), &config));
    drop(runtime);
    let raw_calls = backend.calls.load(Ordering::Relaxed);
    let cli = extract_input_sync(ExtractInputSource::Uri(fixture.into()), None, &config);
    let total_calls = backend.calls.load(Ordering::Relaxed);
    unregister_ocr_backend(BACKEND_NAME).expect("unregister the diagnostic test backend");

    assert!(
        raw_calls > 0 && total_calls > raw_calls,
        "both paths must invoke the backend"
    );
    (
        raw.expect_err("the raw PDF OCR call must fail"),
        cli.expect_err("the CLI PDF OCR call must fail"),
    )
}

fn contains_backend_diagnostics(error: &anyhow::Error) -> [(bool, bool); 2] {
    [format!("{error:?}"), format!("{error:#}")]
        .map(|text| (text.contains(PATH_SENTINEL), text.contains(SECRET_SENTINEL)))
}

#[test]
fn cli_pdf_ocr_failure_omits_backend_diagnostics_in_both_formats() {
    let (raw, cli) = std::thread::Builder::new()
        .stack_size(RUNTIME_WORKER_STACK_SIZE_BYTES)
        .spawn(extract_backend_failure)
        .expect("start the extraction test with the CLI stack budget")
        .join()
        .expect("extraction test thread must complete");
    assert!(matches!(&raw, XbergError::Ocr { .. }), "{raw:?}");
    let raw_debug = format!("{raw:?}");
    assert!(raw_debug.contains(PATH_SENTINEL) && raw_debug.contains(SECRET_SENTINEL));
    assert!(raw.to_string().contains(PATH_SENTINEL));
    let raw = anyhow::Error::new(raw);
    let inner_io = raw.chain().find_map(|error| {
        let XbergError::Ocr {
            message,
            source: Some(source),
        } = error.downcast_ref::<XbergError>()?
        else {
            return None;
        };
        if message != PATH_SENTINEL {
            return None;
        }
        source.downcast_ref::<std::io::Error>()
    });
    assert_eq!(
        inner_io
            .expect("raw chain retains the typed backend OCR and I/O errors")
            .to_string(),
        SECRET_SENTINEL
    );
    assert_eq!(contains_backend_diagnostics(&raw), [(true, true); 2]);

    assert_eq!(contains_backend_diagnostics(&cli), [(false, false); 2]);
    let projected = cli
        .downcast_ref::<XbergError>()
        .expect("CLI retains the typed aggregate");
    let summary = projected.to_string();
    let expected = [
        "OCR error: OCR failed on all 1 page(s) and no text could be recovered from the pages' embedded images",
        concat!(
            "OCR error: OCR failed on all 1 page(s). ",
            "The embedded-image retry ran on 1 of these page(s) and also returned no text."
        ),
    ];
    assert!(expected.contains(&summary.as_str()), "{summary}");
    assert!(projected.source().is_none());
}
