#![cfg(feature = "pdf")]

use std::io::Write;

use flate2::{Compression, write::ZlibEncoder};
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractInput, ExtractionConfig, ImageExtractionConfig, OcrStrategy};

const WIDTH: usize = 101;
const HEIGHT: usize = 100;

fn scanned_rgb_pdf(samples: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    for row in samples.as_chunks::<{ WIDTH * 3 }>().0 {
        encoder.write_all(&[0]).unwrap();
        encoder.write_all(row).unwrap();
    }

    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let predictor = pdf.add_object(dictionary! {
        "Predictor" => 15,
        "Columns" => WIDTH as i64,
        "Colors" => 3,
        "BitsPerComponent" => 8,
    });
    let raster = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => WIDTH as i64,
            "Height" => HEIGHT as i64,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
            "Filter" => vec![Object::Name(b"FlateDecode".to_vec())],
            "DecodeParms" => vec![Object::Reference(predictor)],
        },
        encoder.finish().unwrap(),
    ));
    let contents = pdf.add_object(Stream::new(dictionary! {}, b"q 101 0 0 100 0 0 cm /Scan Do Q".to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), (WIDTH as i64).into(), (HEIGHT as i64).into()],
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => raster } },
        "Contents" => contents,
    });
    pdf.objects.insert(
        pages,
        dictionary! { "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page)] }.into(),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn scanned_pages_exports_packed_rgb_from_an_indirect_predictor() {
    let samples: Vec<u8> = (0..WIDTH * HEIGHT * 3).map(|index| (index * 29 + 17) as u8).collect();
    let input = ExtractInput::from_bytes(scanned_rgb_pdf(&samples), "application/pdf", None);
    let config = ExtractionConfig {
        ocr_strategy: OcrStrategy::ScannedPages { min_confidence: 0.5 },
        images: Some(ImageExtractionConfig {
            extract_images: true,
            run_ocr_on_images: false,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = xberg::extract(input, &config).await.expect("extract scanned PDF");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let document = result.results.first().expect("one extracted document");
    let images = document.images.as_ref().expect("requested embedded images");
    assert_eq!(images.len(), 1, "the RGB image must survive re-encoding");
    let rgb = image::load_from_memory(&images[0].data).unwrap().into_rgb8();
    assert_eq!(rgb.dimensions(), (WIDTH as u32, HEIGHT as u32));
    assert_eq!(rgb.into_raw(), samples);
}
