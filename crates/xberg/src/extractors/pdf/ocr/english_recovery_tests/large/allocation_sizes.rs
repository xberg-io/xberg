use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct DetectOriginalRaster;
#[global_allocator]
static ALLOCATOR: DetectOriginalRaster = DetectOriginalRaster;
static ENABLED: AtomicBool = AtomicBool::new(false);
static MINIMUM: AtomicUsize = AtomicUsize::new(usize::MAX);
static HITS: AtomicUsize = AtomicUsize::new(0);
static HALF_HITS: AtomicUsize = AtomicUsize::new(0);
static HALF_RGB: AtomicUsize = AtomicUsize::new(0);
static MAXIMUM: AtomicUsize = AtomicUsize::new(0);

fn observe(size: usize) {
    if ENABLED.load(Ordering::Relaxed) {
        MAXIMUM.fetch_max(size, Ordering::Relaxed);
        let original = MINIMUM.load(Ordering::Relaxed);
        if size == original || size == original / 3 * 4 {
            HITS.fetch_add(1, Ordering::Relaxed);
            original_raster_allocation(size);
        }
        let half = HALF_RGB.load(Ordering::Relaxed);
        if half > 0 && (size == half || size == half / 3 * 4) {
            HALF_HITS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// A stable breakpoint traces the allocation site without allocating from the allocator. ~keep
#[inline(never)]
fn original_raster_allocation(size: usize) {
    std::hint::black_box(size);
}

// SAFETY: Every operation forwards unchanged arguments and ownership to System.
// Atomic counters allocate no memory and never change the allocation result. ~keep
unsafe impl GlobalAlloc for DetectOriginalRaster {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        observe(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        observe(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

pub(super) fn start(original_rgb_bytes: usize, half_rgb_bytes: usize) {
    HALF_RGB.store(half_rgb_bytes, Ordering::Relaxed);
    HALF_HITS.store(0, Ordering::Relaxed);
    MINIMUM.store(original_rgb_bytes, Ordering::Relaxed);
    HITS.store(0, Ordering::Relaxed);
    MAXIMUM.store(0, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Relaxed);
}

pub(super) fn finish() -> (usize, usize, usize) {
    ENABLED.store(false, Ordering::Relaxed);
    (
        HITS.load(Ordering::Relaxed),
        MAXIMUM.load(Ordering::Relaxed),
        HALF_HITS.load(Ordering::Relaxed),
    )
}

pub(super) fn verify_detector(original_rgb_bytes: usize) {
    start(original_rgb_bytes, 0);
    let original = std::hint::black_box(vec![255u8; original_rgb_bytes]);
    assert_eq!(original.len(), original_rgb_bytes);
    drop(original);
    let (hits, maximum, _) = finish();
    assert!(hits >= 1 && maximum >= original_rgb_bytes);
}
