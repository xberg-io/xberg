#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "build scripts report to cargo on stdout and stderr"
)]
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={ORDER_FILE_ENV}");
    println!("cargo:rerun-if-env-changed={LINK_MAP_ENV}");

    let target = std::env::var("TARGET").unwrap();

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
        // The executable exports ~5k C/C++ symbols from its static libraries, 18 of them weak, which
        // makes dyld coalesce weak definitions across every loaded image at each start, and keeps
        // unused exported C code alive through -dead_strip. Nothing resolves symbols from it: the
        // libraries it dlopens (ONNX Runtime, libheif and its codecs, tree-sitter grammars) import
        // only system libraries and each other. ~keep
        println!("cargo:rustc-link-arg-bins=-Wl,-no_exported_symbols");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }

    link_order_file(&target);
}

const ORDER_FILE_ENV: &str = "XBERG_ORDER_FILE";
const LINK_MAP_ENV: &str = "XBERG_LINK_MAP";

/// Lays out the functions a training run touched contiguously (ADR-0020). Dev builds never set
/// the variable; an entry the binary lacks is skipped by the linker, so a stale file only costs
/// the layout it no longer describes.
fn link_order_file(target: &str) {
    let order_file = std::env::var_os(ORDER_FILE_ENV).filter(|value| !value.is_empty());
    let link_map = std::env::var_os(LINK_MAP_ENV).filter(|value| !value.is_empty());
    if order_file.is_none() && link_map.is_none() {
        return;
    }
    let apple = target.contains("apple");
    if !apple && !target.contains("linux") {
        println!("cargo:warning={ORDER_FILE_ENV}/{LINK_MAP_ENV} ignored: no linker support wired for {target}");
        return;
    }
    if let Some(order_file) = order_file {
        let order_file = PathBuf::from(order_file);
        assert!(
            order_file.is_file(),
            "{ORDER_FILE_ENV}={} is not a file",
            order_file.display()
        );
        println!("cargo:rerun-if-changed={}", order_file.display());
        if apple {
            println!("cargo:rustc-link-arg-bins=-Wl,-order_file,{}", order_file.display());
        } else {
            // GNU ld has no symbol ordering; route the link through the toolchain's rust-lld, the
            // linker rustc already uses by default on x86_64-unknown-linux-gnu. ~keep
            let gcc_ld = rust_lld_driver_dir();
            println!("cargo:rustc-link-arg-bins=-fuse-ld=lld");
            println!("cargo:rustc-link-arg-bins=-B{}", gcc_ld.display());
            println!(
                "cargo:rustc-link-arg-bins=-Wl,--symbol-ordering-file={}",
                order_file.display()
            );
            println!("cargo:rustc-link-arg-bins=-Wl,--no-warn-symbol-ordering");
        }
    }
    if let Some(link_map) = link_map {
        let link_map = Path::new(&link_map).display().to_string();
        if apple {
            println!("cargo:rustc-link-arg-bins=-Wl,-map,{link_map}");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,-Map={link_map}");
        }
    }
}

fn rust_lld_driver_dir() -> PathBuf {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let host = std::env::var("HOST").unwrap();
    let output = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("run rustc --print sysroot");
    assert!(output.status.success(), "rustc --print sysroot failed");
    let sysroot = String::from_utf8(output.stdout).expect("sysroot is UTF-8");
    let dir = Path::new(sysroot.trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin/gcc-ld");
    assert!(
        dir.join("ld.lld").is_file(),
        "{} has no ld.lld; the toolchain ships rust-lld there",
        dir.display()
    );
    dir
}
