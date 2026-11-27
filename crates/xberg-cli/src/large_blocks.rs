//! Large blocks mapped straight from the kernel on macOS.
//!
//! libmalloc keeps freed large blocks mapped in its large cache for reuse, and the pages stay
//! resident and charged to the process. OCR frees a stream of differently sized page rasters,
//! decode buffers and Leptonica images that rarely fit a later request, so the cache only grows:
//! a one-page scan held 93 MB of freed blocks while 54 MB were live, and `MallocLargeCache=0`
//! cut peak RSS by 30-50% across the OCR cohorts with identical output and latency. That
//! variable is read before `main` runs, so the CLI maps its own large blocks instead and unmaps
//! them on free. ~keep

use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(any(test, feature = "ocr"))]
use std::collections::BTreeMap;
#[cfg(any(test, feature = "ocr"))]
use std::ffi::c_void;
use std::ptr;
#[cfg(any(test, feature = "ocr"))]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(any(test, feature = "ocr"))]
use std::sync::{Mutex, PoisonError};

/// Allocations of this size or more are mapped; smaller ones stay with libmalloc.
const MAPPED_THRESHOLD: usize = 1 << 20;

/// The smallest page size macOS uses, so every mapping satisfies at least this alignment.
const MIN_PAGE_ALIGN: usize = 4096;

/// Leptonica pixel buffers this module mapped, as address to mapped size.
///
/// A buffer's address is the only thing the deallocator receives, and Leptonica also frees
/// buffers this module never allocated (data handed to `pixSetData`, a Pix created before the
/// hook was installed). Membership here is therefore the only safe test for "mine": nothing
/// reads memory around a pointer whose origin is unknown.
#[cfg(any(test, feature = "ocr"))]
static MAPPED_PIX: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

/// Entries in [`MAPPED_PIX`], so the many frees of small buffers skip the lock while it is empty.
#[cfg(any(test, feature = "ocr"))]
static MAPPED_PIX_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Global allocator that maps Rust allocations of [`MAPPED_THRESHOLD`] bytes or more.
pub(crate) struct MappedLargeBlocks;

fn is_mapped(layout: Layout) -> bool {
    layout.size() >= MAPPED_THRESHOLD && layout.align() <= MIN_PAGE_ALIGN
}

#[allow(unsafe_code)]
fn map(size: usize) -> *mut u8 {
    // SAFETY: an anonymous private mapping takes no file descriptor and aliases nothing; a
    // failure is reported as MAP_FAILED and turned into the null an allocator returns.
    let block = unsafe {
        libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    if block == libc::MAP_FAILED {
        ptr::null_mut()
    } else {
        block.cast()
    }
}

/// # Safety
///
/// `block` must be a live mapping that [`map`] returned for exactly `size` bytes.
#[allow(unsafe_code)]
unsafe fn unmap(block: *mut u8, size: usize) {
    // SAFETY: the caller passes a mapping of `size` bytes from `map`; the kernel rounds both
    // calls to the same page boundary, so this releases exactly that mapping.
    unsafe { libc::munmap(block.cast(), size) };
}

// SAFETY: `is_mapped` depends on the layout alone, so `dealloc` and `realloc` release a block
// the same way it was allocated. Mappings are page-aligned and zero-filled, which satisfies every
// layout `is_mapped` accepts and `alloc_zeroed`.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for MappedLargeBlocks {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if is_mapped(layout) {
            map(layout.size())
        } else {
            // SAFETY: forwarded unchanged under the caller's `GlobalAlloc::alloc` contract.
            unsafe { System.alloc(layout) }
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if is_mapped(layout) {
            map(layout.size())
        } else {
            // SAFETY: forwarded unchanged under the caller's `GlobalAlloc::alloc_zeroed` contract.
            unsafe { System.alloc_zeroed(layout) }
        }
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        if is_mapped(layout) {
            // SAFETY: a mapped layout was allocated by `map` with this same size.
            unsafe { unmap(block, layout.size()) }
        } else {
            // SAFETY: an unmapped layout was allocated by `System` with this same layout.
            unsafe { System.dealloc(block, layout) }
        }
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the `GlobalAlloc::realloc` contract makes `new_size` non-zero and keeps it,
        // rounded up to `layout.align()`, within `isize::MAX`.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        if !is_mapped(layout) && !is_mapped(new_layout) {
            // SAFETY: both layouts belong to `System`, under the caller's `realloc` contract.
            return unsafe { System.realloc(block, layout, new_size) };
        }
        // SAFETY: `new_layout` has a non-zero size, as `alloc` requires.
        let moved = unsafe { self.alloc(new_layout) };
        if !moved.is_null() {
            // SAFETY: both blocks are live, distinct and hold at least the copied length; the
            // old block was allocated by this allocator with `layout`.
            unsafe {
                ptr::copy_nonoverlapping(block, moved, layout.size().min(new_size));
                self.dealloc(block, layout);
            }
        }
        moved
    }
}

/// Leptonica allocator for pixel data: `malloc` below [`MAPPED_THRESHOLD`], a tracked mapping at
/// or above it.
///
/// # Safety
///
/// Leptonica calls it with the byte size of a pixel buffer and releases the result only through
/// [`release_pix_data`].
#[cfg(any(test, feature = "ocr"))]
#[allow(unsafe_code)]
unsafe extern "C" fn allocate_pix_data(size: usize) -> *mut c_void {
    if size < MAPPED_THRESHOLD {
        // SAFETY: malloc returns an owned block or null; `release_pix_data` frees any block it
        // does not find in `MAPPED_PIX` with `free`.
        return unsafe { libc::malloc(size) };
    }
    let block = map(size);
    if block.is_null() {
        return ptr::null_mut();
    }
    let mut mapped = MAPPED_PIX.lock().unwrap_or_else(PoisonError::into_inner);
    mapped.insert(block as usize, size);
    MAPPED_PIX_COUNT.store(mapped.len(), Ordering::Release);
    block.cast()
}

/// Leptonica deallocator paired with [`allocate_pix_data`].
///
/// A pointer found in [`MAPPED_PIX`] is unmapped; any other pointer came from `malloc`, either
/// from [`allocate_pix_data`] below the threshold or from memory Leptonica was handed, and is
/// freed with `free` exactly as Leptonica's default deallocator would.
///
/// # Safety
///
/// `data` must be null, a pointer [`allocate_pix_data`] returned, or a `malloc` block, and
/// nothing may have released it yet.
#[cfg(any(test, feature = "ocr"))]
#[allow(unsafe_code)]
unsafe extern "C" fn release_pix_data(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    if MAPPED_PIX_COUNT.load(Ordering::Acquire) != 0 {
        let size = {
            let mut mapped = MAPPED_PIX.lock().unwrap_or_else(PoisonError::into_inner);
            let size = mapped.remove(&(data as usize));
            MAPPED_PIX_COUNT.store(mapped.len(), Ordering::Release);
            size
        };
        if let Some(size) = size {
            // SAFETY: the table holds only live mappings `map` returned for exactly `size` bytes,
            // and removing the entry makes this the single release.
            unsafe { unmap(data.cast(), size) };
            return;
        }
    }
    // SAFETY: not a tracked mapping, so by the contract above a `malloc` block.
    unsafe { libc::free(data) };
}

#[cfg(all(feature = "ocr", target_os = "macos"))]
#[allow(unsafe_code)]
unsafe extern "C" {
    // Signature from Leptonica's pix.h/allheaders.h (alloc_fn, dealloc_fn).
    fn setPixMemoryManager(
        allocator: unsafe extern "C" fn(usize) -> *mut c_void,
        deallocator: unsafe extern "C" fn(*mut c_void),
    );
}

/// Route Leptonica's pixel buffers through [`allocate_pix_data`] and [`release_pix_data`].
///
/// Leptonica frees every buffer with the deallocator current at that moment. A buffer that came
/// from `malloc` (a Pix created before this runs, or data given to `pixSetData`) is still freed
/// correctly, so installing late is safe; the first statement of `main` installs it anyway so
/// every large buffer is mapped.
#[cfg(all(feature = "ocr", target_os = "macos"))]
#[allow(unsafe_code)]
pub(crate) fn map_large_pix_data() {
    // SAFETY: both callbacks are plain functions that live for the whole process, and the
    // deallocator frees any pointer it does not track with `free`, as the default one does.
    unsafe { setPixMemoryManager(allocate_pix_data, release_pix_data) };
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    unsafe extern "C" {
        // libmalloc: the zone owning `ptr`, or null for memory no malloc zone handed out.
        fn malloc_zone_from_ptr(ptr: *const c_void) -> *mut c_void;
    }

    fn layout(size: usize) -> Layout {
        Layout::from_size_align(size, 8).expect("valid layout")
    }

    #[cfg(target_os = "macos")]
    fn owned_by_libmalloc(block: *const u8) -> bool {
        // SAFETY: the zone lookup only compares the address against zone ranges.
        !unsafe { malloc_zone_from_ptr(block.cast()) }.is_null()
    }

    #[test]
    fn size_class_is_decided_by_size_and_alignment_alone() {
        assert!(!is_mapped(layout(0)), "a zero-size layout never maps");
        assert!(!is_mapped(layout(MAPPED_THRESHOLD - 1)));
        assert!(is_mapped(layout(MAPPED_THRESHOLD)));
        assert!(is_mapped(
            Layout::from_size_align(MAPPED_THRESHOLD, MIN_PAGE_ALIGN).expect("valid layout")
        ));
        assert!(!is_mapped(
            Layout::from_size_align(MAPPED_THRESHOLD, MIN_PAGE_ALIGN * 2).expect("valid layout")
        ));
    }

    #[test]
    fn zero_size_values_never_reach_a_mapping() {
        let empty: Vec<u8> = Vec::with_capacity(0);
        assert_eq!(empty.capacity(), 0);
        let mut grown: Vec<u8> = Vec::new();
        grown.resize(MAPPED_THRESHOLD, 1);
        grown.clear();
        grown.shrink_to_fit();
        assert_eq!(grown.capacity(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn each_block_is_released_by_the_allocator_that_made_it() {
        let allocator = MappedLargeBlocks;
        for size in [16, MAPPED_THRESHOLD - 1, MAPPED_THRESHOLD, MAPPED_THRESHOLD * 4 + 5] {
            let block_layout = layout(size);
            // SAFETY: non-zero layout; the block is released with the layout it was made with.
            unsafe {
                let block = allocator.alloc(block_layout);
                assert!(!block.is_null(), "size {size}");
                assert_eq!(
                    owned_by_libmalloc(block),
                    !is_mapped(block_layout),
                    "size {size} must come from the allocator its layout selects"
                );
                allocator.dealloc(block, block_layout);
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn realloc_moves_a_block_between_allocators_when_it_crosses_the_threshold() {
        let allocator = MappedLargeBlocks;
        let small = layout(4096);
        // SAFETY: every block is released with the layout it currently has.
        unsafe {
            let block = allocator.alloc(small);
            assert!(owned_by_libmalloc(block));
            let large = allocator.realloc(block, small, MAPPED_THRESHOLD);
            assert!(!large.is_null());
            assert!(!owned_by_libmalloc(large), "growing past the threshold maps the block");
            let back = allocator.realloc(large, layout(MAPPED_THRESHOLD), MAPPED_THRESHOLD - 1);
            assert!(!back.is_null());
            assert!(
                owned_by_libmalloc(back),
                "shrinking below the threshold returns it to libmalloc"
            );
            allocator.dealloc(back, layout(MAPPED_THRESHOLD - 1));
        }
    }

    #[test]
    fn mapped_block_is_zeroed_and_writable() {
        let allocator = MappedLargeBlocks;
        let large = layout(MAPPED_THRESHOLD + 3);
        // SAFETY: non-zero layout; the block is released with the same layout.
        unsafe {
            let block = allocator.alloc_zeroed(large);
            assert!(!block.is_null());
            assert_eq!(block as usize % MIN_PAGE_ALIGN, 0, "a mapped block is page-aligned");
            let bytes = std::slice::from_raw_parts_mut(block, large.size());
            assert!(bytes.iter().all(|&byte| byte == 0));
            bytes[large.size() - 1] = 7;
            assert_eq!(bytes[large.size() - 1], 7);
            allocator.dealloc(block, large);
        }
    }

    #[test]
    fn realloc_keeps_bytes_across_the_threshold_both_ways() {
        let allocator = MappedLargeBlocks;
        let small = layout(64);
        // SAFETY: every block is released with the layout it currently has.
        unsafe {
            let block = allocator.alloc(small);
            assert!(!block.is_null());
            for offset in 0..64 {
                block.add(offset).write(offset as u8);
            }

            let grown = allocator.realloc(block, small, MAPPED_THRESHOLD * 2);
            assert!(!grown.is_null());
            let grown_layout = layout(MAPPED_THRESHOLD * 2);
            assert!((0..64).all(|offset| grown.add(offset).read() == offset as u8));
            grown.add(MAPPED_THRESHOLD * 2 - 1).write(9);

            let regrown = allocator.realloc(grown, grown_layout, MAPPED_THRESHOLD * 3);
            assert!(!regrown.is_null());
            let regrown_layout = layout(MAPPED_THRESHOLD * 3);
            assert!((0..64).all(|offset| regrown.add(offset).read() == offset as u8));
            assert_eq!(regrown.add(MAPPED_THRESHOLD * 2 - 1).read(), 9);

            let shrunk = allocator.realloc(regrown, regrown_layout, 32);
            assert!(!shrunk.is_null());
            assert!((0..32).all(|offset| shrunk.add(offset).read() == offset as u8));
            allocator.dealloc(shrunk, layout(32));
        }
    }

    #[test]
    fn over_aligned_large_layout_stays_with_the_system_allocator() {
        let over_aligned = Layout::from_size_align(MAPPED_THRESHOLD, MIN_PAGE_ALIGN * 4).expect("valid layout");
        assert!(!is_mapped(over_aligned));
        let allocator = MappedLargeBlocks;
        // SAFETY: non-zero layout; released with the same layout.
        unsafe {
            let block = allocator.alloc(over_aligned);
            assert!(!block.is_null());
            assert_eq!(block as usize % over_aligned.align(), 0);
            allocator.dealloc(block, over_aligned);
        }
    }

    fn is_tracked(data: *mut c_void) -> bool {
        MAPPED_PIX
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&(data as usize))
    }

    #[test]
    fn pix_data_round_trips_below_and_above_the_threshold() {
        for size in [4, 4096, MAPPED_THRESHOLD - 1, MAPPED_THRESHOLD, MAPPED_THRESHOLD * 3] {
            // SAFETY: each buffer is written within its size and released exactly once.
            unsafe {
                let data = allocate_pix_data(size);
                assert!(!data.is_null(), "size {size}");
                assert_eq!(data as usize % 16, 0, "size {size}");
                assert_eq!(is_tracked(data), size >= MAPPED_THRESHOLD, "size {size}");
                let bytes = data.cast::<u8>();
                bytes.write(1);
                bytes.add(size - 1).write(2);
                assert_eq!(bytes.add(size - 1).read(), 2);
                release_pix_data(data);
                assert!(!is_tracked(data), "size {size} must leave the table when released");
            }
        }
        // SAFETY: a null pointer is accepted and ignored.
        unsafe { release_pix_data(ptr::null_mut()) };
    }

    /// `pixSetData` hands Leptonica memory it never allocated, and `pixDestroy` then frees it
    /// through the hook. A block that is large and filled with 0xFF would be unmapped with a
    /// garbage size by any deallocator that trusted a header in front of it.
    #[test]
    fn foreign_malloc_blocks_are_freed_not_unmapped() {
        for size in [64, MAPPED_THRESHOLD * 2] {
            // SAFETY: the block is a fresh malloc allocation released exactly once.
            unsafe {
                let foreign = libc::malloc(size);
                assert!(!foreign.is_null());
                ptr::write_bytes(foreign.cast::<u8>(), 0xFF, size);
                assert!(!is_tracked(foreign));
                release_pix_data(foreign);
            }
        }
    }

    #[test]
    fn foreign_blocks_are_freed_while_mapped_buffers_are_live() {
        // SAFETY: every block is released exactly once, by the function matching its origin.
        unsafe {
            let mapped = allocate_pix_data(MAPPED_THRESHOLD);
            assert!(is_tracked(mapped));
            let foreign = libc::malloc(MAPPED_THRESHOLD + 8);
            ptr::write_bytes(foreign.cast::<u8>(), 0xFF, MAPPED_THRESHOLD + 8);
            release_pix_data(foreign);
            assert!(is_tracked(mapped), "freeing a foreign block must not disturb a mapping");
            mapped.cast::<u8>().add(MAPPED_THRESHOLD - 1).write(3);
            release_pix_data(mapped);
            assert!(!is_tracked(mapped));
        }
    }

    #[test]
    fn mapped_buffers_released_on_other_threads_leave_no_entries() {
        let handles: Vec<_> = (0..8)
            .map(|index| {
                std::thread::spawn(move || {
                    let mut addresses = Vec::new();
                    for round in 0..16 {
                        let size = if (index + round) % 2 == 0 {
                            MAPPED_THRESHOLD + round * 4096
                        } else {
                            256 + round
                        };
                        // SAFETY: the buffer is written within its size and released once.
                        unsafe {
                            let data = allocate_pix_data(size);
                            assert!(!data.is_null());
                            data.cast::<u8>().add(size - 1).write(1);
                            addresses.push(data as usize);
                            release_pix_data(data);
                        }
                    }
                    addresses
                })
            })
            .collect();
        for handle in handles {
            for address in handle.join().expect("worker") {
                assert!(!is_tracked(address as *mut c_void));
            }
        }
    }
}
