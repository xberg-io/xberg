---
status: proposed
date: 2026-11-26
decision-makers: ["Na'aman Hirschfeld"]
consulted: []
informed: []
amends: ["0021"]
---

# Give the macOS CLI one malloc magazine from its launchers

## Context and Problem Statement

libmalloc's scalable zone keeps one magazine per CPU, each with its own
metadata, free lists and partly used regions. Running the macOS CLI with
`MallocMaxMagazines=1` over 38 native PDFs (Apple Silicon, 14 CPUs, macOS
27.2, hyperfine medians, byte-identical output) lowered peak RSS on every
document: median -2.5 MB (range -0.8 to -3.6 MB, from a median of 37.4 MB),
phys_footprint median -2.4 MB (from 19.4 MB), instructions median -28 M (from
878 M) and wall time median -1.3 ms (from 58 ms; 36 of 38 documents faster).
Like `MallocLargeCache` in ADR 0021, libmalloc reads the variable while
libSystem initializes, before any code of the program runs. How should the
CLI get it?

## Decision Drivers

* Peak RSS and latency on small native PDFs, where fixed per-process costs
  dominate.
* No fixed cost per run that exceeds the gain.
* A value the user set must win.
* No effect on other platforms, and none on processes the CLI does not own.

## Considered Options

* Re-execute the binary with the variable set.
* Obtain one magazine from inside the process.
* Set the variable in the launchers that exec the binary on macOS.

## Decision Outcome

Chosen option: "Set the variable in the launchers that exec the binary on
macOS", because each launcher already exists and already starts a new process,
so the setting costs nothing there.

Every launcher sets `MallocMaxMagazines=1` in the environment of the binary it
starts, only on macOS and only when the variable is absent from its own
environment; a set value, even an empty one, is passed through unchanged:

* the Python package `xberg-cli` (`cli-proxy/pypi/xberg_cli/downloader.py`,
  `native_env`, used by the bundled-binary path in `cli.py` and by the
  download path in `downloader.run`);
* the npm package `@xberg-io/xberg-cli` (`cli-proxy/npm/bin/xberg.js`,
  `nativeEnv`);
* the agent plugin's MCP launcher (`plugin/.ai-rulez/scripts/mcp-launch.sh`,
  `exec_cli`), for a cached, PATH, Homebrew or downloaded binary. When it
  falls back to npx it leaves the variable unset, so Node runs with libmalloc's
  defaults and the npm launcher sets it for the binary.

The CLI starts no subprocesses at runtime: OCR, PDF and Office extraction run
in-process, and the only `std::process::Command` uses are in tests. The
variable therefore reaches no other program.

Running the binary directly (release archive, `cargo install`, cargo-binstall,
Homebrew on Apple Silicon) keeps libmalloc's defaults.

### Consequences

* Good, because the npm, PyPI and plugin installs get the measured RSS and
  latency gain without a per-run cost: each launcher pays its own interpreter
  start-up already.
* Good, because output is unchanged: only where small blocks come from changes.
* Bad, because a direct invocation of the binary does not get it. The
  Homebrew formula installs the binary itself on Apple Silicon; a
  `write_env_script` wrapper there would add a bash start-up, measured at
  2.1 ms per run (3.2 ms against 1.1 ms for `/usr/bin/true`, hyperfine, 300
  runs), more than the 1.3 ms the setting saves. The Intel formula already
  wraps the binary for `ORT_DYLIB_PATH` and can add it at no cost.
* Bad, because one magazine puts the small allocations of all threads behind
  one magazine lock. The measurement covers extraction runs; `xberg serve` and `xberg mcp`
  under concurrent load were not measured. A user who sees contention there
  sets `MallocMaxMagazines` to another value, which the launchers keep.
* Neutral, because Linux and Windows are unchanged.

### Confirmation

`cli-proxy/pypi/tests/test_native_env.py`, the magazine test in
`cli-proxy/npm/test.mjs` and the `malloc_magazine` tests in
`scripts/tests/bats/plugin_mcp_launch.bats` run each launcher against a
stand-in binary that prints the value it received: `1` on macOS when unset,
the user's value when set (including empty), unset elsewhere, and unset for
npx. To confirm the effect on a release, run a native PDF through the npm or
PyPI launcher and directly under `/usr/bin/time -l`; the launcher run should
peak about 2-3 MB lower.

## Pros and Cons of the Options

### Re-execute the binary with the variable set

* Good, because it covers every way of starting the binary.
* Bad, because a second exec costs about 6.7 ms and 46 M instructions per run
  (measured on `xberg --version`), five times the 1.3 ms the setting saves on
  the median native PDF.

### Obtain one magazine from inside the process

A test program (four and fourteen threads mixing tiny, small and medium
allocations with cross-thread frees) compared the size of libmalloc's metadata
in `vmmap` at fourteen threads, which grows with the number of magazines:

| Variant | Malloc Metadata |
|---|---|
| default | 224K |
| `MallocMaxMagazines=1` in the environment | 128K |
| `setenv` from a priority-101 constructor | 224K |
| `malloc_create_zone(0, 0)` | 416K |
| `malloc_create_zone(0, 0)` with `MallocMaxMagazines=1` | 224K |

* Bad, because the constructor runs after libmalloc has read the variable, as
  ADR 0021 found for `MallocLargeCache`.
* Bad, because a zone from `malloc_create_zone` takes its magazine count from
  the same start-up setting, so routing the global allocator through a private
  zone changes nothing. The public `malloc_zone_t` interface has no setting for
  it, and `malloc_zone_pressure_relief` returns memory after the fact without
  lowering the peak.
* Bad, because the remaining in-process routes replace libmalloc for small
  blocks: a global lock in front of it still selects the magazine by CPU and
  made the test program 7-14 times slower, and mimalloc raised native PDF RSS
  by 2-11 MB (ADR 0021).

### Set the variable in the launchers

* Good, because it is free where a launcher already runs.
* Bad, because it covers only the launchers xberg ships.

## More Information

* ADR 0021, which this record amends: the same start-up constraint for
  `MallocLargeCache`, and the mimalloc measurement.
* Homebrew `Language::Java.overridable_java_home_env` uses the
  `"${VAR:-default}"` form inside `write_env_script` for the same purpose.
* [MADR](https://adr.github.io/madr/)
