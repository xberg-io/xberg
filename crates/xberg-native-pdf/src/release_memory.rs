//! Hand freed heap pages back to the OS between extraction phases.

/// Return pages the allocator holds free to the OS, where the platform allocator keeps them resident.
///
/// macOS's malloc leaves freed blocks of the page passes (tens of KB each, freed on one worker thread)
/// resident and unreusable by the other workers' magazines, so a later phase maps fresh pages beside them.
/// Each call walks every malloc zone, so call it per phase, not per page. No-op elsewhere.
pub fn return_freed_memory_to_os() {
    #[cfg(target_os = "macos")]
    macos::pressure_relief();
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos {
    unsafe extern "C" {
        fn malloc_zone_pressure_relief(zone: *mut core::ffi::c_void, goal: usize) -> usize;
    }

    pub(super) fn pressure_relief() {
        // SAFETY: a null zone means every zone and a zero goal means release as much as possible; the
        // call takes no pointers from us and only madvises pages the allocator already holds free.
        unsafe {
            malloc_zone_pressure_relief(core::ptr::null_mut(), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn returning_freed_memory_keeps_live_allocations_intact() {
        let kept: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        drop(vec![1u8; 4 << 20]);
        super::return_freed_memory_to_os();
        super::return_freed_memory_to_os();
        assert!(kept.iter().enumerate().all(|(i, b)| *b == (i as u32 % 251) as u8));
    }
}
