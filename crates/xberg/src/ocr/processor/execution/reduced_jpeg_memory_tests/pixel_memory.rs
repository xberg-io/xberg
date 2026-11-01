use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

const PREFIX: usize = 16;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

// Exact Leptonica1.87 pix.h alloc_fn/dealloc_fn and allheaders.h signature.
// This process-global hook is installed only inside one exact child test. ~keep
unsafe extern "C" {
    fn setPixMemoryManager(
        allocator: unsafe extern "C" fn(usize) -> *mut c_void,
        deallocator: unsafe extern "C" fn(*mut c_void),
    );
}

#[derive(Clone, Copy)]
pub(super) struct Observation {
    pub allocations: usize,
    pub frees: usize,
    pub live: usize,
    pub peak: usize,
    pub max_active: usize,
}

unsafe extern "C" fn allocate(size: usize) -> *mut c_void {
    let Some(total) = size.checked_add(PREFIX) else {
        return std::ptr::null_mut();
    };
    // malloc retains its native alignment after the16-byte size prefix. Track
    // requested image payload only; instrumentation bytes are not image bytes. ~keep
    let base = unsafe { libc::malloc(total) }.cast::<u8>();
    if base.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { base.cast::<usize>().write(size) };
    let live = LIVE.fetch_add(size, Ordering::SeqCst) + size;
    PEAK.fetch_max(live, Ordering::SeqCst);
    ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
    let active = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    MAX_ACTIVE.fetch_max(active, Ordering::SeqCst);
    unsafe { base.add(PREFIX).cast() }
}

unsafe extern "C" fn release(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }
    let base = unsafe { pointer.cast::<u8>().sub(PREFIX) };
    let size = unsafe { base.cast::<usize>().read() };
    LIVE.fetch_sub(size, Ordering::SeqCst);
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
    FREES.fetch_add(1, Ordering::SeqCst);
    unsafe { libc::free(base.cast()) };
}

pub(super) fn install() {
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
    // No native API or Pix exists yet; the child exits after every owner drops.
    // Callbacks use malloc/free directly and never allocate tracker containers. ~keep
    unsafe { setPixMemoryManager(allocate, release) };
}

pub(super) fn observation() -> Observation {
    Observation {
        allocations: ALLOCATIONS.load(Ordering::SeqCst),
        frees: FREES.load(Ordering::SeqCst),
        live: LIVE.load(Ordering::SeqCst),
        peak: PEAK.load(Ordering::SeqCst),
        max_active: MAX_ACTIVE.load(Ordering::SeqCst),
    }
}
