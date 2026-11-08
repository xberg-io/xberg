# Verification record for the #2334 accounting increment

Base: `c962c7be61`, on branch `codex/2334-nli-qualification-contract`.
This record covers contract/CLI behavior and runtime type checking. It contains
no measured model quality, CPU qualification result, default selection or
permission to publish artifacts.

## Decisions

- Keep an independent leaf workspace under the benchmark-harness directory,
  following the caption exporter precedent. The existing harness enables
  `xberg/full`; contract checks need only the SDK's portable local NLI DTOs.
- Use the existing `LocalTextClassifier` constructor, explicit `prepare`, and
  `classify` methods in the optional runtime. Do not implement tokenization,
  sessions, scoring or inference again.
- Bind canonical plan SHA-256 independently of a report, including input hashes,
  labels, split/family identity, provenance receipts, candidate artifacts,
  hypothesis template, thresholds, limits and runtime identity.
- Keep `qualified=false` mandatory. SDK-default threading is recorded as zero;
  no effective-thread-count or isolated-resource claim follows from that value.
- Leave shared workspace Cargo files, main harness CLI, layout selectors,
  generated bindings and model publishing untouched.

## Observed checks

All compilation used one job, debug information disabled, incremental builds
disabled and `/Users/naamanhirschfeld/workspace/xberg-io/xberg/target/codex2334`.
Cargo and ORT offline flags were set; no model inputs were acquired or loaded.
Disk availability was checked above the authorized 40 GB floor: initially
266 GiB and over 230 GiB near completion.

| Check | Observed result |
| --- | --- |
| Initial permissive validation stub | The collected empty-work/count test failed because validation returned `Ok(())` |
| SDK admission regression before correction | One collected test failed at `SDK must reject /limits/max_tokens` |
| Impossible-abstention regression before correction | One collected test failed because an impossible empty result validated successfully |
| `mise run nli:test` after corrections | 15 integration tests passed: 13 contract tests and 2 CLI tests; none ignored |
| Optional runtime `cargo check --offline --locked --features runtime` | Passed after corrections using the leaf manifest |
| Scoped `poly lint` | 12 files checked, zero findings, errors or skipped files |
| Scoped `poly fmt` | All 12 files formatted |
| `mise tasks validate` | All 143 tasks valid |
| Offline locked Cargo metadata | Leaf workspace has exactly one member; root workspace has 19 members and excludes the leaf |
| Shared-file diff check | Root Cargo.toml/Cargo.lock and existing harness Cargo.toml/main.rs unchanged |

Contract controls include missing/duplicate/unexpected/skipped inference,
unknown backend, corrupt input hashes, missing artifact companions, nested
duplicate JSON keys, runtime/model/plan identity mismatch, and split/family
leakage. Negative tests check intended error categories. A positive Cartesian
control contains exactly 2 documents × 2 candidates × 2 arms × 2 repeats = 16
rows; replacing its last row with the first fails as a duplicate.

The CLI control freezes a real temporary plan file, validates a matching report,
refuses overwrite and rejects a wrong externally supplied plan digest. These
fixtures contain synthetic predictions and fake artifact pins; they are not
inference evidence.

## Independent review and commit hooks

The first independent read-only review found two material gaps: SDK-invalid
configuration/request bounds could pass freeze, and mathematically impossible
abstentions could pass result validation. Both received failing regression tests
before correction. A fresh independent review of the corrected staged diff
reported no findings.

The installed commit hook attempted workspace-wide Clippy. That commit's own
process tree was stopped; other agents' processes were left alone. Scoped
lint/format checks were rerun successfully. The local commit uses the repository's
subagent exception for hooks that would compile; the workspace-wide hook is not
claimed as passed.

## Inputs and work still needed for measured qualification

Supply trusted pre-staged model/tokenizer/configuration/external-weight sets,
immutable artifact and source revisions, license/export receipts, independent
source-family corpus labels and split/leakage/training-overlap audits. Pilot
quality/resource budgets must be frozen before untouched held-out assessment.
CPU/runtime isolation, effective thread verification, scoring/calibration,
unknown rejection, paired confidence intervals, cold-load/RSS measurements and
downstream binding qualification remain separate work. The runtime runner was
type-checked, not exercised against a real model or qualified corpus.

## Linux ARM identity follow-up

Review of `0fbf497332045da9941917b25b6f92c7ddca66f2` identified that
Linux CPU identity required `model name`, absent on common ARM systems. The
original model-name-only expression was extracted unchanged into a pure parser:
three collected tests yielded two passes and the expected ARM failure, `CPU
identity is unavailable`, before the correction.

The parser now retains available x86 or ARM descriptors and board identifiers,
failing only when none are available. Seven CPU tests cover x86, ARM architecture
aliases, partial ARM information, missing/empty information, exact key matching,
record ordering/deduplication and heterogeneous core descriptor associations.
An independent delta reviewer found that the initial field-wise sets lost
part/revision associations. A new one-test negative control failed with identical
identities for swapped revisions before correction; complete per-core records
now retain those associations separately from platform identifiers.

After correction, `mise run nli:test` collected and passed 22 integration tests
(13 contract, 2 CLI, 7 CPU), with none ignored. No model was loaded and none of
these synthetic parser fixtures constitutes measured qualification.

Fetched and merged upstream main `407ccf3d20e0343dfa504621bf13d0072dd012b3`
without conflicts in merge commit `bb49c1f9433194eae9e8b988c22fb2a39123667e`.
The merge hook failed because the installed poly hook loader required an
`ai-rulez` source lock entry; no governance or peer files were changed to repair
it. The merge commit used a process-local hook bypass. Scoped checks remain the
authorized verification, not a claim that the shared hooks passed. Disk space
before this follow-up remained above 178 GiB, exceeding the 40 GB floor.

The final optional runtime check passed with `cargo check --offline --locked`
and `--features runtime` against the leaf manifest, using the same dedicated
jobs=1/debug=0 target. Scoped lint examined all four changed Rust files with zero
findings/errors/skips; format verification examined all six changed files.
A fresh independent read-only review of the corrected staged delta, runtime
caller and regression oracles reported no material findings. The follow-up
commit uses the same process-local hook bypass for the unresolved shared hook
lock issue; no shared configuration repair or broad compilation is claimed.
