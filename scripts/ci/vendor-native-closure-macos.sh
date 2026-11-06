#!/usr/bin/env bash
# Bundle the non-system absolute dynamic-library imports of a macOS Mach-O
# binary and their transitive absolute imports beside the binary. Rewrite those
# load commands to @loader_path so they do not depend on build-host paths.
#
# This is the macOS analog of vendor-native-closure.sh (which uses ldd / patchelf /
# $ORIGIN on Linux); here we use otool / install_name_tool / codesign. The CLI
# release build links libheif dynamically (the `heic` feature), and without this
# the prebuilt binary carries an absolute Homebrew load command
# (/usr/local/opt/libheif/lib/libheif.1.dylib or /opt/homebrew/...) that is absent
# on users' machines. See xberg-io/xberg#1357.
#
# Homebrew records inter-library dependencies as absolute `opt` paths.
# The final check rejects unsupported imports, but does not resolve existing
# @rpath, @loader_path, or @executable_path imports.
set -euo pipefail

log() { echo "vendor-native-closure-macos: $*" >&2; }
die() {
  log "$*"
  exit 1
}

WORKDIR=""
cleanup() { [ -n "${WORKDIR:-}" ] && rm -rf "$WORKDIR"; }
trap cleanup EXIT

# Load paths that belong to the base macOS system, or are already relocated, and
# must not be vendored.
is_allowed_unvendored_path() {
  case "$1" in
  /usr/lib/* | /System/* | @loader_path/* | @executable_path/* | @rpath/*) return 0 ;;
  *) return 1 ;;
  esac
}

# ~keep A renamed dylib can have a different LC_ID_DYLIB basename; only inspect loads.
deps_of() {
  local load_commands
  load_commands="$(otool -arch all -l "$1")" || return 1
  printf '%s\n' "$load_commands" | MACHO_PATH="$1" LC_ALL=C awk '
    function complete() {
      return !in_command || (command && size && (!dependency || named))
    }
    {
      path = ENVIRON["MACHO_PATH"]
      suffix = substr($0, length(path) + 1)
      if (index($0, path) == 1 && (suffix == ":" || suffix ~ /^ \(architecture [^ )]+\):$/)) {
        if ((slice && !in_command) || !complete()) exit 1
        slice = 1
        in_command = next_number = 0
        next
      }
      if ($0 ~ /^Load command [0-9]+$/) {
        if (!slice || !complete() || $3 != next_number++) exit 1
        in_command = 1
        command = size = dependency = named = 0
        next
      }
      if (!in_command) exit 1
      if ($1 == "cmd") {
        if (command || NF != 2 || $2 !~ /^LC_[A-Z0-9_]+$/) exit 1
        command = 1
        dependency = ($2 ~ /^LC_(LOAD_DYLIB|LOAD_WEAK_DYLIB|REEXPORT_DYLIB|LOAD_UPWARD_DYLIB|LAZY_LOAD_DYLIB)$/)
      } else if ($1 == "cmdsize") {
        if (!command || size || NF != 2 || $2 !~ /^[1-9][0-9]*$/) exit 1
        size = $2
      } else if (dependency && $1 == "name") {
        if (!size || named || $0 !~ /[[:space:]]+\(offset [0-9]+\)$/) exit 1
        offset = $(NF) + 0
        sub(/^[[:space:]]*name[[:space:]]+/, "")
        sub(/[[:space:]]+\(offset [0-9]+\)$/, "")
        if ($0 == "" || offset < 24 || offset + length($0) + 1 > size) exit 1
        if (!seen[$0]++) print
        named = 1
      }
    }
    END { if (!slice || !in_command || !complete()) exit 1 }
  '
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

basename_key() { printf '%s' "${1##*/}" | LC_ALL=C tr '[:upper:]' '[:lower:]'; }

# ~keep Inspect the entire source graph before copying or modifying any artifact.
plan_bundle() {
  local binary="$1" base_dir="$2" plan="$WORKDIR/sources"
  local item dep base key previous root_key index=0
  root_key="$(basename_key "$binary")"
  mkdir "$WORKDIR/names"
  printf '%s\n' "$binary" >"$plan"
  while IFS= read -r item; do
    index=$((index + 1))
    deps_of "$item" >"$WORKDIR/loads-$index" || die "failed to inspect dependencies of ${item##*/}"
    while IFS= read -r dep; do
      is_allowed_unvendored_path "$dep" && continue
      [[ "$dep" = /* ]] || die "unsupported dependency path in ${item##*/}: $dep"
      [ -f "$dep" ] || die "dependency not found on disk: $dep"
      base="${dep##*/}"
      key="$(basename_key "$dep")"
      [ "$key" != "$root_key" ] || die "dependency basename collision with ${binary##*/}: $dep"
      if [ -f "$WORKDIR/names/$key" ]; then
        IFS= read -r previous <"$WORKDIR/names/$key"
        [[ "$previous" -ef "$dep" && "${previous##*/}" == "$base" ]] ||
          die "dependency basename collision: $previous and $dep"
        continue
      fi
      [[ ! -e "$base_dir/$base" && ! -L "$base_dir/$base" ]] ||
        die "dependency basename collision with existing bundle entry: $base"
      printf '%s\n' "$dep" >"$WORKDIR/names/$key"
      printf '%s\n' "$dep" >>"$plan"
    done <"$WORKDIR/loads-$index"
  done <"$plan"
}

relocate_bundle() {
  local binary="$1" base_dir="$2" source item dep base index=0
  while IFS= read -r source; do
    index=$((index + 1))
    base="${source##*/}"
    item="$base_dir/$base"
    if [ "$index" != 1 ]; then
      cp "$source" "$item"
      chmod u+w "$item"
      install_name_tool -id "@loader_path/$base" "$item" || die "install_name_tool -id failed for $base"
      log "vendored $base beside ${binary##*/}"
    fi
    while IFS= read -r dep; do
      is_allowed_unvendored_path "$dep" && continue
      install_name_tool -change "$dep" "@loader_path/${dep##*/}" "$item" ||
        die "install_name_tool -change failed for $dep in $base"
    done <"$WORKDIR/loads-$index"
    resign "$item"
  done <"$WORKDIR/sources"
}

verify_bundle() {
  local base_dir="$1" source item deps dep
  while IFS= read -r source; do
    item="$base_dir/${source##*/}"
    deps="$(deps_of "$item")" || die "failed to inspect final closure of ${item##*/}"
    while IFS= read -r dep; do
      [ -n "$dep" ] || continue
      is_allowed_unvendored_path "$dep" || die "unsupported dependency path in ${item##*/}: $dep"
    done <<<"$deps"
  done <"$WORKDIR/sources"
}

main() {
  local binary="${1:?usage: $(basename "$0") <macho-binary>}" base_dir
  base_dir="$(cd "$(dirname "$binary")" && pwd)"
  binary="$base_dir/${binary##*/}"
  [ -f "$binary" ] || die "no such binary: $binary"

  WORKDIR="$(mktemp -d)"
  plan_bundle "$binary" "$base_dir"
  relocate_bundle "$binary" "$base_dir"
  verify_bundle "$base_dir"
  log "verified absolute dependency relocation for ${binary##*/}"
}

main "$@"
