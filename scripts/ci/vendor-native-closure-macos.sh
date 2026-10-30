#!/usr/bin/env bash
# Bundle the complete non-system dynamic-library closure of a macOS Mach-O binary
# next to it, rewriting every load command to @loader_path so the shipped artifact
# is self-contained and does not depend on Homebrew install paths at runtime.
#
# This is the macOS analog of vendor-native-closure.sh (which uses ldd / patchelf /
# $ORIGIN on Linux); here we use otool / install_name_tool / codesign. The CLI
# release build links libheif dynamically (the `heic` feature), and without this
# the prebuilt binary carries an absolute Homebrew load command
# (/usr/local/opt/libheif/lib/libheif.1.dylib or /opt/homebrew/...) that is absent
# on users' machines. See xberg-io/xberg#1357.
#
# Homebrew records inter-library dependencies as absolute `opt` paths (not @rpath),
# so walking the absolute-path closure captures everything; the final check fails
# the build if any /opt or /usr/local load path survives.
set -euo pipefail

log() { echo "vendor-native-closure-macos: $*" >&2; }
die() {
  log "$*"
  exit 1
}

# are out of scope at exit and would trip `set -u`).
WORKDIR=""
cleanup() { [ -n "${WORKDIR:-}" ] && rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Load paths that belong to the base macOS system, or are already relocated, and
# must not be vendored.
is_system_path() {
  case "$1" in
  /usr/lib/* | /System/* | @loader_path/* | @executable_path/* | @rpath/*) return 0 ;;
  *) return 1 ;;
  esac
}

# Emit the dependency load paths of a Mach-O file, skipping otool's header line
# and the file's own LC_ID_DYLIB entry (a dylib lists its own install name first).
deps_of() {
  local file="$1" self dep load_commands
  self="$(basename "$file")"
  load_commands="$(otool -L "$file")" || return 1
  printf '%s\n' "$load_commands" | tail -n +2 | awk '{print $1}' | while IFS= read -r dep; do
    [ -n "$dep" ] || continue
    [ "$(basename "$dep")" = "$self" ] && continue
    printf '%s\n' "$dep"
  done
}

# A failed re-sign is not a warning-level event: dyld refuses an unsigned library
# and AMFI SIGKILLs the loading process for an invalid signature, so a bad result
# here ships a CLI that dies on launch. Verify rather than trusting the exit code,
# since the signature is what has to be good -- not merely codesign returning 0.
resign() {
  codesign --force --sign - "$1" || die "ad-hoc re-sign failed for $(basename "$1")"
  codesign --verify --strict "$1" ||
    die "no valid code signature on $(basename "$1") after ad-hoc re-signing"
}

main() {
  local binary="${1:?usage: $(basename "$0") <macho-binary>}"
  local base_dir
  base_dir="$(cd "$(dirname "$binary")" && pwd)"
  binary="$base_dir/$(basename "$binary")"
  [ -f "$binary" ] || die "no such binary: $binary"

  local queue seen item dep base dest dependencies final_report
  WORKDIR="$(mktemp -d)"
  queue="$WORKDIR/queue"
  seen="$WORKDIR/seen"
  : >"$seen"
  printf '%s\n' "$binary" >"$queue"

  while [ -s "$queue" ]; do
    item="$(head -n1 "$queue")"
    tail -n +2 "$queue" >"$queue.tmp" && mv "$queue.tmp" "$queue"

    dependencies="$(deps_of "$item")" || die "failed to inspect dependencies of $(basename "$item")"
    while IFS= read -r dep; do
      [ -n "$dep" ] || continue
      is_system_path "$dep" && continue
      base="$(basename "$dep")"
      dest="$base_dir/$base"
      # Repoint this Mach-O file at the bundled copy of the dependency.
      install_name_tool -change "$dep" "@loader_path/$base" "$item" ||
        die "install_name_tool -change failed for $dep in $(basename "$item")"
      if ! grep -qxF "$base" "$seen" 2>/dev/null; then
        printf '%s\n' "$base" >>"$seen"
        [ -e "$dep" ] || die "dependency not found on disk: $dep"
        # cp follows the source symlink, copying the real dylib under $base.
        cp "$dep" "$dest"
        chmod u+w "$dest"
        install_name_tool -id "@loader_path/$base" "$dest" ||
          die "install_name_tool -id failed for $base"
        printf '%s\n' "$dest" >>"$queue"
        log "vendored $base beside $(basename "$binary")"
      fi
    done <<<"$dependencies"

    resign "$item"
  done

  # Nothing outside the bundle may remain (mirrors the Intel ONNX Runtime check).
  local residual
  final_report="$WORKDIR/final-dependencies"
  otool -L "$binary" >"$final_report" || die "failed to inspect final closure of $(basename "$binary")"
  while IFS= read -r base; do
    otool -L "$base_dir/$base" >>"$final_report" || die "failed to inspect final closure of $base"
  done <"$seen"
  residual="$(awk '{print $1}' "$final_report" | grep -E '^(/opt/|/usr/local/)' || true)"
  if [ -n "$residual" ]; then
    log "residual non-system load paths after vendoring:"
    printf '%s\n' "$residual" >&2
    die "dependency closure incomplete"
  fi
  log "verified self-contained closure for $(basename "$binary")"
}

main "$@"
