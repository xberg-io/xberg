//! File I/O utilities.
//!
//! This module provides async and sync file reading utilities with proper error handling.
//! For large files (> 1 MiB) on non-WASM platforms, memory-mapped I/O is used to avoid
//! heap-allocating the entire file contents, reducing memory pressure and syscall overhead.

use crate::{Result, XbergError};
use std::path::Path;

/// Size threshold above which memory-mapped I/O is preferred over `read()`.
///
/// Files smaller than this are read with a regular `read()` call since the
/// mmap overhead (open, fstat, mmap syscalls + TLB pressure) outweighs the
/// benefit for small allocations.
#[cfg(not(target_arch = "wasm32"))]
const MMAP_THRESHOLD_BYTES: u64 = 1_048_576;

/// An owned buffer of file bytes.
///
/// On non-WASM platforms this may be backed by a memory-mapped file (zero heap
/// allocation for the file contents) or by a `Vec<u8>` for small files.
/// On WASM it is always a `Vec<u8>`.
///
/// Implements `Deref<Target = [u8]>` so callers can pass `&FileBytes` as `&[u8]`
/// without any additional copy.
#[cfg_attr(alef, alef(skip))]
pub struct FileBytes {
    inner: FileBytesInner,
}

enum FileBytesInner {
    /// Regular heap-allocated buffer (small files or WASM).
    Heap(Vec<u8>),
    /// Memory-mapped file (large files on native platforms).
    #[cfg(not(target_arch = "wasm32"))]
    Mapped(memmap2::Mmap),
}

impl std::ops::Deref for FileBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match &self.inner {
            FileBytesInner::Heap(v) => v.as_slice(),
            #[cfg(not(target_arch = "wasm32"))]
            FileBytesInner::Mapped(m) => m.as_ref(),
        }
    }
}

impl AsRef<[u8]> for FileBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

/// Open a file and return its bytes with zero-copy for large files.
///
/// On non-WASM targets, files larger than [`MMAP_THRESHOLD_BYTES`] are
/// memory-mapped so that the file contents are never copied to the heap.
/// The mapping is read-only; the file must not be modified while the returned
/// [`FileBytes`] is alive, which is safe for document extraction.
///
/// On WASM or for small files, falls back to a plain `std::fs::read`.
///
/// # Errors
///
/// Returns `XbergError::Io` for any I/O failure.
#[allow(unsafe_code)]
pub(crate) fn open_file_bytes(path: &Path) -> Result<FileBytes> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let metadata = std::fs::metadata(path).map_err(crate::XbergError::from)?;
        if metadata.len() > MMAP_THRESHOLD_BYTES {
            let file = std::fs::File::open(path).map_err(crate::XbergError::from)?;
            let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(crate::XbergError::from)?;
            return Ok(FileBytes {
                inner: FileBytesInner::Mapped(mmap),
            });
        }
    }
    let bytes = std::fs::read(path).map_err(crate::XbergError::from)?;
    Ok(FileBytes {
        inner: FileBytesInner::Heap(bytes),
    })
}

/// Read a file asynchronously.
///
/// On native targets with the async runtime this performs a non-blocking
/// `tokio::fs::read`. On wasm32 (where tokio refuses to build its `fs` feature)
/// or when the `tokio-runtime` feature is off, it falls back to the
/// platform-aware synchronous reader ([`open_file_bytes`]) — there is no async
/// runtime to block in those configurations, so the read stays correct.
///
/// This helper is always present regardless of features so that async extractor
/// paths can call one reader on every target rather than branching at each call
/// site.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// The file contents as bytes.
///
/// # Errors
///
/// Returns `XbergError::Io` for I/O errors (these always bubble up).
pub(crate) async fn read_file_async(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    #[cfg(all(feature = "tokio-runtime", not(target_arch = "wasm32")))]
    {
        tokio::fs::read(path.as_ref()).await.map_err(crate::XbergError::from)
    }
    // wasm32, or native without the async runtime: no runtime to block, so the sync reader
    // is the correct choice (and `tokio::fs` is unavailable on wasm32 regardless). ~keep
    #[cfg(not(all(feature = "tokio-runtime", not(target_arch = "wasm32"))))]
    {
        std::fs::read(path.as_ref()).map_err(crate::XbergError::from)
    }
}

/// Read a file synchronously.
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// The file contents as bytes.
///
/// # Errors
///
/// Returns `XbergError::Io` for I/O errors (these always bubble up).
#[cfg(test)]
pub(crate) fn read_file_sync(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    std::fs::read(path.as_ref()).map_err(crate::XbergError::from)
}

/// Check if a file exists.
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// `true` if the file exists, `false` otherwise.
pub(crate) fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Validate that a file exists.
///
/// # Arguments
///
/// * `path` - Path to validate
///
/// # Errors
///
/// Returns `XbergError::Io` if file doesn't exist.
pub(crate) fn validate_file_exists(path: impl AsRef<Path>) -> Result<()> {
    if !file_exists(&path) {
        return Err(XbergError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File does not exist: {}", path.as_ref().display()),
        )));
    }
    Ok(())
}

/// Traverse a directory and return all file paths matching a pattern.
///
/// # Arguments
///
/// * `dir` - Directory to traverse
/// * `recursive` - Whether to recursively traverse subdirectories
/// * `filter` - Optional filter function to match files
///
/// # Returns
///
/// Vector of file paths that match the criteria.
///
/// # Errors
///
/// Returns `XbergError::Io` for I/O errors.
#[cfg(test)]
pub(crate) fn traverse_directory<F>(
    dir: impl AsRef<Path>,
    recursive: bool,
    filter: Option<F>,
) -> Result<Vec<std::path::PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let dir = dir.as_ref();
    let mut files = Vec::new();

    if !dir.is_dir() {
        return Err(XbergError::from(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("Path is not a directory: {}", dir.display()),
        )));
    }

    traverse_directory_impl(dir, recursive, &filter, &mut files)?;
    Ok(files)
}

#[cfg(test)]
fn traverse_directory_impl<F>(
    dir: &Path,
    recursive: bool,
    filter: &Option<F>,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<()>
where
    F: Fn(&Path) -> bool,
{
    let entries = std::fs::read_dir(dir).map_err(crate::XbergError::from)?;

    for entry in entries {
        let entry = entry.map_err(crate::XbergError::from)?;
        let path = entry.path();

        if path.is_file() {
            let should_include = match filter {
                Some(f) => f(&path),
                None => true,
            };

            if should_include {
                files.push(path);
            }
        } else if path.is_dir() && recursive {
            traverse_directory_impl(&path, recursive, filter, files)?;
        }
    }

    Ok(())
}

/// Get all files in a directory with a specific extension.
///
/// # Arguments
///
/// * `dir` - Directory to search
/// * `extension` - File extension to match (without the dot)
/// * `recursive` - Whether to recursively search subdirectories
///
/// # Returns
///
/// Vector of file paths with the specified extension.
///
/// # Errors
///
/// Returns `XbergError::Io` for I/O errors.
#[cfg(test)]
pub(crate) fn find_files_by_extension(
    dir: impl AsRef<Path>,
    extension: &str,
    recursive: bool,
) -> Result<Vec<std::path::PathBuf>> {
    let ext = extension.to_lowercase();
    traverse_directory(
        dir,
        recursive,
        Some(|path: &Path| {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase() == ext)
                .unwrap_or(false)
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn test_read_file_async() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let content = read_file_async(&file_path).await.unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_read_file_sync() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();

        let content = read_file_sync(&file_path).unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_file_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        assert!(file_exists(&file_path));
        assert!(!file_exists(dir.path().join("nonexistent.txt")));
    }

    #[test]
    fn test_validate_file_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        assert!(validate_file_exists(&file_path).is_ok());
        assert!(validate_file_exists(dir.path().join("nonexistent.txt")).is_err());
    }

    #[test]
    fn test_traverse_directory_non_recursive() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.txt")).unwrap();

        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file4.txt")).unwrap();

        let files = traverse_directory(dir.path(), false, None::<fn(&Path) -> bool>).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_traverse_directory_recursive() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();

        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file3.txt")).unwrap();
        File::create(dir.path().join("subdir").join("file4.pdf")).unwrap();

        let files = traverse_directory(dir.path(), true, None::<fn(&Path) -> bool>).unwrap();
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_traverse_directory_with_filter() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.txt")).unwrap();

        let files = traverse_directory(
            dir.path(),
            false,
            Some(|path: &Path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "txt")
                    .unwrap_or(false)
            }),
        )
        .unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "txt"));
    }

    #[test]
    fn test_find_files_by_extension() {
        let dir = tempdir().unwrap();

        File::create(dir.path().join("file1.txt")).unwrap();
        File::create(dir.path().join("file2.pdf")).unwrap();
        File::create(dir.path().join("file3.TXT")).unwrap();

        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        File::create(dir.path().join("subdir").join("file4.txt")).unwrap();

        let files = find_files_by_extension(dir.path(), "txt", false).unwrap();
        assert_eq!(files.len(), 2);

        let files_recursive = find_files_by_extension(dir.path(), "txt", true).unwrap();
        assert_eq!(files_recursive.len(), 3);
    }

    #[test]
    fn test_traverse_directory_invalid_path() {
        let result = traverse_directory("/nonexistent/directory", false, None::<fn(&Path) -> bool>);
        assert!(result.is_err());
    }

    #[test]
    fn test_traverse_directory_file_not_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let result = traverse_directory(&file_path, false, None::<fn(&Path) -> bool>);
        assert!(result.is_err());
    }

    #[cfg(feature = "tokio-runtime")]
    #[tokio::test]
    async fn test_read_file_async_io_error() {
        let result = read_file_async("/nonexistent/file.txt").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XbergError::Io(_)));
    }

    #[test]
    fn test_read_file_sync_io_error() {
        let result = read_file_sync("/nonexistent/file.txt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XbergError::Io(_)));
    }
}
