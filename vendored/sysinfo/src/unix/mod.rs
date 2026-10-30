// Take a look at the license at the top of the repository in the LICENSE file.

cfg_select! {
    any(target_os = "macos", target_os = "ios") => {
        pub(crate) mod apple;
        pub(crate) use apple as sys;

        #[allow(unused_imports)]
        pub(crate) use libc::__error as libc_errno;
    }
    any(target_os = "linux", target_os = "android") => {
        pub(crate) mod linux;
        pub(crate) use linux as sys;

        #[cfg(target_os = "linux")]
        #[allow(unused_imports)]
        pub(crate) use libc::__errno_location as libc_errno;
        #[cfg(target_os = "android")]
        #[allow(unused_imports)]
        pub(crate) use libc::__errno as libc_errno;
    }
    any(target_os = "freebsd", target_os = "netbsd") => {
        pub(crate) mod bsd;
        pub(crate) use bsd as sys;

        #[allow(unused_imports)]
        pub(crate) use bsd::libc_errno;
    }
    _ => {
        compile_error!("Invalid cfg!");
    }
}

cfg_select! {
    feature = "disk" => {
        pub(crate) struct DisksInner {
            pub(crate) disks: Vec<crate::Disk>,
        }

        impl DisksInner {
            pub(crate) fn from_vec(disks: Vec<crate::Disk>) -> Self {
                Self { disks }
            }

            pub(crate) fn into_vec(self) -> Vec<crate::Disk> {
                self.disks
            }
        }
    }
    _ => {}
}

#[cfg(all(feature = "network", not(target_os = "ios")))]
pub(crate) mod network_helper;

cfg_select! {
    feature = "user" => {
        // On iOS the apple module provides its own `UserInner`/`get_users`
        // stubs, so `unix::users` is unused there.
        #[cfg(not(target_os = "ios"))]
        pub(crate) mod users;
        pub(crate) mod groups;
    }
    _ => {}
}

pub(crate) mod utils;

// Make formattable by rustfmt.
#[cfg(any())]
mod apple;
#[cfg(any())]
mod bsd;
#[cfg(any())]
mod groups;
#[cfg(any())]
mod linux;
#[cfg(any())]
mod users;
