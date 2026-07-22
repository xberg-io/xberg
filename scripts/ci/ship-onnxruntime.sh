#!/usr/bin/env bash
# Copy libonnxruntime.so* beside the xberg native library in a built artifact, so
# the runtime resolves via RPATH=$ORIGIN at load time (#1284).
#
# Usage: ship-onnxruntime.sh <artifact> <ort-lib-dir>
#   <artifact>     a *.tar.gz/*.tgz or a directory
#   <ort-lib-dir>  directory holding libonnxruntime.so* (ORT_LIB_LOCATION)
set -euo pipefail

log() { echo "ship-onnxruntime: $*" >&2; }
die() { log "$*"; exit 1; }
cleanup() { [ -n "${WORKDIR:-}" ] && rm -rf "$WORKDIR"; }

# Copy every libonnxruntime.so* from $2 beside the first xberg native lib under $1.
place_beside_native() {
  local root="$1" ort_lib="$2" native dir
  native="$(find "$root" \
    \( -name 'libxberg_*.so' -o -name '_xberg*.so' -o -name '*.node' \
    -o -name 'php_xberg.so' \) -type f | head -1)"
  [ -n "$native" ] || die "no xberg native library found under $root"
  dir="$(dirname "$native")"
  cp -v "$ort_lib"/libonnxruntime.so* "$dir/"
  log "shipped ONNX Runtime beside $(basename "$native")"
}

ship_tarball() {
  local tar="$1" ort_lib="$2"
  mkdir -p "$WORKDIR/x"
  tar -xzf "$tar" -C "$WORKDIR/x"
  place_beside_native "$WORKDIR/x" "$ort_lib"
  rm -f "$tar"
  tar -czf "$tar" -C "$WORKDIR/x" .
}

main() {
  [ $# -eq 2 ] || die "usage: $(basename "$0") <artifact> <ort-lib-dir>"
  local artifact="$1" ort_lib="$2"
  if [ ! -d "$ort_lib" ] || ! ls "$ort_lib"/libonnxruntime.so* >/dev/null 2>&1; then
    die "no libonnxruntime.so* in '$ort_lib'"
  fi
  WORKDIR="$(mktemp -d)"
  trap cleanup EXIT

  case "$artifact" in
  *.tar.gz | *.tgz) ship_tarball "$artifact" "$ort_lib" ;;
  *)
    [ -d "$artifact" ] || die "unsupported artifact '$artifact'"
    place_beside_native "$artifact" "$ort_lib"
    ;;
  esac
  log "done"
}

main "$@"
