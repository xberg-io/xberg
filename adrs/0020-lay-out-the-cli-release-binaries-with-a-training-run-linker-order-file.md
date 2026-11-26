---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
---

# Lay out the CLI release binaries with a training-run linker order file

## Context and Problem Statement

The fat-LTO macOS arm64 CLI is 132 MB. Extracting a one-page native PDF
(`tiny.pdf`) leaves 794 `__TEXT` pages (12.4 MB) resident at exit, 706 of them
in `__TEXT,__text`, spread over 4,810 code pages: the linker places functions in
codegen order, so the few functions a run calls share pages with code it never
calls. Every extraction pays this before it reads a byte of the document, and
liteparse is resident at 3.5-4 MB. How should the release build place the code a
typical run executes on as few pages as possible?

## Decision Drivers

* Fewer resident code pages on every document, without changing what any
  function computes.
* No unstable toolchain flags: the workspace pins stable Rust 1.100.
* Development and `cargo install` builds stay unaffected.
* A stale or mismatched layout must cost only the layout, never a failed or
  incorrect link, and CI must prove the layout was applied.

## Considered Options

* A linker order file generated per release build from an instrumented
  (`-Cprofile-generate`) training run.
* Profile-guided optimisation (`-Cprofile-use`) of the release build.
* A post-link layout optimiser (BOLT on Linux).
* `opt-level = "s"` for crates outside the hot path.
* No change.

## Decision Outcome

Chosen option: "A linker order file generated per release build", because it is
the only option that targets page placement directly, works on stable Rust on
both Mach-O and ELF, and leaves code generation untouched.

`scripts/ci/rust/order_file.py generate` builds `xberg-cli` with
`-Cprofile-generate` added to `RUSTFLAGS`, runs the training corpus in
`scripts/ci/rust/order-file-corpus.txt` (native PDFs in plain and markdown,
DOCX, XLSX, PPTX, HTML and a PNG, all tracked files, OCR off), and writes every
function with a non-zero counter, grouped by the set of runs that touched it
(widest-shared first), as an order file. `crates/xberg-cli/build.rs` passes the
file named by `XBERG_ORDER_FILE` to the linker: ld64 `-order_file` on macOS,
`--symbol-ordering-file` with `--no-warn-symbol-ordering` on Linux through the
toolchain's rust-lld (GNU ld cannot order symbols). `XBERG_LINK_MAP` makes the
link write a map, which `order_file.py verify-map` reads.

The file is generated per build, not committed. Rust symbol names carry the
crate's `-C metadata` hash, which changes with every version, feature set,
profile setting and toolchain, so a committed file would match nothing after
the next bump. Cargo hashes the profile, the LTO setting, the features, the
`--target` kind and the rustc version into `-C metadata`, but not `RUSTFLAGS`,
so an instrumented build with identical cargo arguments produces the release
build's symbol names. The two builds must therefore share every cargo argument
and `CARGO_PROFILE_RELEASE_*` override; the workflow contract tests pin that.

Enabled on the fat-LTO `cli-binaries` legs (macOS arm64 and x86_64, Linux
x86_64 gnu) and the Benchmarks CLI build. Not on the aarch64 gnu leg: thin LTO
promotes local functions with per-module `.llvm.<hash>` suffixes that differ
between the instrumented and release builds. Not on musl (thin LTO, musl-gcc)
or Windows.

### Consequences

* Good, because the functions a document run executes sit on contiguous pages,
  which cuts resident code and page faults on every extraction.
* Good, because only the link changes; every function compiles as before and differs
  only in its placement, so behaviour cannot change.
* Bad, because each enabled leg builds the CLI twice, once instrumented in a
  separate target directory, roughly doubling that leg's compile time.
* Bad, because the standard library's precompiled functions and C dependencies
  are not instrumented and keep their default placement.
* Bad, because the Linux legs link through rust-lld when the order file is set,
  which aarch64 gnu builds otherwise do with GNU ld.

### Confirmation

`order_file.py verify-map` fails the release leg unless at least 10% of the
order file's non-standard-library functions are present in the link map (a
metadata mismatch leaves ~0%, since functions inlined by LTO are legitimately
absent) and 99% of the present ones are laid out in file order.
`test_cli_order_file_is_trained_on_the_release_build_inputs_and_verified` and
`test_benchmark_cli_order_file_is_trained_on_the_benchmark_build_inputs` in
`scripts/ci/test_publish_workflow_contracts.py` pin the enabled legs and the
equality of the LTO setting, target and cargo arguments between the two builds.
`scripts/ci/rust/test_order_file.py` covers profile parsing, ordering and both
map formats. `scripts/perf/text_residency.py` measures resident `__TEXT` pages per
section at process exit on macOS for before/after comparisons.

## Pros and Cons of the Options

### Per-build order file from an instrumented training run

* Good, because it is stable on Rust 1.100 and supported by ld64 and lld.
* Good, because a missing symbol is skipped, so a stale entry costs nothing.
* Bad, because it doubles the enabled legs' compile time.

### Profile-guided optimisation (`-Cprofile-use`)

* Good, because LLVM also splits hot and cold blocks and steers inlining.
* Bad, because it changes generated code, so the release binary's behaviour on
  untrained paths needs requalification, and a profile that drifts from the
  source silently degrades optimisation.
* Bad, because it needs the same instrumented build plus a committed or
  per-build `.profdata`, so it costs at least as much as the order file.

### BOLT

* Good, because it reorders basic blocks and functions after the link,
  including C code and the standard library.
* Bad, because it is Linux/ELF only and does not touch the macOS binaries, and
  needs relocations kept in the binary plus an `llvm-bolt` install in CI.

### `opt-level = "s"` for cold crates

* Good, because the whole binary shrinks.
* Bad, because it lowers throughput for those crates and does not separate hot
  from cold code inside the crates a run uses.

### No change

* Good, because the build stays as it is.
* Bad, because every run keeps paying for code it never calls.

## More Information

* [MADR](https://adr.github.io/madr/)
* ADR-0017 and ADR-0019 (start-up cost of the CLI release builds).
* [rustc book: profile-guided optimisation](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
