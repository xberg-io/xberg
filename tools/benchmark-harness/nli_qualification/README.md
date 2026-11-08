# Experimental NLI qualification accounting

This standalone benchmark tool calls the existing Rust `LocalTextClassifier`.
It records inference accounting, not a model qualification decision. Every plan
and report must contain `qualified: false`. It cannot select product defaults,
publish models, fetch artifacts, train a head, or call a provider.

The separate package avoids enabling the extraction harness's `xberg/full`
dependency for contract tests. It uses the SDK's local configuration, result and
model-identity types, and follows the repository's vendored sysinfo override.

## Commands

Run contract, CLI and CPU parser tests with `mise run nli:test`. The task inherits
Cargo target, job and profile settings from the caller and requires cached Cargo
dependencies. `mise run nli:ci` fetches locked Cargo dependencies, runs all contract
tests with an executed-count guard, compiles the optional runtime, and lints all
leaf targets with warnings denied. The existing focused PR matrix and Linux Rust
CI job run this task. No model artifacts are fetched or loaded by these checks.
Runtime builds additionally need the runtime feature; build them offline using
the committed leaf lockfile:

```sh
CARGO_NET_OFFLINE=true ORT_OFFLINE=1 HF_HUB_OFFLINE=1 cargo build --offline --locked \
  --manifest-path tools/benchmark-harness/nli_qualification/Cargo.toml --features runtime
```

All inference commands require that runtime-enabled binary:

```sh
nli-qualification identity 30000
nli-qualification freeze draft.json frozen.json
nli-qualification run frozen.json SHA256_FROM_FREEZE new-report.json
nli-qualification validate frozen.json SHA256_FROM_FREEZE new-report.json
```

`identity` supplies the runtime object for the draft plan. It binds the actual
executable SHA-256, compiled dependency lockfile, ORT build-info string, CPU, OS
and architecture. Linux CPU identity records available architecture-specific
`/proc/cpuinfo` descriptors as canonical complete core records plus board-wide
identifiers. Logical CPU numbers and duplicate records are excluded; partial
metadata is accepted, but no stable identifiers is an error. This metadata is
not hardware attestation. Rebuilding the executable requires freezing a new plan.
`threads: 0` explicitly means **SDK default**, not zero threads or a measured
effective thread count: the SDK session-thread setter is currently private.
This runner executes requests serially; it makes no isolated-resource claim.

`freeze` validates and writes canonical JSON to a new file, printing its SHA-256.
Both run and validate require that digest independently of the report. Existing
output files are never overwritten. A failed preparation, inference, identity
check or final validation returns an error and produces no successful report.
Preserve failures separately; this tool does not turn them into skipped rows.

## Frozen plan

The schema is `src/types.rs`; `tests/common/mod.rs` is a synthetic contract
example with fake pins, not a usable model or measured evidence. All fields are
required, including defaults in the SDK configuration after freezing.

- `schema_version: 1`, `qualified: false`, `repeats` and
  `expected_inferences = documents × candidates × arms × repeats`.
- `labels` is the caller vocabulary. `arms` contains `fp32`, `int8` or `uint8`.
  Each candidate must declare every arm exactly once in `variants`.
- Documents have opaque `id` and `family`, `training`, `calibration` or `held_out`
  split, source-document SHA-256, exact UTF-8 `text`, its SHA-256, and `gold`
  labels. An empty gold set represents an unknown-class example.
- Each variant binds the upstream source revision, export-recipe and license
  receipt SHA-256, and the complete `LocalClassificationConfig`. Its immutable
  model revision and checksum manifest bind the graph, external tensors and all
  four tokenizer/configuration companions. No model preset is supplied.
- Provenance binds corpus-licensing/training-overlap and split/leakage audit
  receipts by SHA-256. These references freeze the declared audits; this tool
  does not authenticate their contents or prove that a declared family is correct.

Only document text and the caller vocabulary reach inference. IDs, family,
source metadata, split and gold labels are never model inputs. Matching family,
source hash or text hash across splits is refused, as are conflicting family
assignments for the same source or text. This cannot detect undeclared near
duplicates or upstream training overlap; those require the independent audit.

Reports contain one completed row per exact Cartesian key, measured call
duration, accepted labels (possibly an abstention), and the SDK's effective
model identity. Empty, missing, duplicated, unexpected or skipped work fails
validation. A result with a different artifact, backend, runtime, plan, invalid
probability or label also fails. Synthetic report files can exercise the
validator. Validation proves accounting consistency, not that inference ran:
rows, timings and `runner_sha256` are self-reported and can be hand-written.
Only executing the runner calls the SDK; a successful `validate` command does
not authenticate that execution. `qualified: false` remains mandatory until a
separate measured qualification process exists.

## Remaining qualification inputs

Actual runs need a trusted pre-staged complete model set and license/export
receipts, independently labelled source-family cohorts and leakage audits, and
the intended native CPU runtime. Preparation verifies cached artifacts through
the SDK and does not repair missing assets from the network.

This increment does not compute macro-F1, calibration, unknown rejection,
confidence intervals, peak RSS or cold-load statistics. It does not establish
quantization parity merely from an arm name. Pilot budgets, untouched held-out
assessment, measured resource isolation, complete scoring and downstream binding
qualification remain required by #2334. No model has been qualified by adding
this tool.
