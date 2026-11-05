---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Qualify Windows wheels in isolated userspace without global VC runtime

## Context and Problem Statement

The Windows wheel smoke test runs on a build host with a global Visual C++ runtime. That environment can satisfy
missing wheel dependencies and cannot prove that app-local packaging works on a clean host. Issue 2368 needs an
actual native load and PDF extraction without borrowing the build host's registry, runtime DLLs or search paths.

## Considered Options

* Qualify in digest-pinned Server Core userspace with process isolation
* Remove Visual C++ runtimes from the hosted build runner
* Rely on the PE dependency closure audit and the existing hosted smoke test

## Decision Outcome

Chosen option: "Qualify in digest-pinned Server Core userspace with process isolation", because a disposable
container separates the filesystem and registry while using the Windows Server 2022 runner's compatible kernel.
Removing runtime files from a shared build image is fragile; static imports alone do not prove runtime behavior.

Copy only the wheel, qualification scripts, public PDF fixture and repository-pinned mise Python installation.
Allow Python's own app-local VCRUNTIME, but require loaded MSVCP, CONCRT and VCOMP modules to come from the wheel.
Reject a baseline with global runtime registrations or DLLs, or one that can already load MSVCP. Run an intact
wheel's closure, notice and RECORD audits, native load and exact-text PDF extraction. In a separate container,
remove a required wheel-local MSVCP dependency and require both the closure audit and the real native loader to
fail while Python itself still works. Count both cases and retain source, image, interpreter and wheel provenance.

The manual workflow defaults to all existing build platforms. Its Windows-only selector runs the same Windows
build and clean-userspace qualification without waiting for unrelated platform builds.

### Consequences

* Good, because a global runtime cannot silently satisfy a missing wheel-local MSVCP dependency.
* Good, because a real missing-runtime negative control checks that the loader boundary is exercised.
* Bad, because qualification needs Windows container support and a large Server Core image pull.
* Bad, because this qualifies the pinned Server Core/Python environment rather than every Windows release.

### Confirmation

`.github/workflows/ci-python-wheel-smoke.yaml` invokes `python:wheel:clean-host` after building the exact source
wheel. `scripts/ci/qualify-windows-wheel-clean-host.ps1` runs the two disposable containers and verifies receipt
identities and counts. `scripts/ci/windows_wheel_clean_host.py` audits the artifact and records loaded module
paths and hashes after extraction. `python:wheel:clean-host:test` covers portable rejection and provenance guards.

The policy is accepted; actual clean-userspace runtime qualification remains pending until this workflow succeeds
and its receipts are inspected. Local portable guard tests do not establish that Windows runtime result.

## More Information

* [Issue 2368](https://github.com/xberg-io/xberg/issues/2368)
