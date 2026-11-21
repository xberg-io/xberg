# xberg-libheif

Safe Rust bindings around `libheif-sys` for decoding HEIF / HEIC / AVIF
containers â vendored into Xberg from the original
[libheif-rs](https://github.com/Cykooz/libheif-rs) by
[Kirill Kuzminykh (Cykooz)](https://github.com/Cykooz).

Vendored at upstream version **v2.7.0** (MIT-licensed). The vendored copy
preserves the upstream MIT license; see `LICENSE`. See `ATTRIBUTIONS.md`
at the repo root for full attribution details.

This crate exists so Xberg can pin, patch, and stabilise the HEIF
decoder surface against the rest of the workspace. We continue to depend
on upstream `libheif-sys` from crates.io for the underlying C bindings.

## Modifications from upstream

- Workspace dependency alignment (`libc`, `image` pinned via workspace).
- Pinned to Rust edition 2021 (matches upstream); the rest of the workspace is
  edition 2024 but this crate keeps the upstream pattern because libheif-rs
  uses unwrapped unsafe operations inside `unsafe fn`, which is a hard error
  under edition 2024.
- Workspace lints applied via `[lints] workspace = true`.
- Trimmed README to focus on the Xberg use case.

The public API is unchanged from upstream `libheif-rs` v2.7.0.

## System requirements

`libheif` (the C library) must be installed at build and runtime, with
HEVC and AV1 codec backends (`libde265`, `libaom`):

- **macOS**: `brew install libheif`
- **Debian/Ubuntu**: `apt install libheif-dev`
- **Fedora**: `dnf install libheif-devel`
- **Windows**: `vcpkg install libheif[core,aom,libde265]:x64-windows`, then set
  `VCPKGRS_TRIPLET=x64-windows` and `VCPKGRS_DYNAMIC=1` so `libheif-sys` links
  the import library (the DLLs must be on `PATH` at run time)

`libheif` is LGPL-licensed and is therefore linked **dynamically only** — it is
never statically linked or vendored into Xberg binaries. Release archives ship
the shared libraries and the LGPL-3.0 and GPL-3.0 texts beside the executable.

## License

MIT, per upstream.
