#!/usr/bin/env bash
# Build the libheif decode stack (libde265, libaom, libheif) from source at
# MACOSX_DEPLOYMENT_TARGET, into a prefix that macOS artifacts bundle instead
# of Homebrew bottles, which target the runner's own macOS (#1243).
# Decode-only by default, mirroring the manylinux build-libheif step. The
# Elixir arm64 profile includes the HEVC and AV1 encoders. ~keep
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
source "$REPO_ROOT/scripts/lib/retry.sh"
source "$REPO_ROOT/scripts/lib/macho.sh"

LIBDE265_VERSION=1.1.3
LIBDE265_SHA256=554228bd17788c99a7e63b37ab5634722190e6e2bf60c1dcb01cef328e133905
LIBAOM_VERSION=3.15.2
LIBAOM_SHA256=67bb54b245f33ed98600e08269e6139986e48114e14042474ddd8885801dfddc
LIBHEIF_VERSION=1.23.7
LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475

X265_VERSION=4.3
X265_SHA256=83c53e4c8bbb8f1e33ed59e10a7d621d1d7801ca853910c3eb41f038b8ffb121

LIBDE265_URL="https://github.com/strukturag/libde265/releases/download/v$LIBDE265_VERSION/libde265-$LIBDE265_VERSION.tar.gz"
LIBHEIF_URL="https://github.com/strukturag/libheif/releases/download/v$LIBHEIF_VERSION/libheif-$LIBHEIF_VERSION.tar.gz"
X265_URL="https://github.com/Multicorewareinc/x265/releases/download/$X265_VERSION/x265_$X265_VERSION.tar.gz"

PROFILE="${XBERG_HEIF_PROFILE:-decode}"
if [ "$PROFILE" = elixir-encoders ]; then
  LIBAOM_VERSION=3.15.3
  LIBAOM_SHA256=12c01dc731ed7b5a999e28120b45bf58a002ba3435a4ba2cfbc1e084a27c9c04
fi
LIBAOM_URL="https://storage.googleapis.com/aom-releases/libaom-$LIBAOM_VERSION.tar.gz"
PREFIX="${XBERG_HEIF_PREFIX:-/tmp/xberg-heif}"
TARGET="${MACOSX_DEPLOYMENT_TARGET:-15.0}"
WORK=""
JOBS="${CMAKE_BUILD_PARALLEL_LEVEL:-$(sysctl -n hw.ncpu)}"
if [[ ! "$JOBS" =~ ^[1-9][0-9]*$ ]]; then
  echo "::error::CMAKE_BUILD_PARALLEL_LEVEL must be a positive integer" >&2
  exit 1
fi

# Absolute install names so delocate can resolve every load command. ~keep
CMAKE_COMMON=(
  -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_INSTALL_PREFIX="$PREFIX"
  -DCMAKE_INSTALL_LIBDIR=lib
  -DCMAKE_INSTALL_NAME_DIR="$PREFIX/lib"
  -DCMAKE_OSX_DEPLOYMENT_TARGET="$TARGET"
  -DBUILD_SHARED_LIBS=ON
)

ensure_nasm() {
  [ "$(uname -m)" = "x86_64" ] || return 0
  command -v nasm >/dev/null 2>&1 || retry_with_backoff brew install nasm
}

build_dep() {
  local name="$1" version="$2" url="$3" sha256="$4"
  shift 4
  echo "::group::$name $version"
  retry_with_backoff curl -fsSL --retry 3 -o "$WORK/$name.tar.gz" "$url"
  printf '%s  %s\n' "$sha256" "$WORK/$name.tar.gz" | shasum -a 256 -c -
  tar -xzf "$WORK/$name.tar.gz" -C "$WORK"
  if [ "$name" = x265 ]; then
    build_x265 "$WORK/x265_$version/source"
    echo "::endgroup::"
    return
  fi
  cmake -S "$WORK/$name-$version" -B "$WORK/$name-build" "${CMAKE_COMMON[@]}" "$@"
  cmake --build "$WORK/$name-build" -j "$JOBS"
  cmake --install "$WORK/$name-build"
  echo "::endgroup::"
}

build_x265() {
  local source="$1" depth main12
  local common=(-DENABLE_CLI=OFF -DENABLE_LIBVMAF=OFF -DENABLE_HDR10_PLUS=OFF -DENABLE_SVT_HEVC=OFF)
  for depth in 10 12; do
    main12=OFF
    [ "$depth" = 12 ] && main12=ON
    cmake -S "$source" -B "$WORK/x265-$depth" "${CMAKE_COMMON[@]}" "${common[@]}" \
      -DHIGH_BIT_DEPTH=ON -DEXPORT_C_API=OFF -DENABLE_SHARED=OFF -DMAIN12="$main12"
    cmake --build "$WORK/x265-$depth" -j "$JOBS" --target x265-static
  done
  # Follow upstream's multilib build, linking the two extra bit depths into one dylib. ~keep
  cmake -S "$source" -B "$WORK/x265-build" "${CMAKE_COMMON[@]}" "${common[@]}" \
    -DENABLE_SHARED=ON -DLINKED_10BIT=ON -DLINKED_12BIT=ON \
    "-DEXTRA_LIB=$WORK/x265-10/libx265.a;$WORK/x265-12/libx265.a"
  cmake --build "$WORK/x265-build" -j "$JOBS"
  cmake --install "$WORK/x265-build"
}

deps_of() {
  otool -L "$1" | tail -n +3 | sed 's/^[[:space:]]*//; s/ (compatibility.*//'
}

encoder_deps_of() {
  local lib="$1" identity id imports
  identity="$(otool -D "$lib")" || return 1
  id="${identity#*$'\n'}"
  if [[ "$identity" != "$lib:"$'\n'"$id" || "$id" != "$PREFIX"/lib/*.dylib ||
    "${id#"$PREFIX"/lib/}" == */* || "$id" == *$'\n'* || ! -f "$id" ]]; then
    echo "::error::missing or invalid dylib identity in $lib" >&2
    return 1
  fi
  imports="$(otool -L "$lib")" || return 1
  LC_ALL=C MACHO_PATH="$lib" MACHO_ID="$id" awk '
    NR == 1 {
      if ($0 != ENVIRON["MACHO_PATH"] ":") exit 1
      next
    }
    {
      if ($0 !~ /^[[:space:]]+[^[:space:]].* \(compatibility version [^)]*\)$/) exit 1
      sub(/^[[:space:]]+/, "")
      sub(/ \(compatibility version [^)]*\)$/, "")
      if (NR == 2) {
        if ($0 != ENVIRON["MACHO_ID"]) exit 1
      } else {
        print
      }
    }
    END { if (NR < 3) exit 1 }
  ' <<<"$imports" || {
    echo "::error::missing or invalid dylib import rows in $lib" >&2
    return 1
  }
}

verify_dylib_imports() {
  local lib="$1" dep
  while IFS= read -r dep; do
    case "$dep" in
    "$PREFIX"/lib/*)
      if [ "$PROFILE" = elixir-encoders ] && [ ! -f "$dep" ]; then
        echo "::error::$(basename "$lib") imports missing $dep" >&2
        return 1
      fi
      ;;
    /usr/lib/* | /System/*) ;;
    *)
      echo "::error::$(basename "$lib") links outside the prefix: $dep" >&2
      return 1
      ;;
    esac
  done
}

verify_dylib() {
  local lib="$1" minos deps
  if [ "$PROFILE" = elixir-encoders ]; then
    minos="$(macho_minos_by_arch "$lib" 1)" || return 1
    if [ "$minos" != "arm64 $TARGET" ]; then
      echo "::error::$(basename "$lib") has $minos, expected arm64 $TARGET" >&2
      return 1
    fi
    deps="$(encoder_deps_of "$lib")" || return 1
    verify_dylib_imports "$lib" <<<"$deps"
    return
  fi

  minos="$(minos_of "$lib")"
  if [ "$minos" != "$TARGET" ]; then
    echo "::error::$(basename "$lib") targets macOS $minos, expected $TARGET"
    return 1
  fi
  verify_dylib_imports "$lib" < <(deps_of "$lib")
}

verify_encoder_links() {
  local deps dep
  deps="$(encoder_deps_of "$PREFIX/lib/libheif.dylib")" || return 1
  for dep in libaom libde265 libx265; do
    if ! grep -Fq "$PREFIX/lib/$dep." <<<"$deps"; then
      echo "::error::libheif does not link to the pinned $dep" >&2
      return 1
    fi
  done
}

verify_prefix() {
  local lib fail=0 checked=0
  local required=(libheif libde265 libaom)
  [ "$PROFILE" = elixir-encoders ] && required+=(libx265)
  for lib in "${required[@]}"; do
    [ -f "$PREFIX/lib/$lib.dylib" ] || {
      echo "::error::missing $lib.dylib"
      return 1
    }
  done
  for lib in "$PREFIX"/lib/*.dylib; do
    [ -L "$lib" ] && continue
    checked=$((checked + 1))
    verify_dylib "$lib" || fail=1
  done
  [ "$checked" -ge "${#required[@]}" ] || {
    echo "::error::fewer than ${#required[@]} real codec libraries were examined"
    return 1
  }
  if [ "$PROFILE" = elixir-encoders ]; then
    verify_encoder_links || fail=1
  fi
  return "$fail"
}

verify_versions() {
  local spec name expected actual
  local required=("libheif:$LIBHEIF_VERSION" "libde265:$LIBDE265_VERSION" "aom:$LIBAOM_VERSION")
  [ "$PROFILE" = elixir-encoders ] && required+=("x265:$X265_VERSION")
  for spec in "${required[@]}"; do
    name="${spec%%:*}"
    expected="${spec#*:}"
    actual="$(PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig" PKG_CONFIG_PATH="" pkg-config --modversion "$name")" || return 1
    if [ "$actual" != "$expected" ]; then
      echo "::error::$name version $actual does not match $expected" >&2
      return 1
    fi
  done
}

codec_notices() {
  local mode="$1" inventory="$PREFIX/share/xberg-heif-notices" spec name file
  local checked=(sources.tsv)
  local notices=(
    "libheif-$LIBHEIF_VERSION/COPYING"
    "libde265-$LIBDE265_VERSION/COPYING" "libde265-$LIBDE265_VERSION/AUTHORS"
    "libde265-$LIBDE265_VERSION/libde265/COPYING"
    "libaom-$LIBAOM_VERSION/LICENSE" "libaom-$LIBAOM_VERSION/PATENTS" "libaom-$LIBAOM_VERSION/AUTHORS"
    "libaom-$LIBAOM_VERSION/third_party/fastfeat/LICENSE"
    "libaom-$LIBAOM_VERSION/third_party/libyuv/LICENSE"
    "libaom-$LIBAOM_VERSION/third_party/vector/LICENSE"
    "x265_$X265_VERSION/COPYING"
  )
  if [ "$mode" = stage ]; then
    mkdir -p "$inventory"
    source_inventory >"$inventory/sources.tsv"
  fi
  for spec in "${notices[@]}"; do
    name="${spec//\//-}"
    file="$inventory/$name"
    if [ "$mode" = stage ]; then
      cp "$WORK/$spec" "$file"
    fi
    checked+=("$name")
    [ -s "$file" ] || {
      echo "::error::missing upstream codec notice $name" >&2
      return 1
    }
  done
  if ! source_inventory | cmp -s - "$inventory/sources.tsv"; then
    echo "::error::missing or mismatched codec source inventory" >&2
    return 1
  fi
  if [ "$mode" = stage ]; then
    (cd "$inventory" && shasum -a 256 "${checked[@]}") >"$inventory/notices.sha256"
  elif ! (cd "$inventory" && shasum -a 256 "${checked[@]}") | cmp -s - "$inventory/notices.sha256"; then
    echo "::error::missing or mismatched codec notice checksums" >&2
    return 1
  fi
}

source_inventory() {
  printf '%s\t%s\t%s\n' \
    "libheif $LIBHEIF_VERSION" "$LIBHEIF_SHA256" "$LIBHEIF_URL" \
    "libde265 $LIBDE265_VERSION" "$LIBDE265_SHA256" "$LIBDE265_URL" \
    "libaom $LIBAOM_VERSION" "$LIBAOM_SHA256" "$LIBAOM_URL" \
    "x265 $X265_VERSION" "$X265_SHA256" "$X265_URL"
}

build_stack() {
  local aom_encoder=0 heif_encoders=OFF aom_cpu_arg=()
  local heif_extra=()
  if [ "$PROFILE" = elixir-encoders ]; then
    aom_encoder=1
    heif_encoders=ON
    heif_extra=(-DENABLE_PLUGIN_LOADING=OFF -DWITH_X264=OFF -DWITH_OpenH264_DECODER=OFF)
    build_dep x265 "$X265_VERSION" "$X265_URL" "$X265_SHA256"
  fi

  build_dep libde265 "$LIBDE265_VERSION" "$LIBDE265_URL" "$LIBDE265_SHA256" -DENABLE_SDL=OFF

  # The macos-15-intel runner ships a nasm libaom's test_nasm rejects
  # ("multipass optimization not supported"), which aborts configure. This is a
  # decode-only build on an EOL arch, so drop to the generic (pure-C) CPU target
  # on x86_64 to bypass nasm entirely. arm64 keeps its NEON SIMD. ~keep
  [ "$(uname -m)" = "x86_64" ] && aom_cpu_arg=(-DAOM_TARGET_CPU=generic)
  build_dep libaom "$LIBAOM_VERSION" "$LIBAOM_URL" "$LIBAOM_SHA256" \
    -DCONFIG_AV1_ENCODER="$aom_encoder" -DENABLE_APPS=0 -DENABLE_EXAMPLES=0 -DENABLE_TESTS=0 \
    -DENABLE_DOCS=0 -DENABLE_TOOLS=0 ${aom_cpu_arg[@]+"${aom_cpu_arg[@]}"}

  # Pin dependency discovery to this prefix so no Homebrew library leaks in. ~keep
  if [ "$PROFILE" = elixir-encoders ]; then
    local PKG_CONFIG_PATH=""
    export PKG_CONFIG_PATH
  fi
  PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig" build_dep libheif "$LIBHEIF_VERSION" "$LIBHEIF_URL" "$LIBHEIF_SHA256" \
    -DCMAKE_PREFIX_PATH="$PREFIX" -DCMAKE_IGNORE_PREFIX_PATH="/opt/homebrew;/usr/local" \
    -DWITH_LIBDE265=ON -DWITH_AOM_DECODER=ON -DWITH_AOM_ENCODER="$heif_encoders" -DWITH_X265="$heif_encoders" \
    -DWITH_EXAMPLES=OFF -DWITH_GDK_PIXBUF=OFF -DBUILD_TESTING=OFF ${heif_extra[@]+"${heif_extra[@]}"}
}

main() {
  local receipt arch contents
  arch="$(uname -m)"
  receipt="libheif=$LIBHEIF_VERSION libde265=$LIBDE265_VERSION libaom=$LIBAOM_VERSION target=$TARGET arch=$arch"
  case "$PROFILE" in
  decode)
    if [ -e "$PREFIX/.xberg-heif-encoder-profile" ]; then
      echo "::error::decode profile requires a prefix separate from elixir-encoders" >&2
      return 1
    fi
    ;;
  elixir-encoders)
    if [ "$arch" != arm64 ] || [ "$TARGET" != 15.0 ]; then
      echo "::error::elixir-encoders requires arm64 and MACOSX_DEPLOYMENT_TARGET=15.0" >&2
      return 1
    fi
    if [ -z "${XBERG_HEIF_PREFIX:-}" ]; then
      echo "::error::elixir-encoders requires an explicit XBERG_HEIF_PREFIX" >&2
      return 1
    fi
    receipt="$receipt profile=$PROFILE x265=$X265_VERSION"
    CMAKE_COMMON+=(-DCMAKE_OSX_ARCHITECTURES=arm64)
    ;;
  *)
    echo "::error::unknown XBERG_HEIF_PROFILE: $PROFILE" >&2
    return 1
    ;;
  esac
  if [ -f "$PREFIX/lib/libheif.dylib" ] &&
    [ -f "$PREFIX/.xberg-heif-versions" ] &&
    [ "$(cat "$PREFIX/.xberg-heif-versions")" = "$receipt" ] && verify_versions; then
    verify_prefix
    if [ "$PROFILE" = elixir-encoders ]; then
      codec_notices verify
    fi
    echo "Verified libheif stack already present in $PREFIX, skipping build"
    return 0
  fi
  if [ "$PROFILE" = elixir-encoders ]; then
    mkdir -p "$PREFIX"
    contents="$(ls -A "$PREFIX")"
    if [ -n "$contents" ]; then
      echo "::error::elixir-encoders requires a fresh prefix: $PREFIX" >&2
      return 1
    fi
    touch "$PREFIX/.xberg-heif-encoder-profile"
    WORK="$(mktemp -d "${TMPDIR:-/tmp}/xberg-heif.XXXXXXXX")"
  else
    WORK="$(mktemp -d)"
  fi
  trap 'rm -rf "$WORK"' EXIT
  ensure_nasm
  build_stack
  verify_versions
  verify_prefix
  if [ "$PROFILE" = elixir-encoders ]; then
    codec_notices stage
  fi
  printf '%s\n' "$receipt" >"$PREFIX/.xberg-heif-versions"
  echo "libheif stack built for macOS $TARGET in $PREFIX"
}

main "$@"
