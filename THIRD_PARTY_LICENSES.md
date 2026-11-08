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
- **Linking:** **Dynamic**, with one exception (below). Xberg resolves
  `libheif` via `pkg-config` (`-lheif`) against a shared library. The musl CLI
  container build explicitly disables `crt-static`
  (`RUSTFLAGS="-C target-feature=-crt-static"`) so the resulting binary loads
  `libheif.so` at runtime rather than embedding it. The embedded source build
  (`embedded-libheif`) feature has been **removed** from `xberg-libheif`. The
  one static link is the macOS PHP extension, described in the next section.
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

The macOS Elixir NIF archive ships `libheif`, `libde265`, `libaom` and `libx265`
(built from the pinned sources with both encoders) as separate sibling dylibs
resolved through `@loader_path`, beside the upstream license texts and a checksum
inventory. They remain separate, replaceable shared objects.

## PHP macOS extension: static HEIF stack

PIE installs a PHP extension by copying a single file, so sibling dylibs cannot
ship with it (a Homebrew-linked build failed to load on Macs without
`libheif`, xberg-io/xberg#2547). Static linking is therefore used here, and only
here: the macOS PHP extension (`xberg.so`, Apple Silicon, macOS 15 and later)
links the following libraries into the extension binary. The Elixir NIF and every
other binding keep shared libraries.

| Library  | Version | Role                          | License (upstream)          |
| -------- | ------- | ----------------------------- | --------------------------- |
| libheif  | 1.23.7  | HEIF/HEIC/AVIF container      | LGPL-3.0-or-later           |
| libde265 | 1.1.3   | HEVC **decode**               | LGPL-3.0-or-later           |
| libaom   | 3.15.2  | AV1 **decode** (encoder off)  | BSD-2-Clause + patent grant |

The static set is decode-only. Nothing in it can encode, and no GPL code is
linked: `libx265` and every other encoder are excluded, so HEIF **output** is
unavailable in this extension (Xberg reports an encoder-unavailable warning, as
the macOS Python wheel and Node packages already do). HEIC and AVIF decoding is
unaffected.

- **Source and provenance:** every component is built unmodified from its
  pinned upstream release tarball. The URLs and SHA-256 checksums are in
  [`scripts/ci/build-macos-heif-deps.sh`](scripts/ci/build-macos-heif-deps.sh)
  (profile `php-static`) and in `sources.tsv`, shipped inside each macOS PHP
  archive beside the upstream license texts (`libheif-*-COPYING`,
  `libde265-*-COPYING`, `libaom-*-LICENSE`, `libaom-*-PATENTS`) and a
  `notices.sha256` list. Xberg's own `LICENSE` and this file ship there too.
- **Relinking (LGPL-3.0 §4):** the complete source of the extension is public
  (<https://github.com/xberg-io/xberg>, `crates/xberg-php`, MIT). To relink it
  against your own modified `libheif` or `libde265`, build the static archives
  with your sources (`XBERG_HEIF_PROFILE=php-static`
  `XBERG_HEIF_PREFIX=<dir>` `MACOSX_DEPLOYMENT_TARGET=15.0`
  `bash scripts/ci/build-macos-heif-deps.sh`, after changing the pinned URL and
  checksum), then rebuild with `PKG_CONFIG_PATH=<dir>/lib/pkgconfig`
  `SYSTEM_DEPS_LIBHEIF_LINK=static` `cargo build --release -p xberg-php` and
  install the resulting `xberg.so` in place of the shipped one.
- **Gate:** the release workflow runs `scripts/ci/verify-php-macos-extension.sh`
  on every macOS PHP archive. It rejects any import outside `/usr/lib` and
  `/System/Library` (including Homebrew paths), any libheif or codec import, a
  deployment target above macOS 15.0, and a missing or altered notice.

## libwpd + librevenge (WordPerfect) — MPL-2.0

- **Feature:** the `xberg-libwpd` crate, consumed by `xberg`'s `wordperfect`
  feature (included in `formats`, `full` and `windows-target`). Desktop only
  (Linux, macOS and Windows).
- **License:** libwpd and librevenge are dual-licensed **MPL-2.0 OR
  LGPL-2.1+**. Xberg builds and links them under the **MPL-2.0** arm.
- **Linking:** **Static, from source.** `xberg-libwpd` vendors the upstream
  librevenge and libwpd release tarballs as committed `.tar.gz` archives under
  `crates/xberg-libwpd/vendor/`; its build script decompresses them
  (checksum-verified at extract time), compiles them from source, and
  statically links them together with a small C++ shim. MPL-2.0 is file-level
  (weak) copyleft and explicitly permits static linking into a larger,
  differently-licensed work; obligations are limited to keeping the
  libwpd/librevenge source and any modifications to their files available under
  MPL-2.0 and preserving license notices. No copyleft reaches Xberg's MIT core
  or the language bindings. The LGPL arm is intentionally not used, as LGPL
  static linking would impose relink/object-file obligations on redistributed
  binaries.
- **Modifications:** none; both libraries are built from unmodified upstream
  source (librevenge 0.0.6, libwpd 0.10.3). Upstream source:
  <https://libwpd.sourceforge.net/>.
- **Boost (build-time headers) — BSL-1.0:** the C++ shim compiles against a
  `bcp`-extracted header-only subset of the Boost C++ Libraries, vendored as
  `crates/xberg-libwpd/vendor/boost-subset.tar.gz` and decompressed at build
  time. Boost is licensed under the **Boost Software License 1.0** (BSL-1.0), a
  permissive license whose only obligation is preserving the license text; no
  copyleft or attribution-in-binary obligation reaches Xberg. Boost headers are
  a build-time dependency only — no Boost code is redistributed in Xberg binaries.

## ONNX Runtime (OCR / ML features)

- **Feature:** optional (`paddle-ocr`, `layout-detection`, `embeddings`,
  `reranker`, `auto-rotate`, transcription). License: MIT. Linked dynamically
  (system `libonnxruntime.so`) in the musl/container builds; bundled per the
  `ort-bundled` feature (official Microsoft binaries) otherwise.

## Tesseract OCR + Leptonica

- **Feature:** `xberg-tesseract` builds Tesseract 5.5.4 at commit
  `7e548e6eff172f38f97664afe7f7ce40db36c8d6` and Leptonica 1.87.0 at commit
  `13275a278eb55b5746e33f95fbf5a2c8f604b3ab` when `build-tesseract` or
  `build-tesseract-wasm` is enabled.
- **Licenses:** Tesseract is Apache-2.0; the exact upstream text is preserved in
  [`crates/xberg-tesseract/licenses/tesseract-LICENSE`](crates/xberg-tesseract/licenses/tesseract-LICENSE).
  Leptonica is BSD-2-Clause; the exact upstream notice is preserved in
  [`crates/xberg-tesseract/licenses/leptonica-LICENSE`](crates/xberg-tesseract/licenses/leptonica-LICENSE).
- **Linking:** both native libraries are built from their checksum-verified
  source archives and statically linked into the consuming artifact. Their
  permissive licenses permit static linking when the license texts and notices
  accompany redistribution.
- **Model redistribution:** `eng.traineddata` is redistributed from
  `tessdata_fast` commit `87416418657359cb625c412a48b6e1d6d41c29bd` after
  checksum verification. `tessdata_fast` uses the same Apache-2.0 license text
  as Tesseract, so the preserved Tesseract license covers the model artifact.
