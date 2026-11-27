#!/usr/bin/env bash
# Decode the upstream libheif example HEIC with a staged heic-dynamic CLI (ADR-0019), so an
# archive whose libheif is missing or does not load fails before upload. The binary does not
# link libheif; it opens the copy shipped beside it on the first HEIF decode. A run with
# XBERG_LIBHEIF_PATH pointing at a missing file must fail, proving the decode needs that library.
set -euo pipefail

stage="${1:?usage: smoke-cli-heif.sh <staged-cli-directory>}"

LIBHEIF_VERSION=1.23.7
LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475
LIBHEIF_URL="https://github.com/strukturag/libheif/releases/download/v$LIBHEIF_VERSION/libheif-$LIBHEIF_VERSION.tar.gz"
FIXTURE_SIZE=718114
FIXTURE_SHA256=7f8b363e4936c0666a25f64f3a92fda10bd8e5453be4592530b65a55dd98f3f2

log() { echo "smoke-cli-heif: $*" >&2; }
die() {
  log "$*"
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

xberg="$stage/xberg"
[ -x "$xberg" ] || die "no executable xberg in $stage"

workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

curl -fsSL --retry 3 -o "$workdir/libheif.tar.gz" "$LIBHEIF_URL"
[ "$(sha256 "$workdir/libheif.tar.gz")" = "$LIBHEIF_SHA256" ] || die "libheif source archive checksum mismatch"
tar -xzf "$workdir/libheif.tar.gz" -C "$workdir" "libheif-$LIBHEIF_VERSION/examples/example.heic"
fixture="$workdir/libheif-$LIBHEIF_VERSION/examples/example.heic"
[ "$(wc -c <"$fixture" | tr -d ' ')" = "$FIXTURE_SIZE" ] || die "unexpected size for example.heic"
[ "$(sha256 "$fixture")" = "$FIXTURE_SHA256" ] || die "unexpected checksum for example.heic"

# No loader search path, so only the library beside the binary can satisfy the decode. ~keep
run_xberg() {
  env -u DYLD_LIBRARY_PATH -u DYLD_FALLBACK_LIBRARY_PATH -u LD_LIBRARY_PATH "$@" \
    "$xberg" extract "$fixture" --ocr false --format json
}

run_xberg >"$workdir/decoded.json" 2>"$workdir/decoded.err" || {
  cat "$workdir/decoded.err" >&2
  die "the staged CLI could not decode example.heic"
}
python3 - "$workdir/decoded.json" <<'PY' || die "the staged CLI did not report a decoded HEIF image"
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    image = json.load(handle)["result"]["metadata"]["format"]
if image.get("format") != "HEIF" or not image.get("width") or not image.get("height"):
    raise SystemExit(f"unexpected image metadata: {image}")
PY

if run_xberg "XBERG_LIBHEIF_PATH=$workdir/absent-libheif" >"$workdir/absent.out" 2>&1; then
  die "decoding succeeded with XBERG_LIBHEIF_PATH at a missing file; the check cannot tell a loaded libheif apart"
fi
grep -q 'libheif not found' "$workdir/absent.out" || {
  cat "$workdir/absent.out" >&2
  die "a missing libheif failed for another reason than the library search"
}
log "decoded example.heic through the libheif shipped in $stage"
