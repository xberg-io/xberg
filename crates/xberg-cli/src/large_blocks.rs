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
use std::ffi::c_void;
use std::ptr;

/// Allocations of this size or more are mapped; smaller ones stay with libmalloc.
const MAPPED_THRESHOLD: usize = 1 << 20;

/// The smallest page size macOS uses, so every mapping satisfies at least this alignment.
const MIN_PAGE_ALIGN: usize = 4096;

/// Header in front of each Leptonica pixel buffer, holding the size of the whole block.
#[cfg(any(test, feature = "ocr"))]
const PIX_PREFIX: usize = 16;

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

/// Leptonica allocator for pixel data, mapping blocks of [`MAPPED_THRESHOLD`] bytes or more.
///
/// # Safety
///
/// Leptonica calls it with the byte size of a pixel buffer and releases the result only through
/// [`release_pix_data`].
#[cfg(any(test, feature = "ocr"))]
#[allow(unsafe_code)]
unsafe extern "C" fn allocate_pix_data(size: usize) -> *mut c_void {
    let Some(total) = size.checked_add(PIX_PREFIX) else {
        return ptr::null_mut();
    };
    let base: *mut u8 = if total >= MAPPED_THRESHOLD {
        map(total)
    } else {
        // SAFETY: `total` is non-zero; malloc returns an owned block or null.
        unsafe { libc::malloc(total) }.cast()
    };
    if base.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: `base` is at least 16-byte aligned and holds `total > PIX_PREFIX` bytes, so the
    // header write and the returned offset stay inside the block and keep Pix's u32 alignment.
    unsafe {
        base.cast::<usize>().write(total);
        base.add(PIX_PREFIX).cast()
    }
}

/// Leptonica deallocator paired with [`allocate_pix_data`].
///
/// # Safety
///
/// `data` must be null or a pointer [`allocate_pix_data`] returned and nothing released yet.
#[cfg(any(test, feature = "ocr"))]
#[allow(unsafe_code)]
unsafe extern "C" fn release_pix_data(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    // SAFETY: `allocate_pix_data` put the block's total size `PIX_PREFIX` bytes before `data`,
    // and that header lies outside the pixels Leptonica writes.
    unsafe {
        let base = data.cast::<u8>().sub(PIX_PREFIX);
        let total = base.cast::<usize>().read();
        if total >= MAPPED_THRESHOLD {
            unmap(base, total);
        } else {
            libc::free(base.cast());
        }
    }
}

#[cfg(feature = "ocr")]
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
/// Leptonica frees every buffer with the deallocator current at that moment, so this must run
/// before the first Pix exists: the first statement of `main` does.
#[cfg(feature = "ocr")]
#[allow(unsafe_code)]
pub(crate) fn map_large_pix_data() {
    // SAFETY: called once from the start of `main`, before any thread or Pix exists; both
    // callbacks are plain functions that live for the whole process.
    unsafe { setPixMemoryManager(allocate_pix_data, release_pix_data) };
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    fn layout(size: usize) -> Layout {
        Layout::from_size_align(size, 8).expect("valid layout")
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

    #[test]
    fn pix_data_round_trips_below_and_above_the_threshold() {
        for size in [4, 4096, MAPPED_THRESHOLD - PIX_PREFIX, MAPPED_THRESHOLD * 3] {
            // SAFETY: each buffer is written within its size and released exactly once.
            unsafe {
                let data = allocate_pix_data(size).cast::<u8>();
                assert!(!data.is_null(), "size {size}");
                assert_eq!(data as usize % 16, 0, "size {size}");
                data.write(1);
                data.add(size - 1).write(2);
                assert_eq!(data.add(size - 1).read(), 2);
                release_pix_data(data.cast());
            }
        }
        // SAFETY: a null pointer is accepted and ignored.
        unsafe { release_pix_data(ptr::null_mut()) };
    }
}
