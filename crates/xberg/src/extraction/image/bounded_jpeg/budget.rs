use oxiarc_jpeg::{AppSegment, Component, DecodeOptions, FrameHeader, Metadata, ScanHeader, TableSet};

use super::headers::Header;

const COMPONENTS: u64 = 3;
const HUFFMAN_SLOTS_WITH_REPLACEMENT: u64 = 9;
const HUFFMAN_SYMBOLS: u64 = 256;
const HUFFMAN_LOOKUP_ENTRIES: u64 = 512;
// Five plane indices, one scan index and three scan table-reference arrays.
// Byte arrays hold plane size, two scan selectors and build_info component IDs. ~keep
const COMPONENT_INDEX_ARRAYS: u64 = 9;
const COMPONENT_BYTE_ARRAYS: u64 = 4;

pub(super) struct Plan {
    pub width: u32,
    pub height: u32,
    pub denominator: u8,
    pub plane_bytes: u64,
    pub output_bytes: usize,
    pub peak_bytes: u64,
}

fn sum(values: &[u64]) -> Option<u64> {
    values.iter().try_fold(0u64, |total, value| total.checked_add(*value))
}

fn product(values: &[u64]) -> Option<u64> {
    values.iter().try_fold(1u64, |total, value| total.checked_mul(*value))
}

fn metadata_bytes(header: &Header) -> Option<u64> {
    let descriptors = header.metadata_segments.max(4).checked_next_power_of_two()? as u64;
    let descriptor_bytes = sum(&[
        size_of::<AppSegment>() as u64,
        size_of::<Vec<u8>>() as u64,
        size_of::<(u8, u8, Vec<u8>)>() as u64,
    ])?;
    // The slice decoder retains APP/COM twice; it never concatenates ICC profiles.
    // Three capacities cover old/new descriptor allocations during Vec growth. ~keep
    sum(&[
        header.metadata_bytes.checked_mul(2)?,
        product(&[3, descriptors, descriptor_bytes])?,
    ])
}

fn table_bytes() -> Option<u64> {
    let huffman = sum(&[
        product(&[HUFFMAN_LOOKUP_ENTRIES, size_of::<u16>() as u64])?,
        HUFFMAN_SYMBOLS,
    ])?;
    sum(&[
        huffman.checked_mul(HUFFMAN_SLOTS_WITH_REPLACEMENT)?,
        HUFFMAN_SYMBOLS.checked_mul(size_of::<u32>() as u64)?,
        product(&[256, 2 * size_of::<i32>() as u64 + 2 * size_of::<i64>() as u64])?,
        size_of::<TableSet>() as u64,
    ])
}

fn bookkeeping_bytes() -> Option<u64> {
    let component = sum(&[
        size_of::<Component>() as u64,
        COMPONENT_INDEX_ARRAYS * size_of::<usize>() as u64 + COMPONENT_BYTE_ARRAYS,
        size_of::<Vec<u16>>() as u64,
        // Upsampler holds three usize, four u32 and one kernel discriminant;
        // the extra usize covers alignment, without depending on private layout. ~keep
        4 * size_of::<usize>() as u64 + 4 * size_of::<u32>() as u64,
    ])?;
    sum(&[
        component.checked_mul(COMPONENTS)?,
        size_of::<FrameHeader>() as u64,
        size_of::<Metadata>() as u64,
        size_of::<ScanHeader>() as u64,
        size_of::<DecodeOptions>() as u64,
    ])
}

fn plan(header: &Header, encoded_bytes: u64, denominator: u8) -> Option<Plan> {
    let numerator = 8 / u64::from(denominator);
    let width = header.width.div_ceil(u32::from(denominator));
    let height = header.height.div_ceil(u32::from(denominator));
    let horizontal = u64::from(header.horizontal_sampling);
    let vertical = u64::from(header.vertical_sampling);
    let padded_width = product(&[u64::from(header.width).div_ceil(8 * horizontal), horizontal, numerator])?;
    let padded_height = product(&[u64::from(header.height).div_ceil(8 * vertical), vertical, numerator])?;
    // Each component's scaled size is at most the frame MCU's scaled size,
    // including chroma whose scaled IDCT is enlarged by oxiarc-jpeg 0.4.2. ~keep
    let plane_bytes = product(&[padded_width, padded_height, COMPONENTS, size_of::<u16>() as u64])?;
    let output_bytes = product(&[u64::from(width), u64::from(height), COMPONENTS])?;
    let rows = product(&[u64::from(width), COMPONENTS * 2, size_of::<u16>() as u64])?;
    let peak_bytes = sum(&[
        encoded_bytes,
        plane_bytes,
        output_bytes,
        rows,
        metadata_bytes(header)?,
        table_bytes()?,
        bookkeeping_bytes()?,
    ])?;
    Some(Plan {
        width,
        height,
        denominator,
        plane_bytes,
        output_bytes: usize::try_from(output_bytes).ok()?,
        peak_bytes,
    })
}

pub(super) fn select(header: &Header, encoded_bytes: u64, budget: u64) -> Option<Plan> {
    [2, 4, 8]
        .into_iter()
        .filter_map(|denominator| plan(header, encoded_bytes, denominator))
        .find(|plan| plan.peak_bytes <= budget)
}

pub(super) fn minimum_peak(header: &Header, encoded_bytes: u64) -> u64 {
    plan(header, encoded_bytes, 8).map_or(u64::MAX, |plan| plan.peak_bytes)
}
