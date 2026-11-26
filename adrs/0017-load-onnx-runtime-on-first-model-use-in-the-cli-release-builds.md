---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Load ONNX Runtime on first model use in the CLI release builds

## Context and Problem Statement

The Linux and Apple Silicon CLI release binaries linked ONNX Runtime at build
time: pyke's static prebuilt through `ort-bundled`, plus the CoreML framework on
macOS. The dynamic loader and ONNX Runtime's static initialisers then run on
every process start, about 245M instructions and several MB of RSS before any
work, even for text-only extraction that never builds a model session. A
comparable extractor starts in 5.8 ms. How should the CLI obtain ONNX Runtime
so that a run that needs no model does not pay for it?

## Decision Drivers

* A run that builds no model session must not map ONNX Runtime.
* Model features (layout detection, ONNX OCR, embeddings, NER, reranking,
  transcription) keep working from the release archive with no extra install.
* The runtime the CLI loads is located deterministically and is the same
  digest-pinned Microsoft build the other release artifacts ship.
* The language bindings keep their current, already qualified, ONNX Runtime
  linkage.

## Considered Options

* Build the CLI with `ort-dynamic` (ort's `load-dynamic`), ship
  `libonnxruntime` beside the binary, and have xberg resolve and `dlopen` it
  once, on the first model session.
* Keep static linking and defer the cost by lazy initialisation inside xberg.
* Link Microsoft's shared library at build time (`ORT_STRATEGY=system`), as the
  Python wheels and the Windows CLI do.

## Decision Outcome

Chosen option: "Build the CLI with `ort-dynamic`", because it is the only option
that keeps ONNX Runtime out of process start: a static link runs its
initialisers and maps its pages before `main` whatever xberg defers, and a
build-time shared link records a `DT_NEEDED`/`LC_LOAD_DYLIB` entry that the
loader resolves at start.

`ort_discovery::preload_ort_runtime` is the single entry: `ort_environment`
(every xberg session builder, and through it PaddleOCR and GLiNER) and the
Sceptre ORT route call it before any ort API. It resolves the library once,
caches the result, success or failure, and on failure returns an error instead
of letting ort fall back to its own name-based loader search. Resolution order:

1. `ORT_DYLIB_PATH` when set and non-empty, authoritative even when it does not
   load (no fallback past an explicit choice).
2. `libonnxruntime.{so,so.1,dylib}` / `onnxruntime.dll` in the directory of the
   running executable with symlinks resolved, then in `<exe dir>/../lib`.
3. The fixed, previously documented system paths (`/usr/lib`, `/usr/local/lib`,
   the Debian multiarch directories, `/opt/homebrew/lib`, `C:\Program Files\onnxruntime\bin`,
   `C:\Windows\System32`).
4. Otherwise an error naming every path searched.

The Linux glibc and macOS release legs build with `ort-dynamic`, stage
Microsoft's runtime through `setup-onnx-runtime` (SHA-256 pinned in
`checksums.sha256`, re-verified from the cached archive on Linux), copy it into
the archive as `libonnxruntime.so` / `libonnxruntime.dylib`, and fail if the
binary links ONNX Runtime. The `core` and `full` Docker images build with
`ort-dynamic` and already stage the same verified runtime in `/usr/local/lib`,
which is `<exe dir>/../lib` for `/usr/local/bin/xberg`. The benchmark build uses
`ort-dynamic` and the `ORT_DYLIB_PATH` its jobs already export.

Unchanged: the Windows CLI (its archive gates require the `onnxruntime.dll`
import), the musl archives and the Alpine `cli` image (Alpine's `onnxruntime`
package, linked), the `binstall` archives (no ONNX Runtime), the Homebrew
formula (built from source by the tap) and every language binding.

### Trust model

The loaded library has the trust of the executable that loads it. Candidates are
absolute paths only, so neither the current directory nor the loader's search
path (`LD_LIBRARY_PATH`, `DYLD_*`, `PATH`) can inject a library. A library
beside the executable or in its `../lib` is writable by whoever can replace the
executable itself. `ORT_DYLIB_PATH` comes from the environment, which already
controls the process. The system paths are reached only when the archive layout
is broken; `/opt/homebrew/lib` and `/usr/local/lib` are writable by the Homebrew
owner on macOS, which this order keeps behind the shipped library. The release
pipeline verifies the archive digest before the library enters the CLI archive;
nothing re-verifies it at load time, as nothing re-verifies the binary.

### Consequences

* Good, because text-only runs never map ONNX Runtime or CoreML.
* Good, because `ORT_DYLIB_PATH` now selects the runtime the CLI uses (for
  example a CUDA build); with static linking it had no effect.
* Good, because the Linux CLI moves from pyke's prebuilt, which needs glibc
  2.38, to Microsoft's, which the wheels already use.
* Bad, because the archive is one file larger and the binary no longer runs
  model features when copied alone: it reports the missing library and every
  path searched.
* Bad, because a missing or broken library now surfaces at the first model use
  rather than at start.
* Bad, because the npm proxy's download fallback copies only the binary and
  `lib/`, so it drops the library, as it already drops the libheif closure; the
  npm platform packages carry the whole archive directory and are unaffected.

### Confirmation

`crates/xberg/src/ort_discovery/resolution_tests.rs` pins the resolution order
with a temporary release layout. `dynamic_sceptre_tests.rs` proves in a child
process that a broken library and a missing explicit path return typed errors
and never load another candidate. `test_unix_cli_loads_a_shipped_onnx_runtime_instead_of_linking_it`
in `scripts/ci/test_publish_workflow_contracts.py` pins which CLI legs use
`ort-dynamic`, that the runtime is staged after the build with digest
verification, and that the archive ships it. The release job fails when `otool
-L` or `readelf -d` shows `onnxruntime` in the packaged binary.

## Pros and Cons of the Options

### `ort-dynamic` with the runtime shipped beside the binary

* Good, because start-up cost is zero until a model is needed.
* Good, because the runtime is the pinned Microsoft build, replaceable without a
  rebuild.
* Bad, because the archive layout becomes load-bearing for model features.

### Static link with lazy initialisation in xberg

* Good, because the binary stays self-contained.
* Bad, because the static initialisers and the mapping of the linked runtime
  still run at start; xberg cannot defer them.

### Build-time shared link (`ORT_STRATEGY=system`)

* Good, because the loader, not xberg, finds the library through the rpath.
* Bad, because the loader resolves the dependency at process start, so text-only
  runs still load it.

## More Information

* [MADR](https://adr.github.io/madr/)
* ADR-0005 (engine-neutral inference seam over ORT and tract).
* ort `load-dynamic`: `ort::init_from`; `ort-sys` `disable-linking`.
