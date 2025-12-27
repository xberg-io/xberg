use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("Build script error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set".to_string())?;

    let config =
        cbindgen::Config::from_file("cbindgen.toml").map_err(|e| format!("Failed to load cbindgen config: {}", e))?;

    cbindgen::generate_with_config(&crate_dir, config)
        .map_err(|e| format!("Failed to generate C bindings: {}", e))?
        .write_to_file("kreuzberg.h");

    let pc_template = std::fs::read_to_string("kreuzberg-ffi.pc.in")
        .map_err(|e| format!("Failed to read pkg-config template: {}", e))?;

    let version = env::var("CARGO_PKG_VERSION").map_err(|_| "CARGO_PKG_VERSION not set".to_string())?;

    let repo_root = Path::new(&crate_dir).parent().and_then(|p| p.parent()).ok_or_else(|| {
        "CARGO_MANIFEST_DIR did not have expected depth (expected crates/kreuzberg-ffi/...)".to_string()
    })?;

    let dev_prefix = repo_root.to_string_lossy().replace('\\', "/");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_else(|_| "gnu".to_string());

    let libs_private = match target_os.as_str() {
        "linux" => "-lpthread -ldl -lm",
        "macos" => "-framework CoreFoundation -framework Security -lpthread",
        "windows" => match target_env.as_str() {
            "msvc" => "-lws2_32 -luserenv -lbcrypt",
            "gnu" => "-lpthread -lws2_32 -luserenv -lbcrypt -static-libgcc -static-libstdc++",
            _ => "-lws2_32 -luserenv -lbcrypt",
        },
        _ => "",
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|_| "OUT_DIR not set".to_string())?);
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or_else(|| "OUT_DIR did not have expected depth (expected target/{debug,release}/build/...)".to_string())?;

    copy_pdfium_to_profile_dir(profile_dir)?;

    let dev_libdir = profile_dir.to_string_lossy().replace('\\', "/");
    let dev_includedir = format!("{}/crates/kreuzberg-ffi", dev_prefix);
    let dev_pc = format!(
        r#"prefix={}
exec_prefix=${{prefix}}
libdir={}
includedir={}

Name: kreuzberg-ffi
Description: C FFI bindings for Kreuzberg document intelligence library
Version: {}
URL: https://kreuzberg.dev
Libs: -L${{libdir}} -lkreuzberg_ffi
Libs.private: {}
Cflags: -I${{includedir}}
"#,
        dev_prefix, dev_libdir, dev_includedir, version, libs_private
    );
    std::fs::write("kreuzberg-ffi.pc", dev_pc).map_err(|e| format!("Failed to write development pkg-config: {}", e))?;

    let install_pc = pc_template
        .replace("@PREFIX@", "/usr/local")
        .replace("@VERSION@", &version)
        .replace("@LIBS_PRIVATE@", libs_private);
    std::fs::write("kreuzberg-ffi-install.pc", install_pc)
        .map_err(|e| format!("Failed to write installation pkg-config: {}", e))?;

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@loader_path");

        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@executable_path/../target/release");
    }

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=kreuzberg-ffi.pc.in");

    Ok(())
}

/// Copy bundled PDFium library from kreuzberg build output to profile directory.
/// This enables Java/other language bindings to find libpdfium.dylib/so/dll at runtime.
fn copy_pdfium_to_profile_dir(profile_dir: &Path) -> Result<(), String> {
    let build_dir = profile_dir.join("build");

    if let Ok(entries) = fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("kreuzberg-"))
            {
                let out_dir = path.join("out");
                if out_dir.exists() {
                    if copy_pdfium_from_dir(&out_dir, profile_dir).is_err() {
                        continue;
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }

    eprintln!("Warning: bundled PDFium library not found in build output. Some features may not work.");
    eprintln!("If PDFium is needed, ensure the 'bundled-pdfium' feature is enabled.");

    Ok(())
}

/// Copy PDFium library files from source directory to destination.
fn copy_pdfium_from_dir(src_dir: &Path, dest_dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(src_dir).map_err(|e| format!("Failed to read {}: {}", src_dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().ok_or("No file name")?;
        let file_name_str = file_name.to_string_lossy();

        if file_name_str.starts_with("libpdfium") || file_name_str.starts_with("pdfium") {
            let dest_file = dest_dir.join(file_name);

            if dest_file.exists() {
                eprintln!(
                    "PDFium library already exists at {}, skipping copy",
                    dest_file.display()
                );
                return Ok(());
            }

            match fs::copy(&path, &dest_file) {
                Ok(bytes_copied) => {
                    eprintln!(
                        "Copied {} ({} bytes) to {}",
                        path.display(),
                        bytes_copied,
                        dest_file.display()
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to copy {} to {}: {}", path.display(), dest_file.display(), e);
                    return Err(format!("Failed to copy PDFium: {}", e));
                }
            }
        }
    }

    Err(format!("PDFium library not found in {}", src_dir.display()))
}
