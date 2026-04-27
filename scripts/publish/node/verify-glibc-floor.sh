#!/usr/bin/env bash

set -euo pipefail

# Post-build symbol audit for the linux-gnu .node prebuilds. cargo-zigbuild
# 0.22.2 does not pin a glibc minor at link time unless the rust target
# carries a `.<major>.<minor>` suffix — and napi-rs's parseTriple mangles
# such a suffix into the artifact name. So zig will happily emit symbols
# from any glibc up to its own ceiling (2.41 in zig 0.14) if a transitive
# dep references them. We pin the floor here, post-build.
#
# Mirrors the Python pipeline's C23-symbol check at publish.yaml ~697-735,
# extended to also catch GLIBC > 2.28 and any GLIBCXX_* (zig should bundle
# libstdc++ statically; presence of GLIBCXX_* means the packaging changed).

target="${TARGET:?TARGET not set}"

case "$target" in
  x86_64-unknown-linux-gnu) node_file="kreuzberg-node.linux-x64-gnu.node" ;;
  aarch64-unknown-linux-gnu) node_file="kreuzberg-node.linux-arm64-gnu.node" ;;
  *)
    echo "verify-glibc-floor: target $target is not a linux-gnu prebuild — skipping" >&2
    exit 0
    ;;
esac

node_path="crates/kreuzberg-node/artifacts/${node_file}"
if [ ! -f "$node_path" ]; then
  echo "verify-glibc-floor: ${node_path} not found" >&2
  exit 1
fi

if ! command -v objdump >/dev/null 2>&1; then
  echo "verify-glibc-floor: objdump not available on runner" >&2
  exit 1
fi

MAX_FLOOR="GLIBC_2.28"
failed=0

echo "=== Symbol audit: ${node_path} ==="

# Cache objdump output once. The regexes below (GLIBC_<digit>...) are deliberately
# anchored on a digit so non-versioned tags like GLIBC_PRIVATE are not captured.
dynsyms=$(objdump -T "$node_path")

# Max GLIBC version used. We compare against MAX_FLOOR via sort -V; a value
# higher than MAX_FLOOR means a transitive dep pulled in a newer libc symbol
# and silently raised the floor of the prebuild.
max_glibc=$(printf '%s\n' "$dynsyms" | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)*' | sort -uV | tail -1 || true)
if [ -z "$max_glibc" ]; then
  echo "  FAIL: no GLIBC_* version symbols found in ${node_file}."
  echo "  A linux-gnu prebuild should always reference at least one versioned glibc"
  echo "  symbol; an empty result means the audit can't detect floor drift and is"
  echo "  almost certainly looking at the wrong file or a corrupted artifact."
  failed=1
else
  highest=$(printf '%s\n%s\n' "$max_glibc" "$MAX_FLOOR" | sort -V | tail -1)
  if [ "$highest" != "$MAX_FLOOR" ]; then
    echo "  FAIL: ${node_file} requires ${max_glibc} (> ${MAX_FLOOR})"
    echo "  This breaks @kreuzberg/node on RHEL 8 / AlmaLinux 8 / Rocky 8."
    echo "  Likely cause: a Rust dependency has been bumped to a version that"
    echo "  references a newer glibc symbol; revert the bump or stay on this floor."
    failed=1
  else
    echo "  OK: max glibc symbol = ${max_glibc} (≤ ${MAX_FLOOR})"
  fi
fi

# GLIBCXX_* should be empty: zig statically links its bundled libstdc++.
glibcxx=$(printf '%s\n' "$dynsyms" | grep -oE 'GLIBCXX_[0-9]+(\.[0-9]+)*' | sort -uV || true)
if [ -n "$glibcxx" ]; then
  echo "  FAIL: ${node_file} references GLIBCXX symbols:"
  echo "$glibcxx" | sed 's/^/    /'
  echo "  zig is supposed to bundle libstdc++ statically; this means the build"
  echo "  switched off zigbuild or zig's runtime is being shadowed by the host."
  failed=1
else
  echo "  OK: no GLIBCXX_* references (libstdc++ bundled by zig)"
fi

# C23 strtoll/strtoul/etc. variants live in glibc 2.38+ and are emitted by
# GCC 14 when -std=gnu23 is the default. They should never appear under
# zig's clang at -std=gnu17, but verify cheap.
isoc23=$(printf '%s\n' "$dynsyms" | grep -E '__isoc23_' || true)
if [ -n "$isoc23" ]; then
  echo "  FAIL: ${node_file} references C23 glibc helpers:"
  echo "$isoc23" | sed 's/^/    /'
  echo "  This requires glibc ≥ 2.38 and breaks the floor."
  failed=1
else
  echo "  OK: no __isoc23_* references"
fi

if [ "$failed" -eq 1 ]; then
  echo
  echo "Symbol audit FAILED for ${node_file} — refusing to package and publish." >&2
  exit 1
fi

echo
echo "Symbol audit PASSED for ${node_file}."
