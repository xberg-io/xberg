#!/usr/bin/env bash
# Reject a macOS PHP PIE archive whose extension imports a library the archive
# does not ship. PIE copies only xberg.so, so every import must be a base-system
# path; a Homebrew libheif path makes the extension fail to load (xberg-io/xberg#2547).
set -euo pipefail

artifact="${1:?usage: verify-php-macos-extension.sh <php-archive.tgz>}"

log() { echo "verify-php-macos-extension: $*" >&2; }
die() {
  log "$*"
  exit 1
}

[ -f "$artifact" ] || die "artifact does not exist: $artifact"
command -v otool >/dev/null 2>&1 || die "otool is required"

workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

case "$artifact" in
*.tar.gz | *.tgz) tar -xzf "$artifact" -C "$workdir" ;;
*) die "unsupported PHP macOS artifact: $artifact" ;;
esac

extension_count="$(find "$workdir" -type f -name 'xberg.so' | wc -l | tr -d ' ')"
[ "$extension_count" = 1 ] || die "expected exactly one xberg.so, found $extension_count"
extension="$(find "$workdir" -type f -name 'xberg.so' -print -quit)"

# Everything after the extension's own install-name row is a load command.
imports="$(otool -arch all -L "$extension")" || die "xberg.so is not a readable Mach-O file"
rows="$(printf '%s\n' "$imports" | sed -n 's/^[[:space:]]*\(.*\) (compatibility version.*/\1/p')"
[ -n "$rows" ] || die "no dynamic-library imports parsed from xberg.so; the gate examined nothing"
checked="$(printf '%s\n' "$rows" | wc -l | tr -d ' ')"

bad="$(printf '%s\n' "$rows" | grep -v -E '^(/usr/lib/|/System/Library/)' || true)"
if [ -n "$bad" ]; then
  # The extension's own install name is also a row; it is only acceptable when relative.
  self="$(otool -D "$extension" | tail -n +2 | head -1)"
  bad="$(printf '%s\n' "$bad" | grep -v -x -F -- "$self" || true)"
fi
if [ -n "$bad" ]; then
  log "xberg.so imports libraries the PIE archive does not ship:"
  printf '%s\n' "$bad" >&2
  exit 1
fi
log "checked $checked imports; all are base-system libraries"
