# xberg-pdfium-render

A high-level idiomatic Rust wrapper around [Pdfium](https://pdfium.googlesource.com/pdfium/),
the C++ PDF library used by the Google Chromium project.

This is a fork of [pdfium-render](https://github.com/ajrcarey/pdfium-render) maintained as part
of the [Xberg](https://xberg.io) project.

## Upstream

- **Original crate**: [pdfium-render](https://crates.io/crates/pdfium-render)
- **Original author**: Alastair Carey
- **Original license**: MIT OR Apache-2.0

The fork is based on the upstream 0.9.0 development line at or after commit 0dbad90 (2025-12-13),
before upstream commit 7f33ee7 (2026-02-03).

## Differences from upstream

- `PdfPageTextChar::has_unicode_map_error()` keeps the mapping `1 => true`, `0 => false`. Upstream's
  change f14b224 (#281) inverts it, which contradicts the pdfium 7678 header
  (`include/pdfium_7678/fpdf_text.h`: "1 if the character has an invalid unicode mapping"), so it was
  deliberately not adopted.
- Upstream's `thread_safe` mutex wrapper around pdfium calls was not adopted (the `thread_safe` feature here only marks types `Send`/`Sync`). Xberg serialises all
  pdfium access behind a global lock (`PDFIUM_OPERATION_LOCK` in
  `crates/xberg/src/extractors/pdf/pdfium_engine.rs`).

## License

MIT - see [LICENSE](LICENSE) for details.
