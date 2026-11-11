use lopdf::{Document, Object, Stream, dictionary};

/// Where a full-page picture goes relative to the text layer of a one-page PDF.
#[derive(Clone, Copy, Debug)]
pub enum PicturePlacement {
    /// The picture is painted first and the text layer is drawn over it, as on a flyer.
    Background,
    /// The text layer is drawn first and the picture is painted over it.
    Cover,
    /// The picture is painted first and the text layer is drawn over it invisibly (render mode 3),
    /// as a scan's OCR sidecar is.
    BehindInvisibleLayer,
}

/// The first page of `source` with a light, textured picture that covers the whole media box,
/// placed as `placement` says. The page's text layer and resources are otherwise unchanged.
pub fn with_full_page_picture(source: &[u8], placement: PicturePlacement) -> Vec<u8> {
    let mut document = Document::load_mem(source).expect("load the one-page fixture");
    let page = *document.get_pages().get(&1).expect("the fixture has a first page");
    let media_box: Vec<f32> = document
        .get_dictionary(page)
        .and_then(|dictionary| dictionary.get(b"MediaBox"))
        .and_then(Object::as_array)
        .expect("the page has its own media box")
        .iter()
        .map(|value| value.as_float().expect("a numeric media box"))
        .collect();
    let [left, bottom, right, top] = media_box[..] else {
        panic!("a media box has four numbers: {media_box:?}");
    };
    let pixels: Vec<u8> = (0..64u8)
        .map(|index| if (index / 8 + index % 8) % 2 == 0 { 0xE0 } else { 0xF0 })
        .collect();
    let picture = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 8,
            "Height" => 8,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        pixels,
    ));
    document
        .add_xobject(page, "FullPagePicture", picture)
        .expect("add the picture to the page resources");
    let paint = format!(
        "q {} 0 0 {} {left} {bottom} cm /FullPagePicture Do Q\n",
        right - left,
        top - bottom
    )
    .into_bytes();
    let layer = document.get_page_content(page);
    let content = match placement {
        PicturePlacement::Background => [paint.as_slice(), b"q\n", &layer, b"\nQ\n"].concat(),
        PicturePlacement::Cover => [b"q\n".as_slice(), &layer, b"\nQ\n", &paint].concat(),
        PicturePlacement::BehindInvisibleLayer => [paint.as_slice(), b"q 3 Tr\n", &layer, b"\nQ\n"].concat(),
    };
    document
        .change_page_content(page, content)
        .expect("replace the page content");
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize the fixture");
    bytes
}

/// The first page of `source` with its whole content drawn in render mode 3 (invisible), as a
/// scan's OCR sidecar is. Form XObjects the page paints inherit the mode.
pub fn with_invisible_text(source: &[u8]) -> Vec<u8> {
    let mut document = Document::load_mem(source).expect("load the one-page fixture");
    let page = *document.get_pages().get(&1).expect("the fixture has a first page");
    let content = [b"3 Tr\n".as_slice(), &document.get_page_content(page)].concat();
    document
        .change_page_content(page, content)
        .expect("replace the page content");
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize the fixture");
    bytes
}
