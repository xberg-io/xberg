//! A 4-component JPEG with no Adobe APP14 marker is transform 0 by default
//! (ISO 32000-1:2008 Table 13), and `jpeg-decoder` returns the same samples
//! with and without the marker, so the extractor must un-invert both alike.

use jpeg_encoder::{ColorType, Encoder};
use xberg_native_pdf::extractors::images::decode_cmyk_jpeg_to_rgb;

fn encode_cmyk_pixel(cmyk: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    Encoder::new(&mut out, 100)
        .encode(&cmyk, 1, 1, ColorType::Cmyk)
        .expect("encode cmyk jpeg");
    out
}

fn strip_app14(jpeg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(jpeg.len());
    let mut i = 0usize;
    while i < jpeg.len() {
        if i + 3 < jpeg.len() && jpeg[i] == 0xFF && jpeg[i + 1] == 0xEE {
            let len = u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]) as usize;
            i += 2 + len;
            continue;
        }
        out.push(jpeg[i]);
        i += 1;
    }
    out
}

fn decode(jpeg: &[u8]) -> Vec<u8> {
    jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg))
        .decode()
        .expect("decode")
}

#[test]
fn fixture_really_removes_the_app14_marker() {
    let with_marker = encode_cmyk_pixel([255, 0, 0, 0]);
    let without = strip_app14(&with_marker);
    assert!(with_marker.windows(2).any(|w| w == [0xFF, 0xEE]));
    assert!(!without.windows(2).any(|w| w == [0xFF, 0xEE]));
}

#[test]
fn jpeg_decoder_inverts_regardless_of_the_app14_marker() {
    let with_marker = encode_cmyk_pixel([255, 0, 0, 0]);
    let without_marker = strip_app14(&with_marker);
    assert_eq!(decode(&with_marker), decode(&without_marker));
}

#[test]
fn markerless_cmyk_jpeg_renders_like_the_marked_one() {
    let with_marker = encode_cmyk_pixel([255, 0, 0, 0]);
    let without_marker = strip_app14(&with_marker);

    let marked = decode_cmyk_jpeg_to_rgb(&with_marker).expect("marked decodes");
    let unmarked = decode_cmyk_jpeg_to_rgb(&without_marker).expect("markerless decodes");
    assert_eq!(marked, unmarked, "marker presence must not change the colour");
}
