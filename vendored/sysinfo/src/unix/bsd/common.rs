// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(feature = "system")]
#[inline]
pub unsafe fn init_mib(name: &[u8], mib: &mut [libc::c_int]) {
    let mut len = mib.len();
    unsafe {
        libc::sysctlnametomib(name.as_ptr() as _, mib.as_mut_ptr(), &mut len);
    }
}

#[cfg(feature = "system")]
pub(crate) fn boot_time() -> u64 {
    let mut boot_time = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = std::mem::size_of::<libc::timeval>();
    let mut mib: [libc::c_int; 2] = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    unsafe {
        if libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            &mut boot_time as *mut libc::timeval as *mut _,
            &mut len,
            std::ptr::null_mut(),
            0,
        ) < 0
        {
            0
        } else {
            boot_time.tv_sec as _
        }
    }
}
