// Take a look at the license at the top of the repository in the LICENSE file.

mod utils;

cfg_select! {
    feature = "system" => {
        mod cpu;
        mod ffi;
        mod motherboard;
        mod process;
        mod product;
        mod system;

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
        mod disk;

        pub(crate) use self::disk::{DiskInner, DisksInner};
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
        mod network;
        pub(crate) mod network_helper;

        pub(crate) use self::network::{NetworkDataInner, NetworksInner};
    }
    _ => {}
}
cfg_select! {
    feature = "user" => {
        mod groups;
        mod users;

        pub(crate) use self::groups::get_groups;
        pub(crate) use self::users::get_users;
        pub(crate) use self::users::UserInner;
    }
    _ => {}
}
cfg_select! {
    any(feature = "user", feature = "system") => {
        mod sid;

        pub(crate) use self::sid::Sid;
    }
    _ => {}
}

#[doc = include_str!("../../md_doc/is_supported.md")]
pub const IS_SUPPORTED_SYSTEM: bool = true;

// Make formattable by rustfmt.
#[cfg(any())]
mod component;
#[cfg(any())]
mod cpu;
#[cfg(any())]
mod disk;
#[cfg(any())]
mod ffi;
#[cfg(any())]
mod groups;
#[cfg(any())]
mod motherboard;
#[cfg(any())]
mod network;
#[cfg(any())]
mod network_helper;
#[cfg(any())]
mod process;
#[cfg(any())]
mod product;
#[cfg(any())]
mod sid;
#[cfg(any())]
mod system;
#[cfg(any())]
mod users;
