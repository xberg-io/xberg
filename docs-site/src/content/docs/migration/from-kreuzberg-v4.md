---
title: Migrating from Kreuzberg v4
description: How Xberg relates to Kreuzberg, what changed in v5, and how to stay on the v4 LTS line.
---

Xberg is the direct continuation of **Kreuzberg**. The Rust core and extraction API are the same
lineage — v5 is the Xberg-branded release line. The Kreuzberg **v4** line continues as a
long-term-support release.

## Backwards compatibility

- **Existing v4 installs keep working.** Kreuzberg v4 packages remain published under their original
  names and are supported as LTS (see below). Nothing you have deployed on v4 stops functioning.
- **Go module pins keep resolving.** Old imports of `github.com/kreuzberg-dev/kreuzberg` continue to
  resolve from the Go module proxy cache. New v4 releases publish at
  `github.com/kreuzberg-dev/kreuzberg-lts/v4`; v5 is `github.com/xberg-io/xberg`.
- **The v4 LTS line is MIT-licensed** (earlier v4 shipped under Elastic License 2.0).

## The v4 LTS line

Kreuzberg v4 is maintained at **[kreuzberg-dev/kreuzberg-lts](https://github.com/kreuzberg-dev/kreuzberg-lts)**
with docs at **[kreuzberg.dev](https://kreuzberg.dev)**. It receives critical bug and security fixes
**until the end of 2026, on a best-effort basis**. No new features land on v4 — feature work happens
in Xberg.

**Stay on v4 LTS if** you depend on the **R binding** (removed in v5 — v4 is the last line to ship it)
or you are not ready to migrate.

## What changed in v5 (Xberg)

Package identifiers moved from `kreuzberg` to `xberg`:

| Ecosystem | v4 (Kreuzberg) | v5 (Xberg) |
|-----------|----------------|------------|
| Rust (crates.io) | `kreuzberg` | `xberg` |
| Python (PyPI) | `kreuzberg` | `xberg` |
| npm | `@kreuzberg/*` | `@xberg-io/xberg` |
| Maven | `dev.kreuzberg:kreuzberg` | `io.xberg:xberg` |
| NuGet | `Kreuzberg` | `Xberg` |
| Packagist | `kreuzberg/kreuzberg` | `xberg-io/xberg` |
| Go module | `github.com/kreuzberg-dev/kreuzberg` | `github.com/xberg-io/xberg/packages/go` |
| R binding | supported | **removed** (use v4 LTS) |

The extraction API is otherwise compatible in shape; update the import/package name and consult the
[API reference](/reference/api-python/) for your language. See the
[installation guide](/getting-started/installation/) for the current package names.
