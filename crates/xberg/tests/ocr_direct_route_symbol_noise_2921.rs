#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use std::sync::Arc;

use xberg::core::config::{OcrQualityThresholds, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::Table;
use xberg::types::internal::InternalDocument;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, OutputFormat, ResultFormat, extract};

const PHOTO_ONLY_PDF: &[u8] = include_bytes!("fixtures/ocr_photo_noise_2312/photo-only.pdf");
const TITLE: &str = "SECTION TITLE";
const NOISE: [&str; 10] = ["a", "oe", "# =", "ae", "\"", "ee", "=|", "rs", "~", "ie"];
const NOISE_LINES: usize = 40;

struct ScriptedBackend {
    name: &'static str,
    content: String,
    tables: Vec<Table>,
}

impl Plugin for ScriptedBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for ScriptedBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut document = ExtractedDocument::from(InternalDocument::new("image"));
        document.content = self.content.clone();
        document.tables = self.tables.clone();
        Ok(document)
    }
    fn supports_document_processing(&self) -> bool {
        false
    }
}

struct Unregister(&'static str);
impl Drop for Unregister {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(self.0);
    }
}

fn noisy_page() -> String {
    let mut lines = vec![TITLE.to_string()];
    lines.extend(NOISE.iter().cycle().take(NOISE_LINES).map(|line| line.to_string()));
    lines.join("\n")
}

fn real_page() -> String {
    [
        "The committee met on Tuesday to review the annual budget.",
        "Members agreed that the library needs a new roof before winter.",
        "A short vote followed, and the motion passed with two objections.",
        "The treasurer will publish the final figures next month.",
        "Residents may send written comments to the town clerk.",
        "The meeting adjourned at nine in the evening.",
        "Everyone present thanked the volunteers for their long work.",
        "The next session is planned for the first week of spring.",
    ]
    .join("\n")
}

fn table() -> Table {
    Table {
        cells: vec![vec!["a".into(), "b".into()], vec!["1".into(), "2".into()]],
        markdown: "| a | b |\n|---|---|\n| 1 | 2 |".into(),
        page_number: 1,
        ..Default::default()
    }
}

async fn run(
    name: &'static str,
    content: String,
    tables: Vec<Table>,
    prune: bool,
    result_format: ResultFormat,
) -> ExtractedDocument {
    register_ocr_backend(Arc::new(ScriptedBackend { name, content, tables })).unwrap();
    let _guard = Unregister(name);
    let config = ExtractionConfig {
        use_cache: false,
        force_ocr_pages: Some(vec![1]),
        output_format: OutputFormat::Plain,
        result_format,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: name.to_string(),
            quality_thresholds: Some(OcrQualityThresholds {
                prune_symbol_noise_lines: prune,
                ..OcrQualityThresholds::default()
            }),
            ..OcrConfig::default()
        }),
        ..ExtractionConfig::default()
    };
    let mut batch = extract(
        ExtractInput::from_bytes(PHOTO_ONLY_PDF.to_vec(), "application/pdf", None),
        &config,
    )
    .await
    .expect("extract");
    assert!(batch.errors.is_empty(), "{:?}", batch.errors);
    batch.results.remove(0)
}

fn prune_warnings(document: &ExtractedDocument) -> Vec<String> {
    document
        .processing_warnings
        .iter()
        .map(|warning| warning.message.to_string())
        .filter(|message| message.contains("symbol-noise line(s)"))
        .collect()
}

fn lines(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

#[tokio::test]
async fn the_direct_route_prunes_content_pages_and_elements_with_one_warning() {
    let document = run(
        "direct-route-prune-2921",
        noisy_page(),
        Vec::new(),
        true,
        ResultFormat::ElementBased,
    )
    .await;
    assert_eq!(document.content.trim(), TITLE, "{:?}", document.content);
    let pages = document.pages.as_ref().expect("pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content.trim(), TITLE, "{:?}", pages[0].content);
    let elements = document.elements.as_ref().expect("elements");
    let joined = elements
        .iter()
        .map(|element| element.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(joined.trim(), TITLE, "{joined:?}");
    assert_eq!(
        prune_warnings(&document),
        vec![format!(
            "Page 1: removed {NOISE_LINES} symbol-noise line(s) from suspected OCR recognition noise; set prune_symbol_noise_lines to false to keep them."
        )]
    );
}

#[tokio::test]
async fn a_page_with_a_table_keeps_its_noise_lines() {
    let document = run(
        "direct-route-table-2921",
        noisy_page(),
        vec![table()],
        true,
        ResultFormat::Unified,
    )
    .await;
    assert!(lines(&document.content) > NOISE_LINES, "{:?}", document.content);
    assert!(prune_warnings(&document).is_empty());
}

#[tokio::test]
async fn a_page_without_a_noise_verdict_is_untouched() {
    let document = run(
        "direct-route-clean-2921",
        real_page(),
        Vec::new(),
        true,
        ResultFormat::Unified,
    )
    .await;
    assert_eq!(lines(&document.content), 8, "{:?}", document.content);
    assert!(prune_warnings(&document).is_empty());
}

#[tokio::test]
async fn the_flag_off_keeps_every_line_on_the_direct_route() {
    let document = run(
        "direct-route-flag-off-2921",
        noisy_page(),
        Vec::new(),
        false,
        ResultFormat::Unified,
    )
    .await;
    assert_eq!(lines(&document.content), NOISE_LINES + 1, "{:?}", document.content);
    assert_eq!(
        lines(&document.pages.as_ref().expect("pages")[0].content),
        NOISE_LINES + 1
    );
    assert!(prune_warnings(&document).is_empty());
}
