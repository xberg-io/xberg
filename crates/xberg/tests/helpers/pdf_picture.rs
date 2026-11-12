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
    /// The picture is painted first and the text layer is drawn over it in white.
    BehindWhiteLayer,
    /// The picture is painted first and the text layer is drawn over it inside an empty clip.
    BehindClippedLayer,
    /// The picture is painted first, then the text layer three times invisibly (render mode 3), as a
    /// scan's OCR sidecar is, and once visibly over it, as a stamp.
    BesideSidecar,
    /// [`PicturePlacement::Background`] with a dark, blocky picture, which a read of the whole page
    /// turns into noise.
    TexturedBackground,
}

/// A light 8 by 8 checker.
fn light_checker() -> (u32, Vec<u8>) {
    let pixels = (0..64u8)
        .map(|index| if (index / 8 + index % 8) % 2 == 0 { 0xE0 } else { 0xF0 })
        .collect();
    (8, pixels)
}

/// A 64 by 64 picture of grey blocks between 150 and 240, each pixel shifted by a fixed scatter.
fn dark_blocks() -> (u32, Vec<u8>) {
    let pixels = (0..64u32 * 64)
        .map(|index| {
            let (x, y) = (index % 64, index / 64);
            let block = if (x / 8 + y / 8) % 2 == 0 { 150 } else { 210 };
            (block + (x * 37 + y * 91) % 31) as u8
        })
        .collect();
    (64, pixels)
}

/// The first page of `source` with a textured picture that covers the whole media box, placed as
/// `placement` says. The page's text layer and resources are otherwise unchanged.
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
    let (side, pixels) = match placement {
        PicturePlacement::TexturedBackground => dark_blocks(),
        _ => light_checker(),
    };
    let picture = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => side,
            "Height" => side,
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
    let over = |prefix: &[u8]| [prefix, &layer, b"\nQ\n"].concat();
    let content = match placement {
        PicturePlacement::Background | PicturePlacement::TexturedBackground => [paint, over(b"q\n")].concat(),
        PicturePlacement::Cover => [over(b"q\n"), paint].concat(),
        PicturePlacement::BehindInvisibleLayer => [paint, over(b"q 3 Tr\n")].concat(),
        PicturePlacement::BehindWhiteLayer => [paint, over(b"q 1 g\n")].concat(),
        PicturePlacement::BehindClippedLayer => [paint, over(b"q 0 0 1 1 re W n\n")].concat(),
        PicturePlacement::BesideSidecar => {
            let sidecar = over(b"q 3 Tr\n");
            [paint, sidecar.clone(), sidecar.clone(), sidecar, over(b"q\n")].concat()
        }
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
