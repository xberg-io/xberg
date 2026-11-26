#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "build scripts report to cargo on stdout and stderr"
)]
fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");

    let target = std::env::var("TARGET").unwrap();

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
        // The executable exports ~5k C/C++ symbols from its static libraries, 18 of them weak, which
        // makes dyld coalesce weak definitions across every loaded image at each start, and keeps
        // unused exported C code alive through -dead_strip. Nothing resolves symbols from it: the
        // libraries it dlopens (ONNX Runtime, tree-sitter grammars) import only system libraries. ~keep
        println!("cargo:rustc-link-arg-bins=-Wl,-no_exported_symbols");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }
}
