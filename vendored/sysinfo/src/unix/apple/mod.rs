// Take a look at the license at the top of the repository in the LICENSE file.

#[cfg(any(target_os = "ios", feature = "apple-sandbox"))]
pub(crate) mod app_store;

mod ffi;
mod utils;

cfg_select! {
    all(target_os = "macos", any(feature = "disk", feature = "system", feature = "component")) => {
        pub(crate) mod macos;
        pub(crate) use self::macos as inner;
    }
    all(target_os = "ios", any(feature = "system", feature = "component")) => {
        pub(crate) mod ios;
        pub(crate) use self::ios as inner;
    }
    _ => {}
}
cfg_select! {
    feature = "system" => {
        pub mod cpu;
        pub mod motherboard;
        pub mod process;
        pub mod product;
        pub mod system;

        pub(crate) use self::cpu::CpuInner;
        pub(crate) use self::motherboard::MotherboardInner;
        pub(crate) use self::process::ProcessInner;
        pub(crate) use self::product::ProductInner;
        pub(crate) use self::system::SystemInner;
        pub use self::system::{MINIMUM_CPU_UPDATE_INTERVAL, SUPPORTED_SIGNALS};
    }
    _ => {}
}
cfg_select! {
    feature = "disk" => {
        pub mod disk;

        pub(crate) use self::disk::DiskInner;
        pub(crate) use crate::unix::DisksInner;
    }
    _ => {}
}
cfg_select! {
    feature = "component" => {
        pub mod component;

        pub(crate) use self::component::{ComponentInner, ComponentsInner};
    }
    _ => {}
}
cfg_select! {
    feature = "network" => {
        #[cfg(target_os = "macos")]
        pub mod network;
        // iOS does not expose the route/if_mib APIs used by the macOS implementation.
        #[cfg(target_os = "ios")]
        #[path = "../../unknown/network.rs"]
        pub mod network;

        pub(crate) use self::network::{NetworkDataInner, NetworksInner};
    }
    _ => {}
}
cfg_select! {
    feature = "user" => {
        // `users.rs` binds OpenDirectory, which is macOS-only. On iOS the
        // framework does not exist, so we reuse the `app_store` stub the
        // same way `component` and `process` do.
        #[cfg(target_os = "macos")]
        pub mod users;
        #[cfg(target_os = "ios")]
        pub use crate::sys::app_store::users;

        pub(crate) use crate::unix::groups::get_groups;
        #[cfg(target_os = "macos")]
        pub(crate) use crate::unix::users::UserInner;
        #[cfg(target_os = "ios")]
        pub(crate) use self::users::UserInner;
        pub(crate) use self::users::get_users;
    }
    _ => {}
}

#[doc = include_str!("../../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;

// Make formattable by rustfmt.
#[cfg(any())]
mod component;
#[cfg(any())]
mod cpu;
#[cfg(any())]
mod disk;
#[cfg(any())]
mod ios;
#[cfg(any())]
mod macos;
#[cfg(any())]
mod motherboard;
#[cfg(any())]
mod network;
#[cfg(any())]
mod process;
#[cfg(any())]
mod product;
#[cfg(any())]
mod system;
#[cfg(any())]
mod users;
