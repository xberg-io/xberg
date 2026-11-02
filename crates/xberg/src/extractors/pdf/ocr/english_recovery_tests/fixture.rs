use lopdf::{Document, Object, Stream, dictionary};
use sha2::{Digest, Sha256};
use xberg_native_pdf::fonts::truetype_parser::{FontMetrics, TrueTypeFont};
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

const WORDS: [&str; 4] = ["MEMORY", "RECOVERY", "INVOICE", "READY"];
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf"
));
const FONT_SHA: &str = "abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322";

pub(super) fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub(super) fn has_words(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    let tokens: Vec<_> = upper
        .split(|character: char| !character.is_ascii_alphabetic())
        .collect();
    WORDS.iter().all(|word| tokens.contains(word))
}

pub(super) fn assert_words(text: &str) {
    assert!(
        has_words(text),
        "all four real recognized word tokens required: {text:?}"
    );
}

fn page(document: &mut Document, resources: lopdf::Dictionary, content: Vec<u8>) -> Vec<u8> {
    let pages = document.new_object_id();
    let content = document.add_object(Stream::new(dictionary! {}, content));
    let page = document.add_object(dictionary! {
        "Type"=>"Page","Parent"=>pages,"Resources"=>resources,"Contents"=>content,
        "MediaBox"=>vec![0.into(),0.into(),128.into(),96.into()]
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type"=>"Pages","Count"=>1,"Kids"=>vec![Object::Reference(page)]
        }),
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

fn text_pdf() -> Vec<u8> {
    assert_eq!(sha(FONT), FONT_SHA);
    let font = TrueTypeFont::parse(FONT).unwrap();
    let metrics = FontMetrics::from_font(&font);
    for character in WORDS.join("").chars() {
        assert!(font.glyph_id(character as u32).is_some_and(|id| id != 0));
        assert!(font.char_width(character as u32) > 0);
    }
    let mut document = Document::with_version("1.7");
    let file = document.add_object(Stream::new(dictionary! {"Length1"=>FONT.len() as i64}, FONT.to_vec()));
    let bbox = metrics.pdf_bbox();
    let descriptor = document.add_object(dictionary! {
        "Type"=>"FontDescriptor","FontName"=>"DejaVuSans","Flags"=>metrics.flags as i64,
        "FontBBox"=>vec![bbox.0.into(),bbox.1.into(),bbox.2.into(),bbox.3.into()],
        "ItalicAngle"=>0,"Ascent"=>metrics.pdf_ascender(),"Descent"=>metrics.pdf_descender(),
        "CapHeight"=>metrics.pdf_cap_height(),"StemV"=>metrics.stem_v as i64,"FontFile2"=>file
    });
    let widths: Vec<Object> = (32..=126)
        .map(|code| Object::Integer(font.char_width(code) as i64))
        .collect();
    let embedded = document.add_object(dictionary! {
        "Type"=>"Font","Subtype"=>"TrueType","BaseFont"=>"DejaVuSans",
        "Encoding"=>"WinAnsiEncoding","FirstChar"=>32,"LastChar"=>126,
        "Widths"=>widths,"FontDescriptor"=>descriptor
    });
    let mut content = "BT /F0 14 Tf 0 g 12 76 Td\n".to_string();
    for (index, word) in WORDS.iter().enumerate() {
        if index != 0 {
            content.push_str("0 -20 Td\n");
        }
        content.push_str(&format!("({word}) Tj\n"));
    }
    content.push_str("ET\n");
    page(
        &mut document,
        dictionary! {"Font"=>dictionary! {"F0"=>embedded}},
        content.into_bytes(),
    )
}

fn raster(document: &xberg_native_pdf::PdfDocument, dpi: u32) -> xberg_native_pdf::rendering::RenderedImage {
    let mut options = RenderOptions::default();
    options.dpi = dpi;
    options.format = ImageFormat::RawRgba8;
    let rendered = render_page(document, 0, &options).expect("actual native embedded-font/image renderer");
    assert_eq!(
        rendered.data.len(),
        rendered.width as usize * rendered.height as usize * 4
    );
    assert!(rendered.data.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
    rendered
}

pub(super) struct Fixture {
    pub pdf: Vec<u8>,
    pub doc: xberg_native_pdf::PdfDocument,
    pub receipt: serde_json::Value,
}

pub(super) fn fixture() -> Fixture {
    assert!(!has_words("MEMORY RECOVERY INVOICE"));
    assert!(!has_words("MEMORYRECOVERYINVOICEREADY"));
    let source = text_pdf();
    let source_sha = sha(&source);
    let text_document = xberg_native_pdf::PdfDocument::from_bytes(source).unwrap();
    assert_words(&text_document.extract_text_auto(0).unwrap());
    let rendered = raster(&text_document, 600);
    drop(text_document);
    assert_eq!(rendered.height, 800);
    assert!((1066..=1067).contains(&rendered.width));
    let dark = rendered
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[..3].iter().all(|value| *value < 64))
        .count();
    let white = rendered
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[..3].iter().all(|value| *value > 240))
        .count();
    assert!(dark > 500 && white > dark, "real glyph pixels on white background");
    let rgb: Vec<_> = rendered
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect();
    let pixel_sha = sha(&rgb);
    let width = u16::try_from(rendered.width).expect("actual fixture width fits JPEG interface");
    let height = u16::try_from(rendered.height).expect("actual fixture height fits JPEG interface");
    let jpeg = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, width, height);
    drop(rgb);
    drop(rendered);
    let mut document = Document::with_version("1.7");
    let image = document.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Image","Width"=>width as i64,"Height"=>height as i64,
            "BitsPerComponent"=>8,"ColorSpace"=>"DeviceRGB","Filter"=>"DCTDecode"
        },
        jpeg.clone(),
    ));
    let pdf = page(
        &mut document,
        dictionary! {"XObject"=>dictionary! {"Im0"=>image}},
        b"q 128 0 0 96 0 0 cm /Im0 Do Q\n".to_vec(),
    );
    drop(document);
    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    assert_eq!(doc.page_count().expect("actual image-only page count"), 1);
    assert!(
        doc.extract_text_auto(0).unwrap().trim().is_empty(),
        "final PDF has no native text layer"
    );
    let handles = doc.page_image_handles(0).unwrap();
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].paint_order, 0);
    assert_eq!(handles[0].filter_chain, vec![xberg_native_pdf::PdfFilter::DCTDecode]);
    assert_eq!(handles[0].raw_compressed_bytes_with_limit(jpeg.len()).unwrap(), jpeg);
    drop(handles);
    let receipt = serde_json::json!({"font_sha256":FONT_SHA,"source_pdf_sha256":source_sha,
        "pixels_sha256":pixel_sha,"jpeg_sha256":sha(&jpeg),"image_pdf_sha256":sha(&pdf),
        "width":width,"height":height,"dark_pixels":dark,"white_pixels":white,
        "paint_ordinals":[0],"native_text_words":0});
    drop(jpeg);
    Fixture { pdf, doc, receipt }
}

pub(super) fn detached_image(document: &xberg_native_pdf::PdfDocument) -> image::DynamicImage {
    let rendered = raster(document, 150);
    assert_eq!(rendered.height, 200);
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_raw(rendered.width, rendered.height, rendered.data).unwrap())
}
