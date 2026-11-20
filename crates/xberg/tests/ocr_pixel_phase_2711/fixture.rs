use sha2::{Digest, Sha256};

pub const LINES: [&str; 4] = [
    "The report is in a plain folder marked VI today.",
    "The label in a clear box reads VI on paper.",
    "Always keep in a record the Roman number VI exactly.",
    "The section VI is in a separate folder for review.",
];
const FONT: &[u8] = include_bytes!("../../../xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf");
pub const FONT_SHA256: &str = "abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322";

const FONT_WIDTHS: &str = concat!(
    "317.87109375 400.87890625 459.96093750 837.89062500 636.23046875 950.19531250 ",
    "779.78515625 274.90234375 390.13671875 390.13671875 500.00000000 837.89062500 ",
    "317.87109375 360.83984375 317.87109375 336.91406250 636.23046875 636.23046875 ",
    "636.23046875 636.23046875 636.23046875 636.23046875 636.23046875 636.23046875 ",
    "636.23046875 636.23046875 336.91406250 336.91406250 837.89062500 837.89062500 ",
    "837.89062500 530.76171875 1000.00000000 684.08203125 686.03515625 698.24218750 ",
    "770.01953125 631.83593750 575.19531250 774.90234375 751.95312500 294.92187500 ",
    "294.92187500 655.76171875 557.12890625 862.79296875 748.04687500 787.10937500 ",
    "603.02734375 787.10937500 694.82421875 634.76562500 610.83984375 731.93359375 ",
    "684.08203125 988.76953125 685.05859375 610.83984375 685.05859375 390.13671875 ",
    "336.91406250 390.13671875 837.89062500 500.00000000 500.00000000 612.79296875 ",
    "634.76562500 549.80468750 634.76562500 615.23437500 352.05078125 634.76562500 ",
    "633.78906250 277.83203125 277.83203125 579.10156250 277.83203125 974.12109375 ",
    "633.78906250 611.81640625 634.76562500 634.76562500 411.13281250 520.99609375 ",
    "392.08984375 633.78906250 591.79687500 817.87109375 591.79687500 591.79687500 ",
    "524.90234375 636.23046875 336.91406250 636.23046875 837.89062500",
);

pub fn pdf(size: u32, shift_150_pixels: u32) -> Vec<u8> {
    assert_eq!(hex::encode(Sha256::digest(FONT)), FONT_SHA256);
    let x = 36.0 + f64::from(shift_150_pixels) * 72.0 / 150.0;
    let mut content = format!("BT /F1 {size} Tf 0 Tc 0 Tw ");
    for (line, y) in LINES.iter().zip([180, 144, 108, 72]) {
        content.push_str(&format!("1 0 0 1 {x} {y} Tm ({line}) Tj "));
    }
    content.push_str("ET");
    content_pdf(content)
}

#[allow(dead_code)] // Shared with tests that only use the phase fixtures.
pub fn density_controls() -> [(&'static str, Vec<u8>, String); 4] {
    let create = |name, width, height, size, count, two_pages| {
        let mut content = format!("BT /F1 {size} Tf ");
        let mut expected = Vec::new();
        for index in 0..count {
            let line = LINES[index % LINES.len()];
            let y = f64::from(height) - 42.0 - index as f64 * 18.0;
            content.push_str(&format!("1 0 0 1 36 {y} Tm ({line}) Tj "));
            expected.push(line);
        }
        content.push_str("ET");
        let expected = if two_pages {
            expected.repeat(2).join(" ")
        } else {
            expected.join(" ")
        };
        (
            name,
            content_pdf_with_pages(content, width, height, two_pages),
            expected,
        )
    };
    [
        create("letter-small", 612.0, 792.0, 7, 4, false),
        create("a4-small", 595.0, 842.0, 7, 4, false),
        create("letter-ordinary-heavy", 612.0, 792.0, 12, 32, false),
        create("letter-multipage-small", 612.0, 792.0, 7, 4, true),
    ]
}

pub fn content_pdf(content: String) -> Vec<u8> {
    content_pdf_with_pages(content, 432.0, 216.0, false)
}

pub fn content_pdf_with_pages(content: String, width: f32, height: f32, two_pages: bool) -> Vec<u8> {
    content_pdf_with_page_resources(content, width, height, two_pages, "")
}

#[allow(dead_code)] // Used by the density planner's alpha control.
pub fn content_pdf_with_resources(content: String, extra_resources: &str) -> Vec<u8> {
    content_pdf_with_page_resources(content, 612.0, 792.0, false, extra_resources)
}

#[allow(dead_code)]
pub fn content_pdf_with_logo(content: String) -> Vec<u8> {
    build_density_pdf(
        content,
        612.0,
        792.0,
        false,
        "/XObject << /Logo 8 0 R >>",
        Some(stream(
            &[0],
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray",
        )),
    )
}

fn content_pdf_with_page_resources(
    content: String,
    width: f32,
    height: f32,
    two_pages: bool,
    extra_resources: &str,
) -> Vec<u8> {
    build_density_pdf(content, width, height, two_pages, extra_resources, None)
}

fn build_density_pdf(
    content: String,
    width: f32,
    height: f32,
    two_pages: bool,
    extra_resources: &str,
    image: Option<Vec<u8>>,
) -> Vec<u8> {
    assert_eq!(hex::encode(Sha256::digest(FONT)), FONT_SHA256);
    let resources = if extra_resources.is_empty() {
        String::new()
    } else {
        format!(" {extra_resources}")
    };
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >>{resources} >> >>"
    )
    .into_bytes();
    let pages = if two_pages {
        b"<< /Type /Pages /Kids [3 0 R 8 0 R] /Count 2 >>".to_vec()
    } else {
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        pages,
        page.clone(),
        stream(&content.into_bytes(), ""),
        format!("<< /Type /Font /Subtype /TrueType /BaseFont /DejaVuSans /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 126 /Widths [{FONT_WIDTHS}] /FontDescriptor 6 0 R >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /DejaVuSans /Flags 32 /FontBBox [-1021 -463 1794 1233] /ItalicAngle 0 /Ascent 928 /Descent -236 /CapHeight 729 /StemV 80 /FontFile2 7 0 R >>".to_vec(),
    ];
    objects.push(stream(FONT, &format!("/Length1 {}", FONT.len())));
    if two_pages {
        objects.push(page);
    }
    objects.extend(image);
    assemble(&objects)
}

fn stream(data: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in &offsets[1..] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    out
}

pub fn localized_pdf(variant: &str) -> Vec<u8> {
    use xberg_native_pdf::PdfDocument;
    let baseline = PdfDocument::from_bytes(pdf(7, 0)).unwrap();
    let chars = baseline.extract_chars(0).unwrap();
    let line = LINES[0];
    let cut = match variant {
        "gap-left" => line.find("in a").unwrap() + 3,
        "I-right" => line.find("VI").unwrap() + 1,
        _ => panic!("unknown control"),
    };
    let delta = if variant == "gap-left" { -0.48 } else { 0.48 };
    let target = f64::from(chars[cut].origin_x) + delta;
    let mut content = format!(
        "BT /F1 7 Tf 0 Tc 0 Tw 1 0 0 1 36 180 Tm ({}) Tj 1 0 0 1 {target} 180 Tm ({}) Tj ",
        &line[..cut],
        &line[cut..]
    );
    for (line, y) in LINES[1..].iter().zip([144, 108, 72]) {
        content.push_str(&format!("1 0 0 1 36 {y} Tm ({line}) Tj "));
    }
    content.push_str("ET");
    content_pdf(content)
}
