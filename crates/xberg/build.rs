fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage)");

    println!("cargo::rustc-check-cfg=cfg(inference_ort)");
    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        || std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!("cargo::rustc-cfg=inference_ort");
    }

    println!("cargo::rustc-check-cfg=cfg(auto_rotate)");
    if std::env::var_os("CARGO_FEATURE_AUTO_ROTATE").is_some()
        || std::env::var_os("CARGO_FEATURE_AUTO_ROTATE_TRACT").is_some()
    {
        println!("cargo::rustc-cfg=auto_rotate");
    }

    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        && std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!(
            "cargo::warning=features 'ort-bundled' and 'ort-dynamic' are both enabled; bundled ORT remains the default unless dynamic ORT is explicitly selected at runtime"
        );
    }

    // `ort-static` links against a from-source, static ONNX Runtime build (see
    // scripts/ci/build-static-onnxruntime.sh) via ort-sys's "system strategy", which
    // resolves the build location from ORT_LIB_LOCATION/ORT_LIB_PATH at *its own* build
    // time. If neither is set, ort-sys silently falls through to its download/dynamic-link
    // paths (or fails much later, deep in the link step, with a confusing error). Fail
    // fast here instead, before any of that happens, with an actionable message.
    if std::env::var_os("CARGO_FEATURE_ORT_STATIC").is_some()
        && std::env::var_os("ORT_LIB_PATH").is_none()
        && std::env::var_os("ORT_LIB_LOCATION").is_none()
    {
        panic!(
            "the `ort-static` feature requires ORT_LIB_LOCATION (or ORT_LIB_PATH) to point at \
             a from-source, static ONNX Runtime build (--build_shared_lib omitted, --config \
             Release) — see scripts/ci/build-static-onnxruntime.sh. Also set \
             ORT_PREFER_DYNAMIC_LINK=0 so ort-sys does not prefer dynamic linking."
        );
    }

    println!("cargo::rustc-check-cfg=cfg(layout_detection)");
    if std::env::var_os("CARGO_FEATURE_LAYOUT_DETECTION").is_some()
        || std::env::var_os("CARGO_FEATURE_LAYOUT_TRACT").is_some()
    {
        println!("cargo::rustc-cfg=layout_detection");
    }
}
