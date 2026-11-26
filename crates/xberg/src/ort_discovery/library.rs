//! Where to find the ONNX Runtime library for `ort-dynamic` builds, decided without loading it.
//!
//! `xberg-libheif`'s `dynamic` module searches for libheif with the same algorithm: an explicit
//! override, the directory of the image that holds the code, `<prefix>/lib` for a `<prefix>/bin`
//! image, fixed system paths, each discovered file passing the same owner check.

/// How far the search for the ONNX Runtime library may reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SearchPolicy {
    /// Absolute, trusted candidates only, and `ORT_DYLIB_PATH` must be absolute: the release CLI
    /// (`ort-strict-discovery`), which ships the runtime beside the binary.
    Strict,
    /// After the absolute candidates, the platform loader's own search for the bare library name
    /// (`LD_LIBRARY_PATH`, `DYLD_LIBRARY_PATH`, `DYLD_FALLBACK_LIBRARY_PATH`, rpaths, `PATH`), as
    /// ort's loader did before xberg resolved the library: library consumers such as the
    /// language bindings, whose executable is an interpreter that knows nothing of xberg.
    LoaderFallback,
}

/// The running executable with symlinks resolved, so a symlinked install (a package manager's
/// `bin/` link) still finds the library shipped beside the real binary.
#[cfg(ort_dynamic_discovery)]
pub(super) fn running_executable() -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    // Windows canonical paths take the verbatim `\\?\` form; the loader needs no symlink pass. ~keep
    #[cfg(unix)]
    let executable = std::fs::canonicalize(&executable).unwrap_or(executable);
    Some(executable)
}

/// The file this code was loaded from, symlinks resolved: a language binding's shared library,
/// or the executable of a statically linked Rust program. `None` when the loader cannot say.
#[cfg(unix)]
#[allow(unsafe_code)]
pub(super) fn xberg_image() -> Option<std::path::PathBuf> {
    use std::os::unix::ffi::OsStrExt;

    let address = xberg_image as fn() -> Option<std::path::PathBuf>;
    let mut info = std::mem::MaybeUninit::<libc::Dl_info>::zeroed();
    // SAFETY: `address` is code in this image and `info` is writable storage for one `Dl_info`.
    if unsafe { libc::dladdr(address as *const libc::c_void, info.as_mut_ptr()) } == 0 {
        return None;
    }
    // SAFETY: `dladdr` succeeded, so it filled `info`.
    let info = unsafe { info.assume_init() };
    if info.dli_fname.is_null() {
        return None;
    }
    // SAFETY: A non-null `dli_fname` is a NUL-terminated string owned by the loader.
    let name = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(name.to_bytes()));
    // glibc names the main program by `argv[0]`, which may be relative to a directory left since. ~keep
    if !path.is_absolute() {
        return None;
    }
    Some(std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

#[cfg(all(ort_dynamic_discovery, not(unix)))]
pub(super) fn xberg_image() -> Option<std::path::PathBuf> {
    None
}

/// Where [`resolve_ort_library`] says the runtime comes from.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum OrtSource {
    /// `ORT_DYLIB_PATH`, loaded as given.
    Explicit(std::path::PathBuf),
    /// A relative `ORT_DYLIB_PATH` under [`SearchPolicy::Strict`].
    RelativeOverride(std::path::PathBuf),
    Search(OrtSearch),
}

/// The discovered candidates, before any is loaded.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct OrtSearch {
    /// Every candidate path, in search order.
    pub(super) tried: Vec<std::path::PathBuf>,
    /// The candidates that exist and pass [`trusted_library`], symlinks resolved, in search order.
    pub(super) found: Vec<std::path::PathBuf>,
    /// Existing candidates [`trusted_library`] refused, with the reason.
    pub(super) untrusted: Vec<(std::path::PathBuf, String)>,
    /// The bare name for the platform loader once `found` is exhausted
    /// ([`SearchPolicy::LoaderFallback`] only).
    pub(super) loader_name: Option<&'static str>,
}

/// Decide where to load ONNX Runtime from, without loading it.
///
/// 1. `ORT_DYLIB_PATH`, when set and non-empty, is authoritative: it is returned even when it
///    does not exist, so a mistyped override fails loudly instead of loading another runtime.
///    [`SearchPolicy::Strict`] refuses a relative value; otherwise ort resolves it against the
///    executable's directory, then through the platform loader, as it always has.
/// 2. [`ORT_LIBRARY_NAMES`] in the directory of `image` (the CLI executable, or the binding
///    library that holds xberg), then in `<prefix>/lib` when that directory is `<prefix>/bin`.
/// 3. `system`, the documented install locations.
/// 4. Under [`SearchPolicy::LoaderFallback`], the bare library name through the platform loader.
///
/// The candidates of 2 and 3 are absolute and must pass [`trusted_library`], so neither the
/// current directory nor a directory another local user controls can supply the library under
/// [`SearchPolicy::Strict`].
pub(super) fn resolve_ort_library(
    explicit: Option<&std::ffi::OsStr>,
    image: Option<&std::path::Path>,
    system: &[&std::path::Path],
    policy: SearchPolicy,
) -> OrtSource {
    if let Some(explicit) = explicit.filter(|value| !value.is_empty()) {
        let path = std::path::PathBuf::from(explicit);
        if policy == SearchPolicy::Strict && !path.is_absolute() {
            return OrtSource::RelativeOverride(path);
        }
        return OrtSource::Explicit(path);
    }
    let mut search = OrtSearch {
        tried: release_directories(image)
            .iter()
            .flat_map(|directory| ORT_LIBRARY_NAMES.iter().map(move |name| directory.join(name)))
            .chain(system.iter().map(|path| path.to_path_buf()))
            .collect(),
        ..OrtSearch::default()
    };
    for candidate in &search.tried {
        if !candidate.is_file() {
            continue;
        }
        match trusted_library(candidate) {
            Ok(real) if !search.found.contains(&real) => search.found.push(real),
            Ok(_) => {}
            Err(reason) => search.untrusted.push((candidate.clone(), reason)),
        }
    }
    if policy == SearchPolicy::LoaderFallback {
        search.loader_name = ORT_LIBRARY_NAMES.first().copied();
    }
    OrtSource::Search(search)
}

/// The directory holding `image`, then `<prefix>/lib` when that directory is `<prefix>/bin`
/// (`/usr/local/bin/xberg` with `/usr/local/lib`, as the Docker images lay it out). The parent of
/// any other directory may be shared, as `/tmp` is for `/tmp/x/xberg`, and is not searched.
pub(super) fn release_directories(image: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
    let Some(directory) = image.and_then(std::path::Path::parent) else {
        return Vec::new();
    };
    let mut directories = vec![directory.to_path_buf()];
    if directory.file_name() == Some(std::ffi::OsStr::new("bin"))
        && let Some(prefix) = directory.parent()
    {
        directories.push(prefix.join("lib"));
    }
    directories
}

/// The library at `path` with symlinks resolved, if another local user cannot have placed or
/// replaced it: the file and the directory that holds it must be owned by root or the effective
/// user and not writable by others, nor by a group other than the user's private group
/// ([`is_private_group`]). The resolved path is what gets loaded, so a
/// symlink swapped after this check is not followed. Ancestor directories are not checked; a
/// shared one such as `/tmp` is sticky, so another user cannot rename what is below it.
///
/// `xberg-libheif`'s `dynamic::trusted_library` applies the same rule; keep the two in step.
/// Windows has no owner check; no Windows build xberg ships loads ONNX Runtime dynamically. ~keep
pub(super) fn trusted_library(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let real = std::fs::canonicalize(path).map_err(|error| format!("cannot resolve it: {error}"))?;
        let euid = effective_uid();
        for subject in [Some(real.as_path()), real.parent()].into_iter().flatten() {
            let metadata = std::fs::metadata(subject).map_err(|error| format!("`{}`: {error}", subject.display()))?;
            if metadata.uid() != 0 && metadata.uid() != euid {
                return Err(format!(
                    "`{}` is owned by uid {}, not root or uid {euid}",
                    subject.display(),
                    metadata.uid()
                ));
            }
            let shared_group_writable = metadata.mode() & 0o020 != 0 && !is_private_group(metadata.gid(), euid);
            if metadata.mode() & 0o002 != 0 || shared_group_writable {
                return Err(format!(
                    "`{}` is writable by group or others (mode {:o})",
                    subject.display(),
                    metadata.mode() & 0o777
                ));
            }
        }
        Ok(real)
    }
    #[cfg(not(unix))]
    Ok(path.to_path_buf())
}

/// Whether write access through group `gid` reaches no one but the user `euid`: the group is
/// that user's primary group, is named after the user and lists no members. Debian, Ubuntu and
/// Fedora create such a private group for every user and pair it with umask 002, so a library
/// installed there by pip, npm or `cp` is group-writable without being shared. A group that
/// cannot be looked up counts as shared.
#[cfg(unix)]
#[allow(unsafe_code)]
pub(super) fn is_private_group(gid: u32, euid: u32) -> bool {
    const BUFFER_BYTES: usize = 16 * 1024;
    let mut buffer = vec![0 as libc::c_char; BUFFER_BYTES];
    let mut user = std::mem::MaybeUninit::<libc::passwd>::zeroed();
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: `user` and `buffer` are writable for the sizes given and outlive the call; `found`
    // receives null or `user`.
    let status = unsafe { libc::getpwuid_r(euid, user.as_mut_ptr(), buffer.as_mut_ptr(), buffer.len(), &mut found) };
    if status != 0 || found.is_null() {
        return false;
    }
    // SAFETY: A non-null `found` means `user` was filled, its strings pointing into `buffer`.
    let user = unsafe { user.assume_init() };
    // SAFETY: `pw_name` is a NUL-terminated string in `buffer`, which is still live.
    let user_name = unsafe { std::ffi::CStr::from_ptr(user.pw_name) }.to_owned();
    let user_gid = user.pw_gid;

    let mut group = std::mem::MaybeUninit::<libc::group>::zeroed();
    let mut found: *mut libc::group = std::ptr::null_mut();
    // SAFETY: As above; `buffer` is reused now that the user name has been copied out.
    let status = unsafe { libc::getgrgid_r(gid, group.as_mut_ptr(), buffer.as_mut_ptr(), buffer.len(), &mut found) };
    if status != 0 || found.is_null() {
        return false;
    }
    // SAFETY: A non-null `found` means `group` was filled, its strings pointing into `buffer`.
    let group = unsafe { group.assume_init() };
    // SAFETY: `gr_name` is NUL-terminated and `gr_mem` a null-terminated array, both in `buffer`.
    let (group_name, listed_members) = unsafe {
        (
            std::ffi::CStr::from_ptr(group.gr_name),
            !group.gr_mem.is_null() && !(*group.gr_mem).is_null(),
        )
    };
    private_group(&user_name, user_gid, group_name, listed_members, gid)
}

/// The decision of [`is_private_group`] once the user and group are looked up.
#[cfg(unix)]
pub(super) fn private_group(
    user_name: &std::ffi::CStr,
    user_gid: u32,
    group_name: &std::ffi::CStr,
    listed_members: bool,
    gid: u32,
) -> bool {
    user_gid == gid && group_name == user_name && !listed_members
}

#[cfg(unix)]
#[allow(unsafe_code)]
pub(super) fn effective_uid() -> u32 {
    // SAFETY: geteuid takes no arguments, cannot fail and has no side effects.
    unsafe { libc::geteuid() }
}

/// File names the runtime ships under, searched in this order within each directory. The
/// first is the bare name the platform loader is asked for.
#[cfg(target_os = "macos")]
pub(super) const ORT_LIBRARY_NAMES: &[&str] = &["libonnxruntime.dylib"];

#[cfg(target_os = "linux")]
pub(super) const ORT_LIBRARY_NAMES: &[&str] = &["libonnxruntime.so", "libonnxruntime.so.1"];

#[cfg(target_os = "windows")]
pub(super) const ORT_LIBRARY_NAMES: &[&str] = &["onnxruntime.dll"];

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) const ORT_LIBRARY_NAMES: &[&str] = &[];

#[cfg(all(ort_dynamic_discovery, target_os = "macos"))]
pub(super) fn platform_candidates() -> &'static [&'static str] {
    &[
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ]
}

#[cfg(all(ort_dynamic_discovery, target_os = "linux"))]
pub(super) fn platform_candidates() -> &'static [&'static str] {
    &[
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
        "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
    ]
}

#[cfg(all(ort_dynamic_discovery, target_os = "windows"))]
pub(super) fn platform_candidates() -> &'static [&'static str] {
    &[
        "C:\\Program Files\\onnxruntime\\bin\\onnxruntime.dll",
        "C:\\Windows\\System32\\onnxruntime.dll",
    ]
}

#[cfg(all(
    ort_dynamic_discovery,
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
pub(super) fn platform_candidates() -> &'static [&'static str] {
    &[]
}
