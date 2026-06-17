#!/usr/bin/env bash
set -euo pipefail

rid="${RID:-${1:-}}"
if [ -z "$rid" ]; then
  echo "Usage: $0 <rid> (or set RID)" >&2
  exit 2
fi

native_dir="runtimes/${rid}/native"
if [ ! -d "$native_dir" ]; then
  echo "ERROR: Native assets directory not found: $native_dir" >&2
  exit 1
fi

file_count="$(find "$native_dir" -type f | wc -l | tr -d ' ')"
if [ "$file_count" -eq 0 ]; then
  echo "ERROR: No native asset files found in $native_dir" >&2
  ls -la runtimes/ || true
  exit 1
fi

echo "Found $file_count native asset files"
