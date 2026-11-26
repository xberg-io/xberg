---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Open libheif on first HEIF decode in the Unix CLI release builds

## Context and Problem Statement

The Linux gnu and macOS CLI release binaries link libheif, which links libde265
and libaom. dyld and ld.so map the three libraries, validate their signatures
and bind their imports on every process start, also for a PDF that never
touches HEIF. On Apple Silicon that costs 13.6M of the 72M instructions of
`xberg --version`, about 0.9 ms of wall time, measured by linking the same
dylib set into an empty program with and without libheif. How should the CLI
obtain libheif so that a run that decodes no HEIF image does not pay for it?

## Decision Drivers

* A run that decodes no HEIF image must not map libheif or its codecs.
* HEIC, HEIF and AVIF keep working from the release archive with no extra install.
* libheif and libde265 stay separately replaceable shared libraries (LGPL-3.0).
* The language bindings, Docker images, Homebrew formula and Windows CLI keep
  their current, already qualified, libheif linkage.

## Considered Options

* Build the CLI with `heic-dynamic`: xberg-libheif calls libheif through
  wrappers resolved with `dlopen`/`dlsym` on first use, the build suppresses
  `libheif-sys`'s link request, and the archive ships the libheif closure beside
  the binary.
* Build libheif with its codecs as plugins, so only libheif loads at start.
* Link libheif's codecs statically into one libheif dylib.

## Decision Outcome

Chosen option: "Build the CLI with `heic-dynamic`", because it is the only
option that keeps all three libraries out of process start. Plugins still map
libheif, and xberg pairs `heif_init` with `heif_deinit` per image, which would
load and unload the codec plugins per decode. A merged dylib still loads at
start.

`xberg-libheif`'s `dynamic` feature routes every libheif call through
`crate::lh`, which with the feature is `src/dynamic.rs`: the types and
constants of `libheif-sys` and a wrapper with the `libheif-sys` signature for
each of the 173 functions the crate calls. The wrappers shadow the linked
declarations, so the binary references no libheif symbol. The first entry point
that needs libheif loads it once per process and caches the outcome. Every
public entry point that reaches libheif without a context or an image checks
that outcome first, so a missing library is a `PluginLoadingError` (or an empty
list, `None`, or `FileTypeResult::No` where the signature has no error), never
a panic; `LibHeif::new` skips `heif_init`. `xberg_libheif::ensure_available`
exposes the outcome, and xberg reports it as a missing dependency rather than a
malformed file. Resolution mirrors ADR-0017's strict policy:

1. `XBERG_LIBHEIF_PATH` when set and non-empty, the only candidate; a relative
   value is refused.
2. `libheif.1.dylib`/`libheif.dylib` or `libheif.so.1`/`libheif.so` in the
   directory of the running executable with symlinks resolved, then in
   `<prefix>/lib` when that directory is `<prefix>/bin`.
3. Fixed system paths (`/opt/homebrew/lib`, `/usr/local/lib`, the Debian
   multiarch directories, `/usr/lib64`, `/usr/lib`).
4. Otherwise an error naming every path searched and every file skipped. A
   discovered candidate that fails the owner check, does not load, or lacks one
   of the functions is skipped for the next; the first load failure is
   reported when none loads.

The feature is for executables that ship libheif beside themselves; no language
binding enables it, so there is no fallback to the platform loader's search
path.

`libheif-sys` still emits `cargo:rustc-link-lib=heif`. Linux's `--as-needed`
drops an unused entry, but macOS records every `-l` as a load command. The
release legs therefore add a build-script override for the `heif` links key,
`[target.<triple>.heif] rustc-link-lib = []`, to the runner's cargo home before
the build. The package step fails if `otool -L`/`readelf -d` shows libheif,
copies the libheif that `pkg-config` resolves (the one `libheif-sys` built
against) beside the binary, vendors its codec closure with the existing
`vendor-native-closure` scripts, and decodes the upstream example HEIC with the
staged binary.

Unchanged: the Windows CLI archives, the musl and `binstall` archives (no
HEIC), the Docker images, the Homebrew formula and every language binding.

### Trust model

As ADR-0017: candidates other than the explicit override are absolute paths,
so neither the current directory nor `DYLD_*`/`LD_LIBRARY_PATH` can supply the
library, and a discovered file is opened, from its resolved path, only when it
and its directory are owned by root or the effective user and not writable by
others or by a group other than the user's private group.
`dynamic::trusted_library` and xberg's
`ort_discovery::library::trusted_library` implement the same check.

### Consequences

* Good, because PDF, Office and text runs never map libheif, libde265 or libaom.
* Good, because `XBERG_LIBHEIF_PATH` selects the libheif the CLI uses.
* Bad, because a missing or broken libheif now surfaces at the first HEIF
  decode rather than at start.
* Bad, because a `xberg-libheif` call to a libheif function without a wrapper
  fails to link on macOS release builds instead of compiling; the
  `dynamic` test leg catches it on Linux and macOS CI first.
* Bad, because `SecurityLimits::default` still panics in a `dynamic` build when
  the library is missing, and `check_file_type` answers `No`; xberg calls
  neither before a context.

### Confirmation

`crates/xberg-libheif/src/dynamic.rs` tests pin the resolution order, the
`bin`-only `<prefix>/lib` rule, the refused relative override and the owner
check, that a missing path reports every path searched, that a broken library
is skipped for a working one, that the installed libheif exports every wrapped
function, and, in a child process with no loadable library, that the public
entry points return errors instead of panicking. `scripts/ci/rust/run-unit-tests.sh` runs the xberg-libheif suite with
`--features dynamic` on Linux and macOS. `test_unix_cli_opens_a_shipped_libheif_instead_of_linking_it`
in `scripts/ci/test_publish_workflow_contracts.py` pins the feature on exactly
the Unix legs, the link override before the build, the link check, the shipped
library and the smoke test. `scripts/ci/smoke-cli-heif.sh` decodes
`example.heic` from the staged archive and requires a run with
`XBERG_LIBHEIF_PATH` at a missing file to fail with the library-search error.

## Pros and Cons of the Options

### `heic-dynamic` with libheif shipped beside the binary

* Good, because start-up cost is zero until a HEIF image is decoded.
* Bad, because 173 wrapper signatures are copied from `libheif-sys`, and the
  release build depends on a cargo build-script override.

### libheif codecs as plugins

* Good, because libheif's own plugin loader does the `dlopen`.
* Bad, because libheif itself still loads at start, and per-image
  `heif_init`/`heif_deinit` would reload the plugins on every decode.

### Codecs linked statically into libheif

* Good, because one library replaces three.
* Bad, because that library still loads at start.

## More Information

* [MADR](https://adr.github.io/madr/)
* ADR-0016 (Windows libheif decoders linked dynamically for LGPL-3.0).
* ADR-0017 (ONNX Runtime on first model use in the CLI release builds).
