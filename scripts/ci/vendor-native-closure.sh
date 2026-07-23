#!/usr/bin/env bash
set -euo pipefail

log() { echo "vendor-native-closure: $*" >&2; }
die() { log "$*"; exit 1; }
cleanup() { [ -n "${WORKDIR:-}" ] && rm -rf "$WORKDIR"; }

is_base_lib() {
  case "$1" in
  ld-linux* | ld-musl* | libc.so* | libc.musl* | libc-*.so* | libm.so* | libmvec.so* | \
    libdl.so* | librt.so* | libpthread.so* | libresolv.so* | libgcc_s.so* | \
    libstdc++.so* | libssl.so* | libcrypto.so*) return 0 ;;
  *) return 1 ;;
  esac
}

vendor_one() {
  local native="$1" dir queue seen bin lib base
  dir="$(dirname "$native")"
  queue="$(mktemp)"
  seen="$(mktemp)"
  printf '%s\n' "$native" >"$queue"
  while [ -s "$queue" ]; do
    bin="$(head -n1 "$queue")"
    tail -n +2 "$queue" >"$queue.tmp" && mv "$queue.tmp" "$queue"
    ldd "$bin" 2>/dev/null |
      sed -n 's/.*=> *\(\/[^ ]*\).*/\1/p; s/^[[:space:]]*\(\/[^ ]*\) (0x[0-9a-f]*)$/\1/p' |
      while IFS= read -r lib; do
        [ -f "$lib" ] || continue
        base="$(basename "$lib")"
        is_base_lib "$base" && continue
        grep -qxF "$base" "$seen" 2>/dev/null && continue
        printf '%s\n' "$base" >>"$seen"
        cp -L "$lib" "$dir/$base"
        chmod u+w "$dir/$base" 2>/dev/null || true
        printf '%s\n' "$dir/$base" >>"$queue"
        log "vendored $base beside $(basename "$native")"
      done
  done
  rm -f "$queue" "$seen"
  # shellcheck disable=SC2016
  patchelf --set-rpath '$ORIGIN' "$native"
}

vendor_tree() {
  local root="$1" found=0 lib
  while IFS= read -r lib; do
    found=1
    vendor_one "$lib"
  done < <(find "$root" \( -name 'libxberg_*.so' -o -name '*.node' -o -name 'php_xberg.so' \) -type f)
  [ "$found" = 1 ] || die "no xberg native library found under $root"
}

main() {
  local artifact="${1:?usage: $(basename "$0") <artifact>}"
  case "$artifact" in
  *.tar.gz | *.tgz)
    WORKDIR="$(mktemp -d)"
    trap cleanup EXIT
    tar -xzf "$artifact" -C "$WORKDIR"
    vendor_tree "$WORKDIR"
    rm -f "$artifact"
    tar -czf "$artifact" -C "$WORKDIR" .
    ;;
  *.so | *.node) vendor_one "$artifact" ;;
  *) [ -d "$artifact" ] || die "unsupported artifact '$artifact'"; vendor_tree "$artifact" ;;
  esac
  log "done"
}

main "$@"
