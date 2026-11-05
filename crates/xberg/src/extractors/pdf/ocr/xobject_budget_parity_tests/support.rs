use crate::core::config::{ExtractionConfig, OcrConfig};
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;
use lopdf::{Document, Object, Stream, dictionary};
use std::sync::{Arc, Mutex};

pub(super) const BACKEND: &str = "xobject-budget-parity-routing-only-2339";
pub(super) const TEXT: &str = "routing-only fixture response";

#[derive(Clone, Copy)]
pub(super) enum Reply {
    Text,
    Empty,
    Fail,
}

pub(super) struct Observed {
    pub bytes: Vec<u8>,
    pub config: OcrConfig,
}

pub(super) type Records = Arc<Mutex<Vec<Observed>>>;
struct Observer {
    records: Records,
    reply: Reply,
}

impl Plugin for Observer {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for Observer {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        self.records.lock().unwrap().push(Observed {
            bytes: bytes.to_vec(),
            config: config.clone(),
        });
        match self.reply {
            Reply::Fail => Err(crate::XbergError::Ocr {
                message: "routing-only backend rejection".to_string(),
                source: None,
            }),
            Reply::Empty => Ok(ExtractedDocument::default()),
            Reply::Text => Ok(ExtractedDocument {
                content: TEXT.to_string(),
                ..Default::default()
            }),
        }
    }
}

pub(super) struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = crate::plugins::unregister_ocr_backend(BACKEND);
    }
}

pub(super) fn observer(reply: Reply) -> (Guard, Records) {
    let records = Arc::new(Mutex::new(Vec::new()));
    crate::plugins::register_ocr_backend(Arc::new(Observer {
        records: records.clone(),
        reply,
    }))
    .unwrap();
    (Guard, records)
}

pub(super) fn config(outer: usize, inner: usize) -> (ExtractionConfig, OcrConfig) {
    let ocr = OcrConfig {
        backend: BACKEND.to_string(),
        language: vec!["eng".to_string()],
        backend_options: Some(serde_json::json!({"source_dpi":600,"route_marker":"original-config"})),
        security_limits: Some(SecurityLimits {
            max_content_size: inner,
            ..Default::default()
        }),
        ..Default::default()
    };
    (
        ExtractionConfig {
            use_cache: false,
            security_limits: Some(SecurityLimits {
                max_content_size: outer,
                ..Default::default()
            }),
            ocr: Some(ocr.clone()),
            ..Default::default()
        },
        ocr,
    )
}

pub(super) struct Fixture {
    pub bytes: Vec<u8>,
    pub doc: xberg_native_pdf::PdfDocument,
    pub streams: Vec<Vec<u8>>,
}

fn stream(gray: u8, raw: bool) -> Vec<u8> {
    let pixels = vec![gray; 32 * 24 * if raw { 1 } else { 3 }];
    if raw {
        return pixels;
    }
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&pixels, 32, 24)
}

fn add_page(document: &mut Document, streams: &[Vec<u8>], raw: bool) {
    let pages = document.new_object_id();
    let mut images = lopdf::Dictionary::new();
    let mut commands = String::new();
    for (index, bytes) in streams.iter().enumerate() {
        let mut dict = dictionary! { "Type"=>"XObject", "Subtype"=>"Image", "Width"=>32, "Height"=>24, "BitsPerComponent"=>8, "ColorSpace"=>if raw {"DeviceGray"} else {"DeviceRGB"} };
        if !raw {
            dict.set("Filter", "DCTDecode");
        }
        let image = document.add_object(Stream::new(dict, bytes.clone()));
        images.set(format!("Im{index}"), image);
        commands.push_str(&format!("q 32 0 0 24 {} 0 cm /Im{index} Do Q\n", index * 36));
    }
    let content = document.add_object(Stream::new(dictionary! {}, commands.into_bytes()));
    let page = document.add_object(dictionary! { "Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![Object::Integer(0),Object::Integer(0),Object::Integer(72),Object::Integer(72)], "Contents"=>content, "Resources"=>dictionary!{"XObject"=>images} });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages", "Kids"=>vec![Object::Reference(page)], "Count"=>1}),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    document.trailer.set("Root", catalog);
}

pub(super) fn fixture(count: usize, raw: bool) -> Fixture {
    assert!((1..=2).contains(&count));
    let streams: Vec<_> = (0..count).map(|index| stream(64 + index as u8 * 64, raw)).collect();
    if count == 2 {
        assert_ne!(streams[0], streams[1]);
    }
    let mut document = Document::with_version("1.7");
    add_page(&mut document, &streams, raw);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("actual native PDF producer");
    assert_eq!(doc.page_count().unwrap(), 1);
    let handles = doc.page_image_handles(0).expect("actual painted image handles");
    assert_eq!(handles.len(), count);
    for (index, handle) in handles.iter().enumerate() {
        assert_eq!(handle.paint_order, index);
        assert_eq!(
            handle.raw_compressed_bytes_with_limit(streams[index].len()).unwrap(),
            streams[index]
        );
        if raw {
            assert!(handle.filter_chain.is_empty());
        } else {
            assert_eq!(handle.filter_chain, vec![xberg_native_pdf::PdfFilter::DCTDecode]);
        }
    }
    drop(handles);
    Fixture { bytes, doc, streams }
}

pub(super) fn filtered_fixture() -> Fixture {
    let original = fixture(1, false);
    let stored = format!("{}>", hex::encode_upper(&original.streams[0])).into_bytes();
    let mut document = Document::load_mem(&original.bytes).unwrap();
    let mut replaced = 0;
    for object in document.objects.values_mut() {
        if let Object::Stream(stream) = object
            && stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image".as_slice())
        {
            stream.content = stored.clone();
            stream.dict.set("Length", i64::try_from(stored.len()).unwrap());
            stream.dict.set(
                "Filter",
                Object::Array(vec![
                    Object::Name(b"ASCIIHexDecode".to_vec()),
                    Object::Name(b"DCTDecode".to_vec()),
                ]),
            );
            replaced += 1;
        }
    }
    assert_eq!(replaced, 1);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("actual outer-filtered PDF");
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        handles[0].filter_chain,
        vec![
            xberg_native_pdf::PdfFilter::ASCIIHexDecode,
            xberg_native_pdf::PdfFilter::DCTDecode
        ]
    );
    assert_eq!(
        handles[0].raw_compressed_bytes_with_limit(stored.len()).unwrap(),
        stored
    );
    drop(handles);
    Fixture {
        bytes,
        doc,
        streams: original.streams,
    }
}

pub(super) fn complete_fixture() -> Fixture {
    let source = fixture(1, false);
    let mut pdf = Document::load_mem(&source.bytes).unwrap();
    let page = pdf.get_pages()[&1];
    pdf.get_object_mut(page)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("MediaBox", vec![0.into(), 0.into(), 32.into(), 24.into()]);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(doc.get_page_media_box(0).unwrap(), (0.0, 0.0, 32.0, 24.0));
    Fixture {
        bytes,
        doc,
        streams: source.streams,
    }
}
