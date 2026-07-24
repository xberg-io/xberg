# xberg-libwpd

WordPerfect (`.wpd`, `.wp5`, `.wp6`, and the wider WordPerfect binary family
from WP 4.2 through the X-series) text extraction for
[Xberg](https://xberg.io), backed by [libwpd](https://libwpd.sourceforge.net/)
and its document-model dependency
[librevenge](https://wiki.documentfoundation.org/DLP/Libraries/librevenge).

## How it works

libwpd exposes no `extract()` call. It drives librevenge's SAX-like
`RVNGTextInterface`: the caller supplies a concrete implementation and libwpd
invokes its callbacks. A hand-written C++ shim (`src/shim.cpp`) implements that
interface, accumulates a plain-text rendering (paragraphs, list items and
tables), and exposes a small flat C API. `src/lib.rs` wraps it in a safe Rust
surface:

```rust
let text = xberg_libwpd::extract_text(&bytes)?;
let ok = xberg_libwpd::is_supported(&bytes);
```

## Building

`build.rs` downloads the librevenge and libwpd release tarballs (checksum
verified, cached under `OUT_DIR`) and compiles them from source together with
the shim into one static library, using the C++ toolchain via the `cc` crate.
Both libraries are built against their **MPL-2.0** arm.

### Requirements

- A C++17 compiler.
- **boost headers.** librevenge and libwpd both use header-only `boost::spirit`
  at build time. Install boost (`brew install boost`, or
  `apt-get install libboost-dev`) or point `BOOST_INCLUDE_DIR` at a directory
  containing `boost/version.hpp`.
- system zlib (librevenge's zip stream links against it).

## Platform support

Desktop only: Linux (glibc and musl) and macOS. On any other target the crate
compiles to stub functions that return `WpdError::UnsupportedPlatform`, so
wasm/android/windows builds pull in no C++ toolchain.

## Licensing

This crate is MIT. libwpd and librevenge are used under their MPL-2.0 arm; their
source is fetched at build time and is not redistributed in this repository.
MPL-2.0 is file-level copyleft and permits static linking into a
differently-licensed larger work.
