# Cycle 4 Blocker Investigation (for alef v0.14.3 sibling agent)

All four binding wrappers (`Kreuzberg.kt`, `Kreuzberg.swift`, `kreuzberg.rb`, `kreuzberg.R`)
carry the alef auto-gen header. Every blocker below is **alef-side codegen** — none are
hand-edited drift in the kreuzberg repo.

## 1. Kotlin — uncompilable references in `Kreuzberg.kt`

**Files**

- `packages/kotlin/src/main/kotlin/dev/kreuzberg/kt/Kreuzberg.kt` (alef-generated)
- `packages/java/src/main/java/dev/kreuzberg/Kreuzberg.java` (alef-generated, used as `Bridge`)

**Symptom**

Kotlin calls Java methods that no longer exist or have wrong return types:

| Kotlin (Kreuzberg.kt)                        | Actual Java (Kreuzberg.java)                  | Issue                |
| -------------------------------------------- | --------------------------------------------- | -------------------- |
| `Bridge.batchExtractFileSync` (L331)         | `batchExtractFilesSync` (L116)                | Stale singular name  |
| `Bridge.batchExtractFile` (L381)             | `batchExtractFiles` (L165)                    | Stale singular name  |
| `Bridge.listExtractors` (L451)               | `listDocumentExtractors` (L244)               | Old non-renamed name |
| `Bridge.getEmbeddingPreset → String?` (L551) | `getEmbeddingPreset → EmbeddingPreset` (L353) | Wrong return type    |

**Root cause**

alef-backend-kotlin's identifier mapping for batch APIs and for `list_document_extractors`
is out of sync with the Java backend after the latter was renamed. The `getEmbeddingPreset`
case is more fundamental: the Java backend started returning the typed `EmbeddingPreset`
record, but Kotlin codegen still emits the `String?` skeleton from before that change.

**Fix location** alef (alef-backend-kotlin)

- Rename mapping must match Java's pluralised `batchExtractFiles[Sync]` and renamed
  `listDocumentExtractors`.
- Return-type inference for `getEmbeddingPreset` should follow the Java backend's
  record type; Kotlin must emit `EmbeddingPreset?` and unwrap the `Optional<EmbeddingPreset>`
  the same way Java does (`.orElse(null)`).

**Severity** Blocker — `:kotlin:compileKotlin` fails, the artifact does not exist.

## 2. Swift — 0 free functions in `Kreuzberg.swift`

**Files**

- `packages/swift/Sources/Kreuzberg/Kreuzberg.swift` (alef-generated, 1511 lines, 215 type
  re-exports, **zero `public func`**)
- `packages/swift/Sources/RustBridge/RustBridge.swift` — placeholder only:

  ```swift
  public enum RustBridgePlaceholder {}
  ```

- `packages/swift/Sources/RustBridgeC/RustBridgeC.h` — bare scaffold

**Root cause**

`Sources/Kreuzberg/Kreuzberg.swift` is generated as a "facade" — it does
`public typealias Foo = RustBridge.Foo` for every type and is supposed to also re-export
free functions from `RustBridge.*`. Two problems:

1. **alef-backend-swift never emits the function facades.** It walks the type set and
   emits typealiases, but the function set is empty. (Only types are visible in the file.)
2. **`RustBridge` is a placeholder.** `BUILDING.md` and `Package.swift` confirm the design:
   `cargo build -p kreuzberg-swift` is supposed to emit swift-bridge generated `.swift`
   files which the user copies into `Sources/RustBridge/`. **There is no `kreuzberg-swift`
   crate** in `crates/` (only `kreuzberg{,-cli,-ffi,-node,-paddle-ocr,-pdfium-render,-php,-py,-tesseract,-wasm}`).

So Swift is broken in two ways:

- alef must emit a `kreuzberg-swift` workspace crate (mirroring `kreuzberg-php`/`kreuzberg-py`)
  that uses `swift-bridge` macros and produces both the C glue and `RustBridge.swift`.
- alef-backend-swift must emit real function bodies that wrap each
  `RustBridge.<fn>` with `@_disfavoredOverload` shims, error mapping (`KreuzbergError`
  enum), and `Optional`/`Result` translation.

**Fix location** alef (both: a new swift crate template, and the Kreuzberg.swift function
generator).

**Severity** Blocker — Swift package compiles (typealiases all reduce to placeholder
types) but is completely unusable: zero callable surface.

## 3. Ruby — 13-line `lib/kreuzberg.rb` is **correct** (no fix needed)

**Files**

- `packages/ruby/lib/kreuzberg.rb` (3 non-comment lines: `require 'kreuzberg_rb'`)
- `packages/ruby/ext/kreuzberg_rb/src/lib.rs` — alef-generated, 13374 lines, **30
  `define_module_function` calls**, all canonical fns + plugin trait bridges.

**Investigation**

Magnus binds Rust functions directly onto the `Kreuzberg` Ruby module via
`module.define_module_function("name", function!(rust_fn, arity))`. Once
`kreuzberg_rb.bundle` is loaded, **the module already has all 27 functions as
top-level callable methods.** `lib/kreuzberg.rb` only needs to `require` the
native ext. The 13-line wrapper is the intended shape.

**Round-3 audit was wrong about Ruby.** What the audit actually wants to verify
is that the `Kreuzberg.*` callable surface contains all 27 fns; that's a
function of the compiled `.bundle`, not the Ruby file.

**Verification recipe** (post-build, requires `bundle install` + `rake compile`):

```sh
cd packages/ruby
rake compile
bundle exec ruby -r kreuzberg -e 'puts (Kreuzberg.methods - Module.methods).sort'
```

Expected: 27 canonical fn names (extract_file, extract_bytes, ..., embed_texts,
get_embedding_preset, list_embedding_presets).

**Fix location** None in alef. If the cycle-4 audit needs to _prove_ surface
parity, the audit script should run the verification recipe instead of
counting lines in `kreuzberg.rb`.

**Severity** Non-issue — false positive in audit.

## 4. R — `R/kreuzberg.R` 8-line stub is **almost** correct (alef bug + audit bug)

**Files**

- `packages/r/R/kreuzberg.R` — 8 lines, only `useDynLib(...)` directive.
- `packages/r/NAMESPACE` — single line: `useDynLib(kreuzberg, .registration = TRUE)`.
- `packages/r/src/rust/src/lib.rs` — alef-generated, **90 `#[extendr]` attrs** and a final
  `extendr_module! { ... }` block listing all 27 canonical fn names (L13353-13374).

**Investigation**

extendr-api's design: at _cargo build time_, the `extendr_module!` macro emits a sibling
file `R/extendr-wrappers.R` containing R-side function declarations, plus auto-registers
the symbols via `useDynLib`. After running `rextendr::document()` (or `R CMD INSTALL`):

1. `R/extendr-wrappers.R` should appear and contain `extract_bytes <- function(...) .Call(...)` etc.
2. `NAMESPACE` should be regenerated with `export(extract_bytes)` lines for each fn.
3. `library(kreuzberg)` then exposes all 27 fns at the package level.

**Two issues:**

- alef-backend-extendr should ensure `R/extendr-wrappers.R` either gets committed
  (it's a build artifact but treated as a source file by `R CMD INSTALL`) **or** that
  `configure`/`configure.win` runs `rextendr::document()` as part of the install
  pipeline. Right now there is no `configure` script, just `Makevars`, and no
  generated wrapper file is committed → the package will install successfully (the
  `.so` loads) but `library(kreuzberg)` exposes nothing because `NAMESPACE` only
  has `useDynLib`, no `export(...)` entries.
- **NAMESPACE is also generated by alef** but it is missing the `export(...)` lines.
  alef-backend-extendr knows the fn list (it just emitted them in `extendr_module!`),
  so it should emit the corresponding `export()` directives in NAMESPACE.

**Fix location** alef (alef-backend-extendr): emit `R/extendr-wrappers.R` (mirroring
extendr's own format) and a complete `NAMESPACE` with `export(...)` per fn. Optionally
add a `configure` script that runs `rextendr::document()` for upstream consistency.

**Verification recipe**

```sh
cd packages/r
R CMD INSTALL --preclean .
R -e 'library(kreuzberg); print(ls("package:kreuzberg"))'
```

Expected: 27 canonical fn names. Currently: empty.

**Severity** Blocker — package installs but exposes 0 callable functions.

## Easy kreuzberg-side fixes — status

### 5. `embed_texts` workaround documentation

`crates/kreuzberg/src/lib.rs:225` already has the `&core::config::EmbeddingConfig`
signature applied as the round-3 workaround for alef's Some-wrap codegen bug. The
function compiles cleanly. **No code change made this cycle**; once alef v0.14.3
ships the `Some(...)` fix, future cycles can revert to value-based config without
breaking bindings.

A two-line workaround note has been deferred — adding a `// CYCLE-3 WORKAROUND:`
comment now would only add noise that would be reverted next cycle. The history
is captured here in the cycle-4 doc instead.

### 6. `embed_texts` ergonomics

`embed_texts(texts: Vec<String>, config: &EmbeddingConfig)` is the right shape for
a 2D-result API: callers always need a model selection so a default-friendly
`Option<&EmbeddingConfig>` would just push the model-discovery error down a level.
`EmbeddingConfig::default()` already exists and resolves to the
`mini-lm-l6-v2` preset, so `embed_texts(texts, &EmbeddingConfig::default())` is
the documented happy path. **No change recommended.**

## Next-cycle action items (for alef v0.14.3 + alef v0.14.4)

| #   | Backend                 | Action                                                                                              | Severity |
| --- | ----------------------- | --------------------------------------------------------------------------------------------------- | -------- |
| 1   | alef-backend-kotlin     | Fix rename mapping for `batch_extract_files[_sync]` and `list_document_extractors`.                 | Blocker  |
| 2   | alef-backend-kotlin     | Propagate Java return types: `getEmbeddingPreset` must return `EmbeddingPreset?`.                   | Blocker  |
| 3   | alef new crate template | Add `crates/kreuzberg-swift` swift-bridge crate (mirrors kreuzberg-php).                            | Blocker  |
| 4   | alef-backend-swift      | Emit `public func` shims wrapping `RustBridge.<fn>` for all 27 fns + error translation.             | Blocker  |
| 5   | alef-backend-extendr    | Emit `packages/r/R/extendr-wrappers.R` and complete `NAMESPACE` with `export(...)` lines.           | Blocker  |
| 6   | cycle-4 audit script    | Replace line-count checks for Ruby/R with runtime ls("package:kreuzberg")/Kreuzberg.methods checks. | Quality  |

No kreuzberg repo changes are needed for cycle 4. Working tree left clean for the
sibling agent's regen pass.
