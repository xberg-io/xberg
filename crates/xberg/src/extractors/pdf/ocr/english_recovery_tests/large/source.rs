use super::fixture;
use lopdf::{Document, Object, Stream, dictionary};

pub(super) const WIDTH: u16 = 10007;
pub(super) const HEIGHT: u16 = 6343;
pub(super) const HALF_RGB: usize = WIDTH.div_ceil(2) as usize * HEIGHT.div_ceil(2) as usize * 3;
pub(super) const ORIGINAL_RGB: usize = WIDTH as usize * HEIGHT as usize * 3;

pub(super) struct Source {
    pub jpeg: Vec<u8>,
    pub pdf: Vec<u8>,
    pub document: xberg_native_pdf::PdfDocument,
    pub preview: image::DynamicImage,
}

fn pdf(jpeg: &[u8]) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let (w, h) = (f32::from(WIDTH) * 72.0 / 600.0, f32::from(HEIGHT) * 72.0 / 600.0);
    let image = document.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject", "Subtype"=>"Image", "Width"=>i64::from(WIDTH), "Height"=>i64::from(HEIGHT),
            "BitsPerComponent"=>8, "ColorSpace"=>"DeviceRGB", "Filter"=>"DCTDecode"
        },
        jpeg.to_vec(),
    ));
    let content = document.add_object(Stream::new(
        dictionary! {},
        format!("q {w} 0 0 {h} 0 0 cm /Im0 Do Q\n").into_bytes(),
    ));
    let page = document.add_object(dictionary! {
        "Type"=>"Page", "Parent"=>pages, "Contents"=>content,
        "Resources"=>dictionary! {"XObject"=>dictionary! {"Im0"=>image}},
        "MediaBox"=>vec![0.into(), 0.into(), w.into(), h.into()]
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages", "Count"=>1, "Kids"=>vec![page.into()]}),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

pub(super) fn create() -> Source {
    let small = fixture::fixture();
    assert_eq!(
        fixture::sha(&small.pdf),
        small.receipt["image_pdf_sha256"].as_str().unwrap()
    );
    let patch = fixture::detached_image(&small.doc).to_rgb8();
    let patch = image::imageops::resize(
        &patch,
        patch.width() * 8,
        patch.height() * 8,
        image::imageops::FilterType::Nearest,
    );
    let mut image = image::RgbImage::from_pixel(u32::from(WIDTH), u32::from(HEIGHT), image::Rgb([255, 255, 255]));
    image::imageops::replace(&mut image, &patch, 100, 100);
    let jpeg = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(image.as_raw(), WIDTH, HEIGHT);
    drop(image);
    let mut decoder = jpeg_decoder::Decoder::new(jpeg.as_slice());
    let size = decoder.scale(WIDTH.div_ceil(4), HEIGHT.div_ceil(4)).unwrap();
    assert_eq!(size, (2502, 1586));
    let preview = image::RgbImage::from_raw(u32::from(size.0), u32::from(size.1), decoder.decode().unwrap()).unwrap();
    let pdf = pdf(&jpeg);
    let document = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    assert_eq!(document.page_count().unwrap(), 1);
    assert!(document.extract_text_auto(0).unwrap().trim().is_empty());
    let handles = document.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(
        (handles[0].width, handles[0].height),
        (u32::from(WIDTH), u32::from(HEIGHT))
    );
    assert_eq!(handles[0].filter_chain, vec![xberg_native_pdf::PdfFilter::DCTDecode]);
    assert_eq!(handles[0].raw_compressed_bytes_with_limit(jpeg.len()).unwrap(), jpeg);
    drop(handles);
    Source {
        jpeg,
        pdf,
        document,
        preview: image::DynamicImage::ImageRgb8(preview),
    }
}
