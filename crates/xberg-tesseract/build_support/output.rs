use std::fmt::Arguments;
use std::path::Path;

pub fn status(message: Arguments<'_>) {
    eprintln!("{message}");
}

pub fn link_library(library: &str, link_type: &str, link_name: &str) {
    println!("cargo:rustc-link-lib={link_type}={link_name}");
    status(format_args!(
        "Linking with {library} ({link_type} linking): {link_name}"
    ));
}

pub fn available_library(path: &Path) {
    status(format_args!(
        "Library already available at expected location: {}",
        path.display()
    ));
}
