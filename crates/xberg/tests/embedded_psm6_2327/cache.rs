use crate as xberg;

#[path = "fixture.rs"]
mod fixture;
use fixture::{FIRST, LAST, MISSING, NATIVE, config, extract, extract_input, panel_pdf};
use xberg::{ExtractInput, ExtractedDocument, OcrConfig};

#[tokio::test]
async fn cached_default_recovers_embedded_line_in_cold_and_warm_reads() {
    const CHILD: &str = "XBERG_EMBEDDED_PROSE_2327_CACHE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        run_isolated_cache_test(CHILD);
        return;
    }

    use tracing_subscriber::layer::SubscriberExt as _;
    let lookups = CacheLookups::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(lookups.clone())).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cached-panel.pdf");
    let bytes = panel_pdf(false, false);
    std::fs::write(&path, &bytes).unwrap();

    let mut cached = config(None, true);
    cached.ocr = Some(OcrConfig::default());
    let expected = format!("{}\n\n{FIRST}\n\n{MISSING}\n\n{LAST}", NATIVE.join("\n\n"));
    let cold = extract(bytes.clone(), &cached).await;
    assert_eq!(cold.content, expected);
    assert_eq!(cold.pages.as_deref().unwrap()[0].content, expected);
    let cold_lookups = lookups.entries();
    let [(key, false)] = cold_lookups.as_slice() else {
        panic!("one cold primary OCR lookup: {cold_lookups:?}");
    };
    let inner_warm = extract(bytes, &cached).await;
    assert_eq!(
        serde_json::to_value(&cold).unwrap(),
        serde_json::to_value(inner_warm).unwrap()
    );
    assert_eq!(lookups.entries(), [(key.clone(), false), (key.clone(), true)]);

    let uri = ExtractInput::from_uri(path.to_string_lossy().into_owned());
    let uri_cold = extract_input(uri.clone(), &cached).await;
    assert_eq!(uri_cold.content, expected);
    assert_eq!(uri_cold.pages.as_deref().unwrap()[0].content, expected);
    let mut before_outer_warm = vec![(key.clone(), false), (key.clone(), true), (key.clone(), true)];
    assert_eq!(lookups.entries(), before_outer_warm);
    let outer_warm = extract_input(uri, &cached).await;
    assert_eq!(
        serde_json::to_value(&uri_cold).unwrap(),
        serde_json::to_value(outer_warm).unwrap()
    );
    if std::env::var_os(CHILD).as_deref() == Some(std::ffi::OsStr::new("retry-available")) {
        before_outer_warm.push((key.clone(), true));
    }
    assert_eq!(
        lookups.entries(),
        before_outer_warm,
        "outer warm read follows retry eligibility"
    );

    cached.images.as_mut().unwrap().inject_placeholders = true;
    let reference = extract(panel_pdf(false, false), &cached).await;
    let nested = |doc: &ExtractedDocument| {
        serde_json::to_value(doc.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    };
    assert_eq!(nested(&cold), nested(&reference));
    assert_eq!(nested(&uri_cold), nested(&reference));
    assert!(!reference.content.contains(MISSING));

    cached.images = None;
    let ordinary = extract(panel_pdf(false, false), &cached).await;
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.pages.as_deref().unwrap()[0].content, expected);
}

fn run_isolated_cache_test(child: &str) {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().expect("isolated OCR cache");
    let tessdata = root.path().join("tessdata");
    std::fs::create_dir(&tessdata).unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English fixture");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.join("eng.traineddata"), english).unwrap();
    for inventory in ["english-only", "retry-available"] {
        if inventory == "retry-available" {
            // Availability is a filesystem check. The English-only panel never opens this pack. ~keep
            std::fs::write(tessdata.join("khm.traineddata"), b"availability control").unwrap();
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::cached_default_recovers_embedded_line_in_cold_and_warm_reads"
                )
                .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
                .unwrap(),
                "--nocapture",
            ])
            .env(child, inventory)
            .env("XBERG_TESSDATA_SYSTEM_PATHS", "0")
            .env("TESSDATA_PREFIX", &tessdata)
            .env("XBERG_CACHE_DIR", root.path().join(inventory))
            .output()
            .expect("run isolated cache test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
            "{inventory}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[derive(Clone, Default)]
struct CacheLookups(std::sync::Arc<std::sync::Mutex<Vec<(String, bool)>>>);

impl CacheLookups {
    fn entries(&self) -> Vec<(String, bool)> {
        self.0.lock().unwrap().clone()
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CacheLookups {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() == "xberg::ocr::cache" {
            let mut lookup = CacheLookup::default();
            event.record(&mut lookup);
            if let (Some(key), Some(hit)) = (lookup.key, lookup.hit) {
                self.0.lock().unwrap().push((key, hit));
            }
        }
    }
}

#[derive(Default)]
struct CacheLookup {
    key: Option<String>,
    hit: Option<bool>,
}

impl tracing::field::Visit for CacheLookup {
    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}

    fn record_bool(&mut self, field: &tracing::field::Field, hit: bool) {
        if field.name() == "xberg.cache.hit" {
            self.hit = Some(hit);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, key: &str) {
        if field.name() == "xberg.cache.key" {
            self.key = Some(key.to_owned());
        }
    }
}
