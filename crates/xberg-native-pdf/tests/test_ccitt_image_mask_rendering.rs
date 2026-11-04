use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const CCITT_WHITE3_BLACK2_WHITE3: &[u8] = &[0x31, 0xC0];
const CCITT_GROUP3_WHITE3_BLACK2_WHITE3: &[u8] = &[0x8E, 0x00];

fn append_object(pdf: &mut Vec<u8>, offsets: &mut Vec<usize>, number: usize, body: &[u8]) {
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    pdf.extend_from_slice(body);
    pdf.extend_from_slice(b"\nendobj\n");
}

fn build_pdf(page_width: u32, page_height: u32, content: &[u8], image: &[u8], extra_objects: &[&[u8]]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();

    append_object(&mut pdf, &mut offsets, 1, b"<< /Type /Catalog /Pages 2 0 R >>");
    append_object(&mut pdf, &mut offsets, 2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    append_object(
        &mut pdf,
        &mut offsets,
        3,
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width} {page_height}] \
             /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"
        )
        .as_bytes(),
    );
    append_object(
        &mut pdf,
        &mut offsets,
        4,
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            String::from_utf8_lossy(content)
        )
        .as_bytes(),
    );
    append_object(&mut pdf, &mut offsets, 5, image);
    for (index, body) in extra_objects.iter().enumerate() {
        append_object(&mut pdf, &mut offsets, index + 6, body);
    }

    let xref = pdf.len();
    let object_count = offsets.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {object_count}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {object_count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn build_image_mask_pdf(black_is_one: bool, inverted_decode: bool, ascii_hex_wrapper: bool, k: Option<i64>) -> Vec<u8> {
    let content = b"q 8 0 0 1 0 0 cm /Im0 Do Q";
    let decode = if inverted_decode { " /Decode [1 0]" } else { "" };
    let black = if black_is_one { " /BlackIs1 true" } else { "" };
    let k_parameter = k.map_or_else(String::new, |value| format!(" /K {value}"));
    let ccitt_stream = if k.is_none() {
        CCITT_GROUP3_WHITE3_BLACK2_WHITE3
    } else {
        CCITT_WHITE3_BLACK2_WHITE3
    };
    let (filter, params, stream): (&str, String, Vec<u8>) = if ascii_hex_wrapper {
        (
            "[/ASCIIHexDecode /CCITTFaxDecode]",
            format!("[null <<{k_parameter} /Columns 8 /Rows 1{black} >>]"),
            b"31C0>".to_vec(),
        )
    } else {
        (
            "/CCITTFaxDecode",
            format!("<<{k_parameter} /Columns 8 /Rows 1{black} >>"),
            ccitt_stream.to_vec(),
        )
    };
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 1 \
         /BitsPerComponent 1 /ImageMask true /Filter {filter} \
         /DecodeParms {params}{decode} /Length {} >>\nstream\n",
        stream.len()
    )
    .into_bytes();
    image.extend_from_slice(&stream);
    image.extend_from_slice(b"\nendstream");
    build_pdf(8, 1, content, &image, &[])
}

fn build_indirect_decode_params_pdf(array_entry: bool, params_object: &[u8]) -> Vec<u8> {
    let content = b"q 8 0 0 1 0 0 cm /Im0 Do Q";
    let (filter, decode_params, stream): (&str, &str, &[u8]) = if array_entry {
        ("[/ASCIIHexDecode /CCITTFaxDecode]", "[null 6 0 R]", b"31C0>")
    } else {
        ("/CCITTFaxDecode", "6 0 R", CCITT_WHITE3_BLACK2_WHITE3)
    };
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 1 \
         /BitsPerComponent 1 /ImageMask true /Filter {filter} \
         /DecodeParms {decode_params} /Length {} >>\nstream\n",
        stream.len()
    )
    .into_bytes();
    image.extend_from_slice(stream);
    image.extend_from_slice(b"\nendstream");
    build_pdf(8, 1, content, &image, &[params_object])
}

fn push_bits(output: &mut Vec<u8>, bit_len: &mut usize, bits: u16, count: u8) {
    for shift in (0..count).rev() {
        if (*bit_len).is_multiple_of(8) {
            output.push(0);
        }
        if bits & (1 << shift) != 0 {
            let byte_index = *bit_len / 8;
            output[byte_index] |= 1 << (7 - (*bit_len % 8));
        }
        *bit_len += 1;
    }
}

fn repeated_black_run_group4(rows: u32) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut bit_len = 0usize;
    // First row: horizontal; white run 3; black makeup 64 + terminating 0;
    // then vertical-0 to the right edge. Each later identical row is three
    // vertical-0 codes relative to the preceding row. ~keep
    push_bits(&mut compressed, &mut bit_len, 0b001, 3);
    push_bits(&mut compressed, &mut bit_len, 0b1000, 4);
    push_bits(&mut compressed, &mut bit_len, 0b0000001111, 10);
    push_bits(&mut compressed, &mut bit_len, 0b0000110111, 10);
    push_bits(&mut compressed, &mut bit_len, 0b1, 1);
    for _ in 1..rows {
        push_bits(&mut compressed, &mut bit_len, 0b111, 3);
    }
    compressed
}

fn rendered_is_black(pdf: Vec<u8>) -> Vec<bool> {
    let doc = PdfDocument::from_bytes(pdf).expect("open generated PDF");
    let rendered = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render generated PDF");
    assert_eq!((rendered.width, rendered.height), (8, 1));
    rendered
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[0] < 128 && pixel[1] < 128 && pixel[2] < 128)
        .collect()
}

#[test]
fn ccitt_image_mask_renders_black_run_with_default_polarity() {
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(false, false, false, Some(-1))),
        [false, false, false, true, true, false, false, false]
    );
}

#[test]
fn black_is_one_and_decode_are_independent_polarity_controls() {
    let inverse = [true, true, true, false, false, true, true, true];
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(true, false, false, Some(-1))),
        inverse
    );
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(false, true, false, Some(-1))),
        inverse
    );
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(true, true, false, Some(-1))),
        [false, false, false, true, true, false, false, false]
    );
}

#[test]
fn ccitt_filter_chain_uses_positionally_aligned_decode_params() {
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(false, false, true, Some(-1))),
        [false, false, false, true, true, false, false, false]
    );
}

#[test]
fn absent_k_uses_pdf_group3_one_dimensional_default() {
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(false, false, false, None)),
        [false, false, false, true, true, false, false, false]
    );
}

#[test]
fn all_negative_k_values_use_group4() {
    assert_eq!(
        rendered_is_black(build_image_mask_pdf(false, false, false, Some(-2))),
        [false, false, false, true, true, false, false, false]
    );
}

#[test]
fn indirect_decode_params_and_array_entries_are_resolved() {
    const PARAMS: &[u8] = b"<< /K -1 /Columns 8 /Rows 1 >>";
    let expected = [false, false, false, true, true, false, false, false];
    assert_eq!(
        rendered_is_black(build_indirect_decode_params_pdf(false, PARAMS)),
        expected
    );
    assert_eq!(
        rendered_is_black(build_indirect_decode_params_pdf(true, PARAMS)),
        expected
    );
}

#[test]
fn malformed_indirect_decode_params_are_skipped_without_panicking() {
    assert_eq!(
        rendered_is_black(build_indirect_decode_params_pdf(false, b"42")),
        [false; 8]
    );
    assert_eq!(
        rendered_is_black(build_indirect_decode_params_pdf(true, b"42")),
        [false; 8]
    );
}

#[test]
fn german_sized_group4_mask_expands_and_paints_pixels() {
    const WIDTH: u32 = 2016;
    const HEIGHT: u32 = 2852;
    let compressed = repeated_black_run_group4(HEIGHT);
    let packed_size = (WIDTH as usize).div_ceil(8) * HEIGHT as usize;
    assert!(
        packed_size > compressed.len() * 500,
        "fixture must exercise substantial CCITT expansion"
    );

    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width {WIDTH} /Height {HEIGHT} \
         /BitsPerComponent 1 /ImageMask true /Filter /CCITTFaxDecode \
         /DecodeParms << /K -1 /Columns {WIDTH} /Rows {HEIGHT} >> \
         /Length {} >>\nstream\n",
        compressed.len()
    )
    .into_bytes();
    image.extend_from_slice(&compressed);
    image.extend_from_slice(b"\nendstream");
    let content = format!("q 0 1 0 rg {WIDTH} 0 0 {HEIGHT} 0 0 cm /Im0 Do Q");
    let doc = PdfDocument::from_bytes(build_pdf(WIDTH, HEIGHT, content.as_bytes(), &image, &[])).expect("open PDF");
    let mut options = RenderOptions::with_dpi(72);
    options.background = Some([1.0, 0.0, 0.0, 1.0]);
    let rendered = render_page(&doc, 0, &options.as_raw()).expect("render large mask");
    assert_eq!((rendered.width, rendered.height), (WIDTH, HEIGHT));

    let pixel = |x: u32, y: u32| {
        let offset = ((y as usize * WIDTH as usize) + x as usize) * 4;
        &rendered.data[offset..offset + 4]
    };
    let middle_row = HEIGHT / 2;
    assert_eq!(pixel(20, middle_row), [0, 255, 0, 255]);
    assert_eq!(pixel(200, middle_row), [255, 0, 0, 255]);
    assert_eq!(pixel(20, HEIGHT - 2), [0, 255, 0, 255]);
    assert_eq!(pixel(200, HEIGHT - 2), [255, 0, 0, 255]);
}

const BLACK_RUN: [bool; 8] = [false, false, false, true, true, false, false, false];

fn build_group4_grayscale_pdf(color_space: &str, filter: &str, params: &str) -> Vec<u8> {
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 1 /BitsPerComponent 1 \
         /ColorSpace {color_space} /Filter {filter} /DecodeParms {params} /Length {} >>\nstream\n",
        CCITT_WHITE3_BLACK2_WHITE3.len()
    )
    .into_bytes();
    image.extend_from_slice(CCITT_WHITE3_BLACK2_WHITE3);
    image.extend_from_slice(b"\nendstream");
    build_pdf(8, 1, b"q 8 0 0 1 0 0 cm /Im0 Do Q", &image, &[])
}

#[test]
fn group4_grayscale_array_colorspace_renders_with_filter_and_params_arrays_and_missing_rows() {
    assert_eq!(
        rendered_is_black(build_group4_grayscale_pdf(
            "[/DeviceGray]",
            "[/CCITTFaxDecode]",
            "[<< /K -1 /Columns 8 >>]",
        )),
        BLACK_RUN
    );
}

#[test]
fn group4_named_grayscale_accepts_filter_and_params_arrays_with_missing_rows() {
    assert_eq!(
        rendered_is_black(build_group4_grayscale_pdf(
            "/DeviceGray",
            "[/CCITTFaxDecode]",
            "[<< /K -1 /Columns 8 >>]",
        )),
        BLACK_RUN
    );
}

#[test]
fn group4_named_grayscale_accepts_filter_and_params_arrays_with_explicit_rows() {
    assert_eq!(
        rendered_is_black(build_group4_grayscale_pdf(
            "/DeviceGray",
            "[/CCITTFaxDecode]",
            "[<< /K -1 /Columns 8 /Rows 1 >>]",
        )),
        BLACK_RUN
    );
}

#[test]
fn group4_named_grayscale_accepts_scalar_filter_and_params_with_missing_rows() {
    assert_eq!(
        rendered_is_black(build_group4_grayscale_pdf(
            "/DeviceGray",
            "/CCITTFaxDecode",
            "<< /K -1 /Columns 8 >>",
        )),
        BLACK_RUN
    );
}

#[test]
fn malformed_group4_grayscale_colorspace_array_does_not_paint() {
    assert_eq!(
        rendered_is_black(build_group4_grayscale_pdf(
            "[/DeviceGray 1]",
            "[/CCITTFaxDecode]",
            "[<< /K -1 /Columns 8 >>]",
        )),
        [false; 8]
    );
}
