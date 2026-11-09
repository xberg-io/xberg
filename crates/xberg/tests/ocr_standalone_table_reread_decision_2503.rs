#![cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

//! One test per process: the re-read decision is observed through its debug events, which a
//! global subscriber captures from the OCR worker threads.

mod helpers;

use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt as _;
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const LEADER_TABLE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/20.png");
const CLEAN_TABLE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/clean_table.png");
const GARBLED_14PX: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/14.png");
const TWO_COLUMN_PROSE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/two_column.png");
const UPSIDE_DOWN_TABLE: &[u8] = include_bytes!("fixtures/ocr_dot_leader_2503/upside_down_table.png");

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<String>>>);

#[derive(Default)]
struct Fields(String);

impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{}={value:?} ", field.name()));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push_str(&format!("{}={value} ", field.name()));
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Events {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        if fields.0.contains("standalone table re-read") {
            self.0.lock().unwrap().push(fields.0);
        }
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    }
}

fn psm(bytes: &[u8]) -> String {
    let result = helpers::extract_bytes_document_blocking(bytes, "image/png", &config()).unwrap();
    result.metadata.additional["psm"].as_str().unwrap().to_string()
}

#[test]
fn a_leader_table_is_adopted_and_two_column_prose_is_never_read_again() {
    let events = Events::default();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(events.clone()))
        .expect("capture the re-read decision");

    assert_eq!(psm(LEADER_TABLE), "6");
    let leader = std::mem::take(&mut *events.0.lock().unwrap());
    assert!(
        leader.iter().any(|event| event.contains("reading the picture again")),
        "{leader:?}"
    );
    assert!(leader.iter().any(|event| event.contains("adopted")), "{leader:?}");

    assert_eq!(psm(TWO_COLUMN_PROSE), "11", "the page keeps its first read");
    let prose = events.0.lock().unwrap().clone();
    assert!(
        !prose
            .iter()
            .any(|event| event.contains("reading the picture again") || event.contains("adopted")),
        "a page with one gutter must not be read twice: {prose:?}"
    );
    assert!(
        prose.iter().any(|event| event.contains("skipped")),
        "the page is refused by the geometry admission: {prose:?}"
    );

    events.0.lock().unwrap().clear();
    assert_eq!(
        psm(UPSIDE_DOWN_TABLE),
        "11",
        "a second read with no table is not adopted"
    );
    let upside_down = events.0.lock().unwrap().clone();
    assert!(
        upside_down
            .iter()
            .any(|event| event.contains("reading the picture again")),
        "the geometry admits the page to a second read: {upside_down:?}"
    );
    assert!(
        upside_down
            .iter()
            .any(|event| event.contains("rejected") && event.contains("found no table")),
        "{upside_down:?}"
    );
    assert!(
        !upside_down.iter().any(|event| event.contains("adopted")),
        "{upside_down:?}"
    );

    events.0.lock().unwrap().clear();
    assert_eq!(psm(CLEAN_TABLE), "11", "a correct first-read table is kept");
    let clean = std::mem::take(&mut *events.0.lock().unwrap());
    assert!(
        !clean.iter().any(|event| event.contains("reading the picture again")),
        "a correct first-read table must not be read twice: {clean:?}"
    );

    assert_eq!(psm(GARBLED_14PX), "6", "a garbled first-read table is replaced");
    let garbled = events.0.lock().unwrap().clone();
    assert!(
        garbled.iter().any(|event| event.contains("reading the picture again")),
        "{garbled:?}"
    );
    assert!(garbled.iter().any(|event| event.contains("adopted")), "{garbled:?}");
}
