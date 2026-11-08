#!/usr/bin/env bash
# Add the license texts and pinned-source inventory of the statically linked HEIF stack
# (libheif and libde265 are LGPL-3.0, libaom is BSD-2-Clause) to a macOS PHP PIE archive,
# then refresh its checksum sidecar. PIE installs only xberg.so; the notices travel in the
# archive for anyone who downloads it (xberg-io/xberg#2547).
set -euo pipefail

archive="${1:?usage: stage-php-macos-notices.sh <php-archive.tgz> <heif-prefix> <project-root>}"
prefix="${2:?usage: stage-php-macos-notices.sh <php-archive.tgz> <heif-prefix> <project-root>}"
root="${3:?usage: stage-php-macos-notices.sh <php-archive.tgz> <heif-prefix> <project-root>}"

log() { echo "stage-php-macos-notices: $*" >&2; }
die() {
  log "$*"
  exit 1
}

[ -f "$archive" ] || die "archive does not exist: $archive"
inventory="$prefix/share/xberg-heif-notices"
[ -s "$inventory/sources.tsv" ] || die "no staged codec notices under $inventory"
case "$archive" in
*.tar.gz | *.tgz) ;;
*) die "unsupported PHP macOS artifact: $archive" ;;
esac

archive="$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")"
workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

tar -xzf "$archive" -C "$workdir"
[ -f "$workdir/xberg.so" ] || die "xberg.so is not at the archive root"

cp "$inventory"/* "$workdir/"
for notice in LICENSE THIRD_PARTY_LICENSES.md; do
  [ -s "$root/$notice" ] || die "missing project notice $root/$notice"
  cp "$root/$notice" "$workdir/$notice"
done

entries=()
while IFS= read -r entry; do
  entries+=("$entry")
done < <(cd "$workdir" && find . -mindepth 1 -maxdepth 1 -type f -exec basename {} \; | sort)
[ "${#entries[@]}" -ge 15 ] || die "refusing to repack ${#entries[@]} entries"

rm -f "$archive"
COPYFILE_DISABLE=1 tar -czf "$archive" -C "$workdir" "${entries[@]}"
checksum="$(shasum -a 256 "$archive" | awk '{print $1}')"
printf '%s  %s\n' "$checksum" "$(basename "$archive")" >"$archive.sha256"
log "staged codec notices beside xberg.so (${#entries[@]} archive entries, sha256 $checksum)"
