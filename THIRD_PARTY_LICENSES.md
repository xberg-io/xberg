# Third-Party Licenses

Xberg itself is licensed under [MIT](LICENSE). This file documents
notable third-party **native** libraries that Xberg links against or that are
redistributed in published release artifacts, with an emphasis on copyleft
(LGPL/GPL) components and how Xberg stays compliant.

> Rust crate dependencies and their licenses are governed by `deny.toml`
> (`cargo deny check licenses`). This file covers the **system/native** libraries
> that are linked at the C ABI level and are not visible to `cargo deny`.

## libheif (HEIF / HEIC / AVIF) — LGPL

- **Feature:** optional `heic` Cargo feature (part of `full`/`formats`). The
  standalone CLI enables `formats`, so its release binaries link `libheif`
  dynamically.
- **License:** GNU Lesser General Public License (LGPL). See the upstream
  [`COPYING`](https://github.com/strukturag/libheif/blob/master/COPYING) for the
  authoritative version and text.
- **Linking:** **Dynamic only.** Xberg resolves `libheif` via `pkg-config`
  (`-lheif`) against the system shared library; it is never statically linked.
  The musl CLI container build explicitly disables `crt-static`
  (`RUSTFLAGS="-C target-feature=-crt-static"`) so the resulting binary loads
  `libheif.so` at runtime rather than embedding it. The static-build
  (`embedded-libheif`) feature has been **removed** from `xberg-libheif`, so
  there is no supported way to statically link `libheif` into a Xberg build.
- **Redistribution:** the Linux CLI archives and the `full`/`core` images ship
  unmodified `libheif` shared objects separately from the Xberg executable.
  The glibc builds use v1.23.0 from the official release tarball; musl builds
  use Alpine's shared package. The library remains replaceable to satisfy LGPL
  §6. Upstream source: <https://github.com/strukturag/libheif>.

## libheif codec libraries

Depending on how `libheif` was built, its codec backends are linked shared
libraries or dynamically-loaded plugins. Linux CLI packaging vendors the
non-system shared-library closure required by its `libheif` build; container
images install the same codecs from the distro package manager. Each remains a
separate, replaceable shared object and retains its upstream license:

| Library  | Role            | License (upstream)          |
| -------- | --------------- | --------------------------- |
| libde265 | HEVC **decode** | LGPL-3.0-or-later           |
| libdav1d | AV1 **decode**  | BSD-2-Clause                |
| libaom   | AV1 dec/enc     | BSD-2-Clause + patent grant |
| libx265  | HEVC **encode** | **GPL-2.0-or-later**        |

Xberg supports both HEIF-family input decoding and optional HEIF image output.
`libx265` is needed only for the latter and is redistributed only when the
platform's `libheif` build links it.

## ONNX Runtime (OCR / ML features)

- **Feature:** optional (`paddle-ocr`, `layout-detection`, `embeddings`,
  `reranker`, `auto-rotate`, transcription). License: MIT. Linked dynamically
  (system `libonnxruntime.so`) in the musl/container builds; bundled per the
  `ort-bundled` feature (official Microsoft binaries) otherwise.
