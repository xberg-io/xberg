#!/usr/bin/env bash
# Reject a macOS PHP PIE archive whose extension imports a library the archive
# does not ship. PIE copies only xberg.so, so every import must be a base-system
# path; a Homebrew libheif path makes the extension fail to load (xberg-io/xberg#2547).
# The HEIF stack is linked statically, so no libheif or codec dylib may be imported at
# all, and every load command must declare the macOS floor (default 15.0, from
# MACOSX_DEPLOYMENT_TARGET). The upstream license notices must ship in the archive.
set -euo pipefail

artifact="${1:?usage: verify-php-macos-extension.sh <php-archive.tgz>}"
floor="${MACOSX_DEPLOYMENT_TARGET:-15.0}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
# A dylib lists its own install name as a row, which is not an import. A bundle has none.
self="$(otool -D "$extension" | sed -n '2p')"
if [ -n "$bad" ] && [ -n "$self" ]; then
  bad="$(printf '%s\n' "$bad" | grep -v -x -F -- "$self" || true)"
fi
if [ -n "$bad" ]; then
  log "xberg.so imports libraries the PIE archive does not ship:"
  printf '%s\n' "$bad" >&2
  exit 1
fi
imported="$rows"
if [ -n "$self" ]; then
  imported="$(printf '%s\n' "$rows" | grep -v -x -F -- "$self" || true)"
fi
codec="$(printf '%s\n' "$imported" | grep -i -E 'heif|de265|aom|dav1d|x26[45]|openh264|rav1e|svt' || true)"
if [ -n "$codec" ]; then
  log "xberg.so imports a HEIF or codec library; it must be linked statically:"
  printf '%s\n' "$codec" >&2
  exit 1
fi

# shellcheck source=../lib/macho.sh
source "$SCRIPT_DIR/../lib/macho.sh"
slices="$(macho_minos_by_arch "$extension")" || die "xberg.so has no valid macOS deployment floor"
while read -r arch actual; do
  # Older floors are fine, a newer one makes the extension fail to load on the supported macOS.
  if [ "$(printf '%s\n%s\n' "$floor" "$actual" | sort -V | tail -n 1)" != "$floor" ]; then
    die "xberg.so slice $arch targets macOS $actual, above the $floor floor"
  fi
done <<<"$slices"

# The statically linked LGPL libraries (libheif, libde265) require their license texts and
# the pinned source inventory to travel with the binary. ~keep
extension_dir="$(dirname "$extension")"
for notice in LICENSE THIRD_PARTY_LICENSES.md sources.tsv notices.sha256; do
  [ -s "$extension_dir/$notice" ] || die "the archive does not ship $notice"
done
[ "$(wc -l <"$extension_dir/sources.tsv" | tr -d ' ')" = 3 ] || die "sources.tsv must list exactly libheif, libde265 and libaom"
for source in libheif libde265 libaom; do
  grep -q "^$source " "$extension_dir/sources.tsv" || die "sources.tsv does not pin $source"
done
if grep -q -i "x265" "$extension_dir/sources.tsv"; then
  die "sources.tsv lists x265; GPL code must not be linked into the extension"
fi
(cd "$extension_dir" && shasum -a 256 -c notices.sha256 >/dev/null) || die "notices.sha256 does not match the shipped notices"
log "checked $checked imports; all are base-system libraries, no codec imports, floor $floor"
