use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

const PREFIX: usize = 16;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

// SAFETY: Signatures match pinned Leptonica 1.87 pix.h/allheaders.h. The hook
// stores process-lifetime function pointers; a mismatched native ABI is invalid.
unsafe extern "C" {
    fn setPixMemoryManager(
        allocator: unsafe extern "C" fn(usize) -> *mut c_void,
        deallocator: unsafe extern "C" fn(*mut c_void),
    );
}

#[derive(Clone, Copy)]
pub(crate) struct Observation {
    pub allocations: usize,
    pub frees: usize,
    pub live: usize,
    pub peak: usize,
    pub max_active: usize,
}

// SAFETY: Leptonica owns each returned payload and must release it exactly once
// through our paired callback; bypassing it would free an interior pointer.
unsafe extern "C" fn allocate(size: usize) -> *mut c_void {
    let Some(total) = size.checked_add(PREFIX) else {
        return std::ptr::null_mut();
    };
    // SAFETY: Checked total includes the prefix and payload. malloc returns a new
    // owned block or null; this callback alone owns the base until release.
    let base = unsafe { libc::malloc(total) }.cast::<u8>();
    if base.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: Non-null malloc storage has usize alignment and at least PREFIX
    // writable bytes. The prefix is initialized before Leptonica receives it.
    unsafe { base.cast::<usize>().write(size) };
    let live = LIVE.fetch_add(size, Ordering::SeqCst) + size;
    PEAK.fetch_max(live, Ordering::SeqCst);
    ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
    let active = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    MAX_ACTIVE.fetch_max(active, Ordering::SeqCst);
    // SAFETY: PREFIX is inside this allocation (or its end for zero payload).
    // Its 16-byte offset preserves the usize/u32 alignment required by Pix.
    unsafe { base.add(PREFIX).cast() }
}

// SAFETY: The hook must receive only a live payload returned by allocate,
// once. Installing after an existing Pix or using a foreign pointer is invalid.
unsafe extern "C" fn release(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: A non-null paired payload has exactly PREFIX bytes before it
    // in its still-live malloc allocation; no foreign pointers reach this hook.
    let base = unsafe { pointer.cast::<u8>().sub(PREFIX) };
    // SAFETY: allocate initialized this aligned usize prefix before publication;
    // callers may modify payload bytes only, leaving the prefix intact.
    let size = unsafe { base.cast::<usize>().read() };
    LIVE.fetch_sub(size, Ordering::SeqCst);
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
    FREES.fetch_add(1, Ordering::SeqCst);
    // SAFETY: This is the original malloc base, not the exposed interior pointer.
    // Leptonica relinquishes ownership exactly once through this callback.
    unsafe { libc::free(base.cast()) };
}

pub(crate) fn install() {
    assert_eq!(ALLOCATIONS.load(Ordering::SeqCst), 0);
    // SAFETY: Only the fresh exact single-threaded child calls this before ANY Pix
    // or native API exists. Both callbacks remain live until process exit, after
    // every Pix drops. Prior Pix owners or concurrent native calls invalidate it.
    unsafe { setPixMemoryManager(allocate, release) };
}

pub(crate) fn observation() -> Observation {
    Observation {
        allocations: ALLOCATIONS.load(Ordering::SeqCst),
        frees: FREES.load(Ordering::SeqCst),
        live: LIVE.load(Ordering::SeqCst),
        peak: PEAK.load(Ordering::SeqCst),
        max_active: MAX_ACTIVE.load(Ordering::SeqCst),
    }
}
