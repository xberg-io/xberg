use super::*;

#[derive(Clone, Copy)]
struct Kind {
    big_endian: bool,
    bigtiff: bool,
}

const KINDS: [Kind; 4] = [
    Kind {
        big_endian: false,
        bigtiff: false,
    },
    Kind {
        big_endian: true,
        bigtiff: false,
    },
    Kind {
        big_endian: false,
        bigtiff: true,
    },
    Kind {
        big_endian: true,
        bigtiff: true,
    },
];

fn put(out: &mut Vec<u8>, kind: Kind, value: u64, len: usize) {
    let bytes: Vec<u8> = (0..len)
        .map(|index| {
            let shift = if kind.big_endian { len - 1 - index } else { index } * 8;
            (value >> shift) as u8
        })
        .collect();
    out.extend(bytes);
}

/// Directory offsets of a file built by [`build`] with `entries` zeroed entries per directory.
fn directory_offsets(kind: Kind, frames: usize, entries: usize) -> Vec<u64> {
    let header = if kind.bigtiff { 16 } else { 8 };
    let (count, entry, pointer) = if kind.bigtiff { (8, 20, 8) } else { (2, 12, 4) };
    let size = count + entries * entry + pointer;
    (0..frames).map(|index| (header + index * size) as u64).collect()
}

/// A TIFF of `frames` directories with `entries` zeroed entries each, linked in order.
fn build(kind: Kind, frames: usize, entries: usize) -> Vec<u8> {
    let offsets = directory_offsets(kind, frames, entries);
    let mut out = Vec::new();
    out.extend(if kind.big_endian { b"MM" } else { b"II" });
    if kind.bigtiff {
        put(&mut out, kind, 43, 2);
        put(&mut out, kind, 8, 2);
        put(&mut out, kind, 0, 2);
        put(&mut out, kind, offsets[0], 8);
    } else {
        put(&mut out, kind, 42, 2);
        put(&mut out, kind, offsets[0], 4);
    }
    for index in 0..frames {
        put(&mut out, kind, entries as u64, if kind.bigtiff { 8 } else { 2 });
        out.extend(vec![0u8; entries * if kind.bigtiff { 20 } else { 12 }]);
        let next = offsets.get(index + 1).copied().unwrap_or(0);
        put(&mut out, kind, next, if kind.bigtiff { 8 } else { 4 });
    }
    out
}

fn link_at(kind: Kind, frame: usize, frames: usize, entries: usize) -> usize {
    let start = directory_offsets(kind, frames, entries)[frame] as usize;
    let (count, entry) = if kind.bigtiff { (8, 20) } else { (2, 12) };
    start + count + entries * entry
}

fn overwrite(bytes: &mut [u8], kind: Kind, at: usize, value: u64) {
    let mut encoded = Vec::new();
    put(&mut encoded, kind, value, if kind.bigtiff { 8 } else { 4 });
    bytes[at..at + encoded.len()].copy_from_slice(&encoded);
}

#[test]
fn walk_lists_every_frame_in_all_four_layouts() {
    for kind in KINDS {
        let bytes = build(kind, 3, 2);
        let walk = walk_tiff_ifds(&bytes, 10).expect("tiff header");
        assert_eq!(walk.offsets, directory_offsets(kind, 3, 2));
        assert_eq!(walk.end, TiffWalkEnd::Complete);
    }
}

#[test]
fn walk_stops_at_the_cap_and_says_more_follow() {
    let bytes = build(KINDS[0], 5, 1);
    let walk = walk_tiff_ifds(&bytes, 3).unwrap();
    assert_eq!(walk.offsets.len(), 3);
    assert_eq!(walk.end, TiffWalkEnd::CapReached);
}

#[test]
fn walk_at_exactly_the_cap_is_complete() {
    let bytes = build(KINDS[0], 3, 1);
    let walk = walk_tiff_ifds(&bytes, 3).unwrap();
    assert_eq!(walk.offsets.len(), 3);
    assert_eq!(walk.end, TiffWalkEnd::Complete);
}

#[test]
fn walk_reports_a_first_directory_past_the_end_of_the_file() {
    for kind in KINDS {
        let mut bytes = build(kind, 1, 1);
        let at = if kind.bigtiff { 8 } else { 4 };
        let len = bytes.len() as u64;
        overwrite(&mut bytes, kind, at, len + 100);
        let walk = walk_tiff_ifds(&bytes, 10).unwrap();
        assert!(walk.offsets.is_empty());
        assert_eq!(
            walk.end,
            TiffWalkEnd::Broken("directory offset is past the end of the file")
        );
    }
}

#[test]
fn walk_keeps_the_frames_before_a_link_past_the_end_of_the_file() {
    for kind in KINDS {
        let mut bytes = build(kind, 3, 1);
        let at = link_at(kind, 1, 3, 1);
        let len = bytes.len() as u64;
        overwrite(&mut bytes, kind, at, len + 1000);
        let walk = walk_tiff_ifds(&bytes, 10).unwrap();
        assert_eq!(walk.offsets, directory_offsets(kind, 3, 1)[..2].to_vec());
        assert!(matches!(walk.end, TiffWalkEnd::Broken(_)));
    }
}

#[test]
fn walk_breaks_a_cycle_instead_of_looping() {
    for kind in KINDS {
        let mut bytes = build(kind, 3, 1);
        let target = directory_offsets(kind, 3, 1)[1];
        let at = link_at(kind, 2, 3, 1);
        overwrite(&mut bytes, kind, at, target);
        let walk = walk_tiff_ifds(&bytes, 1_000_000).unwrap();
        assert_eq!(walk.offsets.len(), 3);
        assert_eq!(walk.end, TiffWalkEnd::Broken("directory links form a cycle"));
    }
}

#[test]
fn walk_keeps_the_frames_before_a_truncated_directory() {
    for kind in KINDS {
        let mut bytes = build(kind, 3, 4);
        let cut = directory_offsets(kind, 3, 4)[2] as usize + 5;
        bytes.truncate(cut);
        let walk = walk_tiff_ifds(&bytes, 10).unwrap();
        assert_eq!(walk.offsets.len(), 2);
        assert!(matches!(walk.end, TiffWalkEnd::Broken(_)));
    }
}

#[test]
fn walk_rejects_bytes_without_a_tiff_header() {
    assert!(walk_tiff_ifds(b"\x89PNG\r\n\x1a\n", 10).is_none());
    assert!(walk_tiff_ifds(b"II", 10).is_none());
    assert!(walk_tiff_ifds(b"II\x2b\x00\x08\x00", 10).is_none());
    assert!(frame_count(b"not a tiff").is_err());
}

#[test]
fn single_frame_tiff_keeps_exactly_the_chosen_frame_in_all_four_layouts() {
    for kind in KINDS {
        let bytes = build(kind, 3, 2);
        let offsets = directory_offsets(kind, 3, 2);
        for offset in &offsets {
            let single = single_frame_tiff(&bytes, *offset).expect("frame isolates");
            assert_eq!(single.len(), bytes.len());
            let walk = walk_tiff_ifds(&single, 10).unwrap();
            assert_eq!(walk.offsets, vec![*offset]);
            assert_eq!(walk.end, TiffWalkEnd::Complete);
        }
    }
}

#[test]
fn single_frame_tiff_refuses_an_offset_past_the_end_and_a_cut_directory() {
    for kind in KINDS {
        let bytes = build(kind, 2, 2);
        assert!(single_frame_tiff(&bytes, bytes.len() as u64 + 8).is_none());
        let cut = directory_offsets(kind, 2, 2)[1] as usize + 3;
        assert!(single_frame_tiff(&bytes[..cut], directory_offsets(kind, 2, 2)[1]).is_none());
    }
}

#[test]
fn single_frame_tiff_decodes_the_chosen_encoder_frame() {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(&mut cursor).unwrap();
        for value in [10u8, 20, 30] {
            encoder
                .write_image::<tiff::encoder::colortype::Gray8>(1, 1, &[value])
                .unwrap();
        }
    }
    let bytes = cursor.into_inner();
    let walk = walk_tiff_ifds(&bytes, 10).unwrap();
    assert_eq!(walk.offsets.len(), 3);
    for (offset, expected) in walk.offsets.iter().zip([10u8, 20, 30]) {
        let single = single_frame_tiff(&bytes, *offset).unwrap();
        let mut decoder = tiff::decoder::Decoder::new(std::io::Cursor::new(single)).unwrap();
        assert!(!decoder.more_images());
        match decoder.read_image().unwrap() {
            tiff::decoder::DecodingResult::U8(pixels) => assert_eq!(pixels, vec![expected]),
            other => panic!("unexpected decode {other:?}"),
        }
    }
}
