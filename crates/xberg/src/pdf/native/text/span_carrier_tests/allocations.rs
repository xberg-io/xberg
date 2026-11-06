use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static BYTES: Cell<Option<usize>> = const { Cell::new(None) };
}

fn record(bytes: usize) {
    let _ = BYTES.try_with(|counter| {
        if let Some(current) = counter.get() {
            counter.set(Some(current.saturating_add(bytes)));
        }
    });
}

// SAFETY: allocation and deallocation forward the original arguments to System. Only
// successful allocations on the explicitly measured thread update a const-initialized Cell. ~keep
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            record(size);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

pub(super) fn measured<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            BYTES.with(|counter| counter.set(None));
        }
    }
    BYTES.with(|counter| assert!(counter.replace(Some(0)).is_none(), "no nested allocation measurement"));
    let guard = Reset;
    let result = operation();
    let count = BYTES.with(|counter| counter.get().unwrap());
    drop(guard);
    (result, count)
}
