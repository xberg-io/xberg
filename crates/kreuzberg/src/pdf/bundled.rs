//! Runtime extraction of bundled PDFium library.
//!
//! When the `bundled-pdfium` feature is enabled, the PDFium library is embedded in the binary
//! using `include_bytes!` during compilation. This module handles runtime extraction to a
//! temporary directory and provides the path for dynamic loading.
//!
//! # Thread Safety
//!
//! Extraction is protected by a `Mutex` to prevent race conditions during concurrent access.
//! The first thread to call `extract_bundled_pdfium()` will perform the extraction while
//! others wait for completion.
//!
//! To prevent the "file too short" race condition where one thread loads a partially-written
//! file, we use atomic file operations: write to a temporary file, then atomically rename to
//! the final location. This ensures other threads never observe a partial file.
//!
//! # How It Works
//!
//! 1. During build (build.rs): PDFium is copied to OUT_DIR and the build script sets
//!    `KREUZBERG_PDFIUM_BUNDLED_PATH` environment variable
//! 2. At compile time: `include_bytes!` embeds the library binary in the executable
//! 3. At runtime: `extract_bundled_pdfium()` extracts to `$TMPDIR/kreuzberg-pdfium/`
//! 4. Library is reused if already present (based on file size validation)
//! 5. Concurrent calls are serialized with a `Mutex` to prevent partial writes
//! 6. Atomic rename (write temp file → rename) prevents "file too short" race conditions
//!
//! # Example
//!
//! ```rust,ignore
//! # #[cfg(feature = "bundled-pdfium")]
//! # {
//! use kreuzberg::pdf::bundled::extract_bundled_pdfium;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let lib_path = extract_bundled_pdfium()?;
//! println!("Extracted to: {}", lib_path.display());
//! # Ok(())
//! # }
//! # }
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static EXTRACTION_LOCK: Mutex<()> = Mutex::new(());

/// Runtime library name and extraction directory for the bundled PDFium library.
///
/// Returns tuple of (library_name, extraction_directory)
fn bundled_library_info() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("pdfium.dll", "kreuzberg-pdfium")
    } else if cfg!(target_os = "macos") {
        ("libpdfium.dylib", "kreuzberg-pdfium")
    } else {
        ("libpdfium.so", "kreuzberg-pdfium")
    }
}

/// Get the temporary directory for bundled PDFium extraction.
///
/// Uses `std::env::temp_dir()` on all platforms.
fn get_extraction_dir() -> io::Result<PathBuf> {
    let (_, subdir) = bundled_library_info();
    Ok(std::env::temp_dir().join(subdir))
}

/// Check if extracted library exists and is valid.
///
/// Verifies:
/// - File exists at expected path
/// - File size matches embedded size (basic validation)
///
/// Returns `true` if library can be safely reused, `false` if extraction is needed.
fn is_extracted_library_valid(lib_path: &Path, embedded_size: usize) -> bool {
    if !lib_path.exists() {
        return false;
    }

    match fs::metadata(lib_path) {
        Ok(metadata) => {
            let file_size = metadata.len() as usize;
            let size_tolerance = (embedded_size as f64 * 0.01) as usize;
            let min_size = embedded_size.saturating_sub(size_tolerance);
            let max_size = embedded_size.saturating_add(size_tolerance);
            file_size >= min_size && file_size <= max_size
        }
        Err(_) => false,
    }
}

/// Extract bundled PDFium library to temporary directory.
///
/// # Behavior
///
/// - Embeds PDFium library using `include_bytes!`
/// - Extracts to `$TMPDIR/kreuzberg-pdfium/` (non-WASM only)
/// - Reuses extracted library if size matches
/// - Sets permissions to 0755 on Unix
/// - Returns path to extracted library
/// - **Thread-safe**: Synchronized with a global `Mutex` to prevent concurrent writes
///
/// # Concurrency
///
/// This function is fully thread-safe. When multiple threads call it simultaneously,
/// only the first thread performs the actual extraction while others wait. This prevents
/// the "file too short" error that occurs when one thread reads a partially-written file.
///
/// # WASM Handling
///
/// On WASM targets (wasm32-*), this function returns an error with a helpful
/// message directing users to use WASM-specific initialization. WASM PDFium
/// is initialized through the runtime, not via file extraction.
///
/// # Errors
///
/// Returns `std::io::Error` if:
/// - Cannot create extraction directory
/// - Cannot write library file
/// - Cannot set file permissions (Unix only)
/// - Target is WASM (filesystem access not available)
///
/// # Platform-Specific Library Names
///
/// - Linux: `libpdfium.so`
/// - macOS: `libpdfium.dylib`
/// - Windows: `pdfium.dll`
pub fn extract_bundled_pdfium() -> io::Result<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "File extraction is not available in WASM. \
             PDFium for WASM must be initialized via the WebAssembly runtime. \
             Use a WASM-compatible environment with proper module initialization.",
        ));
    }

    let (lib_name, _) = bundled_library_info();
    let extract_dir = get_extraction_dir()?;

    fs::create_dir_all(&extract_dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to create bundled pdfium extraction directory '{}': {}",
                extract_dir.display(),
                e
            ),
        )
    })?;

    let lib_path = extract_dir.join(lib_name);

    let bundled_lib = include_bytes!(env!("KREUZBERG_PDFIUM_BUNDLED_PATH"));

    if is_extracted_library_valid(&lib_path, bundled_lib.len()) {
        return Ok(lib_path);
    }

    let _guard = EXTRACTION_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    if is_extracted_library_valid(&lib_path, bundled_lib.len()) {
        return Ok(lib_path);
    }

    let temp_path = lib_path.with_extension(format!("tmp.{}", std::process::id()));

    fs::write(&temp_path, bundled_lib).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to write bundled pdfium library to temp file '{}': {}",
                temp_path.display(),
                e
            ),
        )
    })?;

    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&temp_path, perms).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            io::Error::new(
                e.kind(),
                format!(
                    "Failed to set permissions on bundled pdfium temp file '{}': {}",
                    temp_path.display(),
                    e
                ),
            )
        })?;
    }

    fs::rename(&temp_path, &lib_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        io::Error::new(
            e.kind(),
            format!(
                "Failed to rename bundled pdfium library from '{}' to '{}': {}",
                temp_path.display(),
                lib_path.display(),
                e
            ),
        )
    })?;

    Ok(lib_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_library_info_windows() {
        if cfg!(target_os = "windows") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "pdfium.dll");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_bundled_library_info_macos() {
        if cfg!(target_os = "macos") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "libpdfium.dylib");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_bundled_library_info_linux() {
        if cfg!(target_os = "linux") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "libpdfium.so");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_get_extraction_dir() {
        let result = get_extraction_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.to_str().is_some());
        assert!(dir.ends_with("kreuzberg-pdfium"));
    }

    #[test]
    fn test_is_extracted_library_valid_missing() {
        let nonexistent = PathBuf::from("/tmp/nonexistent-pdfium-test");
        assert!(!is_extracted_library_valid(&nonexistent, 1000));
    }

    #[test]
    fn test_is_extracted_library_valid_size_match() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-size.dll");
        let test_size = 5_000_000;
        let test_data = vec![0u8; test_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, test_size);
            assert!(is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    fn test_is_extracted_library_valid_size_tolerance() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-tolerance.dll");
        let original_size = 10_000_000;
        let tolerance = (original_size as f64 * 0.01) as usize;

        let actual_size = original_size - tolerance / 2;
        let test_data = vec![0u8; actual_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, original_size);
            assert!(is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    fn test_is_extracted_library_valid_size_mismatch() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-mismatch.dll");
        let original_size = 10_000_000;

        let actual_size = (original_size as f64 * 0.85) as usize;
        let test_data = vec![0u8; actual_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, original_size);
            assert!(!is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    #[cfg(feature = "bundled-pdfium")]
    fn test_extract_bundled_pdfium() {
        let result = extract_bundled_pdfium();
        assert!(result.is_ok());

        let lib_path = result.unwrap();
        assert!(
            lib_path.exists(),
            "Extracted library should exist at: {}",
            lib_path.display()
        );
        assert!(lib_path.file_name().is_some(), "Library path should have filename");

        let (expected_name, _) = bundled_library_info();
        assert_eq!(lib_path.file_name().unwrap(), expected_name);
    }

    #[test]
    #[cfg(feature = "bundled-pdfium")]
    fn test_extract_bundled_pdfium_reuses_existing() {
        let result1 = extract_bundled_pdfium();
        assert!(result1.is_ok());
        let path1 = result1.unwrap();

        let metadata1 = fs::metadata(&path1).expect("Should be able to read metadata");
        let size1 = metadata1.len();

        let result2 = extract_bundled_pdfium();
        assert!(result2.is_ok());
        let path2 = result2.unwrap();

        assert_eq!(path1, path2, "Extraction should return same path on second call");

        let metadata2 = fs::metadata(&path2).expect("Should be able to read metadata");
        let size2 = metadata2.len();
        assert_eq!(size1, size2, "Reused library should have same file size");
    }

    #[test]
    #[cfg(feature = "bundled-pdfium")]
    fn test_extract_bundled_pdfium_concurrent_access() {
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let result = extract_bundled_pdfium();
                    assert!(result.is_ok(), "Concurrent extraction should succeed");
                    result.unwrap()
                })
            })
            .collect();

        let paths: Vec<PathBuf> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread should complete"))
            .collect();

        let first_path = &paths[0];
        assert!(
            paths.iter().all(|p| p == first_path),
            "All concurrent extractions should return the same path"
        );

        assert!(
            first_path.exists(),
            "Extracted library should exist at: {}",
            first_path.display()
        );

        let metadata = fs::metadata(first_path).expect("Should be able to read metadata");
        let file_size = metadata.len();
        assert!(
            file_size > 1_000_000,
            "PDFium library should be at least 1MB, got {} bytes",
            file_size
        );
    }

    #[test]
    #[cfg(unix)]
    #[cfg(feature = "bundled-pdfium")]
    fn test_extract_bundled_pdfium_permissions() {
        let result = extract_bundled_pdfium();
        assert!(result.is_ok());

        let lib_path = result.unwrap();
        let metadata = fs::metadata(&lib_path).expect("Should be able to read metadata");
        let perms = metadata.permissions();
        let mode = perms.mode();

        assert!(
            mode & 0o111 != 0,
            "Library should have executable bit set, got mode: {:#o}",
            mode
        );
    }
}
