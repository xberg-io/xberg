// Derived from fontdb 0.24.0 (https://github.com/RazrFalcon/fontdb),
// Copyright (c) 2020 Yevhenii Reizner, MIT licensed. Modified: `log::warn!`
// call sites were converted to `tracing::warn!`, and `load_font_file` reads
// faces through `scan::read_face_infos` instead of loading the whole file;
// otherwise this is a near-verbatim port — plain `std::fs` directory walking
// with symlink-loop protection, and (on Linux/BSD) fontconfig-driven
// generic-family and directory discovery. ~keep

//! System font directory discovery: per-OS directory lists, the recursive
//! directory walk, and (on Linux/BSD) fontconfig integration.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Database, Source, scan};

impl Database {
    /// Loads font files from `dir` into the `Database`, recursively.
    ///
    /// Loads `ttf`/`otf`/`ttc`/`otc` fonts; skips (and warns on) malformed
    /// files rather than failing.
    ///
    /// `load_system_fonts` is the only caller and has no wasm32 or Android
    /// branch (no target directory list matches either target), so this and
    /// the two helpers below are structurally unreachable — not called,
    /// not dead by mistake — on those targets. ~keep
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    fn load_fonts_dir_impl(&mut self, dir: &Path, seen: &mut HashSet<PathBuf>) {
        let fonts_dir = match std::fs::read_dir(dir) {
            Ok(dir) => dir,
            // No filesystem (wasm32) or unreadable directory: yield nothing
            // rather than failing the whole scan. ~keep
            Err(_) => return,
        };

        for entry in fonts_dir.flatten() {
            let Some((path, file_type)) = self.canonicalize(entry.path(), &entry, seen) else {
                continue;
            };

            if file_type.is_file() {
                if let Some("ttf" | "ttc" | "TTF" | "TTC" | "otf" | "otc" | "OTF" | "OTC") =
                    path.extension().and_then(|e| e.to_str())
                {
                    self.load_font_file(&path);
                }
            } else if file_type.is_dir() {
                self.load_fonts_dir_impl(&path, seen);
            }
        }
    }

    /// Loads every face in `path`, warning (and continuing) per malformed
    /// face rather than failing the directory scan over one bad file.
    ///
    /// Reads only the table directories and the tables the face parse needs
    /// (`scan::read_face_infos`), not the whole file. ~keep
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    fn load_font_file(&mut self, path: &Path) {
        let faces = match scan::read_face_infos(path) {
            Ok(faces) => faces,
            Err(error) => {
                tracing::warn!(
                    target: "xberg_native_pdf::vendor::fontdb",
                    path = %path.display(),
                    %error,
                    "failed to read font file; skipping",
                );
                return;
            }
        };

        for (index, parsed) in faces.into_iter().enumerate() {
            match parsed {
                Ok(info) => {
                    tracing::trace!(
                        target: "xberg_native_pdf::vendor::fontdb",
                        path = %path.display(),
                        face_index = index,
                        "loaded font face",
                    );
                    self.faces.push(info);
                }
                Err(error) => tracing::warn!(
                    target: "xberg_native_pdf::vendor::fontdb",
                    path = %path.display(),
                    face_index = index,
                    %error,
                    "failed to load a font face; skipping",
                ),
            }
        }
    }

    /// Attempts to load system fonts. Supports Windows, macOS, Linux/BSD and
    /// Redox. On wasm32 (no real filesystem) every directory read fails
    /// silently, so this simply leaves the `Database` empty rather than
    /// erroring.
    pub(crate) fn load_system_fonts(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let mut seen = HashSet::default();
            if let Some(system_root) = std::env::var_os("SYSTEMROOT") {
                let system_root_path = Path::new(&system_root);
                self.load_fonts_dir_impl(&system_root_path.join("Fonts"), &mut seen);
            } else {
                self.load_fonts_dir_impl(Path::new("C:\\Windows\\Fonts\\"), &mut seen);
            }

            if let Ok(home) = std::env::var("USERPROFILE") {
                let home_path = Path::new(&home);
                self.load_fonts_dir_impl(&home_path.join("AppData\\Local\\Microsoft\\Windows\\Fonts"), &mut seen);
                self.load_fonts_dir_impl(
                    &home_path.join("AppData\\Roaming\\Microsoft\\Windows\\Fonts"),
                    &mut seen,
                );
            }
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let mut seen = HashSet::default();
            self.load_fonts_dir_impl(Path::new("/Library/Fonts"), &mut seen);
            self.load_fonts_dir_impl(Path::new("/System/Library/Fonts"), &mut seen);
            // Downloadable fonts; location varies by macOS release. ~keep
            if let Ok(dir) = std::fs::read_dir("/System/Library/AssetsV2") {
                for entry in dir.flatten() {
                    if entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("com_apple_MobileAsset_Font")
                    {
                        self.load_fonts_dir_impl(&entry.path(), &mut seen);
                    }
                }
            }
            self.load_fonts_dir_impl(Path::new("/Network/Library/Fonts"), &mut seen);

            if let Ok(home) = std::env::var("HOME") {
                self.load_fonts_dir_impl(&Path::new(&home).join("Library/Fonts"), &mut seen);
            }
        }

        #[cfg(target_os = "redox")]
        {
            let mut seen = HashSet::default();
            self.load_fonts_dir_impl(Path::new("/ui/fonts"), &mut seen);
        }

        // Linux/BSD. ~keep
        #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
        {
            if !self.load_fontconfig() {
                tracing::warn!(
                    target: "xberg_native_pdf::vendor::fontdb",
                    "fontconfig unavailable or empty; falling back to known font directory paths",
                );
                self.load_no_fontconfig();
            }
        }
    }

    /// Scans the hard-coded Linux/BSD font directories directly, without
    /// consulting fontconfig. Used as a fallback when fontconfig parsing
    /// fails or reports no directories.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn load_no_fontconfig(&mut self) {
        let mut seen = HashSet::default();
        self.load_fonts_dir_impl(Path::new("/usr/share/fonts/"), &mut seen);
        self.load_fonts_dir_impl(Path::new("/usr/local/share/fonts/"), &mut seen);

        if let Ok(home) = std::env::var("HOME") {
            let home_path = Path::new(&home);
            self.load_fonts_dir_impl(&home_path.join(".fonts"), &mut seen);
            self.load_fonts_dir_impl(&home_path.join(".local/share/fonts"), &mut seen);
        }
    }

    /// Reads the host's fontconfig configuration (`$FONTCONFIG_FILE`,
    /// `$XDG_CONFIG_HOME/fontconfig/fonts.conf`, `/etc/fonts/{local,fonts}.conf`)
    /// to override the generic-family defaults and discover the actual
    /// configured font directories, which commonly differ from the
    /// hard-coded fallback list in `load_no_fontconfig` — e.g. distros that
    /// symlink or relocate `/usr/share/fonts`.
    ///
    /// Returns `false` (caller falls back to `load_no_fontconfig`) when no
    /// fontconfig directories were found, matching upstream fontdb.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    fn load_fontconfig(&mut self) -> bool {
        let mut fontconfig = fontconfig_parser::FontConfig::default();
        let home = std::env::var("HOME");

        if let Ok(config_file) = std::env::var("FONTCONFIG_FILE") {
            let _ = fontconfig.merge_config(Path::new(&config_file));
        } else {
            let xdg_config_home = if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
                Some(PathBuf::from(val))
            } else if let Ok(ref home) = home {
                // https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
                // $XDG_CONFIG_HOME defaults to $HOME/.config when unset. ~keep
                Some(Path::new(home).join(".config"))
            } else {
                None
            };

            let read_global = match xdg_config_home {
                Some(p) => fontconfig.merge_config(&p.join("fontconfig/fonts.conf")).is_err(),
                None => true,
            };

            if read_global {
                let _ = fontconfig.merge_config(Path::new("/etc/fonts/local.conf"));
            }
            let _ = fontconfig.merge_config(Path::new("/etc/fonts/fonts.conf"));
        }

        for fontconfig_parser::Alias {
            alias,
            default,
            prefer,
            accept,
        } in fontconfig.aliases
        {
            let name = prefer.first().or_else(|| accept.first()).or_else(|| default.first());

            if let Some(name) = name {
                match alias.to_lowercase().as_str() {
                    "serif" => self.set_serif_family(name.as_str()),
                    "sans-serif" | "sans serif" => self.set_sans_serif_family(name.as_str()),
                    "monospace" => self.set_monospace_family(name.as_str()),
                    "cursive" => self.set_cursive_family(name.as_str()),
                    "fantasy" => self.set_fantasy_family(name.as_str()),
                    _ => {}
                }
            }
        }

        if fontconfig.dirs.is_empty() {
            return false;
        }

        let mut seen = HashSet::default();
        for dir in fontconfig.dirs {
            let path = if let Ok(rest) = dir.path.strip_prefix("~") {
                match home {
                    Ok(ref home) => Path::new(home).join(rest),
                    Err(_) => continue,
                }
            } else {
                dir.path
            };
            self.load_fonts_dir_impl(&path, &mut seen);
        }

        true
    }

    /// Resolves a directory entry to a canonical, not-yet-seen path,
    /// following symlinks exactly once per directory tree to avoid
    /// symlink-loop cycles.
    ///
    /// The `seen` set starts empty and is left untouched until the first
    /// symlinked directory is encountered — at that point it's lazily
    /// seeded with the paths of every face already loaded, so a symlink
    /// loop can't re-load a file the walk already visited via a different
    /// path. This keeps the common case (no symlinks) free of a HashSet
    /// lookup per entry. ~keep
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    fn canonicalize(
        &self,
        path: PathBuf,
        entry: &std::fs::DirEntry,
        seen: &mut HashSet<PathBuf>,
    ) -> Option<(PathBuf, std::fs::FileType)> {
        let file_type = entry.file_type().ok()?;
        if !file_type.is_symlink() {
            if !seen.is_empty() {
                if seen.contains(&path) {
                    return None;
                }
                seen.insert(path.clone());
            }

            return Some((path, file_type));
        }

        if seen.is_empty() && file_type.is_dir() {
            seen.reserve(8192 / std::mem::size_of::<PathBuf>());

            for info in &self.faces {
                if let Source::File(path) = &info.source {
                    seen.insert(path.to_path_buf());
                }
            }
        }

        let stat = std::fs::metadata(&path).ok()?;
        if stat.is_symlink() {
            return None;
        }

        let canon = std::fs::canonicalize(path).ok()?;
        if seen.contains(&canon) {
            return None;
        }
        seen.insert(canon.clone());
        Some((canon, stat.file_type()))
    }
}
