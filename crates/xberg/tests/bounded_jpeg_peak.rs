//! Measures the live heap of a reduced JPEG decode and checks it against the bound that admits
//! the decode. The bound follows `jpeg-decoder` internals, so this test fails when a version
//! of that crate allocates more than the model counts.
//!
//! The binary has its own counting global allocator, so it runs as one test: other tests in
//! the same process would add their allocations to the count.
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use xberg::extraction::image::{decode_reduced_jpeg, reduced_decode_bound};

struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grow(bytes: usize) {
    let current = CURRENT.fetch_add(bytes, Ordering::SeqCst) + bytes;
    PEAK.fetch_max(current, Ordering::SeqCst);
}

fn shrink(bytes: usize) {
    CURRENT.fetch_sub(bytes, Ordering::SeqCst);
}

// SAFETY: every method forwards to `System` with the caller's arguments unchanged and only
// updates two counters, so the `GlobalAlloc` contract of `System` carries over. ~keep
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        shrink(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A moving realloc holds the old and the new block at once, so count both first. ~keep
        grow(new_size);
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if moved.is_null() {
            shrink(new_size);
        } else {
            shrink(layout.size());
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

struct Scan {
    name: &'static str,
    bytes: Vec<u8>,
}

fn encode(
    name: &'static str,
    width: u16,
    height: u16,
    color: ColorType,
    sampling: SamplingFactor,
    progressive: bool,
) -> Scan {
    let channels = match color {
        ColorType::Luma => 1,
        _ => 3,
    };
    let pixels: Vec<u8> = (0..usize::from(width) * usize::from(height) * channels)
        .map(|index| ((index * 7 + index / 251) % 256) as u8)
        .collect();
    encode_pixels(name, &pixels, width, height, color, sampling, progressive)
}

fn encode_pixels(
    name: &'static str,
    pixels: &[u8],
    width: u16,
    height: u16,
    color: ColorType,
    sampling: SamplingFactor,
    progressive: bool,
) -> Scan {
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, 85);
    encoder.set_sampling_factor(sampling);
    encoder.set_progressive(progressive);
    encoder.encode(pixels, width, height, color).unwrap();
    Scan { name, bytes }
}

fn measured_peak(bytes: &[u8], denominator: u8) -> (u64, (u32, u32)) {
    let base = CURRENT.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let size = decode_reduced_jpeg(bytes, denominator).unwrap();
    let peak = PEAK.load(Ordering::SeqCst) - base;
    (peak as u64, size)
}

#[test]
fn reduced_decode_stays_within_its_admitted_bound() {
    // The widest scan a JPEG allows, with smooth content so that the encoded bytes stay small:
    // its upsampler line buffers on 64 threads are larger than the rest of the slack in the
    // bound, so a bound without them fails here. ~keep
    let (wide_width, wide_height) = (65_520u16, 1_024u16);
    let smooth: Vec<u8> = (0..usize::from(wide_width) * usize::from(wide_height) * 3)
        .map(|index| (index / 3 / 64 % 256) as u8)
        .collect();
    let scans = [
        encode_pixels(
            "wide 4:2:0",
            &smooth,
            wide_width,
            wide_height,
            ColorType::Rgb,
            SamplingFactor::R_4_2_0,
            false,
        ),
        encode("4:4:4", 2_048, 1_024, ColorType::Rgb, SamplingFactor::R_4_4_4, false),
        encode(
            "progressive 4:2:0",
            2_048,
            1_024,
            ColorType::Rgb,
            SamplingFactor::R_4_2_0,
            true,
        ),
        encode(
            "grayscale",
            2_048,
            1_024,
            ColorType::Luma,
            SamplingFactor::R_4_4_4,
            false,
        ),
    ];
    drop(smooth);
    let mut checked = 0;
    for threads in [1, 4, 64] {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap();
        for scan in &scans {
            for denominator in [2, 4, 8] {
                let (peak, size, bound) = pool.install(|| {
                    let bound = reduced_decode_bound(&scan.bytes, denominator).unwrap();
                    let (peak, size) = measured_peak(&scan.bytes, denominator);
                    (peak, size, bound)
                });
                println!(
                    "{} 1/{denominator} on {threads} threads: {}x{}, encoded {}, peak {peak}, bound {bound}",
                    scan.name,
                    size.0,
                    size.1,
                    scan.bytes.len()
                );
                assert!(
                    peak <= bound,
                    "{} at 1/{denominator} on {threads} threads: peak {peak} exceeds bound {bound}",
                    scan.name
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 36);
}
