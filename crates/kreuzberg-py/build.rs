use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let profile_dir = match profile.as_str() {
        "dev" | "test" => "debug",
        other => other,
    };

    if target.contains("apple-darwin") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-undefined,dynamic_lookup");
    }

    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(&cargo_manifest_dir);

    if let Some(workspace_root) = manifest_path.parent().and_then(|p| p.parent()) {
        let host_lib_dir = workspace_root.join("target").join(profile_dir);
        let target_lib_dir = workspace_root.join("target").join(&target).join(profile_dir);

        if !target.contains("windows") {
            let static_lib_name = if target.contains("windows") {
                "kreuzberg_ffi.lib"
            } else {
                "libkreuzberg_ffi.a"
            };

            for lib_dir in [&host_lib_dir, &target_lib_dir] {
                let static_lib = lib_dir.join(static_lib_name);
                if static_lib.exists() {
                    println!("cargo:rustc-link-arg={}", static_lib.display());
                    if target.contains("darwin") {
                        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
                    } else if target.contains("linux") {
                        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
                    }
                    println!("cargo:rerun-if-changed=build.rs");
                    return;
                }
            }
        }

        let static_lib_name = if target.contains("windows") {
            "kreuzberg_ffi.lib"
        } else {
            "libkreuzberg_ffi.a"
        };
        let mut found_static = false;
        for dir in [host_lib_dir, target_lib_dir] {
            if dir.exists() {
                println!("cargo:rustc-link-search=native={}", dir.display());
                if dir.join(static_lib_name).exists() {
                    found_static = true;
                }
            }
        }
        if found_static {
            println!("cargo:rustc-link-lib=static=kreuzberg_ffi");
            if target.contains("darwin") {
                println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
            } else if target.contains("linux") {
                println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
            }
            println!("cargo:rerun-if-changed=build.rs");
            return;
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}
