#!/usr/bin/env bash
set -euo pipefail
archive="${1:?archive path required}"
checksums="${2:?checksum manifest required}"
name="${archive##*/}"
expected="$(awk -v name="$name" '$2 == name { print $1 }' "$checksums")"
if [[ ! "$expected" =~ ^[a-f0-9]{64}$ ]]; then
  printf 'Expected exactly one pinned SHA-256 for %s\n' "$name" >&2
  exit 1
fi
if [ "$#" -eq 3 ] && [ "$3" = --known ]; then exit 0; fi
if [ "$#" -ne 2 ]; then
  echo "usage: verify-archive ARCHIVE CHECKSUMS [--known]" >&2
  exit 2
fi
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$archive")"
else
  actual="$(shasum -a 256 "$archive")"
fi
actual="${actual%% *}"
if [ "$actual" != "$expected" ]; then
  printf 'SHA-256 mismatch for %s: expected %s, observed %s\n' "$name" "$expected" "$actual" >&2
  exit 1
fi
printf '%s: SHA-256 verified\n' "$name"
