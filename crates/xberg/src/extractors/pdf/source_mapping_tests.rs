//! Path extraction hands the native engine the file mapping itself, not a heap copy of it.

use super::*;
use std::sync::{Mutex, PoisonError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SourceSpan {
    ptr: usize,
    len: usize,
}

impl SourceSpan {
    fn of(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr() as usize,
            len: bytes.len(),
        }
    }
}

static LOADED: Mutex<Vec<(SourceSpan, bool)>> = Mutex::new(Vec::new());
static OPENED: Mutex<Vec<SourceSpan>> = Mutex::new(Vec::new());

pub(super) fn record_loaded(file: &crate::core::io::FileBytes) {
    LOADED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((SourceSpan::of(file), file.is_mapped()));
}

pub(super) fn record_opened(source: &bytes::Bytes) {
    OPENED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(SourceSpan::of(source));
}

/// Tests run in parallel in one process, so each fixture has a length no other test produces.
fn loaded_with_len(len: usize) -> Vec<(SourceSpan, bool)> {
    LOADED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .copied()
        .filter(|(span, _)| span.len == len)
        .collect()
}

fn opened(span: SourceSpan) -> bool {
    OPENED.lock().unwrap_or_else(PoisonError::into_inner).contains(&span)
}

#[cfg(not(target_arch = "wasm32"))]
const MAPPING_THRESHOLD: usize = 1_048_576;

const SENTINEL: &str = "Mapped source sentinel prose fills the only page of this ordinary native document";

/// A one-page text PDF plus an unreferenced, uncompressed stream of `padding` noise bytes: bulk
/// that no page reads.
fn padded_text_pdf(padding: usize) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal(SENTINEL)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        content.encode().expect("fixture page content must encode"),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "Contents" => content_id,
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut state = 0x2545_f491_u32;
    let noise = (0..padding)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_be_bytes()[0]
        })
        .collect::<Vec<u8>>();
    document.add_object(Stream::new(dictionary! {}, noise).with_compression(false));

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must serialize");
    bytes
}

fn write_fixture(content: &[u8]) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("temporary PDF");
    std::fs::write(file.path(), content).expect("fixture PDF must be written");
    file
}

fn plain_text(internal: InternalDocument) -> String {
    crate::extraction::derive::derive_extraction_result(internal, true, crate::core::config::OutputFormat::Plain)
        .content
}

async fn path_and_memory_text(content: &[u8], path: &std::path::Path) -> (String, String) {
    let config = ExtractionConfig::default();
    let from_path = PdfExtractor::new()
        .extract_path(path, "application/pdf", &config)
        .await
        .expect("path extraction must succeed");
    let from_memory = PdfExtractor::new()
        .extract_content(content, "application/pdf", &config)
        .await
        .expect("in-memory extraction must succeed");
    (plain_text(from_path), plain_text(from_memory))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn a_large_pdf_file_reaches_the_native_engine_as_its_mapping() {
    let content = padded_text_pdf(1_300_000 + 4_099);
    assert!(
        content.len() > MAPPING_THRESHOLD,
        "the fixture must exceed the mapping threshold, got {} bytes",
        content.len()
    );
    let file = write_fixture(&content);

    let (from_path, from_memory) = path_and_memory_text(&content, file.path()).await;

    let loaded = loaded_with_len(content.len());
    assert_eq!(loaded.len(), 1, "one load of the fixture, got {loaded:?}");
    let (mapping, mapped) = loaded[0];
    assert!(mapped, "a file over the threshold must be mapped");
    assert!(
        opened(mapping),
        "the native document must read the mapping at {mapping:?}, not a copy of it"
    );
    assert!(
        from_path.contains(SENTINEL),
        "path extraction lost the text: {from_path:?}"
    );
    assert_eq!(
        from_path, from_memory,
        "the mapped source must extract like the in-memory bytes"
    );
}

#[tokio::test]
async fn a_small_pdf_file_is_read_onto_the_heap_and_extracts_like_memory() {
    let content = padded_text_pdf(4_321);
    let file = write_fixture(&content);

    let (from_path, from_memory) = path_and_memory_text(&content, file.path()).await;

    let loaded = loaded_with_len(content.len());
    assert_eq!(loaded.len(), 1, "one load of the fixture, got {loaded:?}");
    let (heap, mapped) = loaded[0];
    assert!(!mapped, "a file under the threshold must not be mapped");
    assert!(
        opened(heap),
        "the native document must take the read buffer without a copy"
    );
    assert!(
        from_path.contains(SENTINEL),
        "path extraction lost the text: {from_path:?}"
    );
    assert_eq!(from_path, from_memory);
}

#[test]
fn the_native_engine_keeps_the_shared_source_it_was_given() {
    let shared = bytes::Bytes::from(padded_text_pdf(5_003));

    let output = extract_native_blocking(
        shared.clone(),
        ExtractionConfig::default(),
        #[cfg(feature = "layout-detection")]
        None,
        #[cfg(feature = "layout-detection")]
        None,
        #[cfg(feature = "layout-detection")]
        None,
        #[cfg(feature = "layout-detection")]
        None,
        #[cfg(feature = "layout-detection")]
        None,
    )
    .expect("the native extraction must succeed");

    assert_eq!(output.source.as_ptr(), shared.as_ptr());
    assert_eq!(output.source.len(), shared.len());
}
