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

pub fn content_pdf(content: String) -> Vec<u8> {
    assert_eq!(hex::encode(Sha256::digest(FONT)), FONT_SHA256);
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 432 216] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_vec(),
        stream(&content.into_bytes(), ""),
        format!("<< /Type /Font /Subtype /TrueType /BaseFont /DejaVuSans /Encoding /WinAnsiEncoding /FirstChar 32 /LastChar 126 /Widths [{FONT_WIDTHS}] /FontDescriptor 6 0 R >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /DejaVuSans /Flags 32 /FontBBox [-1021 -463 1794 1233] /ItalicAngle 0 /Ascent 928 /Descent -236 /CapHeight 729 /StemV 80 /FontFile2 7 0 R >>".to_vec(),
    ];
    objects.push(stream(FONT, &format!("/Length1 {}", FONT.len())));
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
