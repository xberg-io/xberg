#!/usr/bin/env bash

set -euo pipefail

target="${TARGET:?TARGET not set}"
use_cross="${USE_CROSS:-false}"
use_napi_cross="${USE_NAPI_CROSS:-false}"
use_zigbuild="${USE_ZIGBUILD:-false}"

echo "=== Building Node Native Module ==="
echo "Target: $target"
echo "Use cross: $use_cross"
echo "Use napi-cross: $use_napi_cross"
echo "Use zigbuild: $use_zigbuild"

args=(--platform --release --target "$target" --output-dir ./artifacts)
if [ "$use_napi_cross" = "true" ]; then
  args+=(--use-napi-cross)
fi
if [ "$use_cross" = "true" ]; then
  args+=(--use-cross)
fi
if [ "$use_zigbuild" = "true" ]; then
  args+=(--cross-compile)
  # openssl-sys's build script emits a single `-I/usr/include` for the expando
  # probe. Ubuntu's multilib layout splits the arch-independent openssl headers
  # at /usr/include/openssl/ from the arch-specific ones at
  # /usr/include/<triplet>/openssl/. Under zig's sysroot, the second path isn't
  # picked up automatically, so the probe fails to find opensslconf.h. Symlink
  # the arch-specific headers into /usr/include/openssl/ so -I/usr/include works.
  case "$target" in
  x86_64-unknown-linux-gnu) triplet=x86_64-linux-gnu ;;
  aarch64-unknown-linux-gnu) triplet=aarch64-linux-gnu ;;
  *) triplet="" ;;
  esac
  if [ -n "$triplet" ]; then
    for h in opensslconf.h configuration.h; do
      src="/usr/include/$triplet/openssl/$h"
      dst="/usr/include/openssl/$h"
      if [ -f "$src" ] && [ ! -e "$dst" ]; then
        if ! ln -sf "$src" "$dst" 2>/dev/null && ! sudo ln -sf "$src" "$dst" 2>/dev/null; then
          echo "warning: could not symlink $src -> $dst; openssl-sys probe may fail" >&2
        fi
      fi
    done
  fi
fi

echo "Running: pnpm --filter '{./crates/kreuzberg-node}' exec napi build ${args[*]}"
pnpm --filter '{./crates/kreuzberg-node}' exec napi build "${args[@]}"

artifacts_dir="crates/kreuzberg-node/artifacts"
echo ""
echo "=== Build Output ==="
ls -lah "$artifacts_dir" 2>/dev/null || echo "Artifacts directory not found!"
echo "=== Checking for .node files ==="
find "$artifacts_dir" -name "*.node" -print 2>/dev/null || echo "No .node files found!"

# Verify that at least one .node file was created
node_files=$(find "$artifacts_dir" -name "*.node" 2>/dev/null | wc -l)
if [ "$node_files" -eq 0 ]; then
  echo "ERROR: Native module build succeeded but no .node file was generated" >&2
  echo "Expected to find .node files in $artifacts_dir" >&2
  exit 1
fi
echo "✓ Found $node_files .node file(s)"
