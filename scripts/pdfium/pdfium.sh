#!/usr/bin/env bash
# Pinned, checksum-verified libpdfium provisioning shared by the `pdfium:*` mise tasks, CI and the
# install scripts. The pin lives in scripts/pdfium-version and the digests in
# scripts/pdfium-runtime-checksums.sha256; both are committed, and a download is never trusted
# for its own checksum.
#
#   pdfium.sh version
#   pdfium.sh platform
#   pdfium.sh dir        [--version N] [--platform P]   print the library directory (no download)
#   pdfium.sh provision  [--version N] [--platform P]   download + verify if missing, print the directory
#   pdfium.sh env        [--version N] [--platform P]   print export lines for the provisioned directory
#   pdfium.sh latest     [--write-checksums FILE]       newest upstream build vs the pin (read-only)
#   pdfium.sh bump N     [--apply] [--allow-binding-mismatch]
#   pdfium.sh check                                     fail when a hardcoded pin disagrees with the pin file
#
# PDFIUM_VERSION overrides the pin for one run (it needs a matching manifest entry);
# PDFIUM_CHECKSUMS_FILE overrides the manifest (used to test an unpinned build with digests taken
# from the release API); PDFIUM_CACHE_DIR overrides the cache root.
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
pin_file="$root/scripts/pdfium-version"
default_checksums="$root/scripts/pdfium-runtime-checksums.sha256"
checksums="${PDFIUM_CHECKSUMS_FILE:-$default_checksums}"
verify_archive="$root/scripts/ci/actions/setup-onnx-runtime/verify-archive.sh"
upstream="bblanchon/pdfium-binaries"

die() {
  printf 'pdfium: %s\n' "$*" >&2
  exit 1
}

pinned_version() {
  local v
  v="$(tr -d '[:space:]' <"$pin_file")"
  [[ "$v" =~ ^[0-9]+$ ]] || die "$pin_file must hold one numeric Chromium build, found '$v'"
  printf '%s' "$v"
}

host_platform() {
  local os arch
  case "$(uname -s)" in
  Linux*)
    if (ldd --version 2>&1 || true) | grep -qi musl; then os="linux-musl"; else os="linux"; fi
    ;;
  Darwin*) os="mac" ;;
  MINGW* | MSYS* | CYGWIN*) os="win" ;;
  *) die "unsupported OS $(uname -s)" ;;
  esac
  case "$(uname -m)" in
  x86_64 | amd64) arch="x64" ;;
  arm64 | aarch64) arch="arm64" ;;
  *) die "unsupported architecture $(uname -m)" ;;
  esac
  printf '%s-%s' "$os" "$arch"
}

parse_common() {
  version="${PDFIUM_VERSION:-$(pinned_version)}"
  platform="$(host_platform)"
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --version)
      version="${2:?--version needs a value}"
      shift 2
      ;;
    --platform)
      platform="${2:?--platform needs a value}"
      shift 2
      ;;
    *) die "unknown argument $1" ;;
    esac
  done
  [[ "$version" =~ ^[0-9]+$ ]] || die "invalid Chromium build '$version'"
  [[ "$platform" =~ ^(linux|linux-musl|mac|win)-(x64|arm64)$ ]] || die "invalid platform '$platform'"
}

cache_root() {
  printf '%s' "${PDFIUM_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/xberg/pdfium}"
}

install_dir() { printf '%s/%s/%s' "$(cache_root)" "$version" "$platform"; }

lib_subdir() {
  case "$platform" in win-*) printf 'bin' ;; *) printf 'lib' ;; esac
}

lib_file() {
  case "$platform" in
  win-*) printf 'pdfium.dll' ;;
  mac-*) printf 'libpdfium.dylib' ;;
  *) printf 'libpdfium.so' ;;
  esac
}

lib_dir() { printf '%s/%s' "$(install_dir)" "$(lib_subdir)"; }

expected_sha() {
  awk -v name="$1" '{ sub(/\r$/, "") } $2 == name { print $1 }' "$checksums"
}

cmd_provision() {
  parse_common "$@"
  local asset="pdfium-${platform}.tgz" archive_name dest lib sha stamp
  archive_name="${version}-${asset}"
  dest="$(install_dir)"
  lib="$(lib_dir)/$(lib_file)"
  bash "$verify_archive" "$archive_name" "$checksums" --known || die "no pinned SHA-256 for $archive_name in $checksums"
  sha="$(expected_sha "$archive_name")"
  stamp="$dest/.verified-sha256"

  if [ -f "$lib" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$sha" ]; then
    printf 'pdfium %s (%s) already provisioned\n' "$version" "$platform" >&2
    printf '%s\n' "$(lib_dir)"
    return 0
  fi

  local parent tmp
  parent="$(cache_root)/$version"
  mkdir -p "$parent"
  tmp="$(mktemp -d "$parent/.tmp.XXXXXX")"
  # shellcheck disable=SC2064 # expand now: tmp is function-local and gone when the trap fires
  trap "rm -rf '$tmp'" EXIT
  mkdir "$tmp/extracted"
  printf 'Downloading pdfium %s (%s)...\n' "$version" "$platform" >&2
  curl -fL --retry 5 --retry-delay 2 --retry-max-time 180 --retry-all-errors -sS \
    -o "$tmp/$archive_name" \
    "https://github.com/${upstream}/releases/download/chromium/${version}/${asset}" ||
    die "download failed (offline?); a missing libpdfium is a failure, not a skip"
  bash "$verify_archive" "$tmp/$archive_name" "$checksums" >&2 || die "checksum verification failed; nothing was installed"
  tar -xzf "$tmp/$archive_name" -C "$tmp/extracted" || die "extraction failed; nothing was installed"
  [ -f "$tmp/extracted/$(lib_subdir)/$(lib_file)" ] || die "archive does not contain $(lib_subdir)/$(lib_file)"
  printf '%s\n' "$sha" >"$tmp/extracted/.verified-sha256" || die "could not write the verification stamp"
  rm -rf "$dest" || die "could not remove the previous install at $dest"
  mv "$tmp/extracted" "$dest" || die "could not install pdfium into $dest"
  printf '%s\n' "$(lib_dir)"
}

cmd_dir() {
  parse_common "$@"
  printf '%s\n' "$(lib_dir)"
}

cmd_env() {
  parse_common "$@"
  local dir
  dir="$(cmd_provision "$@" 2>&2)" || die "libpdfium is unavailable"
  [ -f "$dir/$(lib_file)" ] || die "$dir/$(lib_file) is missing after provisioning"
  printf 'export PDFIUM_DYNAMIC_LIB_PATH=%q\n' "$dir"
  printf 'export LD_LIBRARY_PATH=%q\n' "$dir:${LD_LIBRARY_PATH:-}"
  printf 'export DYLD_LIBRARY_PATH=%q\n' "$dir:${DYLD_LIBRARY_PATH:-}"
  printf 'export XBERG_REQUIRE_PDFIUM=1\n'
}

release_json_field() {
  # $1 = release endpoint, $2 = gh --jq expression
  gh api "repos/${upstream}/releases/$1" --jq "$2"
}

release_checksums() {
  # Prints "<sha256>  <build>-<asset>" for the eight assets the manifest carries.
  local build="$1" endpoint="$2" out
  out="$(release_json_field "$endpoint" \
    '.assets[] | select(.name | test("^pdfium-(linux|linux-musl|mac|win)-(x64|arm64)\\.tgz$")) | "\(.digest) \(.name)"')"
  [ -n "$out" ] || die "release $build lists no pdfium assets"
  printf '%s\n' "$out" | while read -r digest name; do
    [[ "$digest" =~ ^sha256:[a-f0-9]{64}$ ]] || die "asset $name has no sha256 digest ('$digest')"
    printf '%s  %s-%s\n' "${digest#sha256:}" "$build" "$name"
  done | sort -k2
}

cmd_latest() {
  local out=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --write-checksums)
      out="${2:?--write-checksums needs a file}"
      shift 2
      ;;
    *) die "unknown argument $1" ;;
    esac
  done
  command -v gh >/dev/null || die "gh is required"
  local tag latest pinned newer=false
  tag="$(release_json_field latest .tag_name)"
  latest="${tag#chromium/}"
  [[ "$latest" =~ ^[0-9]+$ ]] || die "unexpected latest tag '$tag'"
  pinned="$(pinned_version)"
  if [ "$latest" -gt "$pinned" ]; then newer=true; fi
  printf 'pinned=%s\nlatest=%s\nnewer=%s\n' "$pinned" "$latest" "$newer"
  if [ -n "$out" ]; then
    # Digests come from the release API, a channel separate from the archive download.
    {
      printf '# Source: GitHub release asset digests, %s %s (candidate, not committed).\n' "$upstream" "$tag"
      release_checksums "$latest" "tags/chromium%2F${latest}"
    } >"$out"
    printf 'checksums_file=%s\n' "$out"
  else
    printf '\n# candidate checksums for build %s\n' "$latest" >&2
    release_checksums "$latest" "tags/chromium%2F${latest}" >&2
  fi
}

cmd_bump() {
  local build="${1:?usage: pdfium.sh bump <build> [--apply] [--allow-binding-mismatch]}"
  shift
  local apply=false allow=false
  while [ "$#" -gt 0 ]; do
    case "$1" in
    --apply) apply=true ;;
    --allow-binding-mismatch) allow=true ;;
    *) die "unknown argument $1" ;;
    esac
    shift
  done
  [[ "$build" =~ ^[0-9]+$ ]] || die "invalid Chromium build '$build'"
  command -v gh >/dev/null || die "gh is required"
  candidate="$(mktemp)"
  scratch_cache="$(mktemp -d)"
  trap 'rm -rf "$candidate" "$scratch_cache"' EXIT
  {
    printf '# Source: GitHub release asset digest fields, %s chromium/%s.\n' "$upstream" "$build"
    printf '# https://api.github.com/repos/%s/releases/tags/chromium%%2F%s\n' "$upstream" "$build"
    printf '# Local archive names include the Chromium revision so a pin covers one tag and asset.\n'
    release_checksums "$build" "tags/chromium%2F${build}"
  } >"$candidate"
  [ "$(grep -vc '^#' "$candidate")" -eq 8 ] || die "expected 8 asset digests for $build"

  local bindings="$root/crates/xberg-pdfium-render/src/bindgen/pdfium_${build}.rs"
  if [ ! -f "$bindings" ] && [ "$allow" != true ]; then
    printf 'pdfium: no checked-in bindings for %s (%s); the pin must move together with regenerated bindings.\n' \
      "$build" "${bindings#"$root"/}" >&2
    if [ "$apply" = true ]; then die "refusing to apply; pass --allow-binding-mismatch to override"; fi
  fi

  # Verify before writing: fetch the host archive and check it against the API digest.
  PDFIUM_CHECKSUMS_FILE="$candidate" PDFIUM_CACHE_DIR="$scratch_cache" \
    bash "${BASH_SOURCE[0]}" provision --version "$build" >/dev/null || die "host archive did not verify against the release digest"

  printf 'Pin %s -> %s\n' "$(pinned_version)" "$build"
  diff -u "$default_checksums" "$candidate" || true
  if [ "$apply" != true ]; then
    printf '\nDry run: nothing written. Re-run with --apply.\n'
    return 0
  fi
  cp "$candidate" "$default_checksums"
  printf '%s\n' "$build" >"$pin_file"
  printf 'Wrote %s and %s.\n' "${pin_file#"$root"/}" "${default_checksums#"$root"/}"
  printf 'Still to update by hand (see: pdfium.sh check): Dockerfile ARG defaults and the bindgen include.\n'
}

cmd_check() {
  local pin rc=0 f v
  pin="$(pinned_version)"
  for f in "$root"/docker/Dockerfile.*; do
    v="$(sed -n 's/^ARG PDFIUM_VERSION=\([0-9]*\).*/\1/p' "$f")"
    if [ -n "$v" ] && [ "$v" != "$pin" ]; then
      printf 'MISMATCH %s: ARG PDFIUM_VERSION=%s, pin is %s\n' "${f#"$root"/}" "$v" "$pin" >&2
      rc=1
    fi
  done
  grep -q "^[0-9a-f]\{64\}  ${pin}-pdfium-linux-x64\.tgz\$" "$default_checksums" ||
    {
      printf 'MISMATCH checksums file has no %s entries\n' "$pin" >&2
      rc=1
    }
  [ -f "$root/crates/xberg-pdfium-render/src/bindgen/pdfium_${pin}.rs" ] ||
    {
      printf 'MISMATCH no bindgen/pdfium_%s.rs for the pin\n' "$pin" >&2
      rc=1
    }
  [ "$rc" -eq 0 ] && printf 'pdfium pin %s is consistent\n' "$pin"
  return "$rc"
}

sub="${1:-}"
[ "$#" -gt 0 ] && shift
case "$sub" in
version) pinned_version && echo ;;
platform) host_platform && echo ;;
dir) cmd_dir "$@" ;;
provision) cmd_provision "$@" ;;
env) cmd_env "$@" ;;
latest) cmd_latest "$@" ;;
bump) cmd_bump "$@" ;;
check) cmd_check "$@" ;;
*) die "usage: pdfium.sh {version|platform|dir|provision|env|latest|bump|check}" ;;
esac
