#![allow(clippy::uninlined_format_args)]

#[cfg(feature = "build-tesseract")]
mod build_tesseract {
    use cmake::Config;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    const LEPTONICA_VERSION: &str = "1.87.0";
    const TESSERACT_VERSION: &str = "5.5.2";

    fn leptonica_url() -> String {
        format!(
            "https://codeload.github.com/DanBloomberg/leptonica/zip/refs/tags/{}",
            LEPTONICA_VERSION
        )
    }

    fn tesseract_url() -> String {
        format!(
            "https://codeload.github.com/tesseract-ocr/tesseract/zip/refs/tags/{}",
            TESSERACT_VERSION
        )
    }

    fn workspace_cache_dir_from_out_dir() -> Option<PathBuf> {
        let out_dir = env::var_os("OUT_DIR")?;
        let mut path = PathBuf::from(out_dir);
        for _ in 0..4 {
            if !path.pop() {
                return None;
            }
        }
        Some(path.join("kreuzberg-tesseract-cache"))
    }

    fn get_preferred_out_dir() -> PathBuf {
        if let Ok(custom) = env::var("TESSERACT_RS_CACHE_DIR") {
            return PathBuf::from(custom);
        }

        if cfg!(target_os = "windows") {
            return PathBuf::from("C:\\tess");
        }

        if let Some(workspace_cache) = workspace_cache_dir_from_out_dir() {
            return workspace_cache;
        }

        if cfg!(target_os = "macos") {
            let home_dir = env::var("HOME").unwrap_or_else(|_| {
                env::var("USER")
                    .map(|user| format!("/Users/{}", user))
                    .expect("Neither HOME nor USER environment variable set")
            });
            PathBuf::from(home_dir)
                .join("Library")
                .join("Application Support")
                .join("kreuzberg-tesseract")
        } else if cfg!(target_os = "linux") {
            let home_dir = env::var("HOME").unwrap_or_else(|_| {
                env::var("USER")
                    .map(|user| format!("/home/{}", user))
                    .expect("Neither HOME nor USER environment variable set")
            });
            PathBuf::from(home_dir).join(".kreuzberg-tesseract")
        } else {
            panic!("Unsupported operating system");
        }
    }

    fn target_triple() -> String {
        env::var("TARGET").unwrap_or_else(|_| env::var("HOST").unwrap_or_default())
    }

    fn target_matches(target: &str, needle: &str) -> bool {
        target.contains(needle)
    }

    fn is_windows_target(target: &str) -> bool {
        target_matches(target, "windows")
    }

    fn is_macos_target(target: &str) -> bool {
        target_matches(target, "apple-darwin")
    }

    fn is_linux_target(target: &str) -> bool {
        target_matches(target, "linux")
    }

    fn is_msvc_target(target: &str) -> bool {
        is_windows_target(target) && target_matches(target, "msvc")
    }

    fn is_mingw_target(target: &str) -> bool {
        is_windows_target(target) && target_matches(target, "gnu")
    }

    fn is_wasm_target(target: &str) -> bool {
        target_matches(target, "wasm32") || target_matches(target, "wasm64")
    }

    /// Resolve the C++ compiler for CMake, following the cc-rs/Cargo convention:
    /// 1. Check `CXX` env var (explicit override)
    /// 2. Check target-specific `CXX_{target}` env var (e.g. `CXX_x86_64_unknown_linux_musl`)
    /// 3. Fall back to `{fallback}` (e.g. "clang++" or "g++")
    fn resolve_cxx_compiler(target: &str, fallback: &str) -> String {
        // 1. Explicit CXX override
        if let Ok(cxx) = env::var("CXX") {
            return cxx;
        }

        // 2. Target-specific CXX (hyphens → underscores, matching cc-rs convention)
        let target_env = target.replace('-', "_");
        if let Ok(cxx) = env::var(format!("CXX_{target_env}")) {
            return cxx;
        }

        // 3. Default fallback
        fallback.to_string()
    }

    /// Create a g++ wrapper script for musl cross-compilation.
    ///
    /// When cross-compiling from a glibc host to a musl target, plain g++ picks up
    /// glibc C headers, producing objects with glibc-versioned symbols (e.g.
    /// `__isoc23_sscanf@@GLIBC_2.38`) incompatible with musl linking.
    ///
    /// The wrapper uses `-nostdinc` to remove ALL default system include paths, then
    /// adds them back in the correct order:
    ///   1. GCC internal headers (stddef.h, stdarg.h, etc.)
    ///   2. GCC fixed-includes (system header fixups)
    ///   3. C++ standard library headers (libstdc++)
    ///   4. C++ platform-specific headers (bits/c++config.h etc.)
    ///   5. musl C headers (replaces glibc C headers)
    ///
    /// This ordering is critical because libstdc++ headers use `#include_next`
    /// to include C headers. With `-isystem` alone (no `-nostdinc`), the musl
    /// headers come before libstdc++ in the search path, so `#include_next` from
    /// `<cstdlib>` skips them and finds glibc's headers instead. By using
    /// `-nostdinc` and placing musl headers AFTER the C++ headers, `#include_next`
    /// correctly finds musl's C headers.
    ///
    /// Additionally defines glibc compatibility macros as no-ops so that
    /// platform-specific C++ headers (os_defines.h, floatn.h) that reference
    /// `__GLIBC_PREREQ()` etc. compile correctly with musl.
    #[cfg(unix)]
    fn create_musl_cxx_wrapper(target: &str) -> Option<String> {
        use std::os::unix::fs::PermissionsExt;

        let host = env::var("HOST").unwrap_or_default();

        // Only needed for cross-compilation from glibc host to musl target
        if !target.contains("musl") || host.contains("musl") {
            return None;
        }

        // Detect musl include directory: /usr/include/{arch}-linux-musl
        let arch = target.split('-').next().unwrap_or("x86_64");
        let musl_include = format!("/usr/include/{arch}-linux-musl");
        if !Path::new(&musl_include).exists() {
            println!("cargo:warning=musl include dir not found at {musl_include}, skipping wrapper");
            return None;
        }

        // Detect host GCC triplet and version for include path construction
        let host_arch = host.split('-').next().unwrap_or("x86_64");
        let host_triplet = format!("{host_arch}-linux-gnu");

        // Write wrapper script to OUT_DIR
        let out_dir = env::var("OUT_DIR").unwrap();
        let wrapper_path = format!("{out_dir}/musl-g++.sh");
        let wrapper_content = format!(
            r#"#!/bin/sh
# Auto-generated musl-g++ wrapper for cross-compilation.
# Uses -nostdinc to remove default system includes, then adds back
# GCC internal, C++ stdlib, and musl C headers in the correct order
# so that #include_next from libstdc++ finds musl headers (not glibc).

GCC_VER=$(g++ -dumpversion | cut -d. -f1)
TRIPLET="{host_triplet}"
GCC_LIB="/usr/lib/gcc/$TRIPLET/$GCC_VER"

exec g++ -nostdinc \
  -isystem "$GCC_LIB/include" \
  -isystem "$GCC_LIB/include-fixed" \
  -isystem "/usr/include/c++/$GCC_VER" \
  -isystem "/usr/include/$TRIPLET/c++/$GCC_VER" \
  -isystem "{musl_include}" \
  '-D__GLIBC_PREREQ(maj,min)=0' \
  '-D__GLIBC_USE(F)=0' \
  '-D__GNUC_PREREQ(maj,min)=((__GNUC__<<16)+__GNUC_MINOR__>=((maj)<<16)+(min))' \
  '-D__locale_t=locale_t' \
  '-include' '{musl_include}/locale.h' \
  "$@"
"#
        );

        fs::write(&wrapper_path, &wrapper_content).ok()?;
        fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755)).ok()?;

        println!(
            "cargo:warning=Created musl g++ wrapper at {wrapper_path} (musl headers: {musl_include}, host triplet: {host_triplet})"
        );
        Some(wrapper_path)
    }

    #[cfg(not(unix))]
    fn create_musl_cxx_wrapper(_target: &str) -> Option<String> {
        None
    }

    fn prepare_out_dir() -> PathBuf {
        let preferred = get_preferred_out_dir();
        match fs::create_dir_all(&preferred) {
            Ok(_) => preferred,
            Err(err) => {
                println!(
                    "cargo:warning=Failed to create cache dir {:?}: {}. Falling back to temp dir.",
                    preferred, err
                );
                let fallback = env::temp_dir().join("kreuzberg-tesseract-cache");
                fs::create_dir_all(&fallback).expect("Failed to create fallback cache directory in temp dir");
                fallback
            }
        }
    }

    #[allow(dead_code)]
    fn find_emsdk() -> Result<PathBuf, String> {
        if let Ok(emsdk_path) = env::var("EMSDK") {
            let path = PathBuf::from(emsdk_path);
            if path.exists() {
                return Ok(path);
            }
        }

        let common_paths = vec![
            PathBuf::from("/opt/homebrew/opt/emscripten"),
            PathBuf::from("/usr/local/opt/emscripten"),
            PathBuf::from(env::var("HOME").unwrap_or_default()).join(".emsdk"),
            PathBuf::from("/usr/lib/emscripten"),
            PathBuf::from("/opt/emsdk"),
        ];

        for path in common_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        Err("EMSDK not found. Please install via: brew install emscripten (macOS) or apt-get install emscripten (Linux)".to_string())
    }

    pub fn build() {
        let target = target_triple();

        if is_wasm_target(&target) {
            println!(
                "cargo:warning=Detected WASM target: {}, routing to build_wasm()",
                target
            );
            return build_wasm();
        }

        let custom_out_dir = prepare_out_dir();
        let windows_target = is_windows_target(&target);
        let msvc_target = is_msvc_target(&target);
        let mingw_target = is_mingw_target(&target);

        println!("cargo:warning=custom_out_dir: {:?}", custom_out_dir);

        let cache_dir = custom_out_dir.join("cache");

        if env::var("CARGO_CLEAN").is_ok() {
            clean_cache(&cache_dir);
        }

        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let out_dir = custom_out_dir.clone();
        let project_dir = custom_out_dir.clone();
        let third_party_dir = project_dir.join("third_party");

        let leptonica_dir = if third_party_dir.join("leptonica").exists() {
            println!("cargo:warning=Using existing leptonica source");
            third_party_dir.join("leptonica")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &leptonica_url(), "leptonica")
        };

        let tesseract_dir = if third_party_dir.join("tesseract").exists() {
            println!("cargo:warning=Using existing tesseract source");
            third_party_dir.join("tesseract")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &tesseract_url(), "tesseract")
        };

        let (cmake_cxx_flags, additional_defines) = get_os_specific_config();

        let leptonica_install_dir = out_dir.join("leptonica");
        let leptonica_cache_dir = cache_dir.join("leptonica");

        let leptonica_link_name = build_or_use_cached(
            "leptonica",
            &leptonica_cache_dir,
            &leptonica_install_dir,
            || {
                let mut leptonica_config = Config::new(&leptonica_dir);

                let leptonica_src_dir = leptonica_dir.join("src");
                let environ_h_path = leptonica_src_dir.join("environ.h");

                if environ_h_path.exists() {
                    let environ_h = std::fs::read_to_string(&environ_h_path)
                        .expect("Failed to read environ.h")
                        .replace("#define  HAVE_LIBZ          1", "#define  HAVE_LIBZ          0")
                        .replace("#ifdef  NO_CONSOLE_IO", "#define NO_CONSOLE_IO\n#ifdef  NO_CONSOLE_IO");
                    std::fs::write(environ_h_path, environ_h).expect("Failed to write environ.h");
                }

                let makefile_static_path = leptonica_dir.join("prog").join("makefile.static");

                let leptonica_src_cmakelists = leptonica_dir.join("src").join("CMakeLists.txt");

                if leptonica_src_cmakelists.exists() {
                    let cmakelists = std::fs::read_to_string(&leptonica_src_cmakelists)
                        .expect("Failed to read leptonica src CMakeLists.txt");
                    let patched = cmakelists.replace(
                        "if(MINGW)\n  set_target_properties(\n    leptonica PROPERTIES SUFFIX\n                         \"-${PROJECT_VERSION}${CMAKE_SHARED_LIBRARY_SUFFIX}\")\nendif(MINGW)\n",
                        "if(MINGW AND BUILD_SHARED_LIBS)\n  set_target_properties(\n    leptonica PROPERTIES SUFFIX\n                         \"-${PROJECT_VERSION}${CMAKE_SHARED_LIBRARY_SUFFIX}\")\nendif()\n",
                    );
                    if patched != cmakelists {
                        std::fs::write(&leptonica_src_cmakelists, patched)
                            .expect("Failed to patch leptonica src CMakeLists.txt");
                    }
                }

                if makefile_static_path.exists() {
                    let makefile_static = std::fs::read_to_string(&makefile_static_path)
                        .expect("Failed to read makefile.static")
                        .replace(
                            "ALL_LIBS =	$(LEPTLIB) -ltiff -ljpeg -lpng -lz -lm",
                            "ALL_LIBS =	$(LEPTLIB) -lm",
                        );
                    std::fs::write(makefile_static_path, makefile_static).expect("Failed to write makefile.static");
                }

                if windows_target {
                    if mingw_target {
                        leptonica_config.generator("Unix Makefiles");
                        leptonica_config.define("CMAKE_MAKE_PROGRAM", "mingw32-make");
                        leptonica_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd;/D;-D;-I;-L");
                    } else if msvc_target && env::var("VSINSTALLDIR").is_ok() {
                        leptonica_config.generator("NMake Makefiles");
                    }
                    leptonica_config.define("CMAKE_CL_SHOWINCLUDES_PREFIX", "");
                }

                if env::var("CI").is_err() && env::var("RUSTC_WRAPPER").unwrap_or_default() == "sccache" {
                    leptonica_config.env("CC", "sccache cc").env("CXX", "sccache c++");
                }

                let leptonica_install_dir_cmake = normalize_cmake_path(&leptonica_install_dir);

                leptonica_config
                    .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
                    .define("CMAKE_BUILD_TYPE", "Release")
                    .define("BUILD_PROG", "OFF")
                    .define("BUILD_SHARED_LIBS", "OFF")
                    .define("ENABLE_ZLIB", "OFF")
                    .define("ENABLE_PNG", "OFF")
                    .define("ENABLE_JPEG", "OFF")
                    .define("ENABLE_TIFF", "OFF")
                    .define("ENABLE_WEBP", "OFF")
                    .define("ENABLE_OPENJPEG", "OFF")
                    .define("ENABLE_GIF", "OFF")
                    .define("NO_CONSOLE_IO", "ON")
                    .define("CMAKE_CXX_FLAGS", &cmake_cxx_flags)
                    .define("MINIMUM_SEVERITY", "L_SEVERITY_NONE")
                    .define("SW_BUILD", "OFF")
                    .define("HAVE_LIBZ", "0")
                    .define("ENABLE_LTO", "OFF")
                    .define("CMAKE_INSTALL_PREFIX", &leptonica_install_dir_cmake);

                if windows_target {
                    if msvc_target {
                        leptonica_config
                            .define("CMAKE_C_FLAGS_RELEASE", "/MD /O2")
                            .define("CMAKE_C_FLAGS_DEBUG", "/MDd /Od");
                    } else if mingw_target {
                        leptonica_config
                            .define("CMAKE_C_FLAGS_RELEASE", "-O2 -DNDEBUG")
                            .define("CMAKE_C_FLAGS_DEBUG", "-O0 -g");
                    } else {
                        leptonica_config
                            .define("CMAKE_C_FLAGS_RELEASE", "-O2")
                            .define("CMAKE_C_FLAGS_DEBUG", "-O0 -g");
                    }
                }

                for (key, value) in &additional_defines {
                    leptonica_config.define(key, value);
                }

                leptonica_config.build();
            },
        );

        let leptonica_include_dir = leptonica_install_dir.join("include");
        let leptonica_lib_dir = leptonica_install_dir.join("lib");
        let tesseract_install_dir = out_dir.join("tesseract");
        let tesseract_cache_dir = cache_dir.join("tesseract");
        let tessdata_prefix = project_dir.clone();

        let leptonica_install_dir_cmake = normalize_cmake_path(&leptonica_install_dir);
        let leptonica_include_dir_cmake = normalize_cmake_path(&leptonica_include_dir);
        let leptonica_lib_dir_cmake = normalize_cmake_path(&leptonica_lib_dir);
        let tesseract_install_dir_cmake = normalize_cmake_path(&tesseract_install_dir);
        let tessdata_prefix_cmake = normalize_cmake_path(&tessdata_prefix);

        let tesseract_link_name =
            build_or_use_cached("tesseract", &tesseract_cache_dir, &tesseract_install_dir, || {
                let cmakelists_path = tesseract_dir.join("CMakeLists.txt");
                let cmakelists = std::fs::read_to_string(&cmakelists_path)
                    .expect("Failed to read CMakeLists.txt")
                    .replace("set(HAVE_TIFFIO_H ON)", "");
                std::fs::write(&cmakelists_path, cmakelists).expect("Failed to write CMakeLists.txt");

                let mut tesseract_config = Config::new(&tesseract_dir);
                if windows_target {
                    if mingw_target {
                        tesseract_config.generator("Unix Makefiles");
                        tesseract_config.define("CMAKE_MAKE_PROGRAM", "mingw32-make");
                        tesseract_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd;/D;-D;-I;-L");
                    } else if msvc_target && env::var("VSINSTALLDIR").is_ok() {
                        tesseract_config.generator("NMake Makefiles");
                    }
                    tesseract_config.define("CMAKE_CL_SHOWINCLUDES_PREFIX", "");
                }

                if env::var("CI").is_err() && env::var("RUSTC_WRAPPER").unwrap_or_default() == "sccache" {
                    tesseract_config.env("CC", "sccache cc").env("CXX", "sccache c++");
                }
                tesseract_config
                    .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
                    .define("CMAKE_BUILD_TYPE", "Release")
                    .define("BUILD_TRAINING_TOOLS", "OFF")
                    .define("BUILD_SHARED_LIBS", "OFF")
                    .define("DISABLE_ARCHIVE", "ON")
                    .define("DISABLE_CURL", "ON")
                    .define("DISABLE_OPENCL", "ON")
                    .define("Leptonica_DIR", &leptonica_install_dir_cmake)
                    .define("LEPTONICA_INCLUDE_DIR", &leptonica_include_dir_cmake)
                    .define("LEPTONICA_LIBRARY", &leptonica_lib_dir_cmake)
                    .define("CMAKE_PREFIX_PATH", &leptonica_install_dir_cmake)
                    .define("CMAKE_INSTALL_PREFIX", &tesseract_install_dir_cmake)
                    .define("TESSDATA_PREFIX", &tessdata_prefix_cmake)
                    .define("DISABLE_TIFF", "ON")
                    .define("DISABLE_PNG", "ON")
                    .define("DISABLE_JPEG", "ON")
                    .define("DISABLE_WEBP", "ON")
                    .define("DISABLE_OPENJPEG", "ON")
                    .define("DISABLE_ZLIB", "ON")
                    .define("DISABLE_LIBXML2", "ON")
                    .define("DISABLE_LIBICU", "ON")
                    .define("DISABLE_LZMA", "ON")
                    .define("DISABLE_GIF", "ON")
                    .define("DISABLE_DEBUG_MESSAGES", "ON")
                    .define("debug_file", "/dev/null")
                    .define("HAVE_LIBARCHIVE", "OFF")
                    .define("HAVE_LIBCURL", "OFF")
                    .define("HAVE_TIFFIO_H", "OFF")
                    .define("GRAPHICS_DISABLED", "ON")
                    .define("DISABLED_LEGACY_ENGINE", "OFF")
                    .define("USE_OPENCL", "OFF")
                    .define("OPENMP_BUILD", "OFF")
                    .define("BUILD_TESTS", "OFF")
                    .define("ENABLE_LTO", "OFF")
                    .define("BUILD_PROG", "OFF")
                    .define("BUILD_TESSERACT_BINARY", "OFF")
                    .define("SW_BUILD", "OFF")
                    .define("LEPT_TIFF_RESULT", "FALSE")
                    .define("INSTALL_CONFIGS", "ON")
                    .define("USE_SYSTEM_ICU", "ON")
                    .define("CMAKE_CXX_FLAGS", &cmake_cxx_flags);

                for (key, value) in &additional_defines {
                    tesseract_config.define(key, value);
                }

                tesseract_config.build();
            });

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", third_party_dir.display());
        println!("cargo:rerun-if-changed={}", leptonica_dir.display());
        println!("cargo:rerun-if-changed={}", tesseract_dir.display());

        println!("cargo:rustc-link-search=native={}", leptonica_lib_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            tesseract_install_dir.join("lib").display()
        );

        // Link libraries in the correct order for static linking:
        // 1. tesseract first (depends on leptonica and C++ stdlib)
        // 2. leptonica (depends on C++ stdlib)
        // 3. C++ standard library and system libraries (via set_os_specific_link_flags)
        //
        // IMPORTANT: For static linking, the linker resolves symbols in order.
        // Libraries must be listed BEFORE the libraries they depend on.
        // The C++ stdlib must come LAST because both tesseract and leptonica
        // depend on it for symbols like operator new, operator delete, etc.
        #[cfg(feature = "dynamic-linking")]
        let link_type = "dylib";
        #[cfg(not(feature = "dynamic-linking"))]
        let link_type = "static";

        println!("cargo:rustc-link-lib={}={}", link_type, tesseract_link_name);
        println!(
            "cargo:warning=Linking with tesseract ({} linking): {}",
            link_type, tesseract_link_name
        );
        println!("cargo:rustc-link-lib={}={}", link_type, leptonica_link_name);
        println!(
            "cargo:warning=Linking with leptonica ({} linking): {}",
            link_type, leptonica_link_name
        );

        // Link C++ standard library and system libraries AFTER tesseract and leptonica.
        // This is critical for static linking on Linux (especially aarch64) where
        // tesseract's C++ code needs symbols like operator new/delete from libstdc++.
        set_os_specific_link_flags();

        println!("cargo:warning=Leptonica include dir: {:?}", leptonica_include_dir);
        println!("cargo:warning=Leptonica lib dir: {:?}", leptonica_lib_dir);
        println!("cargo:warning=Tesseract install dir: {:?}", tesseract_install_dir);
        println!("cargo:warning=Tessdata dir: {:?}", tessdata_prefix);
    }

    fn get_os_specific_config() -> (String, Vec<(String, String)>) {
        let mut cmake_cxx_flags = String::new();
        let mut additional_defines = Vec::new();
        let target = target_triple();
        let target_macos = is_macos_target(&target);
        let target_linux = is_linux_target(&target);
        let target_windows = is_windows_target(&target);
        let target_msvc = is_msvc_target(&target);
        let target_mingw = is_mingw_target(&target);
        let target_musl = target.contains("musl");

        if target_macos {
            cmake_cxx_flags.push_str("-stdlib=libc++ ");
            cmake_cxx_flags.push_str("-std=c++17 ");
        } else if target_linux {
            cmake_cxx_flags.push_str("-std=c++17 ");
            if target_musl {
                // For musl: use g++ with musl-gcc specs (avoids libc++/musl locale
                // incompatibilities). The wrapper redirects C headers to musl while
                // keeping libstdc++ intact.
                let cxx_compiler =
                    create_musl_cxx_wrapper(&target).unwrap_or_else(|| resolve_cxx_compiler(&target, "g++"));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            } else if env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                cmake_cxx_flags.push_str("-stdlib=libc++ ");
                let cxx_compiler = resolve_cxx_compiler(&target, "clang++");
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            } else {
                let cxx_compiler = resolve_cxx_compiler(&target, "g++");
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            }
        } else if target_windows {
            if target_msvc {
                cmake_cxx_flags.push_str("/EHsc /MP /std:c++17 /DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "/MD /O2".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "/MDd /Od".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "/MD /O2 /DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "/MDd /Od /DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push(("CMAKE_MSVC_RUNTIME_LIBRARY".to_string(), "MultiThreadedDLL".to_string()));
            } else if target_mingw {
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "-O2 -DNDEBUG".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "-O0 -g".to_string()));
                additional_defines.push(("CMAKE_C_COMPILER".to_string(), "gcc".to_string()));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), "g++".to_string()));
                additional_defines.push(("CMAKE_SYSTEM_NAME".to_string(), "Windows".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "-O2 -DNDEBUG -DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "-O0 -g -DTESSERACT_STATIC".to_string(),
                ));
            } else {
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "-O2 -DNDEBUG".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "-O0 -g".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "-O2 -DNDEBUG -DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "-O0 -g -DTESSERACT_STATIC".to_string(),
                ));
            }
        }

        cmake_cxx_flags.push_str("-DUSE_STD_NAMESPACE ");
        additional_defines.push(("CMAKE_POSITION_INDEPENDENT_CODE".to_string(), "ON".to_string()));

        if target_windows && target_msvc {
            cmake_cxx_flags.push_str("/permissive- ");
            additional_defines.push(("CMAKE_EXE_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
            additional_defines.push(("CMAKE_SHARED_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
            additional_defines.push(("CMAKE_MODULE_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
        }

        (cmake_cxx_flags, additional_defines)
    }

    fn set_os_specific_link_flags() {
        let target = target_triple();
        let target_macos = is_macos_target(&target);
        let target_linux = is_linux_target(&target);
        let target_windows = is_windows_target(&target);
        let target_mingw = is_mingw_target(&target);
        let target_musl = target.contains("musl");

        if target_macos {
            println!("cargo:rustc-link-lib=c++");
        } else if target_linux {
            if target_musl {
                // musl builds use g++ with libstdc++
                println!("cargo:rustc-link-lib=stdc++");
            } else if env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                println!("cargo:rustc-link-lib=c++");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
                println!("cargo:rustc-link-lib=stdc++fs");
            }
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=m");
            if !target_musl {
                println!("cargo:rustc-link-lib=dl");
            }
        } else if target_windows {
            if target_mingw {
                println!("cargo:rustc-link-lib=stdc++");
            }
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=ws2_32");
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=shell32");
        }

        println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
    }

    fn download_and_extract(target_dir: &Path, url: &str, name: &str) -> PathBuf {
        use zip::ZipArchive;

        fs::create_dir_all(target_dir).expect("Failed to create target directory");

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .http1_only()
            .build()
            .expect("Failed to create HTTP client");

        println!("cargo:warning=Downloading {} from {}", name, url);
        let max_attempts = 5;
        let mut content = None;

        for attempt in 1..=max_attempts {
            let err_msg = match client.get(url).send() {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes() {
                            Ok(bytes) => {
                                content = Some(bytes.to_vec());
                                break;
                            }
                            Err(err) => format!("Failed to read response: {}", err),
                        }
                    } else {
                        format!("HTTP {}", resp.status().as_u16())
                    }
                }
                Err(err) => err.to_string(),
            };

            if attempt == max_attempts {
                panic!(
                    "Failed to download {} after {} attempts: {}",
                    name, max_attempts, err_msg
                );
            }

            let backoff = 2u64.pow((attempt - 1).min(4));
            println!(
                "cargo:warning=Download attempt {}/{} for {} failed ({}). Retrying in {}s...",
                attempt, max_attempts, name, err_msg, backoff
            );
            std::thread::sleep(std::time::Duration::from_secs(backoff));
        }

        let content = content.expect("unreachable: download loop must either succeed or panic");

        println!("cargo:warning=Downloaded {} bytes for {}", content.len(), name);

        let temp_file = target_dir.join(format!("{}.zip", name));
        fs::write(&temp_file, content).expect("Failed to write archive to file");

        let extract_dir = target_dir.join(name);
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).expect("Failed to remove existing directory");
        }
        fs::create_dir_all(&extract_dir).expect("Failed to create extraction directory");

        let mut archive = ZipArchive::new(fs::File::open(&temp_file).unwrap()).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let file_path = file.mangled_name();
            let file_path = file_path.to_str().unwrap();

            let path = Path::new(file_path);
            let path = path.strip_prefix(path.components().next().unwrap()).unwrap();

            if path.as_os_str().is_empty() {
                continue;
            }

            let target_path = extract_dir.join(path);

            if file.is_dir() {
                fs::create_dir_all(target_path).unwrap();
            } else {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                let mut outfile = fs::File::create(target_path).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }

        fs::remove_file(temp_file).expect("Failed to remove temporary zip file");

        extract_dir
    }

    fn normalize_cmake_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn clean_cache(cache_dir: &Path) {
        println!("Cleaning cache directory: {:?}", cache_dir);
        if cache_dir.exists() {
            fs::remove_dir_all(cache_dir).expect("Failed to remove cache directory");
        }
    }

    #[allow(dead_code)]
    fn setup_emsdk_environment(emsdk_root: &Path) {
        let emsdk_root_str = emsdk_root.to_string_lossy().to_string();

        unsafe {
            env::set_var("EMSDK", &emsdk_root_str);
        }
        println!("cargo:warning=Set EMSDK={}", emsdk_root_str);

        let emscripten_path = format!("{}/upstream/emscripten", emsdk_root_str);
        unsafe {
            env::set_var("EMSCRIPTEN", &emscripten_path);
        }
        println!("cargo:warning=Set EMSCRIPTEN={}", emscripten_path);

        let em_config = format!("{}/.emscripten", emsdk_root_str);
        unsafe {
            env::set_var("EM_CONFIG", &em_config);
        }
        println!("cargo:warning=Set EM_CONFIG={}", em_config);

        let emsdk_bin_paths = [
            format!("{}/upstream/emscripten", emsdk_root_str),
            format!("{}/upstream/bin", emsdk_root_str),
        ];

        let current_path = env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{}{}{}",
            emsdk_bin_paths.join(":"),
            if current_path.is_empty() { "" } else { ":" },
            current_path
        );
        unsafe {
            env::set_var("PATH", &new_path);
        }
        println!("cargo:warning=Updated PATH with EMSDK tools");
    }

    #[allow(dead_code)]
    fn build_leptonica_wasm(leptonica_src: &Path, leptonica_install: &Path, emsdk_dir: &Path) {
        let toolchain_file = emsdk_dir.join("upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake");

        let mut config = Config::new(leptonica_src);

        config.define("CMAKE_TOOLCHAIN_FILE", &toolchain_file);

        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("LIBWEBP_SUPPORT", "OFF")
            .define("OPENJPEG_SUPPORT", "OFF")
            .define("ENABLE_ZLIB", "OFF")
            .define("ENABLE_PNG", "OFF")
            .define("ENABLE_JPEG", "OFF")
            .define("ENABLE_TIFF", "OFF")
            .define("ENABLE_WEBP", "OFF")
            .define("ENABLE_OPENJPEG", "OFF")
            .define("ENABLE_GIF", "OFF")
            .define("BUILD_PROG", "OFF")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("NO_CONSOLE_IO", "ON")
            .define("HAVE_LIBZ", "0")
            .define("ENABLE_LTO", "OFF")
            .define("CMAKE_INSTALL_PREFIX", leptonica_install);

        config.build();
    }

    #[allow(dead_code)]
    fn build_wasm() {
        println!("cargo:warning=Building for WASM target with Emscripten SDK");

        let custom_out_dir = prepare_out_dir();
        let cache_dir = custom_out_dir.join("cache");
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let project_dir = custom_out_dir.clone();
        let third_party_dir = project_dir.join("third_party");

        println!("cargo:warning=Looking for pre-installed Emscripten SDK...");
        let emsdk_dir = match find_emsdk() {
            Ok(path) => {
                println!("cargo:warning=Found EMSDK at: {}", path.display());
                path
            }
            Err(err) => {
                panic!("{}

Installation instructions:
  macOS:    brew install emscripten
  Ubuntu:   sudo apt-get install emscripten
  Arch:     sudo pacman -S emscripten
  Manual:   git clone https://github.com/emscripten-core/emsdk.git ~/.emsdk && cd ~/.emsdk && ./emsdk install latest && ./emsdk activate latest

After installation, set EMSDK environment variable or ensure it's in a standard location.", err);
            }
        };

        setup_emsdk_environment(&emsdk_dir);

        let leptonica_dir = if third_party_dir.join("leptonica").exists() {
            println!("cargo:warning=Using existing leptonica source");
            third_party_dir.join("leptonica")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &leptonica_url(), "leptonica")
        };

        let tesseract_dir = if third_party_dir.join("tesseract").exists() {
            println!("cargo:warning=Using existing tesseract source");
            third_party_dir.join("tesseract")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &tesseract_url(), "tesseract")
        };

        let leptonica_install_dir = custom_out_dir.join("leptonica");
        let leptonica_cache_dir = cache_dir.join("leptonica");

        let _leptonica_link_name =
            build_or_use_cached("leptonica", &leptonica_cache_dir, &leptonica_install_dir, || {
                println!("cargo:warning=Building Leptonica for WASM...");
                build_leptonica_wasm(&leptonica_dir, &leptonica_install_dir, &emsdk_dir);
            });

        let tesseract_install_dir = custom_out_dir.join("tesseract");
        let tesseract_cache_dir = cache_dir.join("tesseract");

        let _tesseract_link_name =
            build_or_use_cached("tesseract", &tesseract_cache_dir, &tesseract_install_dir, || {
                println!("cargo:warning=Building Tesseract for WASM (SIMD enabled)...");
                build_tesseract_wasm(
                    &tesseract_dir,
                    &tesseract_install_dir,
                    &leptonica_install_dir,
                    &emsdk_dir,
                    true,
                );
            });

        let leptonica_lib_dir = leptonica_install_dir.join("lib");
        let tesseract_lib_dir = tesseract_install_dir.join("lib");

        println!("cargo:rustc-link-search=native={}", leptonica_lib_dir.display());
        println!("cargo:rustc-link-search=native={}", tesseract_lib_dir.display());

        println!("cargo:rustc-link-lib=static=tesseract");
        println!("cargo:rustc-link-lib=static=leptonica");

        println!("cargo:warning=WASM build completed successfully!");
        println!("cargo:warning=Leptonica install dir: {:?}", leptonica_install_dir);
        println!("cargo:warning=Tesseract install dir: {:?}", tesseract_install_dir);
    }

    #[allow(dead_code)]
    fn build_tesseract_wasm(
        src_dir: &Path,
        tesseract_install: &Path,
        leptonica_install: &Path,
        emsdk_dir: &Path,
        enable_simd: bool,
    ) {
        let toolchain_file = emsdk_dir.join("upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake");

        let mut config = Config::new(src_dir);

        config.define("CMAKE_TOOLCHAIN_FILE", &toolchain_file);

        config.define("Leptonica_DIR", leptonica_install);
        config.define("CMAKE_PREFIX_PATH", leptonica_install);

        let mut cxx_flags = String::from("-DTESSERACT_IMAGEDATA_AS_PIX ");
        if enable_simd {
            cxx_flags.push_str("-msimd128 ");
        }
        cxx_flags.push_str("-fno-exceptions -fPIC -Os");

        let c_flags = "-fno-exceptions -fPIC -Os";

        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("BUILD_TESSERACT_BINARY", "OFF")
            .define("BUILD_TRAINING_TOOLS", "OFF")
            .define("INSTALL_CONFIGS", "ON")
            .define("BUILD_TESTS", "OFF")
            .define("BUILD_PROG", "OFF")
            .define("SYNTAX_LOG", "OFF")
            .define("DISABLE_ARCHIVE", "ON")
            .define("DISABLE_CURL", "ON")
            .define("DISABLE_OPENCL", "ON")
            .define("DISABLE_TIFF", "ON")
            .define("DISABLE_PNG", "ON")
            .define("DISABLE_JPEG", "ON")
            .define("DISABLE_WEBP", "ON")
            .define("DISABLE_OPENJPEG", "ON")
            .define("DISABLE_ZLIB", "ON")
            .define("DISABLE_LIBXML2", "ON")
            .define("DISABLE_LIBICU", "ON")
            .define("DISABLE_LZMA", "ON")
            .define("DISABLE_GIF", "ON")
            .define("DISABLE_DEBUG_MESSAGES", "ON")
            .define("GRAPHICS_DISABLED", "ON")
            .define("DISABLED_LEGACY_ENGINE", "ON")
            .define("USE_OPENCL", "OFF")
            .define("OPENMP_BUILD", "OFF")
            .define("ENABLE_LTO", "ON")
            .define("HAVE_SSE4_1", if enable_simd { "ON" } else { "OFF" })
            .define("HAVE_AVX", "OFF")
            .define("HAVE_AVX2", "OFF")
            .define("HAVE_AVX512F", "OFF")
            .define("HAVE_FMA", "OFF")
            .define("CMAKE_INSTALL_PREFIX", tesseract_install)
            .define("CMAKE_CXX_FLAGS", &cxx_flags)
            .define("CMAKE_C_FLAGS", c_flags);

        config.build();
    }

    fn build_or_use_cached<F>(name: &str, cache_dir: &Path, install_dir: &Path, build_fn: F) -> String
    where
        F: FnOnce(),
    {
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_triple = env::var("TARGET")
            .unwrap_or_else(|_| env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string()));
        let is_windows = target_triple.contains("windows");
        let is_windows_gnu = is_windows && target_env == "gnu";

        let lib_name = if is_windows && !is_windows_gnu {
            format!("{}.lib", name)
        } else {
            format!("lib{}.a", name)
        };

        let cached_path = cache_dir.join(&lib_name);
        let marker_path = cache_dir.join(format!("{}.target", name));
        let out_path = install_dir.join("lib").join(&lib_name);

        let possible_lib_names: Vec<String> = if is_windows {
            let mut base = match name {
                "leptonica" => vec![
                    "leptonica.lib".to_string(),
                    "libleptonica.lib".to_string(),
                    "leptonica-static.lib".to_string(),
                    format!("leptonica-{}.lib", LEPTONICA_VERSION),
                    "leptonica-1.86.0.lib".to_string(),
                    "leptonica-1.84.1.lib".to_string(),
                    "leptonicad.lib".to_string(),
                    "libleptonica_d.lib".to_string(),
                    format!("leptonica-{}d.lib", LEPTONICA_VERSION),
                    "leptonica-1.86.0d.lib".to_string(),
                    "leptonica-1.84.1d.lib".to_string(),
                ],
                "tesseract" => vec![
                    "tesseract.lib".to_string(),
                    "libtesseract.lib".to_string(),
                    "tesseract-static.lib".to_string(),
                    "tesseract53.lib".to_string(),
                    "tesseract54.lib".to_string(),
                    "tesseract55.lib".to_string(),
                    "tesseractd.lib".to_string(),
                    "libtesseract_d.lib".to_string(),
                    "tesseract53d.lib".to_string(),
                    "tesseract54d.lib".to_string(),
                    "tesseract55d.lib".to_string(),
                ],
                _ => vec![format!("{}.lib", name)],
            };

            if is_windows_gnu {
                match name {
                    "leptonica" => {
                        base.push(format!("libleptonica-{}.a", LEPTONICA_VERSION));
                        base.push("libleptonica.a".to_string());
                    }
                    "tesseract" => {
                        base.push(format!("libtesseract{}.a", TESSERACT_VERSION.replace('.', "")));
                        base.push("libtesseract.a".to_string());
                        base.push("libtesseract55.a".to_string());
                    }
                    _ => {
                        base.push(format!("lib{}.a", name));
                    }
                }
            }

            base
        } else {
            vec![format!("lib{}.a", name)]
        };

        fs::create_dir_all(cache_dir).expect("Failed to create cache directory");
        fs::create_dir_all(out_path.parent().unwrap()).expect("Failed to create output directory");

        let candidate_lib_dirs = [
            install_dir.join("lib"),
            install_dir.join("lib64"),
            install_dir.join("lib").join("tesseract"),
        ];

        let cache_valid = cached_path.exists()
            && {
                match fs::read_to_string(&marker_path) {
                    Ok(cached_target) => {
                        let valid = cached_target.trim() == target_triple;
                        if !valid {
                            println!(
                                "cargo:warning=Cached {} library is for wrong architecture (cached: {}, current: {}), rebuilding",
                                name,
                                cached_target.trim(),
                                target_triple
                            );
                            let _ = fs::remove_file(&cached_path);
                            let _ = fs::remove_file(&marker_path);
                        }
                        valid
                    }
                    Err(_) => {
                        println!(
                            "cargo:warning=Cached {} library missing target marker, rebuilding",
                            name
                        );
                        let _ = fs::remove_file(&cached_path);
                        false
                    }
                }
            };

        let link_name_to_use = if cache_valid {
            println!("cargo:warning=Using cached {} library for {}", name, target_triple);
            if let Err(e) = fs::copy(&cached_path, &out_path) {
                println!("cargo:warning=Failed to copy cached library: {}", e);
                build_fn();
            }
            name.to_string()
        } else {
            println!("Building {} library", name);
            build_fn();

            let mut found_lib_name = None;
            'search: for lib_name in &possible_lib_names {
                for dir in &candidate_lib_dirs {
                    let lib_path = dir.join(lib_name);
                    if lib_path.exists() {
                        println!("cargo:warning=Found {} library at: {}", name, lib_path.display());
                        let link_name = if lib_name.ends_with(".lib") {
                            lib_name.strip_suffix(".lib").unwrap_or(lib_name).to_string()
                        } else if lib_name.ends_with(".a") {
                            lib_name
                                .strip_prefix("lib")
                                .and_then(|s| s.strip_suffix(".a"))
                                .unwrap_or(lib_name)
                                .to_string()
                        } else {
                            lib_name.to_string()
                        };
                        found_lib_name = Some((lib_path, link_name));
                        break 'search;
                    }
                }
            }

            if let Some((lib_path, link_name)) = found_lib_name {
                if out_path.exists() {
                    println!(
                        "cargo:warning=Library already available at expected location: {}",
                        out_path.display()
                    );
                } else if let Err(e) = fs::copy(&lib_path, &out_path) {
                    println!("cargo:warning=Failed to copy library to standard location: {}", e);
                }
                if let Err(e) = fs::copy(&lib_path, &cached_path) {
                    println!("cargo:warning=Failed to cache library: {}", e);
                } else if let Err(e) = fs::write(&marker_path, &target_triple) {
                    println!("cargo:warning=Failed to write cache marker: {}", e);
                } else {
                    println!("cargo:warning=Cached {} library for {}", name, target_triple);
                }
                link_name
            } else {
                println!(
                    "cargo:warning=Library {} not found! Searched for: {:?}",
                    name, possible_lib_names
                );
                for dir in &candidate_lib_dirs {
                    println!("cargo:warning=Checked directory: {}", dir.display());
                    if let Ok(entries) = fs::read_dir(dir) {
                        println!("cargo:warning=Files in {}:", dir.display());
                        for entry in entries.flatten() {
                            println!("cargo:warning=  - {}", entry.file_name().to_string_lossy());
                        }
                    } else {
                        println!("cargo:warning=Directory not accessible: {}", dir.display());
                    }
                }
                name.to_string()
            }
        };

        for dir in candidate_lib_dirs.iter().filter(|d| d.exists()) {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }

        // Return the link name instead of outputting the link directive here
        // This allows the caller to control the linking order
        link_name_to_use
    }
}

fn main() {
    #[cfg(feature = "build-tesseract")]
    {
        build_tesseract::build();
    }

    #[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract")))]
    {
        println!("cargo:warning=Using dynamic linking with system-installed Tesseract libraries");
        println!("cargo:rustc-link-lib=dylib=tesseract");
        println!("cargo:rustc-link-lib=dylib=leptonica");
    }
}
