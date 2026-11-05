---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
amends: ["0007"]
---

# Reuse immutable Windows wheels for isolated qualification

## Context and Problem Statement

A clean-userspace guard can fail after a successful Windows wheel build and upload. Rebuilding the same wheel to retry qualification takes almost an hour and can obscure whether the verifier or the wheel changed. The pinned Server Core image also ships `msvcp140_clr0400.dll`, which does not satisfy a wheel import of the ordinary MSVCP DLL.

## Considered Options

* Rebuild the wheel on each qualification attempt.
* Allow a caller to upload or name any wheel.
* Select an immutable wheel from a verified earlier Windows smoke run.

## Decision Outcome

Chosen option: "Select an immutable wheel from a verified earlier Windows smoke run", because the verifier can change independently without accepting an arbitrary input artifact. The default workflow still builds and qualifies its own wheel. An explicit Windows-only run ID selects a single immutable artifact from the same repository's completed main-branch smoke workflow, tied to the exact source SHA and a successful Windows upload step. The download verifies its digest, the host script verifies the verifier checkout, and the evidence records both commits and the wheel hash.

The baseline may record the exact inbox CLR sidecar in System32 or SysWOW64. This exception applies only to the filesystem census. The registry, bare loader, loaded-module origin, and missing ordinary-runtime checks in ADR 0007 still apply.

### Consequences

* Good, because rerunning the isolated test exercises the same wheel without rebuilding it.
* Good, because an unrelated, expired or unproven artifact fails before container launch.
* Bad, because replay requires a retained artifact and GitHub Actions read access.

### Confirmation

`python:wheel:clean-host:test` exercises exact source and verifier identities, wrong artifacts, and the narrow filesystem exception. The same intact and missing-runtime container receipts remain required by ADR 0007.
