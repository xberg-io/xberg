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
  linkage, and the Intel macOS bindings, which already load it dynamically,
  keep finding it where they did.

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
Sceptre ORT route call it before any ort API. It resolves the library once and
caches the result, success or failure, so a failure is returned as an error
instead of reaching ort, whose next API call would `dlopen` its default name
and panic. Resolution order:

1. `ORT_DYLIB_PATH` when set and non-empty, authoritative even when it does not
   load (no fallback past an explicit choice).
2. `libonnxruntime.{so,so.1,dylib}` / `onnxruntime.dll` in the directory of the
   image that holds xberg, symlinks resolved, then in `<prefix>/lib` when that
   directory is `<prefix>/bin`.
3. The fixed, previously documented system paths (`/usr/lib`, `/usr/local/lib`,
   the Debian multiarch directories, `/opt/homebrew/lib`, `C:\Program Files\onnxruntime\bin`,
   `C:\Windows\System32`).
4. Otherwise an error naming every path searched and every file skipped.

A discovered file that exists but does not load is skipped for the next
candidate; the first such failure is reported when none loads.

Two policies share this order. The release CLI enables `ort-strict-discovery`
(through its `ort-dynamic` feature): the image is the running executable,
`ORT_DYLIB_PATH` must be absolute, and nothing else is consulted. A library
consumer without that feature, such as the Intel macOS language bindings
(`macos-intel-target`), runs inside an interpreter whose executable says nothing
about xberg, so the image is the shared library that holds xberg, found with
`dladdr` (falling back to the executable), a relative `ORT_DYLIB_PATH` keeps
ort's meaning (beside the executable, then the loader), and after step 3 the
bare library name goes to the platform loader, which searches
`LD_LIBRARY_PATH`, `DYLD_LIBRARY_PATH`, `DYLD_FALLBACK_LIBRARY_PATH`, rpaths
and `PATH` as ort's own loader did before this change.

The Linux glibc and macOS release legs build with `ort-dynamic`, stage
Microsoft's runtime through `setup-onnx-runtime` (SHA-256 pinned in
`checksums.sha256`, re-verified from the cached archive on Linux), copy it into
the archive as `libonnxruntime.so` / `libonnxruntime.dylib`, and fail if the
binary links ONNX Runtime. The `core` and `full` Docker images build with
`ort-dynamic` and already stage the same verified runtime in `/usr/local/lib`,
which is `<prefix>/lib` for `/usr/local/bin/xberg`. The benchmark build uses
`ort-dynamic` and the `ORT_DYLIB_PATH` its jobs already export.

Unchanged: the Windows CLI (its archive gates require the `onnxruntime.dll`
import), the musl archives and the Alpine `cli` image (Alpine's `onnxruntime`
package, linked), the `binstall` archives (no ONNX Runtime), the Homebrew
formula (built from source by the tap) and the language bindings that link ONNX
Runtime. The Intel macOS bindings already loaded it dynamically; they now also
search beside the binding library and skip untrusted discovered files, and
still reach the loader's search path last.

### Trust model

A discovered library runs with the privileges of the process that loads it, so
a discovered file is loaded only when it, and the directory that holds it,
both with symlinks resolved, are owned by root or the effective user and are
not writable by others, nor by a group other than the user's private group;
the resolved path is what gets loaded. A private group (named after the user,
the user's primary group, no listed members) is how Debian, Ubuntu and Fedora
pair umask 002 with safety, and pip, npm or `cp` there leave the library
group-writable. This
matters because the executable's directory says nothing about who controls its
neighbours: a binary copied alone to `/tmp/x/xberg` would otherwise search
`/tmp/lib`, which any local user can create. For the same reason `<prefix>/lib`
is searched only for an executable in a directory named `bin`. Ancestor
directories are not checked; a shared one such as `/tmp` is sticky, so another
user cannot rename the directories below it. Windows has no owner check; no
Windows build xberg ships loads ONNX Runtime dynamically.

Under `ort-strict-discovery` every candidate is absolute and owner-checked, so
neither the current directory nor the loader's search path (`LD_LIBRARY_PATH`,
`DYLD_*`, `PATH`) can supply the library, and a relative `ORT_DYLIB_PATH` is
refused rather than resolved against the current directory. `ORT_DYLIB_PATH`
itself is not owner-checked: it comes from the environment, which already
controls the process. The system paths are reached only when the archive
layout is broken; `/opt/homebrew/lib` and `/usr/local/lib` belong to the
Homebrew owner on macOS, which this order keeps behind the shipped library, and
a group-writable Homebrew prefix is skipped. Library consumers end with the
loader's search path, as they did before; whoever sets those variables already
controls the process. The release pipeline verifies the archive digest before
the library enters the CLI archive; nothing re-verifies it at load time, as
nothing re-verifies the binary.

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

`crates/xberg/src/ort_discovery/resolution_tests.rs` pins the resolution order,
both policies, the `bin`-only `<prefix>/lib` rule and the owner check with a
temporary release layout. `dynamic_sceptre_tests.rs` proves in a child process
that a missing explicit path returns a typed error and never loads another
candidate, that a broken candidate is skipped for a working one, and that a
library consumer finds the runtime through `LD_LIBRARY_PATH` /
`DYLD_LIBRARY_PATH`. `test_unix_cli_loads_a_shipped_onnx_runtime_instead_of_linking_it`
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
