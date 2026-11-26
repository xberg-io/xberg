//! Measures the live heap of the two image-budget operations the renderer runs beside a page canvas,
//! an in-place JPEG decode and a pre-resize, and checks each against the bytes the image budget
//! charges for it. The bounds follow `zune-jpeg` and `fast_image_resize` internals, so this test
//! fails when a version of either crate allocates more than the model counts.
//!
//! The binary has its own counting global allocator, so it runs as one test: other tests in the same
//! process would add their allocations to the count.
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use xberg_native_pdf::rendering::budget_probe;

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

/// Bytes the counter saw at its peak while `operation` ran, beyond what was live before it.
fn measured_peak<T>(operation: impl FnOnce() -> T) -> (u64, T) {
    let base = CURRENT.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let output = operation();
    let peak = PEAK.load(Ordering::SeqCst) - base;
    (peak as u64, output)
}

struct Scan {
    name: &'static str,
    bytes: Vec<u8>,
    size: (u32, u32),
}

fn encode(
    name: &'static str,
    (width, height): (u16, u16),
    color: ColorType,
    sampling: SamplingFactor,
    restart_interval: u16,
    progressive: bool,
) -> Scan {
    let channels = match color {
        ColorType::Luma => 1,
        _ => 3,
    };
    let pixels: Vec<u8> = (0..usize::from(width) * usize::from(height) * channels)
        .map(|index| ((index * 7 + index / 251) % 256) as u8)
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, 85);
    encoder.set_sampling_factor(sampling);
    encoder.set_restart_interval(restart_interval);
    encoder.set_progressive(progressive);
    encoder.encode(&pixels, width, height, color).unwrap();
    Scan {
        name,
        bytes,
        size: (u32::from(width), u32::from(height)),
    }
}

#[test]
fn in_place_decodes_and_pre_resizes_stay_within_their_charges() {
    let size = (2_048, 1_024);
    let scans = [
        encode("4:4:4", size, ColorType::Rgb, SamplingFactor::R_4_4_4, 0, false),
        encode("4:2:0", size, ColorType::Rgb, SamplingFactor::R_4_2_0, 0, false),
        encode("grayscale", size, ColorType::Luma, SamplingFactor::R_4_4_4, 0, false),
        encode(
            "4:2:0 with DRI 4",
            size,
            ColorType::Rgb,
            SamplingFactor::R_4_2_0,
            4,
            false,
        ),
    ];
    assert!(
        scans[3].bytes.windows(2).any(|marker| marker == [0xff, 0xdd]),
        "the restart fixture must carry a DRI segment"
    );
    let mut checked = 0;
    for scan in &scans {
        let bound = budget_probe::in_place_decode_bound(&scan.bytes).expect("a single-scan frame decodes in place");
        let (peak, decoded) = measured_peak(|| budget_probe::decode_in_place(&scan.bytes, scan.size));
        assert_eq!(decoded.unwrap(), scan.size, "{}", scan.name);
        println!(
            "in-place {}: encoded {}, peak {peak}, charge {bound}",
            scan.name,
            scan.bytes.len()
        );
        assert!(
            peak <= bound,
            "in-place {}: peak {peak} exceeds charge {bound}",
            scan.name
        );
        checked += 1;
    }
    let progressive = encode("progressive", size, ColorType::Rgb, SamplingFactor::R_4_2_0, 0, true);
    assert_eq!(
        budget_probe::in_place_decode_bound(&progressive.bytes),
        None,
        "a progressive frame is never decoded in place"
    );

    let source = (2_048u32, 1_024u32);
    for target in [(1_600, 800), (2_048, 700), (1_200, 1_024), (300, 150)] {
        let mut pixels: Vec<u8> = (0..source.0 * source.1 * 4).map(|index| (index % 251) as u8).collect();
        let bound = budget_probe::resize_bound(source, target);
        let (peak, resized) = measured_peak(|| budget_probe::resize(&mut pixels, source, target));
        assert_eq!(resized, Some(target.0 as usize * target.1 as usize * 4), "{target:?}");
        println!("pre-resize to {target:?}: peak {peak}, charge {bound}");
        assert!(
            peak <= bound,
            "pre-resize to {target:?}: peak {peak} exceeds charge {bound}"
        );
        checked += 1;
    }

    // The native downscale resizes gray or RGB samples before widening them; its modelled peak,
    // source included, must bound the measured one and stay within what the RGBA path holds (the
    // full-size RGBA raster beside its pre-resize), which the JPEG charges count.
    for components in [1u8, 3] {
        for target in [(1_600, 800), (2_048, 700), (1_200, 1_024), (300, 150)] {
            let length = source.0 as usize * source.1 as usize * usize::from(components);
            let bound = budget_probe::native_downscale_bound(source, target, components);
            let (peak, widened) = measured_peak(|| {
                let samples: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
                budget_probe::native_downscale(samples, source, target, components)
            });
            assert_eq!(widened, Some(target.0 as usize * target.1 as usize * 4), "{target:?}");
            println!("native {components}-byte downscale to {target:?}: peak {peak}, charge {bound}");
            assert!(
                peak <= bound,
                "native {components}-byte downscale to {target:?}: peak {peak} exceeds charge {bound}"
            );
            let rgba_path = u64::from(source.0 * source.1) * 4 + budget_probe::resize_bound(source, target);
            assert!(
                bound <= rgba_path,
                "native {components}-byte downscale to {target:?}: charge {bound} exceeds the RGBA path's {rgba_path}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}
