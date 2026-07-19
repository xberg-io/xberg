fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");

    // `inference_ort` marks builds where the ONNX Runtime engine is linked, so the
    // inference seam ([`crate::inference`]) can compile-time select ONNX Runtime
    // over tract. Every ORT-backed capability enables `ort-bundled` (the default)
    // or opts into `ort-dynamic`; either implies the `ort` crate is present. On
    // no-ORT targets (WASM, Android x86_64) neither is set, so the seam falls back
    // to the pure-Rust tract backend.
    println!("cargo::rustc-check-cfg=cfg(inference_ort)");
    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        || std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!("cargo::rustc-cfg=inference_ort");
    }

    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        && std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!(
            "cargo::warning=features 'ort-bundled' and 'ort-dynamic' are both enabled; bundled ORT remains the default unless dynamic ORT is explicitly selected at runtime"
        );
    }
}
