//! A flate bomb must abort at `ratio * compressed_len` bytes, not after full inflation.

use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg_native_pdf::decoders::decode_stream;

struct PeakAlloc;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every operation delegates unchanged to System; the counters do not allocate.
#[allow(
    unsafe_code,
    reason = "measuring real allocations requires delegating the GlobalAlloc interface"
)]
unsafe impl GlobalAlloc for PeakAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let now = if new_size >= layout.size() {
                CURRENT.fetch_add(new_size - layout.size(), Ordering::SeqCst) + (new_size - layout.size())
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::SeqCst) - (layout.size() - new_size)
            };
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOC: PeakAlloc = PeakAlloc;

#[test]
fn flate_bomb_aborts_without_materialising_the_full_inflation() {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let zeros = vec![0u8; 1 << 20];
    for _ in 0..64 {
        encoder.write_all(&zeros).unwrap();
    }
    let bomb = encoder.finish().unwrap();
    drop(zeros);
    assert!(
        bomb.len() < 100_000,
        "fixture is not a small bomb: {} bytes",
        bomb.len()
    );

    let baseline = CURRENT.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);
    let result = decode_stream(&bomb, &["FlateDecode".to_string()]);
    let peak_growth = PEAK.load(Ordering::SeqCst) - baseline;

    assert!(result.is_err(), "bomb decoded successfully");
    let ratio_budget = bomb.len() * 101;
    assert!(
        peak_growth < ratio_budget * 3,
        "peak allocation {peak_growth} exceeds 3x the ratio budget {ratio_budget}"
    );
}
