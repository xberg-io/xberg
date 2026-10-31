#!/usr/bin/env bash
set -euo pipefail

if [ "$(uname -s)" != "Darwin" ]; then
  exit 0
fi

ffi="target/release/libxberg_ffi.dylib"
if [ ! -f "$ffi" ]; then
  exit 0
fi

# `install_name_tool -change` is a silent no-op returning 0 when the old path is
# not among the file's load commands, so the absent-dependency case never needed
# suppressing -- a nonzero exit here is a genuine failure worth surfacing.
install_name_tool -change @rpath/libpdfium.dylib @loader_path/libpdfium.dylib "$ffi"

shopt -s nullglob
for ort in target/release/libonnxruntime*.dylib; do
  install_name_tool -change @rpath/"$(basename "$ort")" @loader_path/"$(basename "$ort")" "$ffi"
done

# The point of this script is that nothing is left resolving through @rpath, which
# a JNI library loaded out of a JAR cannot satisfy. Nothing verified that before,
# so a failed rewrite surfaced as an UnsatisfiedLinkError for users instead.
load_commands="$(otool -L "$ffi")"
residual="$(printf '%s\n' "$load_commands" | tail -n +2 | awk '{print $1}' |
  grep -E '^@rpath/(libpdfium|libonnxruntime)' || true)"
if [ -n "$residual" ]; then
  echo "error: $ffi still resolves these through @rpath after rewriting:" >&2
  printf '%s\n' "$residual" >&2
  exit 1
fi
