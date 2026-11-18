use super::make_xlsx;
use crate::cancellation::CancellationToken;
use crate::core::config::ExtractionConfig;
use crate::core::runtime::{current_office_pool, global_runtime};
use crate::extractors::{ExcelExtractor, PptxExtractor};
use crate::plugins::InternalDocumentExtractor;
use std::future::poll_fn;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::task::Poll;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing_subscriber::layer::{Context, SubscriberExt};

const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PPTX: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const DECK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/office/merged_table.pptx");
const DEADLINE: Duration = Duration::from_secs(5);

const DIMENSION_SCAN: &str = "xberg::excel::worksheet_dimensions";
const WORKSHEET_PARSE: &str = "xberg::excel::worksheet";

struct SheetGate {
    target: &'static str,
    parses: Arc<AtomicUsize>,
    seen: Arc<AtomicUsize>,
    reached_second: Mutex<Option<oneshot::Sender<()>>>,
    allow_parse: Mutex<mpsc::Receiver<()>>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for SheetGate {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() == WORKSHEET_PARSE {
            self.parses.fetch_add(1, Ordering::Relaxed);
        }
        if event.metadata().target() == self.target && self.seen.fetch_add(1, Ordering::Relaxed) == 1 {
            self.reached_second.lock().unwrap().take().unwrap().send(()).unwrap();
            self.allow_parse.lock().unwrap().recv_timeout(DEADLINE).unwrap();
        }
    }
}

fn three_sheets() -> Vec<u8> {
    let sheets = (1..=3)
        .map(|n| format!(r#"<sheet name="Sheet{n}" sheetId="{n}" r:id="rId{n}"/>"#))
        .collect::<String>();
    let relationships = (1..=3)
        .map(|n| {
            format!(
                r#"<Relationship Id="rId{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            )
        })
        .collect::<String>();
    let xml = br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Retained table value</t></is></c></row></sheetData></worksheet>"#;
    make_xlsx(
        &format!("<sheets>{sheets}</sheets>"),
        &relationships,
        &[
            ("xl/worksheets/sheet1.xml", xml.to_vec()),
            ("xl/worksheets/sheet2.xml", xml.to_vec()),
            ("xl/worksheets/sheet3.xml", xml.to_vec()),
        ],
    )
}

#[test]
#[serial_test::serial(office_admission)]
fn cancelled_and_abandoned_excel_read_release_shared_admission_before_remaining_sheets() {
    let data = three_sheets();
    let tmp = tempfile::Builder::new().suffix(".xlsx").tempfile().unwrap();
    std::fs::write(tmp.path(), &data).unwrap();
    let stock = super::read_excel_bytes(&data, ".xlsx", &Default::default()).unwrap().0;
    assert_eq!(
        stock.sheets.iter().map(|sheet| sheet.name.as_str()).collect::<Vec<_>>(),
        ["Sheet1", "Sheet2", "Sheet3"]
    );
    for drop_request in [false, true] {
        assert_pptx_proceeds_after_excel_stops(&data, tmp.path(), drop_request, WORKSHEET_PARSE);
    }
}

#[test]
#[serial_test::serial(office_admission)]
fn cancelled_and_abandoned_excel_reads_stop_during_xlsx_dimension_preflight() {
    let data = three_sheets();
    let tmp = tempfile::Builder::new().suffix(".xlsx").tempfile().unwrap();
    std::fs::write(tmp.path(), &data).unwrap();
    for drop_request in [false, true] {
        assert_pptx_proceeds_after_excel_stops(&data, tmp.path(), drop_request, DIMENSION_SCAN);
    }
}

fn assert_pptx_proceeds_after_excel_stops(data: &[u8], workbook: &Path, drop_request: bool, target: &'static str) {
    let runtime = global_runtime().unwrap();
    let seen = Arc::new(AtomicUsize::new(0));
    let parses = Arc::new(AtomicUsize::new(0));
    let (reached_tx, reached_rx) = oneshot::channel();
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let subscriber = tracing_subscriber::registry().with(SheetGate {
        target,
        parses: Arc::clone(&parses),
        seen: Arc::clone(&seen),
        reached_second: Mutex::new(Some(reached_tx)),
        allow_parse: Mutex::new(release_rx),
    });
    tracing::callsite::rebuild_interest_cache();
    tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let pool = current_office_pool().unwrap();
            let _other_slots = tokio::time::timeout(
                DEADLINE,
                pool.admission.acquire_many((pool.slots - 1).try_into().unwrap()),
            )
            .await
            .unwrap()
            .unwrap();
            let cancel = CancellationToken::new();
            let config = ExtractionConfig {
                cancel_token: (!drop_request).then(|| cancel.clone()),
                max_archive_depth: 0,
                ..Default::default()
            };
            let excel = ExcelExtractor::new();
            let mut read = if drop_request {
                excel.extract_content(data, XLSX, &config)
            } else {
                excel.extract_path(workbook, XLSX, &config)
            };
            assert!(poll_fn(|cx| Poll::Ready(read.as_mut().poll(cx))).await.is_pending());
            tokio::time::timeout(DEADLINE, reached_rx).await.unwrap().unwrap();
            assert_eq!(pool.admission.available_permits(), 0);
            let pptx = PptxExtractor::new();
            let pptx_config = ExtractionConfig {
                max_archive_depth: 0,
                ..Default::default()
            };
            let mut deck = pptx.extract_path(Path::new(DECK), PPTX, &pptx_config);
            assert!(poll_fn(|cx| Poll::Ready(deck.as_mut().poll(cx))).await.is_pending());
            if drop_request {
                drop(read);
                release_tx.send(()).unwrap();
            } else {
                cancel.cancel();
                release_tx.send(()).unwrap();
                assert!(matches!(
                    tokio::time::timeout(DEADLINE, read).await.unwrap(),
                    Err(crate::XbergError::Cancelled)
                ));
            }
            assert_eq!(
                tokio::time::timeout(DEADLINE, deck).await.unwrap().unwrap().mime_type,
                PPTX
            );
        });
    });
    tracing::callsite::rebuild_interest_cache();
    assert_eq!(seen.load(Ordering::Relaxed), 2, "Excel still began the third worksheet");
    if target == DIMENSION_SCAN {
        assert_eq!(
            parses.load(Ordering::Relaxed),
            0,
            "Excel continued to worksheet parsing"
        );
    }
}
