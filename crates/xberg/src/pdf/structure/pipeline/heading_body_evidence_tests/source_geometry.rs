use super::super::reconstruct_pdf_lines;
use super::{
    SegmentData, active_pages, assert_local_following, fixtures, paragraph, plain, promote_with_active_gate,
    spans_visual_line_break,
};
use sha2::{Digest, Sha256};

type RawSpan = (&'static str, [u32; 7], [bool; 3]);

const SOURCE_RECORDS_SHA256: &str = "b704945b853903e05731976dadd21b218944d1421ba314f778b42990ac0392db";
const RAW_SPANS: [RawSpan; 26] = [
    (
        "–",
        [
            0x435afcee, 0x4427b800, 0x40800000, 0x41000000, 0x41000000, 0x4427b800, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "< 30",
        [
            0x4309bfbe, 0x4425c7ef, 0x4195df3c, 0x41180000, 0x41180000, 0x4425c7ef, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "X",
        [
            0x4359dba6, 0x4425d6b8, 0x40c38520, 0x41200000, 0x41200000, 0x4425d6b8, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "-",
        [
            0x43636eda, 0x4425d6b8, 0x40551eb9, 0x41200000, 0x41200000, 0x4425d6b8, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "µ",
        [
            0x43694e15, 0x4425d6b8, 0x40a00001, 0x41200000, 0x41200000, 0x4425d6b8, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "0",
        [
            0x436f38d6, 0x442529eb, 0x40800000, 0x41000000, 0x41000000, 0x442529eb, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "t",
        [
            0x4344b8d5, 0x44243ee9, 0x4031eb86, 0x41200000, 0x41200000, 0x44243ee9, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "c",
        [
            0x434828b4, 0x4423ce45, 0x406353f9, 0x41000000, 0x41000000, 0x4423ce45, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "=",
        [
            0x434e7b23, 0x44243ee9, 0x40db3334, 0x41200000, 0x41200000, 0x44243ee9, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "(σ unknown)",
        [
            0x42f164dc, 0x4422ee55, 0x42535b25, 0x41180000, 0x41180000, 0x4422ee55, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "s",
        [
            0x435a3efb, 0x4422f676, 0x4078f5c4, 0x41200000, 0x41200000, 0x4422f676, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "/",
        [
            0x435fdf7e, 0x4422c6c8, 0x4031eb86, 0x41200000, 0x41200000, 0x4422c6c8, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "n",
        [
            0x436cbe36, 0x4422f676, 0x40a00001, 0x41200000, 0x41200000, 0x4422f676, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "–",
        [
            0x435c672b, 0x441f43d8, 0x40800000, 0x41000000, 0x41000000, 0x441f43d8, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "< 30",
        [
            0x4309bf7d, 0x441d53c7, 0x4195df3c, 0x41180000, 0x41180000, 0x441d53c7, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "X",
        [
            0x435b45e3, 0x441d6290, 0x40c38520, 0x41200000, 0x41200000, 0x441d6290, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "-",
        [
            0x4364d916, 0x441d6290, 0x40551eb9, 0x41200000, 0x41200000, 0x441d6290, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "µ",
        [
            0x436ab851, 0x441d6290, 0x40a00001, 0x41200000, 0x41200000, 0x441d6290, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "0",
        [
            0x4370a312, 0x441cb5c3, 0x40800000, 0x41000000, 0x41000000, 0x441cb5c3, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "Z",
        [
            0x43435b64, 0x441bcac1, 0x40b1eb86, 0x41200000, 0x41200000, 0x441bcac1, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "c",
        [
            0x434992f1, 0x441b5a1d, 0x406353f9, 0x41000000, 0x41000000, 0x441b5a1d, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "=",
        [
            0x434fe560, 0x441bcac1, 0x40db3334, 0x41200000, 0x41200000, 0x441bcac1, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "(σ known)",
        [
            0x42fbf5c3, 0x441a7a2d, 0x4229199c, 0x41180000, 0x41180000, 0x441a7a2d, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "σ",
        [
            0x435af374, 0x441a824e, 0x409f5c2a, 0x41200000, 0x41200000, 0x441a824e, 0x00000000,
        ],
        [false, true, false],
    ),
    (
        "/",
        [
            0x4361ff7c, 0x441a52a0, 0x4031eb86, 0x41200000, 0x41200000, 0x441a52a0, 0x00000000,
        ],
        [false, false, false],
    ),
    (
        "n",
        [
            0x436ede34, 0x441a824e, 0x40a00001, 0x41200000, 0x41200000, 0x441a824e, 0x00000000,
        ],
        [false, true, false],
    ),
];

fn source_records() -> Vec<SegmentData> {
    RAW_SPANS
        .into_iter()
        .map(|(text, bits, style)| {
            let [x, y, width, height, font_size, baseline_y, rotation_degrees] = bits.map(f32::from_bits);
            SegmentData {
                text: text.to_owned(),
                x,
                y,
                width,
                height,
                font_size,
                is_bold: style[0],
                is_italic: style[1],
                is_monospace: style[2],
                baseline_y,
                rotation_degrees,
                assigned_role: None,
            }
        })
        .collect()
}

fn source_digest<'a>(records: impl IntoIterator<Item = &'a SegmentData>) -> String {
    let mut digest = Sha256::new();
    for span in records {
        digest.update((span.text.len() as u64).to_le_bytes());
        digest.update(span.text.as_bytes());
        for scalar in [
            span.x,
            span.y,
            span.width,
            span.height,
            span.font_size,
            span.baseline_y,
            span.rotation_degrees,
        ] {
            digest.update(scalar.to_bits().to_le_bytes());
        }
        digest.update([span.is_bold as u8, span.is_italic as u8, span.is_monospace as u8]);
        assert!(span.assigned_role.is_none());
        digest.update([0]);
    }
    hex::encode(digest.finalize())
}

fn canonical_body() -> super::PdfParagraph {
    let records = source_records();
    assert_eq!(records.len(), 26);
    assert_eq!(source_digest(&records), SOURCE_RECORDS_SHA256);
    let borrowed: Vec<_> = records.iter().collect();
    let lines = reconstruct_pdf_lines(&borrowed);
    assert_eq!(lines.len(), 8);
    assert_eq!(
        lines.iter().map(|line| line.segments.len()).collect::<Vec<_>>(),
        vec![1, 5, 3, 4, 1, 5, 3, 4]
    );
    assert!(lines.iter().all(|line| line.valid_native_scripts().is_empty()));
    assert_eq!(
        source_digest(lines.iter().flat_map(|line| &line.segments)),
        SOURCE_RECORDS_SHA256,
        "canonical reconstruction must retain every captured source field"
    );
    let anchor = &lines[1].segments[0];
    let subscript = &lines[1].segments[4];
    assert_eq!((anchor.text.as_str(), subscript.text.as_str()), ("< 30", "0"));
    assert_eq!(anchor.baseline_y.to_bits(), 0x4425c7ef);
    assert_eq!(subscript.baseline_y.to_bits(), 0x442529eb);
    assert_eq!(subscript.y.to_bits(), subscript.baseline_y.to_bits());
    assert!(spans_visual_line_break(anchor, subscript));
    assert_ne!(anchor.baseline_y.to_bits(), subscript.baseline_y.to_bits());
    let body = paragraph(lines, 8.0, false);
    assert_eq!(body.word_count, 30);
    assert!(!body.is_formula && !body.is_list_item && body.layout_class.is_none());
    body
}

#[test]
fn should_not_promote_two_cell_headers_with_canonical_mixed_baselines_2275() {
    let header = fixtures::two_cell_header();
    let left = &header.lines[0].segments[0];
    let right = &header.lines[0].segments[1];
    assert_eq!((right.x - left.x - left.width).to_bits(), 0x4154f350);
    assert!(right.x > left.x + left.width);
    let mut pages = active_pages(vec![
        fixtures::existing_section(),
        header,
        plain("Sample Size", true, 357.195984, 721.182922, 58.300003),
        canonical_body(),
        plain(
            "An independent right column continues its ordinary explanatory discussion.",
            false,
            315.0,
            580.0,
            225.0,
        ),
    ]);
    assert_local_following(&pages[0], 1, 3);
    promote_with_active_gate(&mut pages, 1, None);
}
