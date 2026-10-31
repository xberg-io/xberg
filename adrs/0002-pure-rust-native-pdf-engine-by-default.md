---
status: accepted
date: 2026-08-19
decision-makers: ["Na'aman Hirschfeld"]
---

# Pure-Rust native PDF engine by default

## Context and Problem Statement

PDF extraction ran on Pdfium, a native library each build and package had to ship. Its build and release cost appears
in the history: composite actions to download, cache and stage the library (`download-pdfium`, `cache-pdfium`,
`stage-pdfium-runtime`), download mirrors and 503 fallbacks (32ff58ff79, 11a47ba330), and a bundled `pdfium-render`
fork. From 2026-03-15, `pdf_oxide`, a pure-Rust engine, was integrated as an alternative backend (b12d1fa980,
d409616f03). 124ba41808 (2026-05-07) then removed Pdfium and made `pdf_oxide` the only backend.

## Considered Options

* Pdfium as the default, with `pdf_oxide` as an alternative backend (2026-03-15 to 2026-05-07)
* `pdf_oxide` as the only PDF backend (2026-05-07 to 2026-08-19)
* The pure-Rust engine as the default, with Pdfium selectable through `PdfConfig.backend`

## Decision Outcome

Chosen option: "The pure-Rust engine as the default, with Pdfium selectable through `PdfConfig.backend`", because
the default build then needs no native PDF library. That is the reason the 1.0.0 CHANGELOG gives for removing Pdfium
("no native pdfium dependency"). No commit records why Pdfium came back as a selectable engine.

* 149a9130e6 (2026-08-19) added `PdfConfig.backend`, defaulting to the pure-Rust engine. ae520e4ef6 (2026-08-21)
  rejects a Pdfium request that the build cannot serve, and fcfb6620b3 (2026-08-21) implemented the Pdfium engine
  with a narrower surface: page count, per-page text and Info-dictionary metadata. The work is tracked as #700 and
  #702 (`crates/xberg/Cargo.toml:129` and `:146`), and none of these commits states a reason for the restoration.
* c8ed4a98cb (2026-08-22) vendored the pure-Rust engine as `crates/xberg-native-pdf`, and 9ffbdcae55 renamed the
  backend to `native`.
* adb7170386 (2026-09-27, GH#1863) compiled `pdf-pdfium` into the published Python, Node, Ruby, PHP and Elixir
  packages. There, Pdfium is a runtime choice through `PdfConfig.backend`, and `libpdfium` is loaded at run time and
  never bundled. The FFI crate only declares the feature without enabling it, and the Windows, Intel-macOS, WASM,
  Android and iOS artifacts do not carry the Pdfium engine.
* Selecting Pdfium where the engine is not compiled in is an error, never a silent fallback to the native engine.

### Consequences

* Good, because the default backend in every build, the WASM target included, extracts PDFs without a native library
  to ship or discover.
* Good, because xberg owns the default engine's source and can fix extraction defects in this repository.
* Bad, because PDF parsing quality is this project's own maintenance burden, not an upstream one.
* Bad, because a caller who selects Pdfium must supply `libpdfium` and point `PDFIUM_DYNAMIC_LIB_PATH` at it, and
  gets a narrower result than the native engine produces.

### Confirmation

* `crates/xberg/Cargo.toml:133` sets `pdf = ["pdf-native"]`, and `:153` defines
  `pdf-pdfium = ["dep:pdfium-render", "tokio-runtime"]`.
* `crates/xberg/src/core/config/pdf.rs:20` marks `PdfBackend::Native` as `#[default]`.
* `alef.toml:1460` enables `pdf-pdfium` in the shared `[[crates]].features` array that the Python, Node, Ruby, PHP
  and Elixir bindings inherit, and `alef.toml:2047` only declares it for the FFI crate in `extra_features`.
* The pdf-backends skill (`.ai-rulez/skills/pdf-backends/SKILL.md`) records the native default and the no-fallback
  rule.
