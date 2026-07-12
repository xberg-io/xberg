# Platform support matrix

Authoritative record of which prebuilt native artifacts each language binding ships, and the known
gaps. Source of truth is `.github/workflows/publish.yaml` — this table is derived from those build
matrices. Keep it in sync when a matrix leg is added or dropped.

Legend: ✅ prebuilt shipped · ❌ not shipped · — not applicable

## Desktop / server

| Binding (registry) | Linux x64 (glibc) | Linux arm64 (glibc) | Linux x64 (musl) | Linux arm64 (musl) | macOS arm64 | macOS x64 (Intel) | Windows x64 |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **CLI** (standalone + npm proxy) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Java** (Maven Central) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **C#** (NuGet) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Elixir** (Hex) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Node** (npm) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ ¹ | ✅ |
| **Python** (PyPI) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Go** (module + C FFI) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **PHP** (Composer / PIE) ² | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Dart** (pub.dev) ³ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **C FFI** (GitHub release) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Zig** (Zig package) ⁴ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ✅ |
| **Ruby** (RubyGems) | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ |

## Apple / mobile / portable

| Binding (registry) | macOS arm64 | iOS arm64 | Android arm64-v8a | Android x86_64 | wasm32 |
|---|:---:|:---:|:---:|:---:|:---:|
| **Swift** (SwiftPM artifactbundle) ⁵ | ✅ | ✅ | — | — | — |
| **Kotlin / Android** (Maven Central) ⁶ | — | — | ✅ | ✅ | — |
| **WASM** (npm) | — | — | — | — | ✅ ⁷ |

## Known gaps & rationale

1. **Node · macOS x64 (Intel) — dropped (rc.23).** pyke ships no static x64-mac ORT, and
   Microsoft's last x86_64-macOS ONNX Runtime dylib is 1.23.2 (the CLI vendors that one), so at the
   time CI provisioned ORT via Homebrew, whose bottle dynamically links a ~252-lib abseil closure
   at absolute Homebrew paths. The self-containment vendor step
   (`scripts/ci/vendor-macos-node-dylibs.sh`) correctly rejected the non-portable package, and the
   Intel-mac node leg was dropped. Intel Mac users run the arm64 binding under Rosetta or use the
   WASM package. In **rc.22** this leg *failed* (so no node package published at all); the drop
   lands in **rc.23**.
2. **PHP** builds against **8.3, 8.4, 8.5** on every listed platform.
3. **Dart** ships the server-mode native; the full pub.dev package has a known size blocker (all-platform
   natives exceed the 100 MB cap) tracked separately — see the release notes.
4. **Zig** consumes the **C FFI** GitHub-release artifacts, so its platform coverage equals C FFI's.
5. **Swift** targets Apple platforms only — macOS (Apple Silicon) and iOS (arm64). Intel-mac
   (`include-macos-x86_64: false`) and iOS-simulator-x86_64 are excluded; there is no Linux or Windows
   SwiftPM artifact.
6. **Kotlin/Android** ships the two Android ABIs — `arm64-v8a` (devices) and `x86_64` (emulator).
   The x86_64-emulator native uses the ORT-free `android-target` feature set (no PaddleOCR/layout/
   embeddings/auto-rotate); arm64 devices get the full ORT-enabled build.
7. **WASM** is a single `wasm32` artifact, portable across any WASM runtime (browser + Node). It uses
   the `wasm-target` feature set (`ocr-wasm`, `excel-wasm`, `tree-sitter-wasm`; no native ORT).

## Cross-cutting gaps

- **musl (Alpine / static Linux):** shipped only by **CLI, Java, C#, Elixir, Node**. Python, Ruby, Go,
  PHP, Dart, C FFI, and Zig ship glibc-only Linux — musl consumers must build from source.
- **Windows:** every desktop binding ships Windows x64 **except Ruby** (no RubyGems Windows native) and
  the Apple/mobile/wasm bindings (n/a).
- **Intel Mac (macOS x64):** shipped by most bindings; **not** by Node (see gap ¹) or Swift.
- **Linux arm64 musl** exists only where full musl is listed (CLI/Java/C#/Elixir/Node).
