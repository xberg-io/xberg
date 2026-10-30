// Take a look at the license at the top of the repository in the LICENSE file.

pub mod ffi;

cfg_select! {
    all(feature = "system", not(feature = "apple-sandbox")) => {
        pub(crate) mod cpu;
        pub mod system;
        pub mod process;
    }
    all(feature = "system", feature = "apple-sandbox") => {
        pub use crate::sys::app_store::process;
    }
    _ => {}
}

#[cfg(any(
    feature = "disk",
    all(
        not(feature = "apple-sandbox"),
        any(
            feature = "system",
            all(
                feature = "component",
                any(target_arch = "x86", target_arch = "x86_64")
            )
        )
    ),
))]
pub(crate) mod utils;

#[cfg(feature = "disk")]
pub mod disk;

cfg_select! {
    feature = "component" => {
        #[cfg(feature = "apple-sandbox")]
        pub use crate::sys::app_store::component;
        #[cfg(not(feature = "apple-sandbox"))]
        pub mod component;
    }
    _ => {}
}

// Make formattable by rustfmt.
#[cfg(any())]
mod component;
#[cfg(any())]
mod cpu;
#[cfg(any())]
mod process;
#[cfg(any())]
mod system;
