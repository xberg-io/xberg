//! An overprinted `Separation` or `DeviceN` paint shows in the composite render: a process colorant
//! name (Cyan, Magenta, Yellow, Black) marks its own channel and keeps the others, `/All` marks
//! every channel, and a spot ink paints its alternate colour. OCR reads this render, so a paint that left the backdrop showed no
//! text at all.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, SeparationPlate, render_page, render_separations};

/// Tint 0 to 1 of one process colorant in a `DeviceCMYK` alternate.
fn ramp(cmyk: [u8; 4]) -> String {
    format!(
        "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [{} {} {} {}] /N 1 >>",
        cmyk[0], cmyk[1], cmyk[2], cmyk[3]
    )
}

/// A 100 x 100 page with the colour spaces this file uses, the overprint state `/OP1`
/// (OPM 1) and `/OP0` (OPM 0), and Helvetica as `/F1`.
fn page(content: &str) -> Vec<u8> {
    let resources = format!(
        "/ColorSpace << /CsK [/Separation /Black /DeviceCMYK {k}] /CsC [/Separation /Cyan /DeviceCMYK {c}] \
         /CsS [/Separation /SpotInk /DeviceCMYK {c}] /CsNK [/DeviceN [/Black] /DeviceCMYK {k}] /CsA [/Separation /All /DeviceCMYK {k}] >> \
         /ExtGState << /OP1 << /Type /ExtGState /OP true /op true /OPM 1 >> \
         /OP0 << /Type /ExtGState /OP true /op true /OPM 0 >> >> \
         /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >> >>",
        k = ramp([0, 0, 0, 1]),
        c = ramp([1, 0, 0, 0]),
    );
    let mut buf: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {resources} >> /Contents 4 0 R >>"),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    ];
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    buf
}

fn render(content: &str) -> (Vec<u8>, usize) {
    let doc = PdfDocument::from_bytes(page(content)).expect("parse");
    let mut options = RenderOptions::with_dpi(72);
    options.format = ImageFormat::RawRgba8;
    let image = render_page(&doc, 0, &options).expect("render");
    (image.data, image.width as usize)
}

/// The RGB of the pixel at the page centre, inside every filled square of this file.
fn centre(content: &str) -> [u8; 3] {
    let (data, width) = render(content);
    let at = (width / 2 * width + width / 2) * 4;
    [data[at], data[at + 1], data[at + 2]]
}

/// How many pixels of the render are dark on every channel.
fn dark_pixels(content: &str) -> usize {
    let (data, _) = render(content);
    data.chunks_exact(4).filter(|px| px[..3].iter().all(|&v| v < 96)).count()
}

fn plate_centre(content: &str, ink: &str) -> u8 {
    let doc = PdfDocument::from_bytes(page(content)).expect("parse");
    let plates: Vec<SeparationPlate> = render_separations(&doc, 0, 72).expect("separations");
    let plate = plates
        .iter()
        .find(|plate| plate.ink_name == ink)
        .unwrap_or_else(|| panic!("no {ink} plate"));
    let width = plate.width as usize;
    plate.data[width / 2 * width + width / 2]
}

const SQUARE: &str = "10 10 80 80 re f";

/// Process cyan as the CMYK conversion shows it, about `[0, 173, 239]`.
fn is_cyan(rgb: [u8; 3]) -> bool {
    rgb[0] < 60 && rgb[1] > 140 && rgb[2] > 200
}

#[test]
fn an_overprinted_separation_black_fill_paints_black() {
    for state in ["/OP1", "/OP0"] {
        let rgb = centre(&format!("{state} gs /CsK cs 1 scn {SQUARE}"));
        assert!(rgb.iter().all(|&v| v < 60), "{state}: {rgb:?}");
    }
}

#[test]
fn an_overprinted_separation_black_text_paints_its_glyphs() {
    let text = "BT /F1 40 Tf 8 35 Td (HIH) Tj ET";
    let plain = dark_pixels(&format!("/CsK cs 1 scn {text}"));
    let overprinted = dark_pixels(&format!("/OP1 gs /CsK cs 1 scn {text}"));
    assert!(plain > 200, "the control text must render: {plain} dark pixels");
    assert!(overprinted * 10 >= plain * 9, "{overprinted} of {plain} dark pixels");
}

#[test]
fn an_overprinted_process_colorant_keeps_the_other_channels_of_the_backdrop() {
    let backdrop = format!("0 1 0 0 k {SQUARE} ");
    let knockout = centre(&format!("{backdrop}/CsC cs 1 scn {SQUARE}"));
    assert!(is_cyan(knockout), "cyan: {knockout:?}");
    let overprinted = centre(&format!("{backdrop}/OP1 gs /CsC cs 1 scn {SQUARE}"));
    assert!(
        overprinted[0] < 60 && overprinted[1] < 100 && overprinted[2] > 100,
        "cyan over magenta: {overprinted:?}"
    );
}

#[test]
fn an_overprinted_devicen_black_fill_paints_black() {
    let rgb = centre(&format!("/OP1 gs /CsNK cs 1 scn {SQUARE}"));
    assert!(rgb.iter().all(|&v| v < 60), "{rgb:?}");
}

#[test]
fn an_overprinted_spot_ink_paints_its_alternate_colour() {
    let rgb = centre(&format!("/OP1 gs /CsS cs 1 scn {SQUARE}"));
    assert!(is_cyan(rgb), "{rgb:?}");
}

#[test]
fn an_overprinted_all_separation_marks_every_process_channel() {
    let content = format!("0 1 0 0 k {SQUARE} /OP1 gs /CsA cs 1 scn {SQUARE}");
    let rgb = centre(&content);
    assert!(rgb.iter().all(|&v| v < 60), "{rgb:?}");
    assert_eq!(plate_centre(&content, "Cyan"), 255);
}

#[test]
fn an_overprinted_separation_black_marks_the_black_plate() {
    assert_eq!(plate_centre(&format!("/OP1 gs /CsK cs 1 scn {SQUARE}"), "Black"), 255);
}

#[test]
fn an_overprinted_spot_ink_marks_its_own_plate_and_no_process_plate() {
    let content = format!("/OP1 gs /CsS cs 1 scn {SQUARE}");
    assert_eq!(plate_centre(&content, "SpotInk"), 255);
    assert_eq!(plate_centre(&content, "Cyan"), 0);
}
